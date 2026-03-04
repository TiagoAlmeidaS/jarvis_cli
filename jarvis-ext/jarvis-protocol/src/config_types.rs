use serde::Deserialize;
use serde::Serialize;

/// Controls summarization of model reasoning traces.
/// See <https://platform.openai.com/docs/guides/reasoning?api-mode=responses#reasoning-summaries>
#[derive(Debug, Serialize, Deserialize, Default, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ReasoningSummary {
    #[default]
    Auto,
    Concise,
    Detailed,
    None,
}

/// Controls output length/detail on GPT-5 models via the Responses API.
#[derive(Hash, Debug, Serialize, Deserialize, Default, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Verbosity {
    Low,
    #[default]
    Medium,
    High,
}
