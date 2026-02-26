use serde::Deserialize;
use serde::Serialize;

/// Configuração para integração WhatsApp
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhatsAppConfig {
    pub api_url: String,
    pub access_token: String,
    pub verify_token: String,
    pub phone_number_id: String,
    pub webhook_port: u16,
}

impl Default for WhatsAppConfig {
    fn default() -> Self {
        Self {
            api_url: "https://graph.facebook.com/v18.0".to_string(),
            access_token: String::new(),
            verify_token: String::new(),
            phone_number_id: String::new(),
            webhook_port: 8080,
        }
    }
}
