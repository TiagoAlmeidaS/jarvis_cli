use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Plataforma de mensageria
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Platform {
    WhatsApp,
    Telegram,
}

impl Platform {
    pub fn as_str(&self) -> &'static str {
        match self {
            Platform::WhatsApp => "whatsapp",
            Platform::Telegram => "telegram",
        }
    }
}

/// Tipo de mensagem suportado
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageType {
    Text(String),
    Image {
        url: String,
        caption: Option<String>,
    },
    Document {
        url: String,
        filename: String,
    },
    Audio {
        url: String,
    },
    Video {
        url: String,
        caption: Option<String>,
    },
    Location {
        latitude: f64,
        longitude: f64,
    },
    Command {
        command: String,
        args: Vec<String>,
    },
}

/// Contato/usuário que envia mensagem
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contact {
    pub id: String,
    pub name: Option<String>,
    pub username: Option<String>,
}

/// Mensagem recebida de uma plataforma
#[derive(Debug, Clone)]
pub struct IncomingMessage {
    pub id: String,
    pub platform: Platform,
    pub from: Contact,
    pub chat_id: String,
    pub message_type: MessageType,
    pub timestamp: DateTime<Utc>,
    pub context: Option<crate::conversation::ConversationContext>,
}

/// Mensagem a ser enviada para uma plataforma
#[derive(Debug, Clone)]
pub struct OutgoingMessage {
    pub chat_id: String,
    pub message_type: MessageType,
    pub reply_to: Option<String>,
}

/// ID de mensagem retornado após envio
#[derive(Debug, Clone)]
pub struct MessageId {
    pub id: String,
    pub platform: Platform,
}
