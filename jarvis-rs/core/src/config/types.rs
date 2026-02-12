//! Types used to define the fields of [`crate::config::Config`].

// Note this file should generally be restricted to simple struct/enum
// definitions that do not contain business logic.

use crate::config_loader::RequirementSource;
pub use jarvis_protocol::config_types::AltScreenMode;
pub use jarvis_protocol::config_types::ModeKind;
pub use jarvis_protocol::config_types::Personality;
pub use jarvis_protocol::config_types::WebSearchMode;
use jarvis_utils_absolute_path::AbsolutePathBuf;
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;
use std::time::Duration;
use wildmatch::WildMatchPattern;

use schemars::JsonSchema;
use serde::Deserialize;
use serde::Deserializer;
use serde::Serialize;
use serde::de::Error as SerdeError;

pub const DEFAULT_OTEL_ENVIRONMENT: &str = "dev";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum McpServerDisabledReason {
    Unknown,
    Requirements { source: RequirementSource },
}

impl fmt::Display for McpServerDisabledReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            McpServerDisabledReason::Unknown => write!(f, "unknown"),
            McpServerDisabledReason::Requirements { source } => {
                write!(f, "requirements ({source})")
            }
        }
    }
}

#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct McpServerConfig {
    #[serde(flatten)]
    pub transport: McpServerTransportConfig,

    /// When `false`, Jarvis skips initializing this MCP server.
    #[serde(default = "default_enabled")]
    pub enabled: bool,

    /// Reason this server was disabled after applying requirements.
    #[serde(skip)]
    pub disabled_reason: Option<McpServerDisabledReason>,

    /// Startup timeout in seconds for initializing MCP server & initially listing tools.
    #[serde(
        default,
        with = "option_duration_secs",
        skip_serializing_if = "Option::is_none"
    )]
    pub startup_timeout_sec: Option<Duration>,

    /// Default timeout for MCP tool calls initiated via this server.
    #[serde(default, with = "option_duration_secs")]
    pub tool_timeout_sec: Option<Duration>,

    /// Explicit allow-list of tools exposed from this server. When set, only these tools will be registered.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enabled_tools: Option<Vec<String>>,

    /// Explicit deny-list of tools. These tools will be removed after applying `enabled_tools`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disabled_tools: Option<Vec<String>>,

    /// Optional OAuth scopes to request during MCP login.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scopes: Option<Vec<String>>,
}

// Raw MCP config shape used for deserialization and JSON Schema generation.
// Keep this in sync with the validation logic in `McpServerConfig`.
#[derive(Deserialize, Clone, JsonSchema)]
#[schemars(deny_unknown_fields)]
pub(crate) struct RawMcpServerConfig {
    // stdio
    pub command: Option<String>,
    #[serde(default)]
    pub args: Option<Vec<String>>,
    #[serde(default)]
    pub env: Option<HashMap<String, String>>,
    #[serde(default)]
    pub env_vars: Option<Vec<String>>,
    #[serde(default)]
    pub cwd: Option<PathBuf>,
    pub http_headers: Option<HashMap<String, String>>,
    #[serde(default)]
    pub env_http_headers: Option<HashMap<String, String>>,

    // streamable_http
    pub url: Option<String>,
    pub bearer_token: Option<String>,
    pub bearer_token_env_var: Option<String>,

    // shared
    #[serde(default)]
    pub startup_timeout_sec: Option<f64>,
    #[serde(default)]
    pub startup_timeout_ms: Option<u64>,
    #[serde(default, with = "option_duration_secs")]
    #[schemars(with = "Option<f64>")]
    pub tool_timeout_sec: Option<Duration>,
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub enabled_tools: Option<Vec<String>>,
    #[serde(default)]
    pub disabled_tools: Option<Vec<String>>,
    #[serde(default)]
    pub scopes: Option<Vec<String>>,
}

