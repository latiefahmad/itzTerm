//! Passphrase-encrypted vault of saved SSH hosts, for moving a host list (and
//! its keychain credentials) between machines.
//!
//! Desktop-only. The browser client has no OS keychain, so exporting
//! credentials there is impossible; the renderer gates this with
//! `isTauriContext()` and the commands are refused outside Tauri.
//!
//! Layout: [`bundle`] holds the serde types, [`crypto`] the Argon2id + AES-GCM
//! sealing, [`plan`] the pure merge rules. This module joins them to the
//! profile manager and the filesystem.
//!
//! Private key material travels inside the backup for key-auth hosts whose key
//! file is readable, so the host keeps working on a machine that never had it.
//! Keys that cannot be read are reported per host — the user needs to know
//! before relying on the file on another one.

pub(crate) mod bundle;
pub(crate) mod crypto;
pub(crate) mod plan;

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::ssh::keys::{self, KeyFileStatus};
use crate::ssh::profile_manager::ProfileManager;
use crate::ssh::SSHManager;

pub use plan::{plan_import, ExistingProfile, ImportPlan, VaultImportMode};

use bundle::{
    b64_encode, seal_params_from_envelope, VaultEnvelope, VaultFormatError, VaultPayload,
    VaultProfile,
};
use crypto::SealParams;

/// Upper bound on a vault file we will read into memory. Generous for a host
/// list, small enough that a mistaken (or hostile) path cannot exhaust RAM.
const MAX_VAULT_FILE_BYTES: u64 = 24 * 1024 * 1024;

// ---------------------------------------------------------------------------
// IPC-facing request and summary types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportVaultRequest {
    /// Destination path chosen by the user in a save dialog.
    pub path: String,
    pub passphrase: String,
    /// `None` exports every profile.
    #[serde(default)]
    pub profile_ids: Option<Vec<String>>,
    /// Do everything except write the file. Lets the UI show what will be
    /// included, and which keys are missing, before anything is created.
    #[serde(default)]
    pub dry_run: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportVaultRequest {
    pub path: String,
    pub passphrase: String,
    #[serde(default)]
    pub mode: VaultImportMode,
}

