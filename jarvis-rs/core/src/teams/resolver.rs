use std::fmt;

/// A model specification resolved into its provider and model name components.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedModel {
    /// The provider identifier (e.g., "anthropic", "openrouter", "google", "ollama").
    pub provider_id: String,

    /// The model name within that provider (e.g., "claude-3.5-sonnet", "deepseek/deepseek-r1:free").
    pub model_name: String,
}

impl fmt::Display for ResolvedModel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.provider_id, self.model_name)
    }
}

/// Error returned when a model specification cannot be parsed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelSpecError {
    pub spec: String,
    pub reason: String,
}

impl fmt::Display for ModelSpecError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid model spec '{}': {}", self.spec, self.reason)
    }
}

impl std::error::Error for ModelSpecError {}

/// Resolve a model specification string into its provider and model components.
///
/// The format is `"provider/model"` where:
/// - `provider` is the first path segment (e.g., "anthropic", "openrouter", "google", "ollama")
/// - `model` is everything after the first `/` (may contain additional `/` characters)
///
/// # Examples
///
/// ```
/// use jarvis_core::teams::resolver::resolve_model_spec;
///
/// let simple = resolve_model_spec("anthropic/claude-3.5-sonnet").unwrap();
/// assert_eq!(simple.provider_id, "anthropic");
/// assert_eq!(simple.model_name, "claude-3.5-sonnet");
///
/// let nested = resolve_model_spec("openrouter/deepseek/deepseek-r1:free").unwrap();
/// assert_eq!(nested.provider_id, "openrouter");
/// assert_eq!(nested.model_name, "deepseek/deepseek-r1:free");
/// ```
pub fn resolve_model_spec(spec: &str) -> Result<ResolvedModel, ModelSpecError> {
    let spec_trimmed = spec.trim();

    if spec_trimmed.is_empty() {
        return Err(ModelSpecError {
            spec: spec.to_string(),
            reason: "model spec cannot be empty".to_string(),
        });
    }

    let Some((provider, model)) = spec_trimmed.split_once('/') else {
        return Err(ModelSpecError {
            spec: spec.to_string(),
            reason: "expected format 'provider/model' with at least one '/'".to_string(),
        });
    };

    let provider = provider.trim();
    let model = model.trim();

    if provider.is_empty() {
        return Err(ModelSpecError {
            spec: spec.to_string(),
            reason: "provider cannot be empty".to_string(),
        });
    }

    if model.is_empty() {
        return Err(ModelSpecError {
            spec: spec.to_string(),
            reason: "model name cannot be empty".to_string(),
        });
    }

    Ok(ResolvedModel {
        provider_id: provider.to_string(),
        model_name: model.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_simple_provider_model() {
        let result = resolve_model_spec("anthropic/claude-3.5-sonnet").unwrap();
        assert_eq!(
            result,
            ResolvedModel {
                provider_id: "anthropic".to_string(),
                model_name: "claude-3.5-sonnet".to_string(),
            }
        );
    }

    #[test]
    fn resolve_nested_model_path() {
        let result = resolve_model_spec("openrouter/deepseek/deepseek-r1:free").unwrap();
        assert_eq!(
            result,
            ResolvedModel {
                provider_id: "openrouter".to_string(),
                model_name: "deepseek/deepseek-r1:free".to_string(),
            }
        );
    }

    #[test]
    fn resolve_google_model() {
        let result = resolve_model_spec("google/gemini-2.0-flash").unwrap();
        assert_eq!(
            result,
            ResolvedModel {
                provider_id: "google".to_string(),
                model_name: "gemini-2.0-flash".to_string(),
            }
        );
    }

    #[test]
    fn resolve_ollama_model() {
        let result = resolve_model_spec("ollama/llama3").unwrap();
        assert_eq!(
            result,
            ResolvedModel {
                provider_id: "ollama".to_string(),
                model_name: "llama3".to_string(),
            }
        );
    }

    #[test]
    fn resolve_with_whitespace() {
        let result = resolve_model_spec("  anthropic / claude-3.5-sonnet  ").unwrap();
        assert_eq!(result.provider_id, "anthropic");
        assert_eq!(result.model_name, "claude-3.5-sonnet");
    }

    #[test]
    fn reject_empty_string() {
        let err = resolve_model_spec("").unwrap_err();
        assert_eq!(err.reason, "model spec cannot be empty");
    }

    #[test]
    fn reject_whitespace_only() {
        let err = resolve_model_spec("   ").unwrap_err();
        assert_eq!(err.reason, "model spec cannot be empty");
    }

    #[test]
    fn reject_no_slash() {
        let err = resolve_model_spec("just-a-model-name").unwrap_err();
        assert!(err.reason.contains("expected format"));
    }

    #[test]
    fn reject_empty_provider() {
        let err = resolve_model_spec("/claude-3.5-sonnet").unwrap_err();
        assert_eq!(err.reason, "provider cannot be empty");
    }

    #[test]
    fn reject_empty_model() {
        let err = resolve_model_spec("anthropic/").unwrap_err();
        assert_eq!(err.reason, "model name cannot be empty");
    }

    #[test]
    fn reject_slash_only() {
        let err = resolve_model_spec("/").unwrap_err();
        assert_eq!(err.reason, "provider cannot be empty");
    }

    #[test]
    fn display_resolved_model() {
        let model = ResolvedModel {
            provider_id: "anthropic".to_string(),
            model_name: "claude-3.5-sonnet".to_string(),
        };
        assert_eq!(model.to_string(), "anthropic/claude-3.5-sonnet");
    }

    #[test]
    fn display_model_spec_error() {
        let err = ModelSpecError {
            spec: "bad".to_string(),
            reason: "no slash".to_string(),
        };
        assert_eq!(err.to_string(), "invalid model spec 'bad': no slash");
    }
}
