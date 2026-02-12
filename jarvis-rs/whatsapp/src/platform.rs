use jarvis_messaging::handler::MessageHandler;
use jarvis_messaging::message::{IncomingMessage, MessageId, OutgoingMessage};
use jarvis_messaging::platform::MessagingPlatform;

use crate::client::WhatsAppClient;
use crate::config::WhatsAppConfig;

/// Implementação da plataforma WhatsApp
pub struct WhatsAppPlatform {
    client: WhatsAppClient,
    config: WhatsAppConfig,
}

impl WhatsAppPlatform {
    pub fn new(client: WhatsAppClient, config: WhatsAppConfig) -> Self {
        Self { client, config }
    }
}

#[async_trait::async_trait]
impl MessagingPlatform for WhatsAppPlatform {
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

    async fn start_webhook_server(
        &self,
        handler: Box<dyn MessageHandler>,
    ) -> anyhow::Result<()> {
        use crate::webhook::WhatsAppWebhookServer;
        let server = WhatsAppWebhookServer::new(handler, self.config.clone());
        server.start().await
    }

    fn platform_name(&self) -> &'static str {
        "WhatsApp"
    }
}
