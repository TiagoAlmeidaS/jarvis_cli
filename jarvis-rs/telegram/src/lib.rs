//! Integração com Telegram Bot API
//!
//! Este crate fornece integração com Telegram para permitir que o Jarvis
//! receba e envie mensagens através da plataforma Telegram.

pub mod client;
pub mod config;
pub mod message;
pub mod platform;
pub mod webhook;

pub use platform::TelegramPlatform;
