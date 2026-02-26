//! Goals management endpoints.

use axum::Json;
use axum::extract::Path;
use axum::extract::Query;
use axum::extract::State;
use axum::http::StatusCode;
use jarvis_daemon_common::CreateGoal;
use jarvis_daemon_common::DaemonDb;
use jarvis_daemon_common::GoalFilter;
use jarvis_daemon_common::GoalMetricType;
use jarvis_daemon_common::GoalPeriod;
use jarvis_daemon_common::GoalStatus;
use serde::Deserialize;
use serde::Serialize;

use crate::state::AppState;

#[derive(Deserialize)]
pub struct GoalsQuery {
    pub pipeline: Option<String>,
    #[serde(default = "default_false")]
    pub all: bool,
}

fn default_false() -> bool {
    false
}

#[derive(Serialize)]
pub struct GoalResponse {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub metric_type: String,
    pub current_value: f64,
    pub target_value: f64,
    pub target_unit: String,
    pub period: String,
    pub status: String,
    pub priority: i32,
    pub progress_pct: f64,
}

#[derive(Serialize)]
pub struct GoalsListResponse {
    pub goals: Vec<GoalResponse>,
}

#[derive(Deserialize)]
pub struct CreateGoalRequest {
    pub name: String,
    pub description: Option<String>,
    pub metric: String,
    pub target: f64,
    pub period: String,
    pub unit: Option<String>,
    pub pipeline: Option<String>,
    pub priority: Option<i32>,
}

/// List goals.
pub async fn list_goals(
    State(state): State<AppState>,
    Query(params): Query<GoalsQuery>,
) -> Result<Json<GoalsListResponse>, StatusCode> {
    let db = state
        .daemon_db
        .as_ref()
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;

    let filter = GoalFilter {
        status: if params.all {
            None
        } else {
            Some(GoalStatus::Active)
        },
        pipeline_id: params.pipeline,
        ..Default::default()
    };

    let goals = db
        .list_goals(&filter)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(GoalsListResponse {
        goals: goals
            .into_iter()
            .map(|g| {
                let pct = if g.target_value > 0.0 {
                    (g.current_value / g.target_value) * 100.0
                } else {
                    0.0
                };
                GoalResponse {
                    id: g.id,
                    name: g.name,
                    description: g.description,
                    metric_type: g.metric_type.to_string(),
                    current_value: g.current_value,
                    target_value: g.target_value,
                    target_unit: g.target_unit,
                    period: g.period.to_string(),
                    status: g.status.to_string(),
                    priority: g.priority,
                    progress_pct: pct,
                }
            })
            .collect(),
    }))
}

/// Get a specific goal.
pub async fn get_goal(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<GoalResponse>, StatusCode> {
    let db = state
        .daemon_db
        .as_ref()
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;

    // Find by partial ID match
    let all = db
        .list_goals(&GoalFilter::default())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let goal = all
        .into_iter()
        .find(|g| g.id.starts_with(&id))
        .ok_or(StatusCode::NOT_FOUND)?;

    let pct = if goal.target_value > 0.0 {
        (goal.current_value / goal.target_value) * 100.0
    } else {
        0.0
    };

    Ok(Json(GoalResponse {
        id: goal.id,
        name: goal.name,
        description: goal.description,
        metric_type: goal.metric_type.to_string(),
        current_value: goal.current_value,
        target_value: goal.target_value,
        target_unit: goal.target_unit,
        period: goal.period.to_string(),
        status: goal.status.to_string(),
        priority: goal.priority,
        progress_pct: pct,
    }))
}

/// Create a new goal.
pub async fn create_goal(
    State(state): State<AppState>,
    Json(req): Json<CreateGoalRequest>,
) -> Result<Json<GoalResponse>, StatusCode> {
    let db = state
        .daemon_db
        .as_ref()
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;

    let metric_type: GoalMetricType = req.metric.parse().map_err(|_| StatusCode::BAD_REQUEST)?;
    let period: GoalPeriod = req.period.parse().map_err(|_| StatusCode::BAD_REQUEST)?;

    let input = CreateGoal {
        name: req.name,
        description: req.description,
        metric_type,
        target_value: req.target,
        target_unit: req.unit,
        period,
        pipeline_id: req.pipeline,
        priority: req.priority,
        deadline: None,
    };

    let goal = db
        .create_goal(&input)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let pct = if goal.target_value > 0.0 {
        (goal.current_value / goal.target_value) * 100.0
    } else {
        0.0
    };

    Ok(Json(GoalResponse {
        id: goal.id,
        name: goal.name,
        description: goal.description,
        metric_type: goal.metric_type.to_string(),
        current_value: goal.current_value,
        target_value: goal.target_value,
        target_unit: goal.target_unit,
        period: goal.period.to_string(),
        status: goal.status.to_string(),
        priority: goal.priority,
        progress_pct: pct,
    }))
}

/// Pause a goal.
pub async fn pause_goal(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    let db = state
        .daemon_db
        .as_ref()
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;

    let all = db
        .list_goals(&GoalFilter::default())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let goal = all
        .into_iter()
        .find(|g| g.id.starts_with(&id))
        .ok_or(StatusCode::NOT_FOUND)?;

    db.set_goal_status(&goal.id, GoalStatus::Paused)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::NO_CONTENT)
}

/// Resume a goal.
pub async fn resume_goal(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    let db = state
        .daemon_db
        .as_ref()
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;

    let all = db
        .list_goals(&GoalFilter::default())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let goal = all
        .into_iter()
        .find(|g| g.id.starts_with(&id))
        .ok_or(StatusCode::NOT_FOUND)?;

    db.set_goal_status(&goal.id, GoalStatus::Active)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::NO_CONTENT)
}
