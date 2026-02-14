//! Shared Google OAuth2 authentication for Search Console and AdSense APIs.
//!
//! Implements the **installed application** (desktop) OAuth2 flow:
//! 1. User runs `jarvis daemon auth google` once.
//! 2. CLI prints a URL, user authorises in browser, pastes the auth code back.
//! 3. The code is exchanged for access + refresh tokens.
//! 4. Tokens are persisted to `~/.jarvis/credentials/google.json`.
//! 5. The daemon uses the refresh token to obtain fresh access tokens silently.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tracing::{debug, info};

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Google OAuth2 client credentials (from Google Cloud Console).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GoogleOAuthConfig {
    pub client_id: String,
    pub client_secret: String,
    /// OAuth scopes to request.
    /// Defaults cover Search Console + AdSense read-only.
    #[serde(default = "default_scopes")]
    pub scopes: Vec<String>,
}

fn default_scopes() -> Vec<String> {
    vec![
        "https://www.googleapis.com/auth/webmasters.readonly".to_string(),
        "https://www.googleapis.com/auth/adsense.readonly".to_string(),
    ]
}

// ---------------------------------------------------------------------------
// Token storage
// ---------------------------------------------------------------------------

/// Persisted OAuth2 tokens.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GoogleTokens {
    pub access_token: String,
    pub refresh_token: String,
    /// Unix timestamp when the access token expires.
    pub expires_at: i64,
    /// Scopes the tokens were granted for.
    #[serde(default)]
    pub scopes: Vec<String>,
}

impl GoogleTokens {
    /// Whether the access token has expired (with a 60 s safety margin).
    pub fn is_expired(&self) -> bool {
        chrono::Utc::now().timestamp() >= self.expires_at - 60
    }
}

/// Default credentials file path: `~/.jarvis/credentials/google.json`.
pub fn default_credentials_path() -> PathBuf {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    home.join(".jarvis").join("credentials").join("google.json")
}

/// Load tokens from disk.
pub fn load_tokens(path: &Path) -> Result<Option<GoogleTokens>> {
    if !path.exists() {
        return Ok(None);
    }
    let data =
        std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    let tokens: GoogleTokens = serde_json::from_str(&data).context("parsing google tokens")?;
    Ok(Some(tokens))
}

/// Save tokens to disk (creates parent directories).
pub fn save_tokens(path: &Path, tokens: &GoogleTokens) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let data = serde_json::to_string_pretty(tokens)?;
    std::fs::write(path, data)?;
    // Restrictive permissions on Unix.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// OAuth2 flow
// ---------------------------------------------------------------------------

const GOOGLE_AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const GOOGLE_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";

/// Generate the URL the user must visit to authorise the app.
pub fn authorization_url(config: &GoogleOAuthConfig) -> String {
    let scopes = config.scopes.join(" ");
    format!(
        "{GOOGLE_AUTH_URL}?client_id={}&redirect_uri=urn:ietf:wg:oauth:2.0:oob\
         &response_type=code&scope={}&access_type=offline&prompt=consent",
        config.client_id,
        urlencoding_encode(&scopes),
    )
}

/// Exchange an authorisation code for access + refresh tokens.
pub async fn exchange_code(config: &GoogleOAuthConfig, code: &str) -> Result<GoogleTokens> {
    let http = reqwest::Client::new();
    let resp = http
        .post(GOOGLE_TOKEN_URL)
        .form(&[
            ("client_id", config.client_id.as_str()),
            ("client_secret", config.client_secret.as_str()),
            ("code", code),
            ("grant_type", "authorization_code"),
            ("redirect_uri", "urn:ietf:wg:oauth:2.0:oob"),
        ])
        .send()
        .await
        .context("token exchange request failed")?;

    if !resp.status().is_success() {
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("Google token exchange failed: {body}");
    }

    let body: TokenResponse = resp.json().await.context("parsing token response")?;
    let now = chrono::Utc::now().timestamp();

    Ok(GoogleTokens {
        access_token: body.access_token,
        refresh_token: body
            .refresh_token
            .ok_or_else(|| anyhow::anyhow!("no refresh_token in response"))?,
        expires_at: now + body.expires_in,
        scopes: config.scopes.clone(),
    })
}

