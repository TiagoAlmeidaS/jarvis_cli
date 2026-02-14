use jarvis_messaging::message::{MessageId, MessageType, OutgoingMessage, Platform};
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

        Ok(payload)
    }
}
