//! Import planning: decide what an incoming vault does to the profiles that
//! already exist. Pure — no keychain, no filesystem, no Tauri — so every merge
//! rule is unit-testable without a running app.

use serde::{Deserialize, Serialize};

use super::bundle::VaultProfile;

/// How an import treats hosts that already exist.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum VaultImportMode {
    /// Keep existing hosts the vault does not mention; skip vault hosts whose
    /// `host + username + port` already exists under a different id.
    #[default]
    Merge,
    /// The vault becomes the full host list: hosts it does not mention are
    /// deleted, along with their keychain credentials.
    Replace,
}

/// One planned change. Split into three lists so the caller can log and count
/// without re-deriving the decision.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ImportPlan {
    /// Profiles that will be inserted.
    pub to_create: Vec<VaultProfile>,
    /// Profiles that will overwrite an existing row with the same id.
    pub to_update: Vec<VaultProfile>,
    /// Vault profiles left alone because an equivalent host already exists.
    pub to_skip: Vec<VaultProfile>,
    /// Existing profile ids deleted in `Replace` mode.
    pub to_delete: Vec<String>,
}

impl ImportPlan {
    /// Total number of vault profiles this plan accounted for. Used to check
    /// that planning lost nothing.
    pub fn accounted_for(&self) -> usize {
        self.to_create.len() + self.to_update.len() + self.to_skip.len()
    }
}

/// Existing profile as the planner sees it: only the identity fields.
#[derive(Debug, Clone)]
pub struct ExistingProfile {
    pub id: String,
    pub host: String,
    pub port: u16,
    pub username: String,
}

/// Decide what to do with each incoming profile.
///
/// `id` is the primary key: a matching id is the same host, updated in place,
/// which makes re-importing the same vault idempotent rather than duplicating.
/// When ids differ but `host + username + port` match, the vault host is
/// treated as the same machine under a different id and skipped in merge mode —
/// the same duplicate rule `import_from_config` uses.
pub fn plan_import(
    existing: &[ExistingProfile],
    incoming: &[VaultProfile],
    mode: VaultImportMode,
) -> ImportPlan {
    let mut plan = ImportPlan::default();

    for profile in incoming {
        if existing.iter().any(|e| e.id == profile.id) {
            plan.to_update.push(profile.clone());
            continue;
        }

        let same_target = existing.iter().any(|e| {
            e.host.eq_ignore_ascii_case(&profile.host)
                && e.username == profile.username
                && e.port == profile.port
        });

        if same_target && mode == VaultImportMode::Merge {
            plan.to_skip.push(profile.clone());
        } else {
            plan.to_create.push(profile.clone());
        }
    }

    if mode == VaultImportMode::Replace {
        let incoming_ids: Vec<&str> = incoming.iter().map(|p| p.id.as_str()).collect();
        plan.to_delete = existing
            .iter()
            .filter(|e| !incoming_ids.contains(&e.id.as_str()))
            .map(|e| e.id.clone())
            .collect();
    }

    plan
}