/// Use the refresh token to obtain a new access token.
pub async fn refresh_access_token(
    config: &GoogleOAuthConfig,
    tokens: &GoogleTokens,
) -> Result<GoogleTokens> {
    debug!("Refreshing Google access token");
    let http = reqwest::Client::new();
    let resp = http
        .post(GOOGLE_TOKEN_URL)
        .form(&[
            ("client_id", config.client_id.as_str()),
            ("client_secret", config.client_secret.as_str()),
            ("refresh_token", tokens.refresh_token.as_str()),
            ("grant_type", "refresh_token"),
        ])
        .send()
        .await
        .context("token refresh request failed")?;

    if !resp.status().is_success() {
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("Google token refresh failed: {body}");
    }

    let body: TokenResponse = resp.json().await.context("parsing refresh response")?;
    let now = chrono::Utc::now().timestamp();

    Ok(GoogleTokens {
        access_token: body.access_token,
        refresh_token: body
            .refresh_token
            .unwrap_or_else(|| tokens.refresh_token.clone()),
        expires_at: now + body.expires_in,
        scopes: tokens.scopes.clone(),
    })
}

/// Ensure we have a valid (non-expired) access token, refreshing if needed.
/// Saves updated tokens to disk after refresh.
pub async fn ensure_valid_token(
    config: &GoogleOAuthConfig,
    creds_path: &Path,
) -> Result<GoogleTokens> {
    let tokens = load_tokens(creds_path)?.ok_or_else(|| {
        anyhow::anyhow!("No Google credentials found. Run `jarvis daemon auth google` first.")
    })?;

    if !tokens.is_expired() {
        return Ok(tokens);
    }

    info!("Google access token expired, refreshing...");
    let refreshed = refresh_access_token(config, &tokens).await?;
    save_tokens(creds_path, &refreshed)?;
    Ok(refreshed)
}

// ---------------------------------------------------------------------------
// Internal types
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    #[serde(default = "default_expires_in")]
    expires_in: i64,
}

fn default_expires_in() -> i64 {
    3600
}

/// Minimal percent-encoding for URL query parameters.
fn urlencoding_encode(s: &str) -> String {
    s.replace(' ', "%20")
        .replace(':', "%3A")
        .replace('/', "%2F")
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn default_scopes_include_search_console_and_adsense() {
        let scopes = default_scopes();
        assert_eq!(scopes.len(), 2);
        assert!(scopes[0].contains("webmasters"));
        assert!(scopes[1].contains("adsense"));
    }

    #[test]
    fn tokens_not_expired_when_fresh() {
        let tokens = GoogleTokens {
            access_token: "access".to_string(),
            refresh_token: "refresh".to_string(),
            expires_at: chrono::Utc::now().timestamp() + 3600,
            scopes: vec![],
        };
        assert!(!tokens.is_expired());
    }

    #[test]
    fn tokens_expired_when_past() {
        let tokens = GoogleTokens {
            access_token: "access".to_string(),
            refresh_token: "refresh".to_string(),
            expires_at: chrono::Utc::now().timestamp() - 10,
            scopes: vec![],
        };
        assert!(tokens.is_expired());
    }

    #[test]
    fn tokens_expired_within_safety_margin() {
        let tokens = GoogleTokens {
            access_token: "access".to_string(),
            refresh_token: "refresh".to_string(),
            expires_at: chrono::Utc::now().timestamp() + 30, // < 60s margin
            scopes: vec![],
        };
        assert!(tokens.is_expired());
    }

    #[test]
    fn authorization_url_contains_required_params() {
        let config = GoogleOAuthConfig {
            client_id: "test-client-id".to_string(),
            client_secret: "secret".to_string(),
            scopes: default_scopes(),
        };
        let url = authorization_url(&config);
        assert!(url.contains("test-client-id"));
        assert!(url.contains("response_type=code"));
        assert!(url.contains("access_type=offline"));
        assert!(url.contains("webmasters"));
    }

    #[test]
    fn save_and_load_tokens_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("google.json");

        let tokens = GoogleTokens {
            access_token: "at-123".to_string(),
            refresh_token: "rt-456".to_string(),
            expires_at: 1700000000,
            scopes: vec!["scope1".to_string()],
        };

        save_tokens(&path, &tokens).unwrap();
        let loaded = load_tokens(&path).unwrap().unwrap();

        assert_eq!(loaded.access_token, "at-123");
        assert_eq!(loaded.refresh_token, "rt-456");
        assert_eq!(loaded.expires_at, 1700000000);
        assert_eq!(loaded.scopes, vec!["scope1"]);
    }

    #[test]
    fn load_tokens_returns_none_when_missing() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nonexistent.json");
        assert!(load_tokens(&path).unwrap().is_none());
    }

    #[test]
    fn parse_oauth_config() {
        let config: GoogleOAuthConfig = serde_json::from_value(serde_json::json!({
            "client_id": "my-id.apps.googleusercontent.com",
            "client_secret": "my-secret"
        }))
        .expect("parse");
        assert_eq!(config.client_id, "my-id.apps.googleusercontent.com");
        assert_eq!(config.scopes.len(), 2); // defaults
    }
}