/// A key path that will not work on the importing machine.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct VaultKeyWarning {
    pub profile_id: String,
    pub name: String,
    pub path: String,
    /// `notFound` | `unreadable` | `notPrivateKey`
    pub status: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportVaultSummary {
    pub profile_count: usize,
    pub with_password: usize,
    pub with_passphrase: usize,
    /// Hosts whose private key travelled inside the backup.
    pub with_key_material: usize,
    pub missing_keys: Vec<VaultKeyWarning>,
    /// Profiles whose keychain read failed even though the profile claimed a
    /// stored secret. Exported without it — surfaced, never silent.
    pub unreadable_secrets: Vec<String>,
    /// `None` when this was a dry run.
    pub bytes_written: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportVaultSummary {
    pub created: usize,
    pub updated: usize,
    pub skipped: usize,
    pub deleted: usize,
    pub credentials_restored: usize,
    /// Hosts whose private key was written to this machine's managed key dir.
    pub keys_restored: usize,
    pub missing_keys: Vec<VaultKeyWarning>,
}

// ---------------------------------------------------------------------------
// Export
// ---------------------------------------------------------------------------

/// Build the export payload and, unless this is a dry run, write the sealed
/// file. Blocking: Argon2id dominates the runtime here, so the command wraps
/// this in `spawn_blocking`.
pub fn export(
    manager: &ProfileManager,
    request: &ExportVaultRequest,
) -> Result<ExportVaultSummary, (String, &'static str)> {
    if request.passphrase.chars().count() < crypto::MIN_PASSPHRASE_LEN {
        return Err((
            format!(
                "Choose a passphrase of at least {} characters",
                crypto::MIN_PASSPHRASE_LEN
            ),
            "SSH_VAULT_PASSPHRASE_TOO_SHORT",
        ));
    }

    let all = manager
        .list()
        .map_err(|e| (e, "SSH_VAULT_EXPORT_FAILED"))?;

    let selected: Vec<_> = match &request.profile_ids {
        Some(ids) => all
            .into_iter()
            .filter(|p| ids.iter().any(|id| id == &p.id))
            .collect(),
        None => all,
    };

    if selected.is_empty() {
        return Err((
            "There are no hosts to export".to_string(),
            "SSH_VAULT_EMPTY",
        ));
    }

    let mut summary = ExportVaultSummary {
        profile_count: 0,
        with_password: 0,
        with_passphrase: 0,
        with_key_material: 0,
        missing_keys: Vec::new(),
        unreadable_secrets: Vec::new(),
        bytes_written: None,
    };

    let mut payload = VaultPayload {
        profiles: Vec::with_capacity(selected.len()),
    };

    for profile in selected {
        // Only ask the keychain for a secret the profile says it has: an
        // unconditional read would produce a stream of warnings for every
        // agent-auth host.
        let password = if profile.has_stored_password {
            match crate::ssh::credential_store::get_password(&profile.id) {
                Ok(value) => value,
                Err(e) => {
                    log::warn!(
                        "[SSH vault] Could not read password for profile {}: {}",
                        profile.id,
                        e
                    );
                    summary.unreadable_secrets.push(profile.id.clone());
                    None
                }
            }
        } else {
            None
        };

        let passphrase = if profile.has_stored_passphrase {
            match crate::ssh::credential_store::get_passphrase(&profile.id) {
                Ok(value) => value,
                Err(e) => {
                    log::warn!(
                        "[SSH vault] Could not read passphrase for profile {}: {}",
                        profile.id,
                        e
                    );
                    summary.unreadable_secrets.push(profile.id.clone());
                    None
                }
            }
        } else {
            None
        };

        if profile.auth_method == "key" {
            if let Some(warning) = key_warning(&profile) {
                summary.missing_keys.push(warning);
            }
        }

        // Read the key only after `key_warning` has had its say, so the two
        // stay consistent: green here means the warning list did not mention
        // this host.
        let private_key_material = if profile.auth_method == "key" {
            profile
                .private_key_path
                .as_deref()
                .and_then(keys::read_material)
        } else {
            None
        };

        let vault_profile = VaultProfile {
            id: profile.id,
            name: profile.name,
            host: profile.host,
            port: profile.port,
            username: profile.username,
            auth_method: profile.auth_method,
            private_key_path: profile.private_key_path,
            jump_host_id: profile.jump_host_id,
            port_forwards: profile
                .port_forwards
                .into_iter()
                .filter_map(|f| serde_json::to_value(f).ok())
                .collect(),
            tags: profile.tags,
            group: profile.group,
            favorite: profile.favorite,
            last_connected: profile.last_connected,
            imported_from: profile.imported_from,
            password,
            passphrase,
            private_key_material,
        };

        summary.profile_count += 1;
        if vault_profile.carries_password() {
            summary.with_password += 1;
        }
        if vault_profile.carries_passphrase() {
            summary.with_passphrase += 1;
        }
        if vault_profile.carries_key_material() {
            summary.with_key_material += 1;
        }
        payload.profiles.push(vault_profile);
    }

    if request.dry_run {
        return Ok(summary);
    }

    let plaintext = serde_json::to_vec(&payload)
        .map_err(|e| (format!("Could not encode the vault: {}", e), "SSH_VAULT_EXPORT_FAILED"))?;

    let params = SealParams::fresh().map_err(|e| (e, "SSH_VAULT_EXPORT_FAILED"))?;
    let ciphertext = crypto::seal(&plaintext, &request.passphrase, &params)
        .map_err(|e| (e, "SSH_VAULT_EXPORT_FAILED"))?;

    let envelope = VaultEnvelope {
        format: crypto::FORMAT.to_string(),
        version: crypto::VERSION,
        created_at: chrono::Utc::now().to_rfc3339(),
        app: "itzterm".to_string(),
        kdf: bundle::KdfParams {
            algo: "argon2id".to_string(),
            m_cost: params.m_cost,
            t_cost: params.t_cost,
            p_cost: params.p_cost,
            salt: b64_encode(&params.salt),
        },
        cipher: bundle::CipherParams {
            algo: "aes-256-gcm".to_string(),
            nonce: b64_encode(&params.nonce),
        },
        ciphertext: b64_encode(&ciphertext),
    };

    let bytes = serde_json::to_vec_pretty(&envelope)
        .map_err(|e| (format!("Could not encode the vault: {}", e), "SSH_VAULT_EXPORT_FAILED"))?;

    write_private_file(Path::new(&request.path), &bytes)
        .map_err(|e| (format!("Could not write the vault file: {}", e), "SSH_VAULT_WRITE_FAILED"))?;

    summary.bytes_written = Some(bytes.len() as u64);
    log::info!(
        "[SSH vault] Exported {} hosts ({} passwords, {} passphrases, {} keys embedded, {} keys missing) to a {} byte file",
        summary.profile_count,
        summary.with_password,
        summary.with_passphrase,
        summary.with_key_material,
        summary.missing_keys.len(),
        bytes.len()
    );

    Ok(summary)
}

/// Write a file readable only by the current user.
///
/// Deliberately not `atomic_file::replace`: that creates files 0644, which is
/// wrong for a container that decrypts to every saved password. Matches the
/// owner-only creation used for OAuth token files.
fn write_private_file(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    use std::io::Write;

    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }

    let mut options = std::fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }

    let mut file = options.open(path)?;
    file.write_all(bytes)?;
    file.flush()?;
    file.sync_all()?;
    Ok(())
}

