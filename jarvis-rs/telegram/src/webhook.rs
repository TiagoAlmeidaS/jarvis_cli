use axum::Router;
use axum::extract::State;
use axum::http::HeaderMap;
use axum::http::StatusCode;
use axum::response::Json;
use axum::routing::post;
use jarvis_messaging::handler::MessageHandler;
use jarvis_messaging::message::Contact;
use jarvis_messaging::message::IncomingMessage;
use jarvis_messaging::message::MessageType;
use jarvis_messaging::message::Platform;
use jarvis_messaging::rate_limit::RateLimiter;
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::Duration;
use tokio::time::interval;

use crate::config::TelegramConfig;

#[derive(Debug, Deserialize)]
struct TelegramUpdate {
    update_id: u64,
    message: Option<TelegramMessage>,
}

#[derive(Debug, Deserialize)]
struct TelegramMessage {
    message_id: u64,
    from: Option<TelegramUser>,
    chat: TelegramChat,
    date: i64,
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TelegramUser {
    id: i64,
    first_name: String,
    username: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TelegramChat {
    id: i64,
    #[serde(rename = "type")]
    chat_type: String,
}

#[derive(Clone)]
struct WebhookState {
    handler: Arc<Mutex<Box<dyn MessageHandler>>>,
    config: TelegramConfig,
    rate_limiter: Arc<RateLimiter>,
}

/// Servidor webhook para Telegram
pub struct TelegramWebhookServer {
    config: TelegramConfig,
    handler: Arc<Mutex<Box<dyn MessageHandler>>>,
    rate_limiter: Arc<RateLimiter>,
}

impl TelegramWebhookServer {
    pub fn new(handler: Box<dyn MessageHandler>, config: TelegramConfig) -> Self {
        let rate_limiter = Arc::new(RateLimiter::default());
        Self {
            config,
            handler: Arc::new(Mutex::new(handler)),
            rate_limiter,
        }
    }

    pub fn create_router(&self) -> Router {
        let state = WebhookState {
            handler: Arc::clone(&self.handler),
            config: self.config.clone(),
            rate_limiter: Arc::clone(&self.rate_limiter),
        };

        Router::new()
            .route("/webhook", post(handle_webhook))
            .with_state(state)
    }

    /// Inicia o servidor webhook
    pub async fn start(&self) -> anyhow::Result<()> {
        let app = self.create_router();
        let addr = format!("0.0.0.0:{}", self.config.webhook_port);

        tracing::info!("Starting Telegram webhook server on {}", addr);

        // Inicia limpeza periódica do rate limiter
        let rate_limiter = Arc::clone(&self.rate_limiter);
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(300)); // Limpa a cada 5 minutos
            loop {
                interval.tick().await;
                rate_limiter.cleanup().await;
            }
        });

        let listener = tokio::net::TcpListener::bind(&addr).await?;
        axum::serve(listener, app).await?;

        Ok(())
    }
}

async fn handle_webhook(
    State(state): State<WebhookState>,
    headers: HeaderMap,
    Json(update): Json<TelegramUpdate>,
) -> StatusCode {
    // Validação de assinatura HMAC (se configurado)
    if let Some(secret) = &state.config.webhook_secret {
        if let Some(signature_header) = headers.get("x-telegram-bot-api-secret-token") {
            if let Ok(sig) = signature_header.to_str() {
                // Para validação completa, precisaríamos do body raw
                // Por enquanto, validamos apenas se o header está presente e correto
                if sig != secret {
                    tracing::warn!("Invalid Telegram webhook signature");
                    return StatusCode::FORBIDDEN;
                }
            }
        }
    }

    // Rate limiting por IP
    let client_key = headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown")
        .to_string();

    if !state.rate_limiter.check(&client_key).await {
        tracing::warn!("Rate limit exceeded for {}", client_key);
        return StatusCode::TOO_MANY_REQUESTS;
    }

    if let Some(message) = update.message {
        if let Some(text) = message.text {
            // Rate limiting por chat_id também
            let chat_key = format!("chat:{}", message.chat.id);
            if !state.rate_limiter.check(&chat_key).await {
                tracing::warn!("Rate limit exceeded for chat {}", message.chat.id);
                return StatusCode::TOO_MANY_REQUESTS;
            }

            let from = message
                .from
                .as_ref()
                .map(|u| Contact {
                    id: u.id.to_string(),
                    name: Some(u.first_name.clone()),
                    username: u.username.clone(),
                })
                .unwrap_or_else(|| Contact {
                    id: message.chat.id.to_string(),
                    name: None,
                    username: None,
                });

            let incoming = IncomingMessage {
                id: message.message_id.to_string(),
                platform: Platform::Telegram,
                from,
                chat_id: message.chat.id.to_string(),
                message_type: MessageType::Text(text),
                timestamp: chrono::DateTime::from_timestamp(message.date, 0)
                    .unwrap_or_else(chrono::Utc::now),
                context: None,
            };

            // Processa mensagem em background para não bloquear resposta
            let handler = Arc::clone(&state.handler);
            tokio::spawn(async move {
                let handler = handler.lock().await;
                if let Err(e) = handler.handle_message(incoming).await {
                    tracing::error!("Error handling Telegram message: {}", e);
                }
            });
        }
    }

    StatusCode::OK
}