impl<'de> Deserialize<'de> for McpServerConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let mut raw = RawMcpServerConfig::deserialize(deserializer)?;

        let startup_timeout_sec = match (raw.startup_timeout_sec, raw.startup_timeout_ms) {
            (Some(sec), _) => {
                let duration = Duration::try_from_secs_f64(sec).map_err(SerdeError::custom)?;
                Some(duration)
            }
            (None, Some(ms)) => Some(Duration::from_millis(ms)),
            (None, None) => None,
        };
        let tool_timeout_sec = raw.tool_timeout_sec;
        let enabled = raw.enabled.unwrap_or_else(default_enabled);
        let enabled_tools = raw.enabled_tools.clone();
        let disabled_tools = raw.disabled_tools.clone();
        let scopes = raw.scopes.clone();

        fn throw_if_set<E, T>(transport: &str, field: &str, value: Option<&T>) -> Result<(), E>
        where
            E: SerdeError,
        {
            if value.is_none() {
                return Ok(());
            }
            Err(E::custom(format!(
                "{field} is not supported for {transport}",
            )))
        }

        let transport = if let Some(command) = raw.command.clone() {
            throw_if_set("stdio", "url", raw.url.as_ref())?;
            throw_if_set(
                "stdio",
                "bearer_token_env_var",
                raw.bearer_token_env_var.as_ref(),
            )?;
            throw_if_set("stdio", "bearer_token", raw.bearer_token.as_ref())?;
            throw_if_set("stdio", "http_headers", raw.http_headers.as_ref())?;
            throw_if_set("stdio", "env_http_headers", raw.env_http_headers.as_ref())?;
            McpServerTransportConfig::Stdio {
                command,
                args: raw.args.clone().unwrap_or_default(),
                env: raw.env.clone(),
                env_vars: raw.env_vars.clone().unwrap_or_default(),
                cwd: raw.cwd.take(),
            }
        } else if let Some(url) = raw.url.clone() {
            throw_if_set("streamable_http", "args", raw.args.as_ref())?;
            throw_if_set("streamable_http", "env", raw.env.as_ref())?;
            throw_if_set("streamable_http", "env_vars", raw.env_vars.as_ref())?;
            throw_if_set("streamable_http", "cwd", raw.cwd.as_ref())?;
            throw_if_set("streamable_http", "bearer_token", raw.bearer_token.as_ref())?;
            McpServerTransportConfig::StreamableHttp {
                url,
                bearer_token_env_var: raw.bearer_token_env_var.clone(),
                http_headers: raw.http_headers.clone(),
                env_http_headers: raw.env_http_headers.take(),
            }
        } else {
            return Err(SerdeError::custom("invalid transport"));
        };

        Ok(Self {
            transport,
            startup_timeout_sec,
            tool_timeout_sec,
            enabled,
            disabled_reason: None,
            enabled_tools,
            disabled_tools,
            scopes,
        })
    }
}

const fn default_enabled() -> bool {
    true
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema)]
#[serde(untagged, deny_unknown_fields, rename_all = "snake_case")]
pub enum McpServerTransportConfig {
    /// https://modelcontextprotocol.io/specification/2025-06-18/basic/transports#stdio
    Stdio {
        command: String,
        #[serde(default)]
        args: Vec<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        env: Option<HashMap<String, String>>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        env_vars: Vec<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        cwd: Option<PathBuf>,
    },
    /// https://modelcontextprotocol.io/specification/2025-06-18/basic/transports#streamable-http
    StreamableHttp {
        url: String,
        /// Name of the environment variable to read for an HTTP bearer token.
        /// When set, requests will include the token via `Authorization: Bearer <token>`.
        /// The actual secret value must be provided via the environment.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        bearer_token_env_var: Option<String>,
        /// Additional HTTP headers to include in requests to this server.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        http_headers: Option<HashMap<String, String>>,
        /// HTTP headers where the value is sourced from an environment variable.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        env_http_headers: Option<HashMap<String, String>>,
    },
}

mod option_duration_secs {
    use serde::Deserialize;
    use serde::Deserializer;
    use serde::Serializer;
    use std::time::Duration;

    pub fn serialize<S>(value: &Option<Duration>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match value {
            Some(duration) => serializer.serialize_some(&duration.as_secs_f64()),
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Duration>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let secs = Option::<f64>::deserialize(deserializer)?;
        secs.map(|secs| Duration::try_from_secs_f64(secs).map_err(serde::de::Error::custom))
            .transpose()
    }
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone, PartialEq, JsonSchema)]
pub enum UriBasedFileOpener {
    #[serde(rename = "vscode")]
    VsCode,

    #[serde(rename = "vscode-insiders")]
    VsCodeInsiders,

    #[serde(rename = "windsurf")]
    Windsurf,

    #[serde(rename = "cursor")]
    Cursor,

    /// Option to disable the URI-based file opener.
    #[serde(rename = "none")]
    None,
}

impl UriBasedFileOpener {
    pub fn get_scheme(&self) -> Option<&str> {
        match self {
            UriBasedFileOpener::VsCode => Some("vscode"),
            UriBasedFileOpener::VsCodeInsiders => Some("vscode-insiders"),
            UriBasedFileOpener::Windsurf => Some("windsurf"),
            UriBasedFileOpener::Cursor => Some("cursor"),
            UriBasedFileOpener::None => None,
        }
    }
}

/// Settings that govern if and what will be written to `~/.jarvis/history.jsonl`.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default, JsonSchema)]
#[schemars(deny_unknown_fields)]
pub struct History {
    /// If true, history entries will not be written to disk.
    pub persistence: HistoryPersistence,

    /// If set, the maximum size of the history file in bytes. The oldest entries
    /// are dropped once the file exceeds this limit.
    pub max_bytes: Option<usize>,
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone, PartialEq, Default, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum HistoryPersistence {
    /// Save all history entries to disk.
    #[default]
    SaveAll,
    /// Do not write history to disk.
    None,
}

// ===== Analytics configuration =====

/// Analytics settings loaded from config.toml. Fields are optional so we can apply defaults.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default, JsonSchema)]
#[schemars(deny_unknown_fields)]
pub struct AnalyticsConfigToml {
    /// When `false`, disables analytics across Jarvis product surfaces in this profile.
    pub enabled: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default, JsonSchema)]
#[schemars(deny_unknown_fields)]
pub struct FeedbackConfigToml {
    /// When `false`, disables the feedback flow across Jarvis product surfaces.
    pub enabled: Option<bool>,
}

