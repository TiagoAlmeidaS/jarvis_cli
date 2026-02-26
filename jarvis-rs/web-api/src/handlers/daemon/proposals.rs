//! Proposals management endpoints.

use axum::Json;
use axum::extract::Path;
use axum::extract::Query;
use axum::extract::State;
use axum::http::StatusCode;
use jarvis_daemon_common::DaemonDb;
use jarvis_daemon_common::ProposalFilter;
use jarvis_daemon_common::ProposalStatus;
use serde::Deserialize;
use serde::Serialize;

use crate::state::AppState;

#[derive(Deserialize)]
pub struct ProposalsQuery {
    pub pipeline: Option<String>,
    #[serde(default = "default_false")]
    pub all: bool,
    #[serde(default = "default_limit")]
    pub limit: i64,
}

fn default_false() -> bool {
    false
}
fn default_limit() -> i64 {
    20
}

#[derive(Serialize)]
pub struct ProposalResponse {
    pub id: String,
    pub pipeline_id: Option<String>,
    pub action_type: String,
    pub title: String,
    pub description: String,
    pub reasoning: String,
    pub risk_level: String,
    pub confidence: f64,
    pub status: String,
    pub auto_approvable: bool,
    pub created_at: i64,
    pub reviewed_at: Option<i64>,
    pub executed_at: Option<i64>,
    pub expires_at: Option<i64>,
}

#[derive(Serialize)]
pub struct ProposalsListResponse {
    pub proposals: Vec<ProposalResponse>,
}

/// List proposals.
pub async fn list_proposals(
    State(state): State<AppState>,
    Query(params): Query<ProposalsQuery>,
) -> Result<Json<ProposalsListResponse>, StatusCode> {
    let db = state
        .daemon_db
        .as_ref()
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;

    let filter = ProposalFilter {
        pipeline_id: params.pipeline,
        status: if params.all {
            None
        } else {
            Some(ProposalStatus::Pending)
        },
        limit: Some(params.limit),
        ..Default::default()
    };

    let proposals = db
        .list_proposals(&filter)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(ProposalsListResponse {
        proposals: proposals
            .into_iter()
            .map(|p| ProposalResponse {
                id: p.id,
                pipeline_id: p.pipeline_id,
                action_type: p.action_type,
                title: p.title,
                description: p.description,
                reasoning: p.reasoning,
                risk_level: p.risk_level,
                confidence: p.confidence,
                status: p.status.to_string(),
                auto_approvable: p.auto_approvable,
                created_at: p.created_at,
                reviewed_at: p.reviewed_at,
                executed_at: p.executed_at,
                expires_at: p.expires_at,
            })
            .collect(),
    }))
}

/// Get a specific proposal.
pub async fn get_proposal(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<ProposalResponse>, StatusCode> {
    let db = state
        .daemon_db
        .as_ref()
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;

    let filter = ProposalFilter::default();
    let all = db
        .list_proposals(&filter)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let proposal = all
        .into_iter()
        .find(|p| p.id.starts_with(&id))
        .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(ProposalResponse {
        id: proposal.id,
        pipeline_id: proposal.pipeline_id,
        action_type: proposal.action_type,
        title: proposal.title,
        description: proposal.description,
        reasoning: proposal.reasoning,
        risk_level: proposal.risk_level,
        confidence: proposal.confidence,
        status: proposal.status.to_string(),
        auto_approvable: proposal.auto_approvable,
        created_at: proposal.created_at,
        reviewed_at: proposal.reviewed_at,
        executed_at: proposal.executed_at,
        expires_at: proposal.expires_at,
    }))
}

/// Approve a proposal.
pub async fn approve_proposal(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    let db = state
        .daemon_db
        .as_ref()
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;

    let filter = ProposalFilter {
        status: Some(ProposalStatus::Pending),
        ..Default::default()
    };
    let pending = db
        .list_proposals(&filter)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let proposal = pending
        .into_iter()
        .find(|p| p.id.starts_with(&id))
        .ok_or(StatusCode::NOT_FOUND)?;

    db.approve_proposal(&proposal.id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::NO_CONTENT)
}

/// Reject a proposal.
pub async fn reject_proposal(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    let db = state
        .daemon_db
        .as_ref()
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;

    let filter = ProposalFilter {
        status: Some(ProposalStatus::Pending),
        ..Default::default()
    };
    let pending = db
        .list_proposals(&filter)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let proposal = pending
        .into_iter()
        .find(|p| p.id.starts_with(&id))
        .ok_or(StatusCode::NOT_FOUND)?;

    db.reject_proposal(&proposal.id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::NO_CONTENT)
}
