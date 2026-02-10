//! Error types for GitHub API operations.

use thiserror::Error;

/// Errors that can occur when interacting with the GitHub API.
#[derive(Debug, Error)]
pub enum GitHubError {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("GitHub API error: {status} - {message}")]
    Api {
        status: u16,
        message: String,
    },

    #[error("Rate limit exceeded. Reset at: {reset_at:?}")]
    RateLimit {
        reset_at: Option<u64>,
    },

    #[error("Authentication failed: {0}")]
    Authentication(String),

    #[error("Invalid repository format: {0}")]
    InvalidRepository(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Invalid URL: {0}")]
    InvalidUrl(#[from] url::ParseError),

    #[error("Other error: {0}")]
    Other(String),
}

impl GitHubError {
    pub fn from_response(status: u16, body: String) -> Self {
        Self::Api {
            status,
            message: body,
        }
    }

    pub fn is_rate_limit(&self) -> bool {
        matches!(self, Self::RateLimit { .. })
    }

    pub fn is_authentication(&self) -> bool {
        matches!(self, Self::Authentication(_))
    }
}
