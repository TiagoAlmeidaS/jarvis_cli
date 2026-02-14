use jarvis_messaging::handler::MessageHandler;
use jarvis_messaging::message::{IncomingMessage, MessageId, OutgoingMessage};
use jarvis_messaging::platform::MessagingPlatform;

use crate::client::TelegramClient;
use crate::config::TelegramConfig;

/// Implementação da plataforma Telegram
pub struct TelegramPlatform {
    client: TelegramClient,
    config: TelegramConfig,
}

impl TelegramPlatform {
    pub fn new(client: TelegramClient, config: TelegramConfig) -> Self {
        Self { client, config }
    }
}

#[async_trait::async_trait]
impl MessagingPlatform for TelegramPlatform {
    async fn send_message(&self, message: OutgoingMessage) -> anyhow::Result<MessageId> {
        self.client.send_message(message).await
    }

    async fn get_conversation_history(
        &self,
        _chat_id: &str,
    ) -> anyhow::Result<Vec<IncomingMessage>> {
        // TODO: Implementar busca de histórico via API
        Ok(Vec::new())
    }

    async fn start_webhook_server(&self, handler: Box<dyn MessageHandler>) -> anyhow::Result<()> {
        use crate::webhook::TelegramWebhookServer;
        let server = TelegramWebhookServer::new(handler, self.config.clone());
        server.start().await
    }

    fn platform_name(&self) -> &'static str {
        "Telegram"
    }
}
