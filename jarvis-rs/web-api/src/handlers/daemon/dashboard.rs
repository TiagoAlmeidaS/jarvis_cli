//! Dashboard endpoint with aggregated data.

use axum::Json;
use axum::extract::Query;
use axum::extract::State;
use axum::http::StatusCode;
use jarvis_daemon_common::ContentFilter;
use jarvis_daemon_common::ContentStatus;
use jarvis_daemon_common::DaemonDb;
use jarvis_daemon_common::ExperimentFilter;
use jarvis_daemon_common::ExperimentStatus;
use jarvis_daemon_common::GoalFilter;
use jarvis_daemon_common::GoalStatus;
use jarvis_daemon_common::JobFilter;
use jarvis_daemon_common::JobStatus;
use jarvis_daemon_common::MetricType;
use serde::Deserialize;
use serde::Serialize;

use crate::state::AppState;

#[derive(Deserialize)]
pub struct DashboardQuery {
    #[serde(default = "default_days")]
    pub days: i64,
    #[serde(default = "default_false")]
    pub compact: bool,
}

fn default_days() -> i64 {
    30
}
fn default_false() -> bool {
    false
}

#[derive(Serialize)]
pub struct DashboardResponse {
    pub health: String,
    pub metrics: MetricsData,
    pub pipelines: Vec<PipelineInfo>,
    pub goals: Vec<GoalInfo>,
    pub experiments: Vec<ExperimentInfo>,
    pub recent_content: Vec<ContentInfo>,
    pub jobs_24h: Jobs24h,
}

#[derive(Serialize)]
pub struct MetricsData {
    pub views: f64,
    pub clicks: f64,
    pub impressions: f64,
    pub ctr: f64,
    pub revenue: RevenueData,
}

#[derive(Serialize)]
pub struct RevenueData {
    pub total_usd: f64,
    pub by_source: Vec<SourceRevenue>,
}

#[derive(Serialize)]
pub struct SourceRevenue {
    pub source: String,
    pub total_usd: f64,
    pub record_count: i64,
}

#[derive(Serialize)]
pub struct PipelineInfo {
    pub id: String,
    pub name: String,
    pub strategy: String,
    pub enabled: bool,
    pub schedule_cron: String,
}

#[derive(Serialize)]
pub struct GoalInfo {
    pub id: String,
    pub name: String,
    pub metric_type: String,
    pub current_value: f64,
    pub target_value: f64,
    pub target_unit: String,
    pub progress_pct: f64,
    pub status: String,
    pub priority: i32,
}

#[derive(Serialize)]
pub struct ExperimentInfo {
    pub id: String,
    pub experiment_type: String,
    pub status: String,
    pub metric: String,
    pub metric_a: f64,
    pub metric_b: f64,
    pub winner: Option<String>,
}

#[derive(Serialize)]
pub struct ContentInfo {
    pub id: String,
    pub title: Option<String>,
    pub platform: String,
    pub status: String,
    pub word_count: Option<i32>,
    pub published_at: Option<i64>,
}

#[derive(Serialize)]
pub struct Jobs24h {
    pub completed: usize,
    pub failed: usize,
}

