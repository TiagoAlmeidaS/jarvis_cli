//! Chat endpoint.

use crate::state::AppState;
use anyhow::Result;
use axum::body::Body;
use axum::body::to_bytes;
use axum::extract::State;
use axum::http::HeaderMap;
use axum::http::StatusCode;
use axum::response::Response;
use jarvis_core::NewThread;
use jarvis_core::ThreadManager;
use jarvis_core::models_manager::manager::RefreshStrategy;
use jarvis_core::protocol::Event;
use jarvis_core::protocol::EventMsg;
use jarvis_core::protocol::Op;
use jarvis_protocol::config_types::ReasoningSummary as ReasoningSummaryConfig;
use jarvis_protocol::openai_models::ReasoningEffort as ReasoningEffortConfig;
use jarvis_protocol::protocol::AskForApproval;
use jarvis_protocol::protocol::SandboxPolicy;
use jarvis_protocol::protocol::SessionSource;
use jarvis_protocol::user_input::UserInput;
use serde::Deserialize;
use serde::Serialize;
use std::sync::Arc;
use tracing::error;

#[derive(Deserialize)]
pub struct ChatRequest {
    pub prompt: String,
    pub thread_id: Option<String>,
}

// Constants for validation
const MAX_PROMPT_LENGTH: usize = 100_000; // 100KB max prompt
const MIN_PROMPT_LENGTH: usize = 1;

/// Validate chat request
fn validate_chat_request(request: &ChatRequest) -> Result<(), (StatusCode, String)> {
    // Validate prompt length
    if request.prompt.len() > MAX_PROMPT_LENGTH {
        return Err((
            StatusCode::PAYLOAD_TOO_LARGE,
            format!(
                "Prompt too long. Maximum length is {} characters",
                MAX_PROMPT_LENGTH
            ),
        ));
    }

    if request.prompt.trim().len() < MIN_PROMPT_LENGTH {
        return Err((
            StatusCode::BAD_REQUEST,
            "Prompt cannot be empty".to_string(),
        ));
    }

    // Validate thread_id format if provided (should be UUID)
    if let Some(ref thread_id) = request.thread_id {
        if !thread_id.trim().is_empty() {
            // Basic UUID validation: should be 36 chars with dashes or 32 hex chars
            let trimmed = thread_id.trim();
            let is_valid_uuid = (trimmed.len() == 36 && trimmed.matches('-').count() == 4)
                || (trimmed.len() == 32 && trimmed.chars().all(|c| c.is_ascii_hexdigit()));

            if !is_valid_uuid {
                return Err((
                    StatusCode::BAD_REQUEST,
                    format!(
                        "Invalid thread_id format. Expected UUID, got: {}",
                        thread_id
                    ),
                ));
            }
        }
    }

    Ok(())
}

#[derive(Serialize)]
pub struct ChatResponse {
    pub reply: String,
    pub thread_id: String,
}

