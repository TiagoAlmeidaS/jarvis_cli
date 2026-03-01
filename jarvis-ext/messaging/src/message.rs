use chrono::DateTime;
use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;

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
    /// Parse mode for rich text (e.g. "Markdown", "MarkdownV2", "HTML").
    pub parse_mode: Option<String>,
    /// Disable link previews in the message (Telegram-specific).
    pub disable_web_page_preview: bool,
}

impl OutgoingMessage {
    /// Create a simple text message with Markdown parse mode and link previews disabled.
    pub fn text(chat_id: impl Into<String>, text: impl Into<String>) -> Self {
        Self {
            chat_id: chat_id.into(),
            message_type: MessageType::Text(text.into()),
            reply_to: None,
            parse_mode: Some("Markdown".to_string()),
            disable_web_page_preview: true,
        }
    }

    /// Create a plain text message with no parse mode.
    pub fn plain_text(chat_id: impl Into<String>, text: impl Into<String>) -> Self {
        Self {
            chat_id: chat_id.into(),
            message_type: MessageType::Text(text.into()),
            reply_to: None,
            parse_mode: None,
            disable_web_page_preview: false,
        }
    }
}

/// ID de mensagem retornado após envio
#[derive(Debug, Clone)]
pub struct MessageId {
    pub id: String,
    pub platform: Platform,
}
