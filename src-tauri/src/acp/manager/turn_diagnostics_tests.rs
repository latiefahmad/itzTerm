use super::*;
use std::time::Duration;

#[test]
fn counters_accumulate_per_kind() {
    let mut diag = TurnDiagnostics::default();
    for _ in 0..3 {
        diag.record_tool_call();
    }
    assert_eq!(diag.tool_calls(), 3);
    assert_eq!(diag.approvals(), 0);
}

#[test]
fn line_carries_elapsed_counts_and_outcome() {
    let mut diag = TurnDiagnostics::default();
    diag.record_tool_call();
    diag.record_tool_call();
    diag.record_approval();
    let line = diag.line(Duration::from_millis(1500), "end_turn");
    assert_eq!(
        line,
        "[acp] turn diag: elapsed=1.5s tool_calls=2 approvals=1 outcome=end_turn"
    );
}

#[test]
fn a_turn_with_no_activity_still_produces_a_line() {
    let line = TurnDiagnostics::default().line(Duration::ZERO, "end_turn");
    assert_eq!(
        line,
        "[acp] turn diag: elapsed=0.0s tool_calls=0 approvals=0 outcome=end_turn"
    );
}
