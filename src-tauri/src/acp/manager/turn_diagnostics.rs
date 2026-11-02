//! One diagnostic line per agent turn.
//!
//! The line answers "why was that turn slow, or long?" without a debugger:
//! how long the turn took, how many tool calls it made, how many approvals it
//! needed, and how it ended. Everything is emitted at `info` so it survives the
//! default log filter — a diagnostic nobody sees fixes nothing.
//!
//! Fields deliberately kept off the line: prompt text, file paths, tool
//! arguments. This is a shape summary, not a transcript.

use std::time::Duration;

/// What one turn did, accumulated as its events stream past.
#[derive(Debug, Clone, Default)]
pub(crate) struct TurnDiagnostics {
    tool_calls: u32,
    approvals: u32,
}

impl TurnDiagnostics {
    pub(crate) fn record_tool_call(&mut self) {
        self.tool_calls = self.tool_calls.saturating_add(1);
    }

    pub(crate) fn record_approval(&mut self) {
        self.approvals = self.approvals.saturating_add(1);
    }

    #[cfg(test)]
    pub(crate) fn tool_calls(&self) -> u32 {
        self.tool_calls
    }

    #[cfg(test)]
    pub(crate) fn approvals(&self) -> u32 {
        self.approvals
    }

    /// Format the line.
    ///
    /// `outcome` is the turn's terminator as it will be shown to the user, so
    /// the log and the transcript can never disagree about how a turn ended.
    pub(crate) fn line(&self, elapsed: Duration, outcome: &str) -> String {
        format!(
            "[acp] turn diag: elapsed={:.1}s tool_calls={} approvals={} outcome={outcome}",
            elapsed.as_secs_f64(),
            self.tool_calls,
            self.approvals,
        )
    }
}
