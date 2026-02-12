use crate::handler::MessageHandler;
use crate::message::{IncomingMessage, MessageId, OutgoingMessage};

/// Trait para plataformas de mensageria
#[async_trait::async_trait]
pub trait MessagingPlatform: Send + Sync {
    /// Envia uma mensagem
    async fn send_message(&self, message: OutgoingMessage) -> anyhow::Result<MessageId>;

    /// Obtém histórico de conversa
    async fn get_conversation_history(
        &self,
        chat_id: &str,
    ) -> anyhow::Result<Vec<IncomingMessage>>;

    /// Inicia servidor webhook para receber mensagens
    async fn start_webhook_server(
        &self,
        handler: Box<dyn MessageHandler>,
    ) -> anyhow::Result<()>;

    /// Nome da plataforma
    fn platform_name(&self) -> &'static str;
}
