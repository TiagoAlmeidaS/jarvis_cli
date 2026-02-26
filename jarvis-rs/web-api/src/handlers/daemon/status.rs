//! Status endpoint for daemon monitoring.

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use jarvis_daemon_common::ContentFilter;
use jarvis_daemon_common::ContentStatus;
use jarvis_daemon_common::DaemonDb;
use jarvis_daemon_common::GoalFilter;
use jarvis_daemon_common::GoalStatus;
use jarvis_daemon_common::JobStatus;
use serde::Serialize;

use crate::state::AppState;

#[derive(Serialize)]
pub struct StatusResponse {
    pub pipelines: PipelinesStatus,
    pub jobs: JobsStatus,
    pub content: ContentStatus,
    pub proposals: ProposalsStatus,
    pub revenue: RevenueStatus,
    pub goals: GoalsStatus,
}

#[derive(Serialize)]
pub struct PipelinesStatus {
    pub total: usize,
    pub enabled: usize,
}

#[derive(Serialize)]
pub struct JobsStatus {
    pub running: i64,
}

#[derive(Serialize)]
pub struct ContentStatus {
    pub published_last_7d: usize,
}

#[derive(Serialize)]
pub struct ProposalsStatus {
    pub pending: i64,
}

#[derive(Serialize)]
pub struct RevenueStatus {
    pub total_usd_30d: f64,
}

#[derive(Serialize)]
pub struct GoalsStatus {
    pub active: usize,
    pub at_risk: usize,
}

/// Get overall daemon status.
pub async fn get_status(State(state): State<AppState>) -> Result<Json<StatusResponse>, StatusCode> {
    let db = state
        .daemon_db
        .as_ref()
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;

    // Get pipelines
    let all_pipelines = db
        .list_pipelines(false)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let enabled = all_pipelines.iter().filter(|p| p.enabled).count();

    // Get running jobs
    let running_jobs = db
        .count_running_jobs()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Get recent content
    let content_filter = ContentFilter {
        since_days: Some(7),
        status: Some(ContentStatus::Published),
        ..Default::default()
    };
    let recent_content = db
        .list_content(&content_filter)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Get pending proposals
    let pending_proposals = db
        .count_pending_proposals()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Get revenue summary
    let revenue_summary = db
        .revenue_summary(30)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Get active goals
    let active_goals = db
        .list_goals(&GoalFilter {
            status: Some(GoalStatus::Active),
            ..Default::default()
        })
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let at_risk = active_goals
        .iter()
        .filter(|g| {
            if g.target_value <= 0.0 {
                return false;
            }
            (g.current_value / g.target_value) < 0.4
        })
        .count();

    Ok(Json(StatusResponse {
        pipelines: PipelinesStatus {
            total: all_pipelines.len(),
            enabled,
        },
        jobs: JobsStatus {
            running: running_jobs,
        },
        content: ContentStatus {
            published_last_7d: recent_content.len(),
        },
        proposals: ProposalsStatus {
            pending: pending_proposals,
        },
        revenue: RevenueStatus {
            total_usd_30d: revenue_summary.total_usd,
        },
        goals: GoalsStatus {
            active: active_goals.len(),
            at_risk,
        },
    }))
}
