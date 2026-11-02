//! Deletion guard for agent permission requests.
//!
//! A destructive tool call always keeps a human in the loop. Every other
//! change an agent makes can be recovered — a file can be read again, a
//! commit can be reset — but a deleted untracked file leaves nothing to read,
//! so "always allow" must not cover it.
//!
//! Two independent signals are checked, because neither is sufficient alone:
//!
//! - the ACP `kind`, which is the only field the protocol standardises. The
//!   renderer's `ToolKind` union is open (`| string`), so an unknown or
//!   missing kind is *not* treated as safe — the command scan below still runs.
//! - the action text (title plus every string in `raw_input`), because an
//!   agent may report `kind: "execute"` and delete through the shell.
//!
//! The guard is deliberately narrow: it removes the "allow always" options and
//! leaves everything else (allow once, reject, cancel) untouched, so it can
//! never make the agent *less* interactive, only less forgetful.

use agent_client_protocol::schema::v1::{
    PermissionOption as AgentPermissionOption, PermissionOptionKind as AgentPermissionOptionKind,
    ToolKind,
};

/// Substrings that mark a command as destructive. Matched against the action
/// text after `normalize_for_scan`, so `"rm -rf"` also matches `"RM  -RF"`.
///
/// The entries are intentionally whole-token-ish: `format` must not match
/// `rm`, and `git clean` must not match `git checkout`.
const DESTRUCTIVE_PATTERNS: &[&str] = &[
    // POSIX
    "rm ",
    "rm -",
    "rmdir ",
    "unlink ",
    "shred ",
    "truncate ",
    // git
    "git clean",
    "git rm",
    "git stash drop",
    "git stash clear",
    "git reset --hard",
    "git branch -d",
    "git branch -D",
    // find / xargs
    "find -delete",
    "find . -delete",
    " -delete",
    // PowerShell
    "remove-item",
    " ri ",
    " del ",
    " erase ",
    " rd ",
    // Windows cmd / misc
    "rd /s",
    "rmdir /s",
    "del /f",
    "del /q",
    "drop table",
    "drop database",
    "truncate table",
    "docker system prune",
    "docker volume rm",
    "kubectl delete",
];

/// Lowercase and collapse whitespace so the patterns above match regardless of
/// the shell's own spacing and casing.
fn normalize_for_scan(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push(' ');
    let mut last_space = true;
    for ch in text.chars() {
        if ch.is_whitespace() {
            if !last_space {
                out.push(' ');
                last_space = true;
            }
        } else {
            for lower in ch.to_lowercase() {
                out.push(lower);
            }
            last_space = false;
        }
    }
    if !out.ends_with(' ') {
        out.push(' ');
    }
    out
}

/// Is this single string a destructive action?
fn text_is_destructive(text: &str) -> bool {
    if text.is_empty() {
        return false;
    }
    let normalized = normalize_for_scan(text);
    DESTRUCTIVE_PATTERNS
        .iter()
        .any(|pattern| normalized.contains(&normalize_for_scan(pattern)))
}

/// Collect every string that could carry the action the agent is asking for.
fn collect_strings(value: &serde_json::Value, out: &mut Vec<String>, depth: usize) {
    // `raw_input` is arbitrary agent JSON; bound the walk so a hostile or
    // merely huge payload cannot stall the driver thread.
    if depth > 6 {
        return;
    }
    match value {
        serde_json::Value::String(s) => out.push(s.clone()),
        serde_json::Value::Array(items) => {
            for item in items {
                collect_strings(item, out, depth + 1);
            }
        }
        serde_json::Value::Object(map) => {
            for (_key, item) in map {
                collect_strings(item, out, depth + 1);
            }
        }
        _ => {}
    }
}

/// The decision for one permission request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DeletionGuard {
    /// True when the request may delete something.
    pub(crate) destructive: bool,
    /// Why the guard fired, for the log line. Empty when it did not.
    pub(crate) reason: &'static str,
}

impl DeletionGuard {
    /// Evaluate a permission request.
    ///
    /// `kind` is the ACP tool kind (`edit`, `delete`, `execute`, ...), `title`
    /// the human label the agent supplied, and `raw_input` the tool arguments.
    pub(crate) fn evaluate(
        kind: Option<ToolKind>,
        title: Option<&str>,
        raw_input: Option<&serde_json::Value>,
    ) -> Self {
        if matches!(kind, Some(ToolKind::Delete)) {
            return Self {
                destructive: true,
                reason: "tool kind is delete",
            };
        }

        if let Some(title) = title {
            if text_is_destructive(title) {
                return Self {
                    destructive: true,
                    reason: "title names a destructive command",
                };
            }
        }

        if let Some(raw_input) = raw_input {
            let mut strings = Vec::new();
            collect_strings(raw_input, &mut strings, 0);
            if strings.iter().any(|s| text_is_destructive(s)) {
                return Self {
                    destructive: true,
                    reason: "arguments name a destructive command",
                };
            }
        }

        Self {
            destructive: false,
            reason: "",
        }
    }
}

/// Remove "allow always" from a request's options.
///
/// Everything else is preserved in order. An empty result is not possible in
/// practice (a request with no allow option at all already only offers reject
/// and cancel), and the caller never depends on there being an allow left.
pub(crate) fn filter_always_allow(
    options: Vec<AgentPermissionOption>,
) -> Vec<AgentPermissionOption> {
    options
        .into_iter()
        .filter(|option| !matches!(option.kind, AgentPermissionOptionKind::AllowAlways))
        .collect()
}
