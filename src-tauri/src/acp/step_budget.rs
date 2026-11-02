//! Host-side step budget for a turn.
//!
//! The ACP agent owns its own loop length: it may report
//! `stop_reason = max_turn_requests` when it stops, but it is not required to,
//! and a wandering agent that never stops is the failure users actually hit.
//! The host therefore counts tool calls itself and refuses a turn the moment
//! it is clearly not converging.
//!
//! Two guards run together:
//!
//! - an absolute budget, so one turn cannot run forever;
//! - a repeat guard, so a short cycle (same tool, same input, three times)
//!   stops the turn while there is still context left to explain why.
//!
//! Both produce a *named* stop reason, so the transcript can say what happened
//! instead of showing a bare error.

use std::collections::HashMap;

/// Why a turn was stopped by the host.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum StepStop {
    /// The turn used its whole tool-call budget.
    BudgetExhausted { steps: u32, budget: u32 },
    /// The same tool was called with identical input too many times.
    Repeating { tool: String, times: u32 },
}

impl StepStop {
    /// The stop reason name written to the transcript and the log.
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::BudgetExhausted { .. } => "step_budget_exhausted",
            Self::Repeating { .. } => "repeating_tool_call",
        }
    }

    /// A sentence for the user, naming the limit that fired.
    pub(crate) fn message(&self) -> String {
        match self {
            Self::BudgetExhausted { steps, budget } => {
                format!("Stopped after {steps} steps (limit {budget}). Continue to keep going.")
            }
            Self::Repeating { tool, times } => {
                format!("Stopped: called {tool} {times} times with the same input.")
            }
        }
    }
}

/// How many identical calls are tolerated before the repeat guard fires.
const REPEAT_LIMIT: u32 = 3;

/// Tracks one turn's step usage.
#[derive(Debug, Clone)]
pub(crate) struct StepBudget {
    budget: u32,
    steps: u32,
    repeats: HashMap<String, u32>,
}

impl StepBudget {
    pub(crate) fn new(budget: u32) -> Self {
        Self {
            budget,
            steps: 0,
            repeats: HashMap::new(),
        }
    }

    #[cfg(test)]
    pub(crate) fn steps(&self) -> u32 {
        self.steps
    }

    /// Record one tool call and decide whether the turn may continue.
    ///
    /// `signature` identifies the call as the user would see it — tool name
    /// plus its normalised arguments. Calls without a signature still count
    /// against the budget.
    pub(crate) fn record(&mut self, signature: Option<&str>) -> Option<StepStop> {
        self.steps = self.steps.saturating_add(1);

        if let Some(signature) = signature {
            let count = self.repeats.entry(signature.to_string()).or_insert(0);
            *count = count.saturating_add(1);
            if *count >= REPEAT_LIMIT {
                return Some(StepStop::Repeating {
                    tool: signature.to_string(),
                    times: *count,
                });
            }
        }

        if self.steps >= self.budget {
            return Some(StepStop::BudgetExhausted {
                steps: self.steps,
                budget: self.budget,
            });
        }

        None
    }
}
