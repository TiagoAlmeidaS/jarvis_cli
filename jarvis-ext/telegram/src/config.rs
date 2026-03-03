use serde::Deserialize;
use serde::Serialize;

/// Configuração para integração Telegram
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramConfig {
    pub bot_token: String,
    pub webhook_url: Option<String>,
    pub webhook_port: u16,
    /// Secret token para validação de webhook (opcional)
    #[serde(default)]
    pub webhook_secret: Option<String>,
}

impl Default for TelegramConfig {
    fn default() -> Self {
        Self {
            bot_token: String::new(),
            webhook_url: None,
            webhook_port: 8081,
            webhook_secret: None,
        }
    }
}