// ===== Messaging configuration =====

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default, JsonSchema)]
#[schemars(deny_unknown_fields)]
pub struct MessagingConfigToml {
    /// When `false`, disables messaging integrations (WhatsApp, Telegram).
    #[serde(default = "default_enabled")]
    pub enabled: bool,

    /// WhatsApp Business API configuration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub whatsapp: Option<WhatsAppConfigToml>,

    /// Telegram Bot API configuration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub telegram: Option<TelegramConfigToml>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema)]
#[schemars(deny_unknown_fields)]
pub struct WhatsAppConfigToml {
    /// When `false`, disables WhatsApp integration.
    #[serde(default = "default_enabled")]
    pub enabled: bool,

    /// Base URL for WhatsApp Business API. Defaults to "https://graph.facebook.com/v18.0".
    #[serde(default)]
    pub api_url: Option<String>,

    /// Access token for WhatsApp Business API.
    /// Can be set via environment variable WHATSAPP_ACCESS_TOKEN.
    pub access_token: Option<String>,

    /// Verify token for webhook verification.
    /// Can be set via environment variable WHATSAPP_VERIFY_TOKEN.
    pub verify_token: Option<String>,

    /// Phone number ID from WhatsApp Business API.
    /// Can be set via environment variable WHATSAPP_PHONE_NUMBER_ID.
    pub phone_number_id: Option<String>,

    /// Port for WhatsApp webhook server. Defaults to 8080.
    #[serde(default)]
    pub webhook_port: Option<u16>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema)]
#[schemars(deny_unknown_fields)]
pub struct TelegramConfigToml {
    /// When `false`, disables Telegram integration.
    #[serde(default = "default_enabled")]
    pub enabled: bool,

    /// Bot token from BotFather.
    /// Can be set via environment variable TELEGRAM_BOT_TOKEN.
    pub bot_token: Option<String>,

    /// Webhook URL for Telegram (optional, for setting webhook via API).
    pub webhook_url: Option<String>,

    /// Port for Telegram webhook server. Defaults to 8081.
    #[serde(default)]
    pub webhook_port: Option<u16>,

    /// Secret token for webhook validation (optional).
    /// Can be set via environment variable TELEGRAM_WEBHOOK_SECRET.
    pub webhook_secret: Option<String>,
}

/// Effective messaging settings after defaults are applied.
#[derive(Debug, Clone, PartialEq)]
pub struct MessagingConfig {
    pub enabled: bool,
    pub whatsapp: Option<WhatsAppConfig>,
    pub telegram: Option<TelegramConfig>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WhatsAppConfig {
    pub enabled: bool,
    pub api_url: String,
    pub access_token: String,
    pub verify_token: String,
    pub phone_number_id: String,
    pub webhook_port: u16,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TelegramConfig {
    pub enabled: bool,
    pub bot_token: String,
    pub webhook_url: Option<String>,
    pub webhook_port: u16,
    pub webhook_secret: Option<String>,
}

impl Default for MessagingConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            whatsapp: None,
            telegram: None,
        }
    }
}

impl From<MessagingConfigToml> for MessagingConfig {
    fn from(toml: MessagingConfigToml) -> Self {
        let whatsapp = toml.whatsapp.and_then(|w| {
            if !w.enabled {
                return None;
            }
            let access_token = w.access_token
                .or_else(|| std::env::var("WHATSAPP_ACCESS_TOKEN").ok())
                .unwrap_or_default();
            let verify_token = w.verify_token
                .or_else(|| std::env::var("WHATSAPP_VERIFY_TOKEN").ok())
                .unwrap_or_default();
            let phone_number_id = w.phone_number_id
                .or_else(|| std::env::var("WHATSAPP_PHONE_NUMBER_ID").ok())
                .unwrap_or_default();

            if access_token.is_empty() || verify_token.is_empty() || phone_number_id.is_empty() {
                return None;
            }

            Some(WhatsAppConfig {
                enabled: true,
                api_url: w.api_url.unwrap_or_else(|| {
                    "https://graph.facebook.com/v18.0".to_string()
                }),
                access_token,
                verify_token,
                phone_number_id,
                webhook_port: w.webhook_port.unwrap_or(8080),
            })
        });

        let telegram = toml.telegram.and_then(|t| {
            if !t.enabled {
                return None;
            }
            let bot_token = t.bot_token
                .or_else(|| std::env::var("TELEGRAM_BOT_TOKEN").ok())
                .unwrap_or_default();
            let webhook_secret = t.webhook_secret
                .or_else(|| std::env::var("TELEGRAM_WEBHOOK_SECRET").ok());

            if bot_token.is_empty() {
                return None;
            }

            Some(TelegramConfig {
                enabled: true,
                bot_token,
                webhook_url: t.webhook_url,
                webhook_port: t.webhook_port.unwrap_or(8081),
                webhook_secret,
            })
        });

        Self {
            enabled: toml.enabled && (whatsapp.is_some() || telegram.is_some()),
            whatsapp,
            telegram,
        }
    }
}

// ===== GitHub configuration =====

/// GitHub settings loaded from config.toml. Fields are optional so we can apply defaults.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default, JsonSchema)]
#[schemars(deny_unknown_fields)]
pub struct GitHubConfigToml {
    /// Name of the secret in jarvis-secrets where the GitHub PAT is stored.
    /// Defaults to "GITHUB_PAT".
    pub pat_secret_name: Option<String>,

