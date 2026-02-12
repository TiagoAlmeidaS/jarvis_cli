use axum::{
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    response::Json,
    routing::get,
    Router,
};
use std::sync::Arc;
use jarvis_messaging::handler::MessageHandler;
use jarvis_messaging::message::{Contact, IncomingMessage, MessageType, Platform};
use jarvis_messaging::rate_limit::RateLimiter;
use jarvis_messaging::security::validate_whatsapp_token;
use serde::Deserialize;
use tokio::sync::Mutex;
use tokio::time::{interval, Duration};

use crate::config::WhatsAppConfig;

#[derive(Debug, Deserialize)]
struct WebhookQuery {
    #[serde(rename = "hub.mode")]
    mode: String,
    #[serde(rename = "hub.verify_token")]
    verify_token: String,
    #[serde(rename = "hub.challenge")]
    challenge: String,
}

#[derive(Debug, Deserialize)]
struct WebhookPayload {
    entry: Vec<WebhookEntry>,
}

#[derive(Debug, Deserialize)]
struct WebhookEntry {
    changes: Vec<WebhookChange>,
}

#[derive(Debug, Deserialize)]
struct WebhookChange {
    value: WebhookValue,
}

#[derive(Debug, Deserialize)]
struct WebhookValue {
    messages: Option<Vec<WhatsAppMessage>>,
}

#[derive(Debug, Deserialize)]
struct WhatsAppMessage {
    id: String,
    from: String,
    timestamp: String,
    #[serde(rename = "type")]
    message_type: String,
    text: Option<WhatsAppText>,
}

#[derive(Debug, Deserialize)]
struct WhatsAppText {
    body: String,
}

#[derive(Clone)]
struct WebhookState {
    handler: Arc<Mutex<Box<dyn MessageHandler>>>,
    config: WhatsAppConfig,
    rate_limiter: Arc<RateLimiter>,
}

/// Servidor webhook para WhatsApp
pub struct WhatsAppWebhookServer {
    config: WhatsAppConfig,
    handler: Arc<Mutex<Box<dyn MessageHandler>>>,
    rate_limiter: Arc<RateLimiter>,
}

impl WhatsAppWebhookServer {
    pub fn new(handler: Box<dyn MessageHandler>, config: WhatsAppConfig) -> Self {
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
            .route("/webhook", get(verify_webhook))
            .route("/webhook", axum::routing::post(handle_webhook))
            .with_state(state)
    }

    /// Inicia o servidor webhook
    pub async fn start(&self) -> anyhow::Result<()> {
        let app = self.create_router();
        let addr = format!("0.0.0.0:{}", self.config.webhook_port);
        
        tracing::info!("Starting WhatsApp webhook server on {}", addr);
        
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

async fn verify_webhook(
    Query(params): Query<WebhookQuery>,
    State(state): State<WebhookState>,
) -> Result<String, StatusCode> {
    // Valida o token
    if params.mode != "subscribe" {
        return Err(StatusCode::BAD_REQUEST);
    }

    if !validate_whatsapp_token(&params.verify_token, &state.config.verify_token) {
        tracing::warn!("Invalid verify token received");
        return Err(StatusCode::FORBIDDEN);
    }

    tracing::info!("Webhook verified successfully");
    Ok(params.challenge)
}

async fn handle_webhook(
    State(state): State<WebhookState>,
    headers: HeaderMap,
    Json(payload): Json<WebhookPayload>,
) -> StatusCode {
    // Rate limiting por IP ou chat_id
    let client_key = headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown")
        .to_string();

    if !state.rate_limiter.check(&client_key).await {
        tracing::warn!("Rate limit exceeded for {}", client_key);
        return StatusCode::TOO_MANY_REQUESTS;
    }

    for entry in payload.entry {
        for change in entry.changes {
            if let Some(messages) = change.value.messages {
                for msg in messages {
                    // Rate limiting por chat_id também
                    let chat_key = format!("chat:{}", msg.from);
                    if !state.rate_limiter.check(&chat_key).await {
                        tracing::warn!("Rate limit exceeded for chat {}", msg.from);
                        continue;
                    }

                    if let Some(text) = msg.text {
                        // Parse timestamp
                        let timestamp = msg.timestamp.parse::<i64>()
                            .ok()
                            .and_then(|ts| chrono::DateTime::from_timestamp(ts, 0))
                            .unwrap_or_else(chrono::Utc::now);

                        let incoming = IncomingMessage {
                            id: msg.id.clone(),
                            platform: Platform::WhatsApp,
                            from: Contact {
                                id: msg.from.clone(),
                                name: None,
                                username: None,
                            },
                            chat_id: msg.from.clone(),
                            message_type: MessageType::Text(text.body),
                            timestamp,
                            context: None,
                        };

                        // Processa mensagem em background para não bloquear resposta
                        let handler = Arc::clone(&state.handler);
                        tokio::spawn(async move {
                            let handler = handler.lock().await;
                            if let Err(e) = handler.handle_message(incoming).await {
                                tracing::error!("Error handling WhatsApp message: {}", e);
                            }
                        });
                    }
                }
            }
        }
    }

    StatusCode::OK
}