/// Report a key that will not resolve here. Agent auth and keyless profiles are
/// not warned about — they are intentional, not broken.
fn key_warning(profile: &crate::ssh::profile_manager::SSHProfile) -> Option<VaultKeyWarning> {
    let path = profile.private_key_path.as_deref().unwrap_or("").trim();
    let info = keys::inspect(path);
    let status = match info.status {
        KeyFileStatus::Ok => return None,
        KeyFileStatus::NotFound => "notFound",
        KeyFileStatus::Unreadable => "unreadable",
        KeyFileStatus::NotPrivateKey => "notPrivateKey",
    };
    Some(VaultKeyWarning {
        profile_id: profile.id.clone(),
        name: profile.name.clone(),
        path: path.to_string(),
        status: status.to_string(),
    })
}

// ---------------------------------------------------------------------------
// Import
// ---------------------------------------------------------------------------

/// Read, decrypt and apply a vault. Blocking for the same reason as [`export`].
pub fn import(
    ssh_manager: &SSHManager,
    request: &ImportVaultRequest,
) -> Result<ImportVaultSummary, (String, &'static str)> {
    let bytes = read_capped(Path::new(&request.path)).map_err(|e| {
        (
            format!("Could not read the vault file: {}", e),
            "SSH_VAULT_READ_FAILED",
        )
    })?;

    let envelope: VaultEnvelope = serde_json::from_slice(&bytes).map_err(|_| {
        (
            VaultFormatError::BadFormat.to_string(),
            VaultFormatError::BadFormat.code(),
        )
    })?;

    let params = seal_params_from_envelope(&envelope)
        .map_err(|e| (e.to_string(), e.code()))?;

    let ciphertext = bundle::b64_decode(&envelope.ciphertext)
        .map_err(|e| (e, "SSH_VAULT_BAD_FORMAT"))?;

    let plaintext = crypto::open(&ciphertext, &request.passphrase, &params).map_err(|e| {
        let err = VaultFormatError::Decrypt(e);
        (err.to_string(), err.code())
    })?;

    let payload: VaultPayload = serde_json::from_slice(&plaintext).map_err(|_| {
        (
            "The vault contents could not be read".to_string(),
            "SSH_VAULT_BAD_FORMAT",
        )
    })?;

    let existing: Vec<ExistingProfile> = ssh_manager
        .profiles
        .list()
        .map_err(|e| (e, "SSH_VAULT_IMPORT_FAILED"))?
        .into_iter()
        .map(|p| ExistingProfile {
            id: p.id,
            host: p.host,
            port: p.port,
            username: p.username,
        })
        .collect();

    let mut plan = plan_import(&existing, &payload.profiles, request.mode);

    // Write carried keys before the profile rows, because a row must point at
    // a key that already exists — otherwise the host list briefly shows hosts
    // that cannot connect.
    let mut keys_restored = 0usize;
    for profile in plan.to_create.iter_mut().chain(plan.to_update.iter_mut()) {
        if !profile.carries_key_material() {
            continue;
        }
        // Cloned out so the path can be reassigned below; the key is only a few
        // kilobytes and this loop runs once per imported key-auth host.
        let material = profile.private_key_material.clone().unwrap_or_default();

        // A key path that already works on this machine wins: the user may
        // have their own copy in a better location, and re-importing a backup
        // must not quietly repoint it into the managed directory.
        if keys::inspect(profile.private_key_path.as_deref().unwrap_or("")).status
            == KeyFileStatus::Ok
        {
            continue;
        }

        match write_managed_key(&profile.id, &material) {
            Ok(path) => {
                profile.private_key_path = Some(path);
                keys_restored += 1;
            }
            Err(e) => {
                // The host still imports; it just needs its key path fixed
                // afterwards. Reported through `missing_keys` below.
                log::warn!(
                    "[SSH vault] Could not write the imported key for profile {}: {}",
                    profile.id,
                    e
                );
            }
        }
    }

    debug_assert_eq!(plan.accounted_for(), payload.profiles.len());

    let touched: Vec<String> = plan
        .to_create
        .iter()
        .chain(plan.to_update.iter())
        .map(|p| p.id.clone())
        .chain(plan.to_delete.iter().cloned())
        .collect();

    let missing_keys = plan
        .to_create
        .iter()
        .chain(plan.to_update.iter())
        .filter_map(|p| vault_key_warning(p))
        .collect::<Vec<_>>();

    let mut credentials_restored = 0usize;
    for profile in plan.to_create.iter().chain(plan.to_update.iter()) {
        if profile.carries_password() || profile.carries_passphrase() {
            credentials_restored += 1;
        }
    }

    ssh_manager
        .profiles
        .apply_import(&plan)
        .map_err(|e| (e, "SSH_VAULT_IMPORT_FAILED"))?;

    // A session may still hold the pre-import secret in process memory for
    // automatic reconnect. Mirrors `ssh_save_profile`.
    for profile_id in &touched {
        ssh_manager.sessions.forget_secret(profile_id);
    }

    log::info!(
        "[SSH vault] Imported: {} created, {} updated, {} skipped, {} deleted, {} credentials restored",
        plan.to_create.len(),
        plan.to_update.len(),
        plan.to_skip.len(),
        plan.to_delete.len(),
        credentials_restored
    );

    Ok(ImportVaultSummary {
        created: plan.to_create.len(),
        updated: plan.to_update.len(),
        skipped: plan.to_skip.len(),
        deleted: plan.to_delete.len(),
        credentials_restored,
        keys_restored,
        missing_keys,
    })
}

