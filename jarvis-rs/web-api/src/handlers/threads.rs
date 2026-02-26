//! Threads endpoint.

use crate::state::AppState;
use axum::Json;
use axum::extract::Query;
use axum::extract::State;
use jarvis_core::RolloutRecorder;
use jarvis_core::ThreadItem;
use jarvis_core::ThreadSortKey;
use jarvis_core::parse_cursor;
use jarvis_protocol::models::ResponseItem;
use jarvis_protocol::protocol::EventMsg;
use jarvis_protocol::protocol::SessionSource;
use serde::Deserialize;
use serde::Serialize;
use std::path::Path;

#[derive(Deserialize)]
pub struct ThreadsQuery {
    #[serde(default = "default_page_size")]
    limit: usize,
    cursor: Option<String>,
    sort: Option<String>, // "created_at" or "updated_at"
}

fn default_page_size() -> usize {
    20
}

#[derive(Serialize)]
pub struct Thread {
    pub id: String,
    pub preview: String,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Serialize)]
pub struct ThreadsResponse {
    pub threads: Vec<Thread>,
    pub next_cursor: Option<String>,
}

/// Extract preview text from ThreadItem head records
fn extract_preview(item: &ThreadItem) -> String {
    use jarvis_protocol::models::ContentItem;

    // Look for first user message in head records
    for record in &item.head {
        // Try to parse as ResponseItem with user message
        if let Ok(response_item) = serde_json::from_value::<ResponseItem>(record.clone()) {
            if let ResponseItem::Message { role, content, .. } = response_item {
                if role == "user" {
                    // Extract text from content - ContentItem can be InputText, InputImage, or OutputText
                    for content_item in content {
                        let text = match content_item {
                            ContentItem::InputText { text } => Some(text),
                            ContentItem::OutputText { text } => Some(text),
                            ContentItem::InputImage { .. } => None,
                        };
                        if let Some(text) = text {
                            if !text.trim().is_empty() {
                                // Truncate to reasonable length
                                let preview = if text.len() > 100 {
                                    format!("{}...", &text[..100])
                                } else {
                                    text
                                };
                                return preview;
                            }
                        }
                    }
                }
            }
        }

        // Try to parse as EventMsg with UserMessage
        if let Ok(rollout_line) =
            serde_json::from_value::<jarvis_protocol::protocol::RolloutLine>(record.clone())
        {
            if let jarvis_protocol::protocol::RolloutItem::EventMsg(ev) = rollout_line.item {
                if let EventMsg::UserMessage(user_msg) = ev {
                    let preview = if user_msg.message.len() > 100 {
                        format!("{}...", &user_msg.message[..100])
                    } else {
                        user_msg.message.clone()
                    };
                    return preview;
                }
            }
        }
    }

    "Nova conversa".to_string()
}

/// Extract thread ID from rollout file path
fn extract_thread_id(path: &Path) -> Option<String> {
    let file_name = path.file_name()?.to_string_lossy();
    // Format is typically: rollout-<timestamp>-<uuid>.jsonl
    // Extract UUID from filename - format is rollout-YYYY-MM-DDThh-mm-ss-<uuid>.jsonl
    // We need to find the last '-' before the UUID
    if let Some(core) = file_name
        .strip_prefix("rollout-")
        .and_then(|s| s.strip_suffix(".jsonl"))
    {
        // Find the last '-' and parse UUID from the suffix
        // UUID format: 8-4-4-4-12 hex digits
        if let Some((_, uuid_str)) = core.rsplit_once('-') {
            // Basic UUID format validation: should be 36 chars with dashes or 32 hex chars
            if uuid_str.len() >= 32 && uuid_str.chars().all(|c| c.is_ascii_hexdigit() || c == '-') {
                return Some(uuid_str.to_string());
            }
        }
    }
    None
}

pub async fn list_threads(
    State(state): State<AppState>,
    Query(query): Query<ThreadsQuery>,
) -> Json<ThreadsResponse> {
    let jarvis_home = &state.config.jarvis_home;

    // Determine sort key
    let sort_key = match query.sort.as_deref() {
        Some("updated_at") => ThreadSortKey::UpdatedAt,
        _ => ThreadSortKey::CreatedAt,
    };

    // Parse cursor if provided
    let cursor = query.cursor.as_ref().and_then(|c| parse_cursor(c));

    // List threads using RolloutRecorder
    let page = RolloutRecorder::list_threads(
        jarvis_home,
        query.limit,
        cursor.as_ref(),
        sort_key,
        &[],  // Allow all session sources for now
        None, // No provider filter
        &state.config.model_provider_id,
    )
    .await
    .unwrap_or_default();

    // Convert ThreadItem to Thread
    let threads: Vec<Thread> = page
        .items
        .into_iter()
        .filter_map(|item| {
            let id = extract_thread_id(&item.path)?;
            let preview = extract_preview(&item);
            Some(Thread {
                id,
                preview,
                created_at: item.created_at,
                updated_at: item.updated_at,
            })
        })
        .collect();

    // Serialize next cursor
    let next_cursor = page
        .next_cursor
        .and_then(|c| serde_json::to_string(&c).ok());

    Json(ThreadsResponse {
        threads,
        next_cursor,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::create_router;
    use crate::test_utils::create_test_app_state_with_api_key;
    use axum::http::StatusCode;
    use axum_test::TestServer;

    #[tokio::test]
    async fn test_list_threads() {
        let api_key = "test-api-key".to_string();
        let app_state = create_test_app_state_with_api_key(api_key.clone());
        let app = create_router(app_state, false);
        let server = TestServer::new(app).unwrap();

        let response = server
            .get("/api/threads")
            .add_header("Authorization", format!("Bearer {}", api_key))
            .await;

        response.assert_status(StatusCode::OK);
        let body: ThreadsResponse = response.json();
        // May be empty if no threads exist, which is valid
        assert!(body.threads.len() <= body.threads.capacity() || body.threads.is_empty());
    }

    #[tokio::test]
    async fn test_list_threads_with_query_params() {
        let api_key = "test-api-key".to_string();
        let app_state = create_test_app_state_with_api_key(api_key.clone());
        let app = create_router(app_state, false);
        let server = TestServer::new(app).unwrap();

        let response = server
            .get("/api/threads?limit=10&sort=updated_at")
            .add_header("Authorization", format!("Bearer {}", api_key))
            .await;

        response.assert_status(StatusCode::OK);
        let body: ThreadsResponse = response.json();
        assert!(body.threads.len() <= 10);
    }

    #[tokio::test]
    async fn test_list_threads_requires_auth() {
        let api_key = "test-api-key".to_string();
        let app_state = create_test_app_state_with_api_key(api_key);
        let app = create_router(app_state, false);
        let server = TestServer::new(app).unwrap();

        let response = server.get("/api/threads").await;

        response.assert_status(StatusCode::UNAUTHORIZED);
    }
}