    /// Base URL for GitHub API. Defaults to "https://api.github.com".
    /// Useful for GitHub Enterprise Server.
    pub api_base_url: Option<String>,
}

/// Effective GitHub settings after defaults are applied.
#[derive(Debug, Clone, PartialEq)]
pub struct GitHubConfig {
    pub pat_secret_name: String,
    pub api_base_url: String,
}

impl Default for GitHubConfig {
    fn default() -> Self {
        GitHubConfig {
            pat_secret_name: "GITHUB_PAT".to_string(),
            api_base_url: "https://api.github.com".to_string(),
        }
    }
}

impl From<GitHubConfigToml> for GitHubConfig {
    fn from(toml: GitHubConfigToml) -> Self {
        GitHubConfig {
            pat_secret_name: toml
                .pat_secret_name
                .unwrap_or_else(|| "GITHUB_PAT".to_string()),
            api_base_url: toml
                .api_base_url
                .unwrap_or_else(|| "https://api.github.com".to_string()),
        }
    }
}

// ===== OTEL configuration =====

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum OtelHttpProtocol {
    /// Binary payload
    Binary,
    /// JSON payload
    Json,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default, JsonSchema)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "kebab-case")]
pub struct OtelTlsConfig {
    pub ca_certificate: Option<AbsolutePathBuf>,
    pub client_certificate: Option<AbsolutePathBuf>,
    pub client_private_key: Option<AbsolutePathBuf>,
}

/// Which OTEL exporter to use.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "kebab-case")]
pub enum OtelExporterKind {
    None,
    Statsig,
    OtlpHttp {
        endpoint: String,
        #[serde(default)]
        headers: HashMap<String, String>,
        protocol: OtelHttpProtocol,
        #[serde(default)]
        tls: Option<OtelTlsConfig>,
    },
    OtlpGrpc {
        endpoint: String,
        #[serde(default)]
        headers: HashMap<String, String>,
        #[serde(default)]
        tls: Option<OtelTlsConfig>,
    },
}

/// OTEL settings loaded from config.toml. Fields are optional so we can apply defaults.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default, JsonSchema)]
#[schemars(deny_unknown_fields)]
pub struct OtelConfigToml {
    /// Log user prompt in traces
    pub log_user_prompt: Option<bool>,

    /// Mark traces with environment (dev, staging, prod, test). Defaults to dev.
    pub environment: Option<String>,

    /// Optional log exporter
    pub exporter: Option<OtelExporterKind>,

    /// Optional trace exporter
    pub trace_exporter: Option<OtelExporterKind>,
}

/// Effective OTEL settings after defaults are applied.
#[derive(Debug, Clone, PartialEq)]
pub struct OtelConfig {
    pub log_user_prompt: bool,
    pub environment: String,
    pub exporter: OtelExporterKind,
    pub trace_exporter: OtelExporterKind,
    pub metrics_exporter: OtelExporterKind,
}

impl Default for OtelConfig {
    fn default() -> Self {
        OtelConfig {
            log_user_prompt: false,
            environment: DEFAULT_OTEL_ENVIRONMENT.to_owned(),
            exporter: OtelExporterKind::None,
            trace_exporter: OtelExporterKind::None,
            metrics_exporter: OtelExporterKind::Statsig,
        }
    }
}

#[derive(Serialize, Debug, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum Notifications {
    Enabled(bool),
    Custom(Vec<String>),
}

impl Default for Notifications {
    fn default() -> Self {
        Self::Enabled(true)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, JsonSchema, Default)]
#[serde(rename_all = "lowercase")]
pub enum NotificationMethod {
    #[default]
    Auto,
    Osc9,
    Bel,
}

impl fmt::Display for NotificationMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NotificationMethod::Auto => write!(f, "auto"),
            NotificationMethod::Osc9 => write!(f, "osc9"),
            NotificationMethod::Bel => write!(f, "bel"),
        }
    }
}

/// Collection of settings that are specific to the TUI.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default, JsonSchema)]
#[schemars(deny_unknown_fields)]
pub struct Tui {
    /// Enable desktop notifications from the TUI when the terminal is unfocused.
    /// Defaults to `true`.
    #[serde(default)]
    pub notifications: Notifications,

    /// Notification method to use for unfocused terminal notifications.
    /// Defaults to `auto`.
    #[serde(default)]
    pub notification_method: NotificationMethod,

    /// Enable animations (welcome screen, shimmer effects, spinners).
    /// Defaults to `true`.
    #[serde(default = "default_true")]
    pub animations: bool,

    /// Show startup tooltips in the TUI welcome screen.
    /// Defaults to `true`.
    #[serde(default = "default_true")]
    pub show_tooltips: bool,

    /// Start the TUI in the specified collaboration mode (plan/default).
    /// Defaults to unset.
    #[serde(default)]
    pub experimental_mode: Option<ModeKind>,

