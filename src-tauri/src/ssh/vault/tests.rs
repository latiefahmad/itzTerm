//! Tests for the SSH host vault: sealing, envelope parsing, and merge rules.
//!
//! Deliberately avoids a real keychain and a real `ProfileManager`: the crypto
//! and planning layers are pure, so they carry the coverage. The keychain
//! round trip is already exercised by `credential_store::self_test` at startup.

use super::bundle::{
    b64_encode, seal_params_from_envelope, VaultEnvelope, VaultFormatError, VaultProfile,
};
use super::crypto::{self, SealParams};
use super::plan::{plan_import, ExistingProfile, VaultImportMode};

/// Cheap KDF parameters so the suite stays fast. Production cost is `M_COST`.
fn fast_params() -> SealParams {
    let mut params = SealParams::fresh().expect("fresh params");
    params.m_cost = 32 * 1024;
    params.t_cost = 1;
    params
}

fn envelope_for(params: &SealParams, ciphertext: &[u8], version: u32) -> VaultEnvelope {
    VaultEnvelope {
        format: crypto::FORMAT.to_string(),
        version,
        created_at: "2026-10-28T12:00:00Z".to_string(),
        app: "itzterm".to_string(),
        kdf: super::bundle::KdfParams {
            algo: "argon2id".to_string(),
            m_cost: params.m_cost,
            t_cost: params.t_cost,
            p_cost: params.p_cost,
            salt: b64_encode(&params.salt),
        },
        cipher: super::bundle::CipherParams {
            algo: "aes-256-gcm".to_string(),
            nonce: b64_encode(&params.nonce),
        },
        ciphertext: b64_encode(ciphertext),
    }
}

fn vault_profile(id: &str, host: &str, username: &str, port: u16) -> VaultProfile {
    VaultProfile {
        id: id.to_string(),
        name: host.to_string(),
        host: host.to_string(),
        port,
        username: username.to_string(),
        auth_method: "password".to_string(),
        private_key_path: None,
        jump_host_id: None,
        port_forwards: Vec::new(),
        tags: None,
        group: None,
        favorite: false,
        last_connected: None,
        imported_from: None,
        password: Some("secret".to_string()),
        passphrase: None,
        private_key_material: None,
    }
}

// ---------------------------------------------------------------------------
// Sealing
// ---------------------------------------------------------------------------

#[test]
fn round_trip_restores_plaintext() {
    let params = fast_params();
    let plaintext = br#"{"profiles":[{"id":"a","password":"s3cret"}]}"#;

    let sealed = crypto::seal(plaintext, "correct horse", &params).expect("seal");
    let opened = crypto::open(&sealed, "correct horse", &params).expect("open");

    assert_eq!(opened.as_slice(), plaintext.as_slice());
}

#[test]
fn wrong_passphrase_does_not_open() {
    let params = fast_params();
    let sealed = crypto::seal(b"payload", "right", &params).expect("seal");

    let err = crypto::open(&sealed, "wrong", &params).expect_err("must fail");
    assert!(err.contains("Wrong passphrase"), "unexpected: {}", err);
}

#[test]
fn flipped_ciphertext_byte_fails_authentication() {
    let params = fast_params();
    let mut sealed = crypto::seal(b"payload", "pw", &params).expect("seal");
    sealed[0] ^= 0xaa;

    assert!(crypto::open(&sealed, "pw", &params).is_err());
}

#[test]
fn tampered_cost_parameters_fail_authentication() {
    let params = fast_params();
    let sealed = crypto::seal(b"payload", "pw", &params).expect("seal");

    // Same passphrase, salt and nonce, but the header claims a different cost,
    // so the associated data no longer matches.
    let mut tampered = SealParams::fresh().expect("fresh");
    tampered.salt = params.salt;
    tampered.nonce = params.nonce;
    tampered.m_cost = params.m_cost * 2;
    tampered.t_cost = params.t_cost;

    assert!(crypto::open(&sealed, "pw", &tampered).is_err());
}

#[test]
fn hostile_memory_cost_is_rejected_before_derivation() {
    let mut params = SealParams::fresh().expect("fresh");
    params.m_cost = crypto::MAX_M_COST * 8;

    let err = crypto::seal(b"payload", "pw", &params).expect_err("must refuse");
    assert!(
        err.contains("outside the accepted range"),
        "unexpected: {}",
        err
    );
}