/// Get dashboard data.
pub async fn get_dashboard(
    State(state): State<AppState>,
    Query(params): Query<DashboardQuery>,
) -> Result<Json<DashboardResponse>, StatusCode> {
    let db = state
        .daemon_db
        .as_ref()
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;

    let days = params.days;
    let now = chrono::Utc::now().timestamp();
    let since = now - (days * 86400);

    // Get pipelines
    let pipelines = db
        .list_pipelines(false)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let enabled = pipelines.iter().filter(|p| p.enabled).count();

    // Get running jobs
    let running_jobs = db
        .count_running_jobs()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Get jobs last 24h
    let completed_jobs = db
        .list_jobs(&JobFilter {
            status: Some(JobStatus::Completed),
            ..Default::default()
        })
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let failed_jobs = db
        .list_jobs(&JobFilter {
            status: Some(JobStatus::Failed),
            ..Default::default()
        })
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let last_24h = now - 86400;
    let completed_24h = completed_jobs
        .iter()
        .filter(|j| j.created_at > last_24h)
        .count();
    let failed_24h = failed_jobs
        .iter()
        .filter(|j| j.created_at > last_24h)
        .count();

    // Get metrics
    let views = db
        .sum_metrics(MetricType::Views, since, None)
        .await
        .unwrap_or(0.0);
    let clicks = db
        .sum_metrics(MetricType::Clicks, since, None)
        .await
        .unwrap_or(0.0);
    let impressions = db
        .sum_metrics(MetricType::Impressions, since, None)
        .await
        .unwrap_or(0.0);
    let ctr = if impressions > 0.0 {
        (clicks / impressions) * 100.0
    } else {
        0.0
    };

    // Get revenue
    let revenue_summary = db
        .revenue_summary(days)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Get goals
    let goals = db
        .list_goals(&GoalFilter {
            status: Some(GoalStatus::Active),
            ..Default::default()
        })
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Get experiments
    let experiments = db
        .list_experiments(&ExperimentFilter {
            status: Some(ExperimentStatus::Running),
            ..Default::default()
        })
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Get recent content
    let recent_content = db
        .list_content(&ContentFilter {
            since_days: Some(days),
            status: Some(ContentStatus::Published),
            limit: Some(10),
            ..Default::default()
        })
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Calculate health
    let at_risk_goals = goals
        .iter()
        .filter(|g| g.target_value > 0.0 && (g.current_value / g.target_value) < 0.4)
        .count();

    let health = if failed_24h > completed_24h && completed_24h > 0 {
        "DEGRADED"
    } else if enabled == 0 {
        "INACTIVE"
    } else if at_risk_goals > 0 {
        "AT_RISK"
    } else {
        "HEALTHY"
    };

    Ok(Json(DashboardResponse {
        health: health.to_string(),
        metrics: MetricsData {
            views,
            clicks,
            impressions,
            ctr,
            revenue: RevenueData {
                total_usd: revenue_summary.total_usd,
                by_source: revenue_summary
                    .by_source
                    .into_iter()
                    .map(|sr| SourceRevenue {
                        source: sr.source,
                        total_usd: sr.total_usd,
                        record_count: sr.record_count,
                    })
                    .collect(),
            },
        },
        pipelines: pipelines
            .into_iter()
            .map(|p| PipelineInfo {
                id: p.id,
                name: p.name,
                strategy: p.strategy.to_string(),
                enabled: p.enabled,
                schedule_cron: p.schedule_cron,
            })
            .collect(),
        goals: goals
            .into_iter()
            .map(|g| {
                let pct = if g.target_value > 0.0 {
                    (g.current_value / g.target_value) * 100.0
                } else {
                    0.0
                };
                GoalInfo {
                    id: g.id,
                    name: g.name,
                    metric_type: g.metric_type.to_string(),
                    current_value: g.current_value,
                    target_value: g.target_value,
                    target_unit: g.target_unit,
                    progress_pct: pct,
                    status: g.status.to_string(),
                    priority: g.priority,
                }
            })
            .collect(),
        experiments: experiments
            .into_iter()
            .map(|e| ExperimentInfo {
                id: e.id,
                experiment_type: e.experiment_type,
                status: e.status,
                metric: e.metric,
                metric_a: e.metric_a,
                metric_b: e.metric_b,
                winner: e.winner,
            })
            .collect(),
        recent_content: recent_content
            .into_iter()
            .map(|c| ContentInfo {
                id: c.id,
                title: c.title,
                platform: c.platform.to_string(),
                status: c.status.to_string(),
                word_count: c.word_count,
                published_at: c.published_at,
            })
            .collect(),
        jobs_24h: Jobs24h {
            completed: completed_24h,
            failed: failed_24h,
        },
    }))
}