    /// Controls whether the TUI uses the terminal's alternate screen buffer.
    ///
    /// - `auto` (default): Disable alternate screen in Zellij, enable elsewhere.
    /// - `always`: Always use alternate screen (original behavior).
    /// - `never`: Never use alternate screen (inline mode only, preserves scrollback).
    ///
    /// Using alternate screen provides a cleaner fullscreen experience but prevents
    /// scrollback in terminal multiplexers like Zellij that follow the xterm spec.
    #[serde(default)]
    pub alternate_screen: AltScreenMode,
}

const fn default_true() -> bool {
    true
}

/// Settings for notices we display to users via the tui and app-server clients
/// (primarily the Jarvis IDE extension). NOTE: these are different from
/// notifications - notices are warnings, NUX screens, acknowledgements, etc.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default, JsonSchema)]
pub struct Notice {
    /// Tracks whether the user has acknowledged the full access warning prompt.
    pub hide_full_access_warning: Option<bool>,
    /// Tracks whether the user has acknowledged the Windows world-writable directories warning.
    pub hide_world_writable_warning: Option<bool>,
    /// Tracks whether the user opted out of the rate limit model switch reminder.
    pub hide_rate_limit_model_nudge: Option<bool>,
    /// Tracks whether the user has seen the model migration prompt
    pub hide_gpt5_1_migration_prompt: Option<bool>,
    /// Tracks whether the user has seen the gpt-5.1-Jarvis-max migration prompt
    #[serde(rename = "hide_gpt-5.1-Jarvis-max_migration_prompt")]
    pub hide_gpt_5_1_codex_max_migration_prompt: Option<bool>,
    /// Tracks acknowledged model migrations as old->new model slug mappings.
    #[serde(default)]
    pub model_migrations: BTreeMap<String, String>,
}

impl Notice {
    /// referenced by config_edit helpers when writing notice flags
    pub(crate) const TABLE_KEY: &'static str = "notice";
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema)]
#[schemars(deny_unknown_fields)]
pub struct SkillConfig {
    pub path: AbsolutePathBuf,
    pub enabled: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq, Eq, JsonSchema)]
#[schemars(deny_unknown_fields)]
pub struct SkillsConfig {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub config: Vec<SkillConfig>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default, JsonSchema)]
#[schemars(deny_unknown_fields)]
pub struct SandboxWorkspaceWrite {
    #[serde(default)]
    pub writable_roots: Vec<AbsolutePathBuf>,
    #[serde(default)]
    pub network_access: bool,
    #[serde(default)]
    pub exclude_tmpdir_env_var: bool,
    #[serde(default)]
    pub exclude_slash_tmp: bool,
}

