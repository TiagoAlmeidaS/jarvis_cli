//! Integração de mensageria com o core do Jarvis
//!
//! Este módulo conecta mensagens recebidas de plataformas de mensageria
//! (WhatsApp, Telegram) com o sistema de tools do Jarvis.

pub mod command_parser;
pub mod handler;
pub mod init;
pub mod router;

pub use handler::MessageToJarvisHandler;
pub use init::{initialize_messaging_servers, initialize_messaging_servers_from_thread};
pub use router::MessagingRouter;
