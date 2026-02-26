//! Logs endpoint with filtering.

use axum::Json;
use axum::extract::Query;
use axum::extract::State;
use axum::http::StatusCode;
use jarvis_daemon_common::DaemonDb;
use jarvis_daemon_common::LogFilter;
use serde::Deserialize;
use serde::Serialize;

use crate::state::AppState;

#[derive(Deserialize)]
pub struct LogsQuery {
    pub pipeline: Option<String>,
    pub job: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: i64,
}

fn default_limit() -> i64 {
    50
}

#[derive(Serialize)]
pub struct LogResponse {
    pub id: i64,
    pub job_id: Option<String>,
    pub pipeline_id: String,
    pub level: String,
    pub message: String,
    pub context_json: Option<serde_json::Value>,
    pub created_at: i64,
}

#[derive(Serialize)]
pub struct LogsListResponse {
    pub logs: Vec<LogResponse>,
}

/// List logs.
pub async fn list_logs(
    State(state): State<AppState>,
    Query(params): Query<LogsQuery>,
) -> Result<Json<LogsListResponse>, StatusCode> {
    let db = state
        .daemon_db
        .as_ref()
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;

    let filter = LogFilter {
        pipeline_id: params.pipeline,
        job_id: params.job,
        limit: Some(params.limit),
        ..Default::default()
    };

    let logs = db
        .list_logs(&filter)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(LogsListResponse {
        logs: logs
            .into_iter()
            .map(|l| {
                let context_json = l
                    .context_json
                    .as_ref()
                    .and_then(|s| serde_json::from_str(s).ok());
                LogResponse {
                    id: l.id,
                    job_id: l.job_id,
                    pipeline_id: l.pipeline_id,
                    level: l.level,
                    message: l.message,
                    context_json,
                    created_at: l.created_at,
                }
            })
            .collect(),
    }))
}