impl From<SandboxWorkspaceWrite> for jarvis_app_server_protocol::SandboxSettings {
    fn from(sandbox_workspace_write: SandboxWorkspaceWrite) -> Self {
        Self {
            writable_roots: sandbox_workspace_write.writable_roots,
            network_access: Some(sandbox_workspace_write.network_access),
            exclude_tmpdir_env_var: Some(sandbox_workspace_write.exclude_tmpdir_env_var),
            exclude_slash_tmp: Some(sandbox_workspace_write.exclude_slash_tmp),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum ShellEnvironmentPolicyInherit {
    /// "Core" environment variables for the platform. On UNIX, this would
    /// include HOME, LOGNAME, PATH, SHELL, and USER, among others.
    Core,

    /// Inherits the full environment from the parent process.
    #[default]
    All,

    /// Do not inherit any environment variables from the parent process.
    None,
}

/// Policy for building the `env` when spawning a process via either the
/// `shell` or `local_shell` tool.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default, JsonSchema)]
#[schemars(deny_unknown_fields)]
pub struct ShellEnvironmentPolicyToml {
    pub inherit: Option<ShellEnvironmentPolicyInherit>,

    pub ignore_default_excludes: Option<bool>,

    /// List of regular expressions.
    pub exclude: Option<Vec<String>>,

    pub r#set: Option<HashMap<String, String>>,

    /// List of regular expressions.
    pub include_only: Option<Vec<String>>,

    pub experimental_use_profile: Option<bool>,
}

pub type EnvironmentVariablePattern = WildMatchPattern<'*', '?'>;

/// Deriving the `env` based on this policy works as follows:
/// 1. Create an initial map based on the `inherit` policy.
/// 2. If `ignore_default_excludes` is false, filter the map using the default
///    exclude pattern(s), which are: `"*KEY*"`, `"*SECRET*"`, and `"*TOKEN*"`.
/// 3. If `exclude` is not empty, filter the map using the provided patterns.
/// 4. Insert any entries from `r#set` into the map.
/// 5. If non-empty, filter the map using the `include_only` patterns.
#[derive(Debug, Clone, PartialEq)]
pub struct ShellEnvironmentPolicy {
    /// Starting point when building the environment.
    pub inherit: ShellEnvironmentPolicyInherit,

    /// True to skip the check to exclude default environment variables that
    /// contain "KEY", "SECRET", or "TOKEN" in their name. Defaults to true.
    pub ignore_default_excludes: bool,

    /// Environment variable names to exclude from the environment.
    pub exclude: Vec<EnvironmentVariablePattern>,

    /// (key, value) pairs to insert in the environment.
    pub r#set: HashMap<String, String>,

    /// Environment variable names to retain in the environment.
    pub include_only: Vec<EnvironmentVariablePattern>,

    /// If true, the shell profile will be used to run the command.
    pub use_profile: bool,
}

impl From<ShellEnvironmentPolicyToml> for ShellEnvironmentPolicy {
    fn from(toml: ShellEnvironmentPolicyToml) -> Self {
        // Default to inheriting the full environment when not specified.
        let inherit = toml.inherit.unwrap_or(ShellEnvironmentPolicyInherit::All);
        let ignore_default_excludes = toml.ignore_default_excludes.unwrap_or(true);
        let exclude = toml
            .exclude
            .unwrap_or_default()
            .into_iter()
            .map(|s| EnvironmentVariablePattern::new_case_insensitive(&s))
            .collect();
        let r#set = toml.r#set.unwrap_or_default();
        let include_only = toml
            .include_only
            .unwrap_or_default()
            .into_iter()
            .map(|s| EnvironmentVariablePattern::new_case_insensitive(&s))
            .collect();
        let use_profile = toml.experimental_use_profile.unwrap_or(false);

        Self {
            inherit,
            ignore_default_excludes,
            exclude,
            r#set,
            include_only,
            use_profile,
        }
    }
}

impl Default for ShellEnvironmentPolicy {
    fn default() -> Self {
        Self {
            inherit: ShellEnvironmentPolicyInherit::All,
            ignore_default_excludes: true,
            exclude: Vec::new(),
            r#set: HashMap::new(),
            include_only: Vec::new(),
            use_profile: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn deserialize_stdio_command_server_config() {
        let cfg: McpServerConfig = toml::from_str(
            r#"
            command = "echo"
        "#,
        )
        .expect("should deserialize command config");

        assert_eq!(
            cfg.transport,
            McpServerTransportConfig::Stdio {
                command: "echo".to_string(),
                args: vec![],
                env: None,
                env_vars: Vec::new(),
                cwd: None,
            }
        );
        assert!(cfg.enabled);
        assert!(cfg.enabled_tools.is_none());
        assert!(cfg.disabled_tools.is_none());
    }

    #[test]
    fn deserialize_stdio_command_server_config_with_args() {
        let cfg: McpServerConfig = toml::from_str(
            r#"
            command = "echo"
            args = ["hello", "world"]
        "#,
        )
        .expect("should deserialize command config");

        assert_eq!(
            cfg.transport,
            McpServerTransportConfig::Stdio {
                command: "echo".to_string(),
                args: vec!["hello".to_string(), "world".to_string()],
                env: None,
                env_vars: Vec::new(),
                cwd: None,
            }
        );
        assert!(cfg.enabled);
    }

    #[test]
    fn deserialize_stdio_command_server_config_with_arg_with_args_and_env() {
        let cfg: McpServerConfig = toml::from_str(
            r#"
            command = "echo"
            args = ["hello", "world"]
            env = { "FOO" = "BAR" }
        "#,
        )
        .expect("should deserialize command config");

        assert_eq!(
            cfg.transport,
            McpServerTransportConfig::Stdio {
                command: "echo".to_string(),
                args: vec!["hello".to_string(), "world".to_string()],
                env: Some(HashMap::from([("FOO".to_string(), "BAR".to_string())])),
                env_vars: Vec::new(),
                cwd: None,
            }
        );
        assert!(cfg.enabled);
    }

    #[test]
    fn deserialize_stdio_command_server_config_with_env_vars() {
        let cfg: McpServerConfig = toml::from_str(
            r#"
            command = "echo"
            env_vars = ["FOO", "BAR"]
        "#,
        )
        .expect("should deserialize command config with env_vars");

        assert_eq!(
            cfg.transport,
            McpServerTransportConfig::Stdio {
                command: "echo".to_string(),
                args: vec![],
                env: None,
                env_vars: vec!["FOO".to_string(), "BAR".to_string()],
                cwd: None,
            }
        );
    }

    #[test]
    fn deserialize_stdio_command_server_config_with_cwd() {
        let cfg: McpServerConfig = toml::from_str(
            r#"
            command = "echo"
            cwd = "/tmp"
        "#,
        )
        .expect("should deserialize command config with cwd");

        assert_eq!(
            cfg.transport,
            McpServerTransportConfig::Stdio {
                command: "echo".to_string(),
                args: vec![],
                env: None,
                env_vars: Vec::new(),
                cwd: Some(PathBuf::from("/tmp")),
            }
        );
    }

    #[test]
    fn deserialize_disabled_server_config() {
        let cfg: McpServerConfig = toml::from_str(
            r#"
            command = "echo"
            enabled = false
        "#,
        )
        .expect("should deserialize disabled server config");

        assert!(!cfg.enabled);
    }

    #[test]
    fn deserialize_streamable_http_server_config() {
        let cfg: McpServerConfig = toml::from_str(
            r#"
            url = "https://example.com/mcp"
        "#,
        )
        .expect("should deserialize http config");

        assert_eq!(
            cfg.transport,
            McpServerTransportConfig::StreamableHttp {
                url: "https://example.com/mcp".to_string(),
                bearer_token_env_var: None,
                http_headers: None,
                env_http_headers: None,
            }
        );
        assert!(cfg.enabled);
    }

    #[test]
    fn deserialize_streamable_http_server_config_with_env_var() {
        let cfg: McpServerConfig = toml::from_str(
            r#"
            url = "https://example.com/mcp"
            bearer_token_env_var = "GITHUB_TOKEN"
        "#,
        )
        .expect("should deserialize http config");

        assert_eq!(
            cfg.transport,
            McpServerTransportConfig::StreamableHttp {
                url: "https://example.com/mcp".to_string(),
                bearer_token_env_var: Some("GITHUB_TOKEN".to_string()),
                http_headers: None,
                env_http_headers: None,
            }
        );
        assert!(cfg.enabled);
    }

    #[test]
    fn deserialize_streamable_http_server_config_with_headers() {
        let cfg: McpServerConfig = toml::from_str(
            r#"
            url = "https://example.com/mcp"
            http_headers = { "X-Foo" = "bar" }
            env_http_headers = { "X-Token" = "TOKEN_ENV" }
        "#,
        )
        .expect("should deserialize http config with headers");

        assert_eq!(
            cfg.transport,
            McpServerTransportConfig::StreamableHttp {
                url: "https://example.com/mcp".to_string(),
                bearer_token_env_var: None,
                http_headers: Some(HashMap::from([("X-Foo".to_string(), "bar".to_string())])),
                env_http_headers: Some(HashMap::from([(
                    "X-Token".to_string(),
                    "TOKEN_ENV".to_string()
                )])),
            }
        );
    }

    #[test]
    fn deserialize_server_config_with_tool_filters() {
        let cfg: McpServerConfig = toml::from_str(
            r#"
            command = "echo"
            enabled_tools = ["allowed"]
            disabled_tools = ["blocked"]
        "#,
        )
        .expect("should deserialize tool filters");

        assert_eq!(cfg.enabled_tools, Some(vec!["allowed".to_string()]));
        assert_eq!(cfg.disabled_tools, Some(vec!["blocked".to_string()]));
    }

    #[test]
    fn deserialize_rejects_command_and_url() {
        toml::from_str::<McpServerConfig>(
            r#"
            command = "echo"
            url = "https://example.com"
        "#,
        )
        .expect_err("should reject command+url");
    }

    #[test]
    fn deserialize_rejects_env_for_http_transport() {
        toml::from_str::<McpServerConfig>(
            r#"
            url = "https://example.com"
            env = { "FOO" = "BAR" }
        "#,
        )
        .expect_err("should reject env for http transport");
    }

    #[test]
    fn deserialize_rejects_headers_for_stdio() {
        toml::from_str::<McpServerConfig>(
            r#"
            command = "echo"
            http_headers = { "X-Foo" = "bar" }
        "#,
        )
        .expect_err("should reject http_headers for stdio transport");

        toml::from_str::<McpServerConfig>(
            r#"
            command = "echo"
            env_http_headers = { "X-Foo" = "BAR_ENV" }
        "#,
        )
        .expect_err("should reject env_http_headers for stdio transport");
    }

    #[test]
    fn deserialize_rejects_inline_bearer_token_field() {
        let err = toml::from_str::<McpServerConfig>(
            r#"
            url = "https://example.com"
            bearer_token = "secret"
        "#,
        )
        .expect_err("should reject bearer_token field");

        assert!(err.to_string().contains("bearer_token"));
    }

    // ===== Messaging configuration tests =====

    #[test]
    fn deserialize_messaging_config_default() {
        let cfg: MessagingConfigToml = toml::from_str("").unwrap_or_default();
        assert!(cfg.enabled);
        assert!(cfg.whatsapp.is_none());
        assert!(cfg.telegram.is_none());
    }

    #[test]
    fn deserialize_messaging_config_disabled() {
        let cfg: MessagingConfigToml = toml::from_str(
            r#"
            enabled = false
        "#,
        )
        .expect("should deserialize disabled messaging config");
        assert!(!cfg.enabled);
    }

    #[test]
    fn deserialize_whatsapp_config() {
        let cfg: MessagingConfigToml = toml::from_str(
            r#"
            [whatsapp]
            enabled = true
            access_token = "test_token"
            verify_token = "test_verify"
            phone_number_id = "123456"
            webhook_port = 9090
        "#,
        )
        .expect("should deserialize WhatsApp config");
        assert!(cfg.enabled);
        let whatsapp = cfg.whatsapp.expect("should have WhatsApp config");
        assert!(whatsapp.enabled);
        assert_eq!(whatsapp.access_token, Some("test_token".to_string()));
        assert_eq!(whatsapp.verify_token, Some("test_verify".to_string()));
        assert_eq!(whatsapp.phone_number_id, Some("123456".to_string()));
        assert_eq!(whatsapp.webhook_port, Some(9090));
    }

    #[test]
    fn deserialize_telegram_config() {
        let cfg: MessagingConfigToml = toml::from_str(
            r#"
            [telegram]
            enabled = true
            bot_token = "test_bot_token"
            webhook_url = "https://example.com/webhook"
            webhook_port = 9091
            webhook_secret = "test_secret"
        "#,
        )
        .expect("should deserialize Telegram config");
        assert!(cfg.enabled);
        let telegram = cfg.telegram.expect("should have Telegram config");
        assert!(telegram.enabled);
        assert_eq!(telegram.bot_token, Some("test_bot_token".to_string()));
        assert_eq!(telegram.webhook_url, Some("https://example.com/webhook".to_string()));
        assert_eq!(telegram.webhook_port, Some(9091));
        assert_eq!(telegram.webhook_secret, Some("test_secret".to_string()));
    }

    #[test]
    fn messaging_config_from_toml_with_env_vars() {
        std::env::set_var("WHATSAPP_ACCESS_TOKEN", "env_token");
        std::env::set_var("WHATSAPP_VERIFY_TOKEN", "env_verify");
        std::env::set_var("WHATSAPP_PHONE_NUMBER_ID", "env_phone");
        std::env::set_var("TELEGRAM_BOT_TOKEN", "env_bot_token");

        let toml = MessagingConfigToml {
            enabled: true,
            whatsapp: Some(WhatsAppConfigToml {
                enabled: true,
                api_url: None,
                access_token: None,
                verify_token: None,
                phone_number_id: None,
                webhook_port: None,
            }),
            telegram: Some(TelegramConfigToml {
                enabled: true,
                bot_token: None,
                webhook_url: None,
                webhook_port: None,
                webhook_secret: None,
            }),
        };

        let config: MessagingConfig = toml.into();
        assert!(config.enabled);
        assert!(config.whatsapp.is_some());
        assert!(config.telegram.is_some());

        let whatsapp = config.whatsapp.unwrap();
        assert_eq!(whatsapp.access_token, "env_token");
        assert_eq!(whatsapp.verify_token, "env_verify");
        assert_eq!(whatsapp.phone_number_id, "env_phone");

        let telegram = config.telegram.unwrap();
        assert_eq!(telegram.bot_token, "env_bot_token");

        std::env::remove_var("WHATSAPP_ACCESS_TOKEN");
        std::env::remove_var("WHATSAPP_VERIFY_TOKEN");
        std::env::remove_var("WHATSAPP_PHONE_NUMBER_ID");
        std::env::remove_var("TELEGRAM_BOT_TOKEN");
    }

    #[test]
    fn messaging_config_from_toml_missing_credentials() {
        let toml = MessagingConfigToml {
            enabled: true,
            whatsapp: Some(WhatsAppConfigToml {
                enabled: true,
                api_url: None,
                access_token: None,
                verify_token: None,
                phone_number_id: None,
                webhook_port: None,
            }),
            telegram: Some(TelegramConfigToml {
                enabled: true,
                bot_token: None,
                webhook_url: None,
                webhook_port: None,
                webhook_secret: None,
            }),
        };

        let config: MessagingConfig = toml.into();
        // Should be disabled if credentials are missing
        assert!(!config.enabled);
        assert!(config.whatsapp.is_none());
        assert!(config.telegram.is_none());
    }
}

#[cfg(test)]
mod messaging_tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_messaging_config_default() {
        let config = MessagingConfig::default();
        assert!(!config.enabled);
        assert!(config.whatsapp.is_none());
        assert!(config.telegram.is_none());
    }

    #[test]
    fn test_whatsapp_config_defaults() {
        let toml = MessagingConfigToml {
            enabled: true,
            whatsapp: Some(WhatsAppConfigToml {
                enabled: true,
                api_url: None,
                access_token: Some("token".to_string()),
                verify_token: Some("verify".to_string()),
                phone_number_id: Some("123".to_string()),
                webhook_port: None,
            }),
            telegram: None,
        };

        let config: MessagingConfig = toml.into();
        assert!(config.enabled);
        let whatsapp = config.whatsapp.expect("should have WhatsApp config");
        assert_eq!(whatsapp.api_url, "https://graph.facebook.com/v18.0");
        assert_eq!(whatsapp.webhook_port, 8080);
    }

    #[test]
    fn test_telegram_config_defaults() {
        let toml = MessagingConfigToml {
            enabled: true,
            whatsapp: None,
            telegram: Some(TelegramConfigToml {
                enabled: true,
                bot_token: Some("token".to_string()),
                webhook_url: None,
                webhook_port: None,
                webhook_secret: None,
            }),
        };

        let config: MessagingConfig = toml.into();
        assert!(config.enabled);
        let telegram = config.telegram.expect("should have Telegram config");
        assert_eq!(telegram.webhook_port, 8081);
    }

    #[test]
    fn test_whatsapp_disabled() {
        let toml = MessagingConfigToml {
            enabled: true,
            whatsapp: Some(WhatsAppConfigToml {
                enabled: false,
                api_url: None,
                access_token: Some("token".to_string()),
                verify_token: Some("verify".to_string()),
                phone_number_id: Some("123".to_string()),
                webhook_port: None,
            }),
            telegram: None,
        };

        let config: MessagingConfig = toml.into();
        assert!(!config.enabled);
        assert!(config.whatsapp.is_none());
    }

    #[test]
    fn test_telegram_disabled() {
        let toml = MessagingConfigToml {
            enabled: true,
            whatsapp: None,
            telegram: Some(TelegramConfigToml {
                enabled: false,
                bot_token: Some("token".to_string()),
                webhook_url: None,
                webhook_port: None,
                webhook_secret: None,
            }),
        };

        let config: MessagingConfig = toml.into();
        assert!(!config.enabled);
        assert!(config.telegram.is_none());
    }
}
