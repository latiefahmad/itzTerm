//! Serde types for the vault file: the plaintext envelope header and the
//! encrypted payload. Pure data — no I/O, no keychain, no Tauri.
//!
//! The header is deliberately readable: a support request can state the format
//! version and KDF cost without revealing a single hostname. Everything
//! identifying a host lives inside the ciphertext.

use serde::{Deserialize, Serialize};

use super::crypto::{self, SealParams};

/// On-disk envelope. Field order matches what `encode` writes, but parsing is
/// name-based so order is not load-bearing.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct VaultEnvelope {
    pub format: String,
    pub version: u32,
    pub created_at: String,
    pub app: String,
    pub kdf: KdfParams,
    pub cipher: CipherParams,
    /// Base64 (standard alphabet) of the AES-GCM ciphertext plus tag.
    pub ciphertext: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct KdfParams {
    pub algo: String,
    pub m_cost: u32,
    pub t_cost: u32,
    pub p_cost: u32,
    /// Base64 of the Argon2 salt.
    pub salt: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CipherParams {
    pub algo: String,
    /// Base64 of the AES-GCM nonce.
    pub nonce: String,
}

/// What the ciphertext decrypts to.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct VaultPayload {
    pub profiles: Vec<VaultProfile>,
}

/// One host, including its keychain secrets when they were readable.
///
/// `password` and `passphrase` are `None` when the secret does not apply to the
/// auth method, was never stored, or could not be read from the keychain during
/// export. The export summary reports the last case separately — a silently
/// credential-less profile is exactly the failure this feature must not have.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct VaultProfile {
    pub id: String,
    pub name: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub auth_method: String,
    #[serde(default)]
    pub private_key_path: Option<String>,
    #[serde(default)]
    pub jump_host_id: Option<String>,
    #[serde(default)]
    pub port_forwards: Vec<serde_json::Value>,
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    #[serde(default)]
    pub group: Option<String>,
    #[serde(default)]
    pub favorite: bool,
    #[serde(default)]
    pub last_connected: Option<String>,
    #[serde(default)]
    pub imported_from: Option<String>,
    #[serde(default)]
    pub password: Option<String>,
    #[serde(default)]
    pub passphrase: Option<String>,
    /// The private key file's contents, so a key-auth host keeps working on a
    /// machine that never had the file. `None` when the key was missing,
    /// unreadable, or in a format itzTerm cannot load — the export summary says
    /// which, per host.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub private_key_material: Option<String>,
}

/// Prints the identifying fields only: the password, passphrase and key
/// material are secrets and must never reach a log line or a panic message.
impl std::fmt::Debug for VaultProfile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VaultProfile")
            .field("id", &self.id)
            .field("host", &self.host)
            .field("port", &self.port)
            .field("username", &self.username)
            .field("auth_method", &self.auth_method)
            .field("private_key_path", &self.private_key_path)
            .field("password", &self.password.as_ref().map(|_| "<redacted>"))
            .field("passphrase", &self.passphrase.as_ref().map(|_| "<redacted>"))
            .field(
                "private_key_material",
                &self.private_key_material.as_ref().map(|_| "<redacted>"),
            )
            .finish()
    }
}

impl VaultProfile {
    /// A profile is only useful after import if it can authenticate: password
    /// auth needs a stored password, key auth needs a readable key file or a
    /// stored passphrase (or an unencrypted key), agent auth needs nothing.
    pub(crate) fn carries_password(&self) -> bool {
        self.password.as_deref().is_some_and(|p| !p.is_empty())
    }

    pub(crate) fn carries_passphrase(&self) -> bool {
        self.passphrase.as_deref().is_some_and(|p| !p.is_empty())
    }

    /// The profile travels with its key file, so import can write it locally.
    pub(crate) fn carries_key_material(&self) -> bool {
        self.private_key_material
            .as_deref()
            .is_some_and(|k| !k.trim().is_empty())
    }
}

/// Decode a base64 string using the standard alphabet.
pub(crate) fn b64_decode(value: &str) -> Result<Vec<u8>, String> {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD
        .decode(value)
        .map_err(|_| "Vault file is damaged: invalid base64".to_string())
}

pub(crate) fn b64_encode(bytes: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

/// Rebuild [`SealParams`] from the header, validating every field first.
///
/// Order matters: format and version are checked before the KDF caps, and the
/// caps before the salt/nonce lengths, so a hostile file is refused on the
/// cheapest possible check.
pub(crate) fn seal_params_from_envelope(
    envelope: &VaultEnvelope,
) -> Result<SealParams, VaultFormatError> {
    if envelope.format != crypto::FORMAT {
        return Err(VaultFormatError::BadFormat);
    }
    if envelope.version > crypto::VERSION {
        return Err(VaultFormatError::UnsupportedVersion(envelope.version));
    }
    if envelope.kdf.algo != "argon2id" {
        return Err(VaultFormatError::BadFormat);
    }
    if envelope.cipher.algo != "aes-256-gcm" {
        return Err(VaultFormatError::BadFormat);
    }
    crypto::check_params(envelope.kdf.m_cost, envelope.kdf.t_cost, envelope.kdf.p_cost)
        .map_err(VaultFormatError::BadParams)?;

    let salt = b64_decode(&envelope.kdf.salt).map_err(|_| VaultFormatError::BadFormat)?;
    let nonce = b64_decode(&envelope.cipher.nonce).map_err(|_| VaultFormatError::BadFormat)?;
    let salt: [u8; crypto::SALT_LEN] = salt
        .try_into()
        .map_err(|_| VaultFormatError::BadFormat)?;
    let nonce: [u8; crypto::NONCE_LEN] = nonce
        .try_into()
        .map_err(|_| VaultFormatError::BadFormat)?;

    Ok(SealParams {
        salt,
        nonce,
        m_cost: envelope.kdf.m_cost,
        t_cost: envelope.kdf.t_cost,
        p_cost: envelope.kdf.p_cost,
    })
}

/// Why a vault file could not be read. Each variant maps to one IPC error code
/// so the UI can say something specific.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum VaultFormatError {
    BadFormat,
    UnsupportedVersion(u32),
    BadParams(String),
    Decrypt(String),
}

impl VaultFormatError {
    /// IPC error code, matching the `SSH_VAULT_*` family.
    pub(crate) fn code(&self) -> &'static str {
        match self {
            Self::BadFormat => "SSH_VAULT_BAD_FORMAT",
            Self::UnsupportedVersion(_) => "SSH_VAULT_UNSUPPORTED_VERSION",
            Self::BadParams(_) => "SSH_VAULT_KDF_PARAMS_REJECTED",
            Self::Decrypt(_) => "SSH_VAULT_DECRYPT_FAILED",
        }
    }
}

impl std::fmt::Display for VaultFormatError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BadFormat => f.write_str("This file is not an itzTerm host vault"),
            Self::UnsupportedVersion(found) => write!(
                f,
                "This vault was written by a newer itzTerm (format version {}; this build supports {}). Update itzTerm to open it.",
                found,
                crypto::VERSION
            ),
            Self::BadParams(detail) => write!(f, "{}", detail),
            Self::Decrypt(detail) => write!(f, "{}", detail),
        }
    }
}
