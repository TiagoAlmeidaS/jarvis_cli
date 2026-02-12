//! Crate comum para integrações de mensageria (WhatsApp, Telegram, etc.)
//!
//! Este crate fornece tipos e traits compartilhados para todas as plataformas
//! de mensageria integradas ao Jarvis.

pub mod conversation;
pub mod handler;
pub mod message;
pub mod platform;
pub mod rate_limit;
pub mod security;

pub use conversation::{ConversationContext, ConversationManager};
pub use handler::MessageHandler;
pub use message::{IncomingMessage, MessageType, OutgoingMessage, Platform};
pub use platform::MessagingPlatform;
pub use rate_limit::RateLimiter;
pub use security::{validate_telegram_signature, validate_whatsapp_token};
