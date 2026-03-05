//! WebSocket handler for real-time daemon updates.

use axum::extract::Query;
use axum::extract::State;
use axum::extract::WebSocketUpgrade;
use axum::http::StatusCode;
use axum::response::Response;
use futures_util::SinkExt;
use futures_util::StreamExt;
use jarvis_daemon_common::DaemonDb;
use serde::Deserialize;
use serde::Serialize;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::state::AppState;

#[derive(Deserialize)]
pub struct WebSocketQuery {
    pub token: Option<String>,
}

#[derive(Serialize)]
pub struct WebSocketEvent {
    pub event_type: String,
    pub data: serde_json::Value,
}

/// WebSocket handler for real-time updates.
pub async fn websocket_handler(
    State(state): State<AppState>,
    Query(params): Query<WebSocketQuery>,
    ws: WebSocketUpgrade,
) -> Result<Response, StatusCode> {
    // Validate token if provided
    if let Some(token) = params.token {
        let api_config = state.config.api.as_ref().ok_or(StatusCode::UNAUTHORIZED)?;
        if token != api_config.api_key {
            return Err(StatusCode::UNAUTHORIZED);
        }
    }

    let db = state.daemon_db.clone();

    Ok(ws.on_upgrade(move |socket| handle_socket(socket, db)))
}

async fn handle_socket(socket: axum::extract::ws::WebSocket, db: Option<Arc<DaemonDb>>) {
    let (sender, mut receiver) = socket.split();
    let sender = Arc::new(Mutex::new(sender));

    // Send initial status
    if let Some(db) = &db {
        if let Ok(status) = get_daemon_status(db).await {
            let msg = axum::extract::ws::Message::Text(
                serde_json::to_string(&status).unwrap_or_default().into(),
            );
            let _ = sender.lock().await.send(msg).await;
        }
    }

    // Spawn a task to periodically send updates
    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(5));
    let db_clone = db.clone();
    let sender_clone = Arc::clone(&sender);

    tokio::spawn(async move {
        loop {
            interval.tick().await;
            if let Some(db) = &db_clone {
                if let Ok(status) = get_daemon_status(db).await {
                    let msg = axum::extract::ws::Message::Text(
                        serde_json::to_string(&status).unwrap_or_default().into(),
                    );
                    let _ = sender_clone.lock().await.send(msg).await;
                }
            }
        }
    });

    // Handle incoming messages
    while let Some(msg) = receiver.next().await {
        match msg {
            Ok(axum::extract::ws::Message::Text(text)) => {
                // Handle client messages if needed
                if text == "ping" {
                    let _ = sender
                        .lock()
                        .await
                        .send(axum::extract::ws::Message::Text("pong".into()))
                        .await;
                }
            }
            Ok(axum::extract::ws::Message::Close(_)) => {
                break;
            }
            Err(e) => {
                tracing::warn!("WebSocket error: {e}");
                break;
            }
            _ => {}
        }
    }
}

async fn get_daemon_status(db: &DaemonDb) -> Result<WebSocketEvent, anyhow::Error> {
    use jarvis_daemon_common::ContentFilter;
    use jarvis_daemon_common::ContentStatus;
    use jarvis_daemon_common::GoalFilter;
    use jarvis_daemon_common::GoalStatus;
    use jarvis_daemon_common::JobStatus;

    let pipelines = db.list_pipelines(false).await?;
    let enabled = pipelines.iter().filter(|p| p.enabled).count();
    let running_jobs = db.count_running_jobs().await?;
    let pending_proposals = db.count_pending_proposals().await?;

    let content_filter = ContentFilter {
        since_days: Some(7),
        status: Some(ContentStatus::Published),
        ..Default::default()
    };
    let recent_content = db.list_content(&content_filter).await?;
    let revenue_summary = db.revenue_summary(30).await?;

    Ok(WebSocketEvent {
        event_type: "status_update".to_string(),
        data: serde_json::json!({
            "pipelines": {
                "total": pipelines.len(),
                "enabled": enabled,
            },
            "jobs": {
                "running": running_jobs,
            },
            "content": {
                "published_last_7d": recent_content.len(),
            },
            "proposals": {
                "pending": pending_proposals,
            },
            "revenue": {
                "total_usd_30d": revenue_summary.total_usd,
            },
        }),
    })
}
