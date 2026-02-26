//! Metrics and revenue endpoints.

use axum::Json;
use axum::extract::Path;
use axum::extract::Query;
use axum::extract::State;
use axum::http::StatusCode;
use jarvis_daemon_common::DaemonDb;
use jarvis_daemon_common::MetricType;
use jarvis_daemon_common::RevenueFilter;
use serde::Deserialize;
use serde::Serialize;

use crate::state::AppState;

#[derive(Deserialize)]
pub struct MetricsSummaryQuery {
    #[serde(default = "default_days")]
    pub days: i64,
}

fn default_days() -> i64 {
    30
}

#[derive(Serialize)]
pub struct MetricsSummaryResponse {
    pub views: f64,
    pub clicks: f64,
    pub impressions: f64,
    pub ctr: f64,
    pub revenue: f64,
}

#[derive(Deserialize)]
pub struct RevenueQuery {
    pub pipeline: Option<String>,
    #[serde(default = "default_days")]
    pub last_days: i64,
    #[serde(default = "default_limit")]
    pub limit: i64,
}

fn default_limit() -> i64 {
    50
}

#[derive(Serialize)]
pub struct RevenueResponse {
    pub id: String,
    pub pipeline_id: String,
    pub source: String,
    pub amount: f64,
    pub currency: String,
    pub period_start: i64,
}

#[derive(Serialize)]
pub struct RevenueListResponse {
    pub records: Vec<RevenueResponse>,
    pub summary: RevenueSummaryResponse,
}

#[derive(Serialize)]
pub struct RevenueSummaryResponse {
    pub total_usd: f64,
    pub period_days: i64,
    pub by_pipeline: Vec<PipelineRevenue>,
    pub by_source: Vec<SourceRevenue>,
}

#[derive(Serialize)]
pub struct PipelineRevenue {
    pub pipeline_id: String,
    pub pipeline_name: Option<String>,
    pub total_usd: f64,
    pub content_count: i64,
}

#[derive(Serialize)]
pub struct SourceRevenue {
    pub source: String,
    pub total_usd: f64,
    pub record_count: i64,
}

/// Get metrics summary.
pub async fn get_metrics_summary(
    State(state): State<AppState>,
    Query(params): Query<MetricsSummaryQuery>,
) -> Result<Json<MetricsSummaryResponse>, StatusCode> {
    let db = state
        .daemon_db
        .as_ref()
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;

    let now = chrono::Utc::now().timestamp();
    let since = now - (params.days * 86400);

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

    let revenue_summary = db
        .revenue_summary(params.days)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(MetricsSummaryResponse {
        views,
        clicks,
        impressions,
        ctr,
        revenue: revenue_summary.total_usd,
    }))
}

/// Get revenue summary.
pub async fn get_revenue_summary(
    State(state): State<AppState>,
    Query(params): Query<RevenueQuery>,
) -> Result<Json<RevenueSummaryResponse>, StatusCode> {
    let db = state
        .daemon_db
        .as_ref()
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;

    let summary = db
        .revenue_summary(params.last_days)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(RevenueSummaryResponse {
        total_usd: summary.total_usd,
        period_days: summary.period_days,
        by_pipeline: summary
            .by_pipeline
            .into_iter()
            .map(|pr| PipelineRevenue {
                pipeline_id: pr.pipeline_id,
                pipeline_name: pr.pipeline_name,
                total_usd: pr.total_usd,
                content_count: pr.content_count,
            })
            .collect(),
        by_source: summary
            .by_source
            .into_iter()
            .map(|sr| SourceRevenue {
                source: sr.source,
                total_usd: sr.total_usd,
                record_count: sr.record_count,
            })
            .collect(),
    }))
}

/// List revenue records.
pub async fn list_revenue(
    State(state): State<AppState>,
    Query(params): Query<RevenueQuery>,
) -> Result<Json<RevenueListResponse>, StatusCode> {
    let db = state
        .daemon_db
        .as_ref()
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;

    let filter = RevenueFilter {
        pipeline_id: params.pipeline,
        since_days: Some(params.last_days),
        limit: Some(params.limit),
        ..Default::default()
    };

    let records = db
        .list_revenue(&filter)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let summary = db
        .revenue_summary(params.last_days)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(RevenueListResponse {
        records: records
            .into_iter()
            .map(|r| RevenueResponse {
                id: r.id,
                pipeline_id: r.pipeline_id,
                source: r.source.to_string(),
                amount: r.amount,
                currency: r.currency,
                period_start: r.period_start,
            })
            .collect(),
        summary: RevenueSummaryResponse {
            total_usd: summary.total_usd,
            period_days: summary.period_days,
            by_pipeline: summary
                .by_pipeline
                .into_iter()
                .map(|pr| PipelineRevenue {
                    pipeline_id: pr.pipeline_id,
                    pipeline_name: pr.pipeline_name,
                    total_usd: pr.total_usd,
                    content_count: pr.content_count,
                })
                .collect(),
            by_source: summary
                .by_source
                .into_iter()
                .map(|sr| SourceRevenue {
                    source: sr.source,
                    total_usd: sr.total_usd,
                    record_count: sr.record_count,
                })
                .collect(),
        },
    }))
}

/// Get content metrics.
pub async fn get_content_metrics(
    State(state): State<AppState>,
    Path(content_id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let db = state
        .daemon_db
        .as_ref()
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;

    let views = db
        .sum_content_metric(&content_id, "views")
        .await
        .unwrap_or(0.0);
    let clicks = db
        .sum_content_metric(&content_id, "clicks")
        .await
        .unwrap_or(0.0);
    let impressions = db
        .sum_content_metric(&content_id, "impressions")
        .await
        .unwrap_or(0.0);
    let revenue = db
        .sum_content_metric(&content_id, "revenue")
        .await
        .unwrap_or(0.0);
    let ctr = db
        .sum_content_metric(&content_id, "ctr")
        .await
        .unwrap_or(0.0);

    Ok(Json(serde_json::json!({
        "content_id": content_id,
        "views": views,
        "clicks": clicks,
        "impressions": impressions,
        "revenue": revenue,
        "ctr": ctr,
    })))
}
