use serde::Deserialize;
use serde::Serialize;

use crate::config_types::Verbosity;

/// See <https://platform.openai.com/docs/guides/reasoning?api-mode=responses#get-started-with-reasoning>
#[derive(Debug, Serialize, Deserialize, Default, Clone, Copy, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum ReasoningEffort {
    None,
    Minimal,
    Low,
    #[default]
    Medium,
    High,
    XHigh,
}

/// A reasoning effort option that can be surfaced for a model.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct ReasoningEffortPreset {
    pub effort: ReasoningEffort,
    pub description: String,
}

/// Canonical user-input modality tags advertised by a model.
#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum InputModality {
    Text,
    Image,
}

/// Backward-compatible default when `input_modalities` is omitted on the wire.
pub fn default_input_modalities() -> Vec<InputModality> {
    vec![InputModality::Text, InputModality::Image]
}

/// Visibility of a model in the picker or APIs.
#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ModelVisibility {
    List,
    Hide,
    None,
}

/// Shell execution capability for a model.
#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ConfigShellToolType {
    Default,
    Local,
    UnifiedExec,
    Disabled,
    ShellCommand,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ApplyPatchToolType {
    Freeform,
    Function,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TruncationMode {
    Bytes,
    Tokens,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
pub struct TruncationPolicyConfig {
    pub mode: TruncationMode,
    pub limit: i64,
}

impl TruncationPolicyConfig {
    pub const fn bytes(limit: i64) -> Self {
        Self {
            mode: TruncationMode::Bytes,
            limit,
        }
    }

    pub const fn tokens(limit: i64) -> Self {
        Self {
            mode: TruncationMode::Tokens,
            limit,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct ModelInfoUpgrade {
    pub model: String,
    pub migration_markdown: String,
}

/// Template for model instructions and personality-specific messages.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct ModelMessages {
    pub instructions_template: Option<String>,
    pub instructions_variables: Option<ModelInstructionsVariables>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct ModelInstructionsVariables {
    pub personality_default: Option<String>,
    pub personality_friendly: Option<String>,
    pub personality_pragmatic: Option<String>,
}

fn default_effective_context_window_percent() -> i64 {
    95
}

/// Model metadata returned by the Jarvis backend `/models` endpoint.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct ModelInfo {
    pub slug: String,
    pub display_name: String,
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_reasoning_level: Option<ReasoningEffort>,
    pub supported_reasoning_levels: Vec<ReasoningEffortPreset>,
    pub shell_type: ConfigShellToolType,
    pub visibility: ModelVisibility,
    pub supported_in_api: bool,
    pub priority: i32,
    pub upgrade: Option<ModelInfoUpgrade>,
    pub base_instructions: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_messages: Option<ModelMessages>,
    pub supports_reasoning_summaries: bool,
    pub support_verbosity: bool,
    pub default_verbosity: Option<Verbosity>,
    pub apply_patch_tool_type: Option<ApplyPatchToolType>,
    pub truncation_policy: TruncationPolicyConfig,
    pub supports_parallel_tool_calls: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_window: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auto_compact_token_limit: Option<i64>,
    #[serde(default = "default_effective_context_window_percent")]
    pub effective_context_window_percent: i64,
    pub experimental_supported_tools: Vec<String>,
    #[serde(default = "default_input_modalities")]
    pub input_modalities: Vec<InputModality>,
}

impl ModelInfo {
    /// Returns the token limit used to trigger automatic compaction.
    /// Falls back to 90% of the context window when not explicitly set.
    pub fn auto_compact_token_limit(&self) -> Option<i64> {
        self.auto_compact_token_limit.or_else(|| {
            self.context_window
                .map(|context_window| (context_window * 9) / 10)
        })
    }
}

/// Response wrapper for `/models`.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Default)]
pub struct ModelsResponse {
    pub models: Vec<ModelInfo>,
}
