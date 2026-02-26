use jarvis_messaging::message::MessageId;
use jarvis_messaging::message::MessageType;
use jarvis_messaging::message::OutgoingMessage;
use jarvis_messaging::message::Platform;
use reqwest::Client;

use crate::config::WhatsAppConfig;

impl WhatsAppClient {
    pub fn config(&self) -> &WhatsAppConfig {
        &self.config
    }
}

/// Cliente para WhatsApp Business API
pub struct WhatsAppClient {
    client: Client,
    config: WhatsAppConfig,
}

impl WhatsAppClient {
    pub fn new(config: WhatsAppConfig) -> Self {
        let client = Client::new();
        Self { client, config }
    }

    pub async fn send_message(&self, message: OutgoingMessage) -> anyhow::Result<MessageId> {
        let url = format!(
            "{}/{}/messages",
            self.config.api_url, self.config.phone_number_id
        );

        let payload = self.build_message_payload(&message)?;

        let response = self
            .client
            .post(&url)
            .bearer_auth(&self.config.access_token)
            .json(&payload)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("WhatsApp API error: {} - {}", status, body);
        }

        let response_json: serde_json::Value = response.json().await?;
        let message_id = response_json["messages"][0]["id"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing message ID in response"))?
            .to_string();

        Ok(MessageId {
            id: message_id,
            platform: Platform::WhatsApp,
        })
    }

    fn build_message_payload(
        &self,
        message: &OutgoingMessage,
    ) -> anyhow::Result<serde_json::Value> {
        let mut payload = serde_json::json!({
            "messaging_product": "whatsapp",
            "recipient_type": "individual",
            "to": message.chat_id,
        });

        match &message.message_type {
            MessageType::Text(text) => {
                payload["type"] = serde_json::json!("text");
                payload["text"] = serde_json::json!({
                    "preview_url": false,
                    "body": text
                });
            }
            MessageType::Image { url, caption } => {
                payload["type"] = serde_json::json!("image");
                payload["image"] = serde_json::json!({
                    "link": url,
                    "caption": caption
                });
            }
            _ => {
                anyhow::bail!("Unsupported message type for WhatsApp");
            }
        }

        Ok(payload)
    }
}