#[test]
fn hostile_pass_count_and_parallelism_are_rejected() {
    let mut too_many_passes = SealParams::fresh().expect("fresh");
    too_many_passes.t_cost = crypto::MAX_T_COST + 1;
    assert!(crypto::seal(b"x", "pw", &too_many_passes).is_err());

    let mut too_parallel = SealParams::fresh().expect("fresh");
    too_parallel.p_cost = crypto::MAX_P_COST + 1;
    assert!(crypto::seal(b"x", "pw", &too_parallel).is_err());
}

#[test]
fn fresh_params_differ_between_calls() {
    let first = SealParams::fresh().expect("fresh");
    let second = SealParams::fresh().expect("fresh");

    assert_ne!(first.salt, second.salt, "salt must be random per vault");
    assert_ne!(first.nonce, second.nonce, "nonce must be random per vault");
}

#[test]
fn associated_data_covers_the_cost_parameters() {
    let base = crypto::associated_data(65536, 3, 1);
    assert_ne!(base, crypto::associated_data(32768, 3, 1));
    assert_ne!(base, crypto::associated_data(65536, 4, 1));
    assert_ne!(base, crypto::associated_data(65536, 3, 2));
}

/// A file this build writes must still be accepted by this build.
#[test]
fn production_parameters_pass_their_own_check() {
    assert!(crypto::check_params(crypto::M_COST, crypto::T_COST, crypto::P_COST).is_ok());
}

// ---------------------------------------------------------------------------
// Envelope parsing
// ---------------------------------------------------------------------------

#[test]
fn envelope_round_trips_through_json() {
    let params = fast_params();
    let sealed = crypto::seal(b"payload", "pw", &params).expect("seal");
    let envelope = envelope_for(&params, &sealed, crypto::VERSION);

    let json = serde_json::to_vec(&envelope).expect("serialize");
    let parsed: VaultEnvelope = serde_json::from_slice(&json).expect("deserialize");

    assert_eq!(parsed.format, crypto::FORMAT);
    assert_eq!(parsed.version, crypto::VERSION);
    assert_eq!(parsed.kdf.m_cost, params.m_cost);
    assert_eq!(parsed.kdf.salt, envelope.kdf.salt);
    assert_eq!(parsed.ciphertext, envelope.ciphertext);
}

#[test]
fn newer_format_version_is_refused() {
    let params = fast_params();
    let envelope = envelope_for(&params, b"sealed", crypto::VERSION + 1);

    let err = seal_params_from_envelope(&envelope).expect_err("must refuse");
    assert_eq!(err, VaultFormatError::UnsupportedVersion(crypto::VERSION + 1));
    assert_eq!(err.code(), "SSH_VAULT_UNSUPPORTED_VERSION");
}

#[test]
fn foreign_format_is_refused() {
    let params = fast_params();
    let mut envelope = envelope_for(&params, b"sealed", crypto::VERSION);
    envelope.format = "some-other-tool".to_string();

    let err = seal_params_from_envelope(&envelope).expect_err("must refuse");
    assert_eq!(err, VaultFormatError::BadFormat);
    assert_eq!(err.code(), "SSH_VAULT_BAD_FORMAT");
}

#[test]
fn unknown_kdf_or_cipher_algo_is_refused() {
    let params = fast_params();

    let mut kdf = envelope_for(&params, b"sealed", crypto::VERSION);
    kdf.kdf.algo = "pbkdf2".to_string();
    assert_eq!(
        seal_params_from_envelope(&kdf).expect_err("must refuse"),
        VaultFormatError::BadFormat
    );

    let mut cipher = envelope_for(&params, b"sealed", crypto::VERSION);
    cipher.cipher.algo = "chacha20poly1305".to_string();
    assert_eq!(
        seal_params_from_envelope(&cipher).expect_err("must refuse"),
        VaultFormatError::BadFormat
    );
}

#[test]
fn hostile_envelope_parameters_are_refused() {
    let params = fast_params();
    let mut envelope = envelope_for(&params, b"sealed", crypto::VERSION);
    envelope.kdf.m_cost = crypto::MAX_M_COST * 32;

    let err = seal_params_from_envelope(&envelope).expect_err("must refuse");
    assert!(matches!(err, VaultFormatError::BadParams(_)));
    assert_eq!(err.code(), "SSH_VAULT_KDF_PARAMS_REJECTED");
}

