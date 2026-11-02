use super::*;
use agent_client_protocol::schema::v1::{
    PermissionOption as AgentPermissionOption, PermissionOptionId as AgentPermissionOptionId,
    PermissionOptionKind as AgentPermissionOptionKind, ToolKind,
};

#[test]
fn delete_kind_fires_without_any_text() {
    let guard = DeletionGuard::evaluate(Some(ToolKind::Delete), None, None);
    assert!(guard.destructive);
    assert_eq!(guard.reason, "tool kind is delete");
}

#[test]
fn benign_requests_pass_through() {
    for kind in [
        Some(ToolKind::Read),
        Some(ToolKind::Edit),
        Some(ToolKind::Search),
        Some(ToolKind::Execute),
        None,
    ] {
        let guard = DeletionGuard::evaluate(kind, Some("Read src/main.rs"), None);
        assert!(!guard.destructive, "kind {kind:?} should not fire");
    }
}

#[test]
fn unknown_kind_still_scans_the_arguments() {
    // `ToolKind` is `#[non_exhaustive]`, so a newer agent can report a kind this
    // build has never heard of. The scan must still run.
    let raw = serde_json::json!({ "command": "rm -rf build" });
    let guard = DeletionGuard::evaluate(Some(ToolKind::Other), Some("Run"), Some(&raw));
    assert!(guard.destructive);
    assert_eq!(guard.reason, "arguments name a destructive command");
}

#[test]
fn shell_delete_under_execute_kind_is_caught() {
    let raw = serde_json::json!({ "command": "pnpm build && rm -rf dist" });
    let guard = DeletionGuard::evaluate(Some(ToolKind::Execute), Some("Run command"), Some(&raw));
    assert!(guard.destructive);
}

#[test]
fn title_alone_is_enough() {
    let guard = DeletionGuard::evaluate(Some(ToolKind::Execute), Some("git clean -fdx"), None);
    assert!(guard.destructive);
    assert_eq!(guard.reason, "title names a destructive command");
}

#[test]
fn nested_arguments_are_scanned() {
    let raw = serde_json::json!({
        "plan": [{ "steps": [{ "run": "Remove-Item -Recurse -Force .\\tmp" }] }]
    });
    let guard = DeletionGuard::evaluate(Some(ToolKind::Other), Some("Apply plan"), Some(&raw));
    assert!(guard.destructive);
}

#[test]
fn casing_and_spacing_do_not_evade_the_scan() {
    let raw = serde_json::json!({ "command": "RM    -RF   /tmp/x" });
    let guard = DeletionGuard::evaluate(Some(ToolKind::Execute), None, Some(&raw));
    assert!(guard.destructive);
}

#[test]
fn ordinary_words_that_merely_contain_rm_do_not_fire() {
    // `format`, `confirm`, `perform` must not be read as `rm`.
    for text in [
        "cargo fmt",
        "Run format on save",
        "confirm the change",
        "perform migration",
    ] {
        let raw = serde_json::json!({ "command": text });
        let guard = DeletionGuard::evaluate(Some(ToolKind::Execute), Some(text), Some(&raw));
        assert!(!guard.destructive, "{text} should not fire");
    }
}

#[test]
fn git_checkout_is_not_git_clean() {
    let raw = serde_json::json!({ "command": "git checkout -b feature" });
    let guard = DeletionGuard::evaluate(Some(ToolKind::Execute), None, Some(&raw));
    assert!(!guard.destructive);
}

#[test]
fn deep_nesting_is_bounded_not_panicking() {
    let mut value = serde_json::json!("leaf");
    for _ in 0..50 {
        value = serde_json::json!({ "n": value });
    }
    let guard = DeletionGuard::evaluate(Some(ToolKind::Other), Some("Deep"), Some(&value));
    assert!(!guard.destructive);
}

fn option(id: &str, kind: AgentPermissionOptionKind) -> AgentPermissionOption {
    AgentPermissionOption::new(AgentPermissionOptionId::new(id), id, kind)
}

#[test]
fn always_allow_is_withheld_and_the_rest_is_kept_in_order() {
    let options = vec![
        option("allow-once", AgentPermissionOptionKind::AllowOnce),
        option("allow-always", AgentPermissionOptionKind::AllowAlways),
        option("reject-once", AgentPermissionOptionKind::RejectOnce),
    ];
    let filtered = filter_always_allow(options);
    let ids: Vec<&str> = filtered.iter().map(|o| o.option_id.0.as_ref()).collect();
    assert_eq!(ids, ["allow-once", "reject-once"]);
}

#[test]
fn filtering_leaves_a_request_without_always_allow_unchanged() {
    let options = vec![
        option("allow-once", AgentPermissionOptionKind::AllowOnce),
        option("reject-always", AgentPermissionOptionKind::RejectAlways),
    ];
    assert_eq!(filter_always_allow(options.clone()), options);
}
