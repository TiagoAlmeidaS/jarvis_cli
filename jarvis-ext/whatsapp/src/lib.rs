//! Integração com WhatsApp Business API
//!
//! Este crate fornece integração com WhatsApp para permitir que o Jarvis
//! receba e envie mensagens através da plataforma WhatsApp.

pub mod client;
pub mod config;
pub mod message;
pub mod platform;
pub mod webhook;

pub use platform::WhatsAppPlatform;
