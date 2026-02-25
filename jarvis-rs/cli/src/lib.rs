pub mod agent_cmd;
pub mod analytics_cmd;
pub mod autonomous_cmd;
pub mod context_cmd;
pub mod daemon_cmd;
pub mod debug_sandbox;
mod exit_status;
pub mod intent_cmd;
pub mod login;
pub mod safety_cmd;
pub mod skills_cmd;

use clap::Parser;
use jarvis_common::CliConfigOverrides;

/// Command structure for Seatbelt sandbox (macOS only).
#[derive(Debug, Parser)]
pub struct SeatbeltCommand {
    #[clap(flatten)]
    pub config_overrides: CliConfigOverrides,

    /// Enable full auto mode (low-friction sandboxed automatic execution).
    #[arg(long = "full-auto", default_value_t = false)]
    pub full_auto: bool,

    /// Log Seatbelt denials to stderr.
    #[arg(long = "log-denials", default_value_t = false)]
    pub log_denials: bool,

    /// Command to run under sandbox.
    #[arg(required = true, trailing_var_arg = true)]
    pub command: Vec<String>,
}

/// Command structure for Landlock sandbox (Linux only).
#[derive(Debug, Parser)]
pub struct LandlockCommand {
    #[clap(flatten)]
    pub config_overrides: CliConfigOverrides,

    /// Enable full auto mode (low-friction sandboxed automatic execution).
    #[arg(long = "full-auto", default_value_t = false)]
    pub full_auto: bool,

    /// Command to run under sandbox.
    #[arg(required = true, trailing_var_arg = true)]
    pub command: Vec<String>,
}

/// Command structure for Windows sandbox.
#[derive(Debug, Parser)]
pub struct WindowsCommand {
    #[clap(flatten)]
    pub config_overrides: CliConfigOverrides,

    /// Enable full auto mode (low-friction sandboxed automatic execution).
    #[arg(long = "full-auto", default_value_t = false)]
    pub full_auto: bool,

    /// Command to run under sandbox.
    #[arg(required = true, trailing_var_arg = true)]
    pub command: Vec<String>,
}
