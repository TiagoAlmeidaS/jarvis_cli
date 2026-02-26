//! Pipeline management endpoints.

use axum::Json;
use axum::extract::Path;
use axum::extract::State;
use axum::http::StatusCode;
use jarvis_daemon_common::CreatePipeline;
use jarvis_daemon_common::DaemonDb;
use jarvis_daemon_common::Strategy;
use serde::Deserialize;
use serde::Serialize;

use crate::state::AppState;

#[derive(Serialize)]
pub struct PipelineResponse {
    pub id: String,
    pub name: String,
    pub strategy: String,
    pub enabled: bool,
    pub schedule_cron: String,
    pub config: serde_json::Value,
}

#[derive(Serialize)]
pub struct PipelinesListResponse {
    pub pipelines: Vec<PipelineResponse>,
}

#[derive(Deserialize)]
pub struct CreatePipelineRequest {
    pub id: String,
    pub name: String,
    pub strategy: String,
    pub schedule_cron: Option<String>,
    pub config: serde_json::Value,
    pub max_retries: Option<i32>,
    pub retry_delay_sec: Option<i32>,
}

/// List all pipelines.
pub async fn list_pipelines(
    State(state): State<AppState>,
) -> Result<Json<PipelinesListResponse>, StatusCode> {
    let db = state
        .daemon_db
        .as_ref()
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;

    let pipelines = db
        .list_pipelines(false)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(PipelinesListResponse {
        pipelines: pipelines
            .into_iter()
            .map(|p| {
                let config: serde_json::Value =
                    serde_json::from_str(&p.config_json).unwrap_or_else(|_| serde_json::json!({}));
                PipelineResponse {
                    id: p.id,
                    name: p.name,
                    strategy: p.strategy.to_string(),
                    enabled: p.enabled,
                    schedule_cron: p.schedule_cron,
                    config,
                }
            })
            .collect(),
    }))
}

/// Get a specific pipeline.
pub async fn get_pipeline(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<PipelineResponse>, StatusCode> {
    let db = state
        .daemon_db
        .as_ref()
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;

    let pipeline = db
        .get_pipeline(&id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let config: serde_json::Value =
        serde_json::from_str(&pipeline.config_json).unwrap_or_else(|_| serde_json::json!({}));

    Ok(Json(PipelineResponse {
        id: pipeline.id,
        name: pipeline.name,
        strategy: pipeline.strategy.to_string(),
        enabled: pipeline.enabled,
        schedule_cron: pipeline.schedule_cron,
        config,
    }))
}

/// Create a new pipeline.
pub async fn create_pipeline(
    State(state): State<AppState>,
    Json(req): Json<CreatePipelineRequest>,
) -> Result<Json<PipelineResponse>, StatusCode> {
    let db = state
        .daemon_db
        .as_ref()
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;

    let strategy: Strategy = req.strategy.parse().map_err(|_| StatusCode::BAD_REQUEST)?;

    let input = CreatePipeline {
        id: req.id,
        name: req.name,
        strategy,
        config_json: req.config,
        schedule_cron: req.schedule_cron.unwrap_or_else(|| "0 3 * * *".to_string()),
        max_retries: req.max_retries,
        retry_delay_sec: req.retry_delay_sec,
    };

    let pipeline = db
        .create_pipeline(&input)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let config: serde_json::Value =
        serde_json::from_str(&pipeline.config_json).unwrap_or_else(|_| serde_json::json!({}));

    Ok(Json(PipelineResponse {
        id: pipeline.id,
        name: pipeline.name,
        strategy: pipeline.strategy.to_string(),
        enabled: pipeline.enabled,
        schedule_cron: pipeline.schedule_cron,
        config,
    }))
}

/// Enable a pipeline.
pub async fn enable_pipeline(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    let db = state
        .daemon_db
        .as_ref()
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;

    db.set_pipeline_enabled(&id, true)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::NO_CONTENT)
}

/// Disable a pipeline.
pub async fn disable_pipeline(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    let db = state
        .daemon_db
        .as_ref()
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;

    db.set_pipeline_enabled(&id, false)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::NO_CONTENT)
}
