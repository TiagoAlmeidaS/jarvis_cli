use std::fmt;

use serde::Deserialize;
use serde::Serialize;

use crate::ThreadId;
use crate::account::PlanType;

#[derive(Debug, Clone, Deserialize, Serialize, Default, PartialEq, Eq)]
pub struct TokenUsage {
    pub input_tokens: i64,
    pub cached_input_tokens: i64,
    pub output_tokens: i64,
    pub reasoning_output_tokens: i64,
    pub total_tokens: i64,
}

impl TokenUsage {
    pub fn is_zero(&self) -> bool {
        self.total_tokens == 0
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct RateLimitSnapshot {
    pub primary: Option<RateLimitWindow>,
    pub secondary: Option<RateLimitWindow>,
    pub credits: Option<CreditsSnapshot>,
    pub plan_type: Option<PlanType>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct RateLimitWindow {
    /// Percentage (0-100) of the window that has been consumed.
    pub used_percent: f64,
    /// Rolling window duration, in minutes.
    pub window_minutes: Option<i64>,
    /// Unix timestamp (seconds since epoch) when the window resets.
    pub resets_at: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct CreditsSnapshot {
    pub has_credits: bool,
    pub unlimited: bool,
    pub balance: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum SessionSource {
    Cli,
    #[default]
    VSCode,
    Exec,
    Mcp,
    SubAgent(SubAgentSource),
    #[serde(other)]
    Unknown,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SubAgentSource {
    Review,
    Compact,
    ThreadSpawn {
        parent_thread_id: ThreadId,
        depth: i32,
    },
    Other(String),
}

impl fmt::Display for SessionSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SessionSource::Cli => f.write_str("cli"),
            SessionSource::VSCode => f.write_str("vscode"),
            SessionSource::Exec => f.write_str("exec"),
            SessionSource::Mcp => f.write_str("mcp"),
            SessionSource::SubAgent(sub_source) => write!(f, "subagent_{sub_source}"),
            SessionSource::Unknown => f.write_str("unknown"),
        }
    }
}

impl fmt::Display for SubAgentSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SubAgentSource::Review => f.write_str("review"),
            SubAgentSource::Compact => f.write_str("compact"),
            SubAgentSource::ThreadSpawn {
                parent_thread_id,
                depth,
            } => write!(f, "thread_spawn_{parent_thread_id}_d{depth}"),
            SubAgentSource::Other(other) => f.write_str(other),
        }
    }
}