#[test]
fn malformed_salt_or_nonce_length_is_refused() {
    let params = fast_params();

    let mut short_salt = envelope_for(&params, b"sealed", crypto::VERSION);
    short_salt.kdf.salt = b64_encode(&[0u8; 4]);
    assert_eq!(
        seal_params_from_envelope(&short_salt).expect_err("must refuse"),
        VaultFormatError::BadFormat
    );

    let mut short_nonce = envelope_for(&params, b"sealed", crypto::VERSION);
    short_nonce.cipher.nonce = b64_encode(&[0u8; 4]);
    assert_eq!(
        seal_params_from_envelope(&short_nonce).expect_err("must refuse"),
        VaultFormatError::BadFormat
    );
}

#[test]
fn envelope_parameters_rebuild_a_working_seal() {
    let params = fast_params();
    let sealed = crypto::seal(b"payload", "pw", &params).expect("seal");
    let envelope = envelope_for(&params, &sealed, crypto::VERSION);

    // The reader must recover exactly the parameters the writer used, or a
    // genuine vault would never open.
    let rebuilt = seal_params_from_envelope(&envelope).expect("params");
    let opened = crypto::open(&sealed, "pw", &rebuilt).expect("open");

    assert_eq!(opened.as_slice(), b"payload");
}

// ---------------------------------------------------------------------------
// Import planning
// ---------------------------------------------------------------------------

fn existing(id: &str, host: &str, username: &str, port: u16) -> ExistingProfile {
    ExistingProfile {
        id: id.to_string(),
        host: host.to_string(),
        port,
        username: username.to_string(),
    }
}

#[test]
fn merge_updates_matching_ids_in_place() {
    let existing_rows = [existing("a", "one.example", "root", 22)];
    let incoming = [vault_profile("a", "one.example", "root", 22)];

    let plan = plan_import(&existing_rows, &incoming, VaultImportMode::Merge);

    assert_eq!(plan.to_update.len(), 1);
    assert!(plan.to_create.is_empty());
    assert!(plan.to_skip.is_empty());
    assert!(plan.to_delete.is_empty());
}

#[test]
fn merge_creates_hosts_that_are_new() {
    let existing_rows = [existing("a", "one.example", "root", 22)];
    let incoming = [vault_profile("b", "two.example", "root", 22)];

    let plan = plan_import(&existing_rows, &incoming, VaultImportMode::Merge);

    assert_eq!(plan.to_create.len(), 1);
    assert!(plan.to_update.is_empty());
}

#[test]
fn merge_skips_same_target_under_a_different_id() {
    let existing_rows = [existing("a", "one.example", "root", 22)];
    let incoming = [vault_profile("different-id", "one.example", "root", 22)];

    let plan = plan_import(&existing_rows, &incoming, VaultImportMode::Merge);

    assert_eq!(plan.to_skip.len(), 1);
    assert!(plan.to_create.is_empty(), "must not duplicate a host");
    assert!(plan.to_update.is_empty());
}

#[test]
fn merge_treats_host_case_insensitively_but_username_strictly() {
    let existing_rows = [existing("a", "One.Example", "root", 22)];

    let same_host = plan_import(
        &existing_rows,
        &[vault_profile("x", "one.example", "root", 22)],
        VaultImportMode::Merge,
    );
    assert_eq!(same_host.to_skip.len(), 1, "hostnames are case-insensitive");

    let other_user = plan_import(
        &existing_rows,
        &[vault_profile("y", "one.example", "deploy", 22)],
        VaultImportMode::Merge,
    );
    assert_eq!(
        other_user.to_create.len(),
        1,
        "a different user on the same host is a different connection"
    );
}

#[test]
fn different_port_is_a_different_host() {
    let existing_rows = [existing("a", "one.example", "root", 22)];
    let incoming = [vault_profile("b", "one.example", "root", 2222)];

    let plan = plan_import(&existing_rows, &incoming, VaultImportMode::Merge);

    assert_eq!(plan.to_create.len(), 1);
}

#[test]
fn replace_inserts_even_when_the_target_already_exists() {
    let existing_rows = [existing("a", "one.example", "root", 22)];
    let incoming = [vault_profile("b", "one.example", "root", 22)];

    let plan = plan_import(&existing_rows, &incoming, VaultImportMode::Replace);

    assert_eq!(plan.to_create.len(), 1);
    assert!(plan.to_skip.is_empty());
    assert_eq!(plan.to_delete, vec!["a".to_string()]);
}

