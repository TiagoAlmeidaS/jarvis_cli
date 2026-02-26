use jarvis_messaging::message::MessageId;
use jarvis_messaging::message::MessageType;
use jarvis_messaging::message::OutgoingMessage;
use jarvis_messaging::message::Platform;
use reqwest::Client;

use crate::config::TelegramConfig;

/// Cliente para Telegram Bot API
pub struct TelegramClient {
    client: Client,
    config: TelegramConfig,
}

impl TelegramClient {
    pub fn new(config: TelegramConfig) -> Self {
        let client = Client::new();
        Self { client, config }
    }

    /// Send a Markdown-formatted text message to the given chat.
    ///
    /// This is a convenience wrapper around [`send_message`] that builds an
    /// [`OutgoingMessage`] with `parse_mode = "Markdown"` and link-preview
    /// disabled — matching the format used by the daemon notifier.
    pub async fn send_text(&self, chat_id: &str, text: &str) -> anyhow::Result<MessageId> {
        self.send_message(OutgoingMessage::text(chat_id, text))
            .await
    }

    pub async fn send_message(&self, message: OutgoingMessage) -> anyhow::Result<MessageId> {
        let url = format!(
            "https://api.telegram.org/bot{}/sendMessage",
            self.config.bot_token
        );

        let payload = self.build_message_payload(&message)?;

        let response = self.client.post(&url).json(&payload).send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("Telegram API error: {} - {}", status, body);
        }

        let response_json: serde_json::Value = response.json().await?;
        let message_id = response_json["result"]["message_id"]
            .as_u64()
            .ok_or_else(|| anyhow::anyhow!("Missing message ID in response"))?
            .to_string();

        Ok(MessageId {
            id: message_id,
            platform: Platform::Telegram,
        })
    }

    fn build_message_payload(
        &self,
        message: &OutgoingMessage,
    ) -> anyhow::Result<serde_json::Value> {
        let mut payload = serde_json::json!({
            "chat_id": message.chat_id,
        });

        match &message.message_type {
            MessageType::Text(text) => {
                payload["text"] = serde_json::json!(text);
            }
            MessageType::Image { url, caption } => {
                payload["photo"] = serde_json::json!(url);
                if let Some(caption) = caption {
                    payload["caption"] = serde_json::json!(caption);
                }
            }
            _ => {
                anyhow::bail!("Unsupported message type for Telegram");
            }
        }

        if let Some(reply_to) = &message.reply_to {
            payload["reply_to_message_id"] = serde_json::json!(reply_to);
        }

        if let Some(parse_mode) = &message.parse_mode {
            payload["parse_mode"] = serde_json::json!(parse_mode);
        }

        if message.disable_web_page_preview {
            payload["disable_web_page_preview"] = serde_json::json!(true);
        }

        Ok(payload)
    }
}