fn vault_key_warning(profile: &VaultProfile) -> Option<VaultKeyWarning> {
    if profile.auth_method != "key" {
        return None;
    }
    let path = profile.private_key_path.as_deref().unwrap_or("").trim();
    let info = keys::inspect(path);
    let status = match info.status {
        KeyFileStatus::Ok => return None,
        KeyFileStatus::NotFound => "notFound",
        KeyFileStatus::Unreadable => "unreadable",
        KeyFileStatus::NotPrivateKey => "notPrivateKey",
    };
    Some(VaultKeyWarning {
        profile_id: profile.id.clone(),
        name: profile.name.clone(),
        path: path.to_string(),
        status: status.to_string(),
    })
}

/// Directory under `~/.ssh` where imported keys are written. Kept apart from
/// the user's own keys so an import never overwrites one.
const MANAGED_KEY_DIR: &str = "itzterm";

/// Write an imported private key into the managed key directory and return its
/// path.
///
/// The store keeps only a path, and the connect pipeline reads the key from
/// that path — so a key that travels inside the backup is useless until it has
/// been written somewhere. `~/.ssh` is the natural home: it is where a user's
/// keys already live, and on Unix a `0600` file there is exactly what the SSH
/// tooling expects.
///
/// The file is named after the profile id, which is unique and stable, so
/// re-importing the same backup overwrites in place instead of accumulating
/// copies.
fn write_managed_key(profile_id: &str, material: &str) -> Result<String, String> {
    let dir = super::config_parser::ssh_dir()
        .ok_or_else(|| "Could not find the home directory".to_string())?
        .join(MANAGED_KEY_DIR);
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("Could not create the key directory: {}", e))?;

    // Only the id, and only its safe characters: a profile id never reaches
    // the filesystem verbatim (no separators, no traversal).
    let safe_id: String = profile_id
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .collect();
    if safe_id.is_empty() {
        return Err("This host has no usable id for its key file".to_string());
    }

    let path = dir.join(format!("{}.key", safe_id));
    write_private_file(&path, material.as_bytes())
        .map_err(|e| format!("Could not write the private key: {}", e))?;

    Ok(path.to_string_lossy().to_string())
}

fn read_capped(path: &Path) -> Result<Vec<u8>, String> {
    let metadata = std::fs::metadata(path).map_err(|e| e.to_string())?;
    if !metadata.is_file() {
        return Err("That path is not a file".to_string());
    }
    if metadata.len() > MAX_VAULT_FILE_BYTES {
        return Err("That file is too large to be a host vault".to_string());
    }
    std::fs::read(path).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests;
