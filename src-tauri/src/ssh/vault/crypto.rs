//! Sealed envelope for the SSH host vault: Argon2id key derivation over a
//! user passphrase, then AES-256-GCM over the serialized payload.
//!
//! The envelope header is bound into the AEAD's associated data, so a tampered
//! or downgraded KDF/cipher parameter fails authentication instead of being
//! silently accepted. Nothing here touches the filesystem or the keychain.
//!
//! KDF parameters are read back from the file, so an imported vault controls
//! how much memory and CPU it costs to open. [`check_params`] caps them before
//! any derivation happens.

use aes_gcm::aead::{Aead, KeyInit, Payload};
use aes_gcm::{Aes256Gcm, Nonce};
use argon2::{Algorithm, Argon2, Params, Version};
use zeroize::Zeroizing;

/// Envelope format identifier. Bumping this is a breaking format change and
/// must be paired with a `version` bump.
pub(crate) const FORMAT: &str = "itzterm-ssh-vault";
/// Envelope schema version this build writes and the newest it accepts.
pub(crate) const VERSION: u32 = 1;

/// Argon2id cost used for new vaults: 64 MiB, 3 passes, 1 lane.
pub(crate) const M_COST: u32 = 65536;
pub(crate) const T_COST: u32 = 3;
pub(crate) const P_COST: u32 = 1;

/// A vault this build will open.
///
/// Caps mirror the intent of `atomic_file::NewerSchema`: refuse plainly rather
/// than guess. The KDF caps are a denial-of-service guard — without them a
/// hostile file could ask for gigabytes of RAM during import.
pub(crate) const MAX_M_COST: u32 = 1_048_576; // 1 GiB
pub(crate) const MAX_T_COST: u32 = 10;
pub(crate) const MAX_P_COST: u32 = 4;
pub(crate) const SALT_LEN: usize = 16;
pub(crate) const NONCE_LEN: usize = 12;
pub(crate) const KEY_LEN: usize = 32;
/// Ciphertext ceiling (~16 MiB). A host list is kilobytes.
pub(crate) const MAX_CIPHERTEXT_LEN: usize = 16 * 1024 * 1024;
/// Minimum passphrase length enforced for new vaults. Import does not enforce
/// this — an older file may have been written under a weaker rule.
pub(crate) const MIN_PASSPHRASE_LEN: usize = 8;

/// Everything needed to derive the key and open the ciphertext.
pub(crate) struct SealParams {
    pub salt: [u8; SALT_LEN],
    pub nonce: [u8; NONCE_LEN],
    pub m_cost: u32,
    pub t_cost: u32,
    pub p_cost: u32,
}

/// Costs are useful in a failure message; salt and nonce are key material and
/// stay out of logs, matching how `SSHProfile` redacts its secrets.
impl std::fmt::Debug for SealParams {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SealParams")
            .field("m_cost", &self.m_cost)
            .field("t_cost", &self.t_cost)
            .field("p_cost", &self.p_cost)
            .field("salt", &"<redacted>")
            .field("nonce", &"<redacted>")
            .finish()
    }
}

impl SealParams {
    /// Parameters for a new vault, with fresh random salt and nonce.
    pub(crate) fn fresh() -> Result<Self, String> {
        let mut salt = [0u8; SALT_LEN];
        let mut nonce = [0u8; NONCE_LEN];
        getrandom::getrandom(&mut salt).map_err(|_| "OS CSPRNG is unavailable".to_string())?;
        getrandom::getrandom(&mut nonce).map_err(|_| "OS CSPRNG is unavailable".to_string())?;
        Ok(Self {
            salt,
            nonce,
            m_cost: M_COST,
            t_cost: T_COST,
            p_cost: P_COST,
        })
    }
}

/// Associated data binding the header to the ciphertext.
///
/// Only the fields that decide how the ciphertext is interpreted are included.
/// `createdAt` and `app` are informational and deliberately excluded so a
/// cosmetic edit does not invalidate the file.
pub(crate) fn associated_data(m_cost: u32, t_cost: u32, p_cost: u32) -> Vec<u8> {
    format!(
        "{}|{}|argon2id|{}|{}|{}|aes-256-gcm",
        FORMAT, VERSION, m_cost, t_cost, p_cost
    )
    .into_bytes()
}