// Handler that accepts both JSON and form data
pub async fn handle_chat(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Body,
) -> Result<Response, axum::response::Response> {
    // Check content-type to determine how to parse
    let content_type = headers
        .get("content-type")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");

    let wants_html = headers
        .get("accept")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("")
        .contains("text/html");

    let chat_request = if content_type.contains("application/json") {
        // Parse as JSON
        let body_bytes = to_bytes(body, usize::MAX).await.map_err(|e| {
            axum::response::Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Body::from(format!("Failed to read body: {}", e)))
                .unwrap()
        })?;
        serde_json::from_slice::<ChatRequest>(&body_bytes).map_err(|e| {
            axum::response::Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(axum::body::Body::from(format!("Invalid JSON: {}", e)))
                .unwrap()
        })?
    } else {
        // Parse as form data
        let body_bytes = to_bytes(body, usize::MAX).await.map_err(|e| {
            axum::response::Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Body::from(format!("Failed to read body: {}", e)))
                .unwrap()
        })?;
        let form_str = std::str::from_utf8(&body_bytes).map_err(|_| {
            axum::response::Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(axum::body::Body::from("Invalid form data encoding"))
                .unwrap()
        })?;
        serde_urlencoded::from_str::<ChatRequest>(form_str).map_err(|e| {
            axum::response::Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(axum::body::Body::from(format!("Invalid form data: {}", e)))
                .unwrap()
        })?
    };

    // Validate request
    validate_chat_request(&chat_request).map_err(|(status, msg)| {
        axum::response::Response::builder()
            .status(status)
            .body(axum::body::Body::from(msg))
            .unwrap()
    })?;

    // Create thread manager
    let thread_manager = Arc::new(ThreadManager::new(
        state.config.jarvis_home.clone(),
        state.auth_manager.clone(),
        SessionSource::Exec,
    ));

    // Start or resume thread
    let NewThread {
        thread_id, thread, ..
    } = if let Some(existing_thread_id) = chat_request.thread_id {
        // Try to resume existing thread
        match jarvis_core::find_thread_path_by_id_str(
            &state.config.jarvis_home,
            &existing_thread_id,
        )
        .await
        {
            Ok(Some(rollout_path)) => {
                // Resume thread from rollout file
                thread_manager
                    .resume_thread_from_rollout(
                        (*state.config).clone(),
                        rollout_path,
                        state.auth_manager.clone(),
                    )
                    .await
                    .map_err(|e| {
                        error!("Failed to resume thread {}: {}", existing_thread_id, e);
                        axum::response::Response::builder()
                            .status(axum::http::StatusCode::NOT_FOUND)
                            .body(axum::body::Body::from(format!(
                                "Thread not found or could not be resumed: {}",
                                e
                            )))
                            .unwrap()
                    })?
            }
            Ok(None) => {
                // Thread not found, start new one
                error!(
                    "Thread {} not found, starting new thread",
                    existing_thread_id
                );
                thread_manager
                    .start_thread((*state.config).clone())
                    .await
                    .map_err(|e| {
                        error!("Failed to start thread: {}", e);
                        axum::response::Response::builder()
                            .status(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
                            .body(axum::body::Body::from(format!(
                                "Failed to start thread: {}",
                                e
                            )))
                            .unwrap()
                    })?
            }
            Err(e) => {
                error!("Error looking up thread {}: {}", existing_thread_id, e);
                // Start new thread on error
                thread_manager
                    .start_thread((*state.config).clone())
                    .await
                    .map_err(|e| {
                        error!("Failed to start thread: {}", e);
                        axum::response::Response::builder()
                            .status(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
                            .body(axum::body::Body::from(format!(
                                "Failed to start thread: {}",
                                e
                            )))
                            .unwrap()
                    })?
            }
        }
    } else {
        thread_manager
            .start_thread((*state.config).clone())
            .await
            .map_err(|e| {
                error!("Failed to start thread: {}", e);
                axum::response::Response::builder()
                    .status(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
                    .body(axum::body::Body::from(format!(
                        "Failed to start thread: {}",
                        e
                    )))
                    .unwrap()
            })?
    };

    // Submit user prompt
    let user_input = UserInput::Text {
        text: chat_request.prompt.clone(),
        text_elements: vec![],
    };

    // Get default model from config
    let default_model = state
        .models_manager
        .get_default_model(
            &state.config.model,
            &state.config,
            RefreshStrategy::OnlineIfUncached,
        )
        .await;

    thread
        .submit(Op::UserTurn {
            items: vec![user_input],
            cwd: state.config.cwd.clone(),
            approval_policy: state.config.approval_policy.value(),
            sandbox_policy: state.config.sandbox_policy.get().clone(),
            model: default_model,
            effort: state.config.model_reasoning_effort,
            summary: state.config.model_reasoning_summary,
            final_output_json_schema: None,
            collaboration_mode: None,
            personality: state.config.personality,
        })
        .await
        .map_err(|e| {
            error!("Failed to submit prompt: {}", e);
            axum::response::Response::builder()
                .status(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
                .body(axum::body::Body::from(format!(
                    "Failed to submit prompt: {}",
                    e
                )))
                .unwrap()
        })?;

    // Collect response events
    let mut reply_parts = Vec::new();
    let mut turn_completed = false;

    while !turn_completed {
        let event = thread.next_event().await.map_err(|e| {
            error!("Failed to get event: {}", e);
            axum::response::Response::builder()
                .status(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
                .body(axum::body::Body::from(format!(
                    "Failed to get event: {}",
                    e
                )))
                .unwrap()
        })?;

        match event.msg {
            EventMsg::AgentMessage(agent_msg) => {
                reply_parts.push(agent_msg.message);
            }
            EventMsg::TurnComplete(_) => {
                turn_completed = true;
            }
            EventMsg::Error(error_event) => {
                error!("Jarvis error: {}", error_event.message);
                return Err(axum::response::Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(axum::body::Body::from(format!(
                        "Error from Jarvis: {}",
                        error_event.message
                    )))
                    .unwrap());
            }
            _ => {
                // Ignore other events for now
            }
        }
    }

    let reply = reply_parts.join("");

    if wants_html {
        // Return HTML fragment for HTMX
        let html = format!(
            r#"
            <div class="space-y-4">
                <div class="bg-gray-700 rounded-lg p-4">
                    <p class="text-sm text-gray-400 mb-2">Você:</p>
                    <p class="text-gray-100 whitespace-pre-wrap">{}</p>
                </div>
                <div class="bg-blue-600 rounded-lg p-4">
                    <p class="text-sm text-blue-200 mb-2">Jarvis:</p>
                    <div class="text-white whitespace-pre-wrap">{}</div>
                </div>
            </div>
            "#,
            html_escape(&chat_request.prompt),
            html_escape(&reply)
        );
        Ok(Response::builder()
            .status(StatusCode::OK)
            .header("content-type", "text/html")
            .body(axum::body::Body::from(html))
            .unwrap())
    } else {
        // Return JSON
        Ok(Response::builder()
            .status(StatusCode::OK)
            .header("content-type", "application/json")
            .body(axum::body::Body::from(
                serde_json::to_string(&ChatResponse {
                    reply,
                    thread_id: thread_id.to_string(),
                })
                .unwrap(),
            ))
            .unwrap())
    }
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::create_router;
    use crate::test_utils::create_test_app_state_with_api_key;
    use axum::http::StatusCode;
    use axum_test::TestServer;

    #[tokio::test]
    async fn test_chat_requires_auth() {
        let api_key = "test-api-key".to_string();
        let app_state = create_test_app_state_with_api_key(api_key);
        let app = create_router(app_state, false);
        let server = TestServer::new(app).unwrap();

        let response = server
            .post("/api/chat")
            .json(&serde_json::json!({
                "prompt": "Hello"
            }))
            .await;

        response.assert_status(StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_chat_invalid_json() {
        let api_key = "test-api-key".to_string();
        let app_state = create_test_app_state_with_api_key(api_key.clone());
        let app = create_router(app_state, false);
        let server = TestServer::new(app).unwrap();

        let response = server
            .post("/api/chat")
            .add_header("Authorization", format!("Bearer {}", api_key))
            .add_header("Content-Type", "application/json")
            .body("invalid json")
            .await;

        response.assert_status(StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_chat_missing_prompt() {
        let api_key = "test-api-key".to_string();
        let app_state = create_test_app_state_with_api_key(api_key.clone());
        let app = create_router(app_state, false);
        let server = TestServer::new(app).unwrap();

        let response = server
            .post("/api/chat")
            .add_header("Authorization", format!("Bearer {}", api_key))
            .add_header("Content-Type", "application/json")
            .json(&serde_json::json!({}))
            .await;

        // Should fail validation or return error
        // The exact status depends on serde deserialization behavior
        assert!(response.status_code() != StatusCode::OK);
    }
}