#[test]
fn replace_keeps_hosts_the_vault_also_contains() {
    let existing_rows = [
        existing("a", "one.example", "root", 22),
        existing("b", "two.example", "root", 22),
    ];
    let incoming = [vault_profile("a", "one.example", "root", 22)];

    let plan = plan_import(&existing_rows, &incoming, VaultImportMode::Replace);

    assert_eq!(plan.to_update.len(), 1);
    assert_eq!(plan.to_delete, vec!["b".to_string()]);
}

#[test]
fn merge_never_deletes_existing_hosts() {
    let existing_rows = [
        existing("a", "one.example", "root", 22),
        existing("b", "two.example", "root", 22),
    ];
    let incoming = [vault_profile("a", "one.example", "root", 22)];

    let plan = plan_import(&existing_rows, &incoming, VaultImportMode::Merge);

    assert!(plan.to_delete.is_empty());
}

#[test]
fn reimporting_the_same_vault_is_idempotent() {
    let incoming = [
        vault_profile("a", "one.example", "root", 22),
        vault_profile("b", "two.example", "root", 22),
    ];

    let first = plan_import(&[], &incoming, VaultImportMode::Merge);
    assert_eq!(first.to_create.len(), 2);

    // Second run: the rows from the first run are now the existing set.
    let existing_rows: Vec<ExistingProfile> = incoming
        .iter()
        .map(|p| ExistingProfile {
            id: p.id.clone(),
            host: p.host.clone(),
            port: p.port,
            username: p.username.clone(),
        })
        .collect();

    let second = plan_import(&existing_rows, &incoming, VaultImportMode::Merge);
    assert!(second.to_create.is_empty(), "must not duplicate on re-import");
    assert_eq!(second.to_update.len(), 2);
}

#[test]
fn plan_accounts_for_every_incoming_profile() {
    let existing_rows = [
        existing("a", "one.example", "root", 22),
        existing("b", "two.example", "root", 22),
        existing("c", "three.example", "root", 22),
    ];
    let incoming = [
        vault_profile("a", "one.example", "root", 22),   // update
        vault_profile("b", "two.example", "root", 22),   // update
        vault_profile("new", "four.example", "root", 22), // create
        vault_profile("clash", "three.example", "root", 22), // skip
    ];

    let plan = plan_import(&existing_rows, &incoming, VaultImportMode::Merge);

    assert_eq!(plan.accounted_for(), incoming.len());
    assert_eq!(plan.to_update.len(), 2);
    assert_eq!(plan.to_create.len(), 1);
    assert_eq!(plan.to_skip.len(), 1);
}

#[test]
fn carries_secret_ignores_empty_strings() {
    let mut profile = vault_profile("a", "one.example", "root", 22);
    assert!(profile.carries_password());

    profile.password = Some(String::new());
    assert!(!profile.carries_password(), "an empty secret is not a secret");

    profile.password = None;
    assert!(!profile.carries_password());
}

#[test]
fn carried_key_material_is_reported_separately_from_paths() {
    let mut profile = vault_profile("a", "one.example", "root", 22);
    profile.auth_method = "key".to_string();
    assert!(!profile.carries_key_material(), "no material yet");

    profile.private_key_material = Some("-----BEGIN OPENSSH PRIVATE KEY-----\n...\n".to_string());
    assert!(profile.carries_key_material());

    profile.private_key_material = Some("   ".to_string());
    assert!(
        !profile.carries_key_material(),
        "whitespace-only material is not a key"
    );
}

#[test]
fn key_material_is_omitted_when_absent_and_round_trips_when_present() {
    let with_material = {
        let mut p = vault_profile("a", "one.example", "root", 22);
        p.private_key_material = Some("KEYDATA".to_string());
        p
    };
    let without_material = vault_profile("b", "two.example", "root", 22);

    let payload = super::bundle::VaultPayload {
        profiles: vec![with_material, without_material],
    };

    let json = serde_json::to_string(&payload).expect("serialize");
    // Absent must be truly absent (not null), so a vault written by this build
    // stays small and older readers are not confused.
    assert!(!json.contains("privateKeyMaterial\":null"));
    assert!(json.contains("privateKeyMaterial\":\"KEYDATA\""));

    let parsed: super::bundle::VaultPayload = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(parsed.profiles[0].private_key_material.as_deref(), Some("KEYDATA"));
    assert_eq!(parsed.profiles[1].private_key_material, None);
}