/// Reject parameters that would make opening the file unreasonably expensive
/// or that this build cannot interpret.
pub(crate) fn check_params(m_cost: u32, t_cost: u32, p_cost: u32) -> Result<(), String> {
    if m_cost < Params::MIN_M_COST || m_cost > MAX_M_COST {
        return Err(format!(
            "Vault KDF memory cost {} KiB is outside the accepted range ({}..={} KiB)",
            m_cost,
            Params::MIN_M_COST,
            MAX_M_COST
        ));
    }
    if t_cost < 1 || t_cost > MAX_T_COST {
        return Err(format!(
            "Vault KDF pass count {} is outside the accepted range (1..={})",
            t_cost, MAX_T_COST
        ));
    }
    if p_cost < 1 || p_cost > MAX_P_COST {
        return Err(format!(
            "Vault KDF parallelism {} is outside the accepted range (1..={})",
            p_cost, MAX_P_COST
        ));
    }
    Ok(())
}

/// Derive the 32-byte AES key from the passphrase. Blocking: Argon2id at the
/// default cost takes hundreds of milliseconds, so call this from a blocking
/// task rather than the async runtime worker.
fn derive_key(passphrase: &str, params: &SealParams) -> Result<Zeroizing<[u8; KEY_LEN]>, String> {
    let argon_params = Params::new(params.m_cost, params.t_cost, params.p_cost, Some(KEY_LEN))
        .map_err(|e| format!("Invalid KDF parameters: {}", e))?;
    let argon = Argon2::new(Algorithm::Argon2id, Version::V0x13, argon_params);
    let mut key = Zeroizing::new([0u8; KEY_LEN]);
    argon
        .hash_password_into(passphrase.as_bytes(), &params.salt, key.as_mut())
        .map_err(|e| format!("Key derivation failed: {}", e))?;
    Ok(key)
}

/// Encrypt `plaintext` under `passphrase`, returning the raw ciphertext
/// (ciphertext bytes followed by the 16-byte GCM tag).
pub(crate) fn seal(
    plaintext: &[u8],
    passphrase: &str,
    params: &SealParams,
) -> Result<Vec<u8>, String> {
    check_params(params.m_cost, params.t_cost, params.p_cost)?;
    let key = derive_key(passphrase, params)?;
    let cipher = Aes256Gcm::new_from_slice(key.as_ref())
        .map_err(|_| "Could not initialize the cipher".to_string())?;
    let nonce = Nonce::from(params.nonce);
    cipher
        .encrypt(
            &nonce,
            Payload {
                msg: plaintext,
                aad: &associated_data(params.m_cost, params.t_cost, params.p_cost),
            },
        )
        .map_err(|_| "Could not encrypt the vault".to_string())
}

/// Decrypt raw ciphertext. A wrong passphrase and a damaged file are
/// indistinguishable here, so both surface as one message.
pub(crate) fn open(
    ciphertext: &[u8],
    passphrase: &str,
    params: &SealParams,
) -> Result<Zeroizing<Vec<u8>>, String> {
    check_params(params.m_cost, params.t_cost, params.p_cost)?;
    if ciphertext.len() > MAX_CIPHERTEXT_LEN {
        return Err("Vault ciphertext is too large".to_string());
    }
    let key = derive_key(passphrase, params)?;
    let cipher = Aes256Gcm::new_from_slice(key.as_ref())
        .map_err(|_| "Could not initialize the cipher".to_string())?;
    let nonce = Nonce::from(params.nonce);
    let plaintext = cipher
        .decrypt(
            &nonce,
            Payload {
                msg: ciphertext,
                aad: &associated_data(params.m_cost, params.t_cost, params.p_cost),
            },
        )
        .map_err(|_| "Wrong passphrase, or the vault file is damaged".to_string())?;
    Ok(Zeroizing::new(plaintext))
}

