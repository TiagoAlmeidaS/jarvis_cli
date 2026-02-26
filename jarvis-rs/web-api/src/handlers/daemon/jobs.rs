//! Job listing and details endpoints.

use axum::Json;
use axum::extract::Path;
use axum::extract::Query;
use axum::extract::State;
use axum::http::StatusCode;
use jarvis_daemon_common::DaemonDb;
use jarvis_daemon_common::JobFilter;
use jarvis_daemon_common::JobStatus;
use serde::Deserialize;
use serde::Serialize;

use crate::state::AppState;

#[derive(Deserialize)]
pub struct JobsQuery {
    pub pipeline: Option<String>,
    pub status: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: i64,
}

fn default_limit() -> i64 {
    20
}

#[derive(Serialize)]
pub struct JobResponse {
    pub id: String,
    pub pipeline_id: String,
    pub status: String,
    pub attempt: i32,
    pub started_at: Option<i64>,
    pub completed_at: Option<i64>,
    pub duration_ms: Option<i64>,
    pub created_at: i64,
    pub error_message: Option<String>,
}

#[derive(Serialize)]
pub struct JobsListResponse {
    pub jobs: Vec<JobResponse>,
}

/// List jobs.
pub async fn list_jobs(
    State(state): State<AppState>,
    Query(params): Query<JobsQuery>,
) -> Result<Json<JobsListResponse>, StatusCode> {
    let db = state
        .daemon_db
        .as_ref()
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;

    let status = params.status.as_deref().and_then(|s| match s {
        "pending" => Some(JobStatus::Pending),
        "running" => Some(JobStatus::Running),
        "completed" => Some(JobStatus::Completed),
        "failed" => Some(JobStatus::Failed),
        "cancelled" => Some(JobStatus::Cancelled),
        _ => None,
    });

    let filter = JobFilter {
        pipeline_id: params.pipeline,
        status,
        limit: Some(params.limit),
    };

    let jobs = db
        .list_jobs(&filter)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(JobsListResponse {
        jobs: jobs
            .into_iter()
            .map(|j| JobResponse {
                id: j.id,
                pipeline_id: j.pipeline_id,
                status: j.status.to_string(),
                attempt: j.attempt,
                started_at: j.started_at,
                completed_at: j.completed_at,
                duration_ms: j.duration_ms,
                created_at: j.created_at,
                error_message: j.error_message,
            })
            .collect(),
    }))
}

/// Get a specific job.
pub async fn get_job(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<JobResponse>, StatusCode> {
    let db = state
        .daemon_db
        .as_ref()
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;

    // List all jobs and find by ID (partial match supported)
    let filter = JobFilter {
        pipeline_id: None,
        status: None,
        limit: Some(100), // Get enough to find the job
    };

    let jobs = db
        .list_jobs(&filter)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let job = jobs
        .into_iter()
        .find(|j| j.id.starts_with(&id))
        .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(JobResponse {
        id: job.id,
        pipeline_id: job.pipeline_id,
        status: job.status.to_string(),
        attempt: job.attempt,
        started_at: job.started_at,
        completed_at: job.completed_at,
        duration_ms: job.duration_ms,
        created_at: job.created_at,
        error_message: job.error_message,
    }))
}
