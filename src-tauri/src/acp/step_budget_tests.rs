use super::step_budget::{StepBudget, StepStop};

#[test]
fn budget_allows_calls_under_the_limit() {
    let mut budget = StepBudget::new(3);
    assert!(budget.record(Some("read:a.rs")).is_none());
    assert!(budget.record(Some("read:b.rs")).is_none());
    assert_eq!(budget.steps(), 2);
}

#[test]
fn budget_blocks_the_call_that_reaches_the_limit() {
    let mut budget = StepBudget::new(2);
    assert!(budget.record(Some("read:a.rs")).is_none());
    let stop = budget
        .record(Some("read:b.rs"))
        .expect("budget should fire");
    assert_eq!(
        stop,
        StepStop::BudgetExhausted {
            steps: 2,
            budget: 2
        }
    );
    assert_eq!(stop.as_str(), "step_budget_exhausted");
}

#[test]
fn identical_calls_fire_the_repeat_guard_before_the_budget() {
    let mut budget = StepBudget::new(100);
    assert!(budget.record(Some("bash:cargo test")).is_none());
    assert!(budget.record(Some("bash:cargo test")).is_none());
    let stop = budget
        .record(Some("bash:cargo test"))
        .expect("repeat should fire");
    assert_eq!(
        stop,
        StepStop::Repeating {
            tool: "bash:cargo test".to_string(),
            times: 3
        }
    );
    assert_eq!(stop.as_str(), "repeating_tool_call");
}

#[test]
fn different_arguments_are_not_a_repeat() {
    let mut budget = StepBudget::new(100);
    for n in 0..5 {
        let signature = format!("read:file{n}.rs");
        assert!(budget.record(Some(&signature)).is_none());
    }
    assert_eq!(budget.steps(), 5);
}

#[test]
fn unnoted_calls_still_count_against_the_budget() {
    let mut budget = StepBudget::new(1);
    let stop = budget.record(None).expect("budget should fire");
    assert_eq!(stop.as_str(), "step_budget_exhausted");
}

#[test]
fn messages_name_the_limit_that_fired() {
    let budget_stop = StepStop::BudgetExhausted {
        steps: 40,
        budget: 40,
    };
    assert!(budget_stop.message().contains("40"));
    let repeat_stop = StepStop::Repeating {
        tool: "read:a.rs".to_string(),
        times: 3,
    };
    assert!(repeat_stop.message().contains("read:a.rs"));
    assert!(repeat_stop.message().contains("3"));
}

#[test]
fn zero_budget_stops_on_the_first_call() {
    let mut budget = StepBudget::new(0);
    let stop = budget.record(Some("read:a.rs")).expect("should fire");
    assert_eq!(stop.as_str(), "step_budget_exhausted");
}
