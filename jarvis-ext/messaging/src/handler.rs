use crate::message::IncomingMessage;

/// Handler para processar mensagens recebidas
#[async_trait::async_trait]
pub trait MessageHandler: Send + Sync {
    /// Processa uma mensagem recebida
    async fn handle_message(&self, message: IncomingMessage) -> anyhow::Result<()>;
}
