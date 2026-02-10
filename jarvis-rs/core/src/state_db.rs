use crate::config::Config;
use crate::features::Feature;
use crate::rollout::list::Cursor;
use crate::rollout::list::ThreadSortKey;
use crate::rollout::metadata;
use chrono::DateTime;
use chrono::NaiveDateTime;
use chrono::Timelike;
use chrono::Utc;
use jarvis_otel::OtelManager;
use jarvis_protocol::ThreadId;
use jarvis_protocol::dynamic_tools::DynamicToolSpec;
use jarvis_protocol::protocol::RolloutItem;
use jarvis_protocol::protocol::SessionSource;
#[cfg(feature = "state")]
use jarvis_state::DB_METRIC_COMPARE_ERROR;
#[cfg(feature = "state")]
pub use jarvis_state::LogEntry;
#[cfg(feature = "state")]
use jarvis_state::ThreadMetadataBuilder;
use serde_json::Value;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::warn;
use uuid::Uuid;

/// Core-facing handle to the optional SQLite-backed state runtime.
#[cfg(feature = "state")]
pub type StateDbHandle = Arc<jarvis_state::StateRuntime>;
#[cfg(not(feature = "state"))]
pub type StateDbHandle = (); // Placeholder when state feature is disabled

/// Initialize the state runtime when the `sqlite` feature flag is enabled. To only be used
/// inside `core`. The initialization should not be done anywhere else.
#[cfg(feature = "state")]
pub(crate) async fn init_if_enabled(
    config: &Config,
    otel: Option<&OtelManager>,
) -> Option<StateDbHandle> {
    let state_path = jarvis_state::state_db_path(config.jarvis_home.as_path());
    if !config.features.enabled(Feature::Sqlite) {
        return None;
    }
    let existed = tokio::fs::try_exists(&state_path).await.unwrap_or(false);
    let runtime = match jarvis_state::StateRuntime::init(
        config.jarvis_home.clone(),
        config.model_provider_id.clone(),
        otel.cloned(),
    )
    .await
    {
        Ok(runtime) => runtime,
        Err(err) => {
            warn!(
                "failed to initialize state runtime at {}: {err}",
                config.jarvis_home.display()
            );
            if let Some(otel) = otel {
                otel.counter("Jarvis.db.init", 1, &[("status", "init_error")]);
            }
            return None;
        }
    };
    if !existed {
        let runtime_for_backfill = Arc::clone(&runtime);
        let config_for_backfill = config.clone();
        let otel_for_backfill = otel.cloned();
        tokio::task::spawn(async move {
            metadata::backfill_sessions(
                runtime_for_backfill.as_ref(),
                &config_for_backfill,
                otel_for_backfill.as_ref(),
            )
            .await;
        });
    }
    Some(runtime)
}

/// Get the DB if the feature is enabled and the DB exists.
#[cfg(feature = "state")]
pub async fn get_state_db(config: &Config, otel: Option<&OtelManager>) -> Option<StateDbHandle> {
    let state_path = jarvis_state::state_db_path(config.jarvis_home.as_path());
    if !config.features.enabled(Feature::Sqlite)
        || !tokio::fs::try_exists(&state_path).await.unwrap_or(false)
    {
        return None;
    }
    jarvis_state::StateRuntime::init(
        config.jarvis_home.clone(),
        config.model_provider_id.clone(),
        otel.cloned(),
    )
    .await
    .ok()
}

/// Open the state runtime when the SQLite file exists, without feature gating.
///
/// This is used for parity checks during the SQLite migration phase.
#[cfg(feature = "state")]
pub async fn open_if_present(jarvis_home: &Path, default_provider: &str) -> Option<StateDbHandle> {
    let db_path = jarvis_state::state_db_path(jarvis_home);
    if !tokio::fs::try_exists(&db_path).await.unwrap_or(false) {
        return None;
    }
    let runtime = jarvis_state::StateRuntime::init(
        jarvis_home.to_path_buf(),
        default_provider.to_string(),
        None,
    )
    .await
    .ok()?;
    Some(runtime)
}

#[cfg(feature = "state")]
fn cursor_to_anchor(cursor: Option<&Cursor>) -> Option<jarvis_state::Anchor> {
    let cursor = cursor?;
    let value = serde_json::to_value(cursor).ok()?;
    let cursor_str = value.as_str()?;
    let (ts_str, id_str) = cursor_str.split_once('|')?;
    if id_str.contains('|') {
        return None;
    }
    let id = Uuid::parse_str(id_str).ok()?;
    let ts = if let Ok(naive) = NaiveDateTime::parse_from_str(ts_str, "%Y-%m-%dT%H-%M-%S") {
        DateTime::<Utc>::from_naive_utc_and_offset(naive, Utc)
    } else if let Ok(dt) = DateTime::parse_from_rfc3339(ts_str) {
        dt.with_timezone(&Utc)
    } else {
        return None;
    }
    .with_nanosecond(0)?;
    Some(jarvis_state::Anchor { ts, id })
}

/// List thread ids from SQLite for parity checks without rollout scanning.
#[allow(clippy::too_many_arguments)]
#[cfg(feature = "state")]
pub async fn list_thread_ids_db(
    context: Option<&jarvis_state::StateRuntime>,
    jarvis_home: &Path,
    page_size: usize,
    cursor: Option<&Cursor>,
    sort_key: ThreadSortKey,
    allowed_sources: &[SessionSource],
    model_providers: Option<&[String]>,
    archived_only: bool,
    stage: &str,
) -> Option<Vec<ThreadId>> {
    let ctx = context?;
    if ctx.jarvis_home() != jarvis_home {
        warn!(
            "state db jarvis_home mismatch: expected {}, got {}",
            ctx.jarvis_home().display(),
            jarvis_home.display()
        );
    }

    let anchor = cursor_to_anchor(cursor);
    let allowed_sources: Vec<String> = allowed_sources
        .iter()
        .map(|value| match serde_json::to_value(value) {
            Ok(Value::String(s)) => s,
            Ok(other) => other.to_string(),
            Err(_) => String::new(),
        })
        .collect();
    let model_providers = model_providers.map(<[String]>::to_vec);
    match ctx
        .list_thread_ids(
            page_size,
            anchor.as_ref(),
            match sort_key {
                ThreadSortKey::CreatedAt => jarvis_state::SortKey::CreatedAt,
                ThreadSortKey::UpdatedAt => jarvis_state::SortKey::UpdatedAt,
            },
            allowed_sources.as_slice(),
            model_providers.as_deref(),
            archived_only,
        )
        .await
    {
        Ok(ids) => Some(ids),
        Err(err) => {
            warn!("state db list_thread_ids failed during {stage}: {err}");
            None
        }
    }
}

/// List thread metadata from SQLite without rollout directory traversal.
#[cfg(feature = "state")]
#[allow(clippy::too_many_arguments)]
pub async fn list_threads_db(
    context: Option<&jarvis_state::StateRuntime>,
    jarvis_home: &Path,
    page_size: usize,
    cursor: Option<&Cursor>,
    sort_key: ThreadSortKey,
    allowed_sources: &[SessionSource],
    model_providers: Option<&[String]>,
    archived: bool,
) -> Option<jarvis_state::ThreadsPage> {
    let ctx = context?;
    if ctx.jarvis_home() != jarvis_home {
        warn!(
            "state db jarvis_home mismatch: expected {}, got {}",
            ctx.jarvis_home().display(),
            jarvis_home.display()
        );
    }

    let anchor = cursor_to_anchor(cursor);
    let allowed_sources: Vec<String> = allowed_sources
        .iter()
        .map(|value| match serde_json::to_value(value) {
            Ok(Value::String(s)) => s,
            Ok(other) => other.to_string(),
            Err(_) => String::new(),
        })
        .collect();
    let model_providers = model_providers.map(<[String]>::to_vec);
    match ctx
        .list_threads(
            page_size,
            anchor.as_ref(),
            match sort_key {
                ThreadSortKey::CreatedAt => jarvis_state::SortKey::CreatedAt,
                ThreadSortKey::UpdatedAt => jarvis_state::SortKey::UpdatedAt,
            },
            allowed_sources.as_slice(),
            model_providers.as_deref(),
            archived,
        )
        .await
    {
        Ok(page) => Some(page),
        Err(err) => {
            warn!("state db list_threads failed: {err}");
            None
        }
    }
}

/// Look up the rollout path for a thread id using SQLite.
#[cfg(feature = "state")]
pub async fn find_rollout_path_by_id(
    context: Option<&jarvis_state::StateRuntime>,
    thread_id: ThreadId,
    archived_only: Option<bool>,
    stage: &str,
) -> Option<PathBuf> {
    let ctx = context?;
    ctx.find_rollout_path_by_id(thread_id, archived_only)
        .await
        .unwrap_or_else(|err| {
            warn!("state db find_rollout_path_by_id failed during {stage}: {err}");
            None
        })
}

/// Get dynamic tools for a thread id using SQLite.
#[cfg(feature = "state")]
pub async fn get_dynamic_tools(
    context: Option<&jarvis_state::StateRuntime>,
    thread_id: ThreadId,
    stage: &str,
) -> Option<Vec<DynamicToolSpec>> {
    let ctx = context?;
    match ctx.get_dynamic_tools(thread_id).await {
        Ok(tools) => tools,
        Err(err) => {
            warn!("state db get_dynamic_tools failed during {stage}: {err}");
            None
        }
    }
}

/// Persist dynamic tools for a thread id using SQLite, if none exist yet.
#[cfg(feature = "state")]
pub async fn persist_dynamic_tools(
    context: Option<&jarvis_state::StateRuntime>,
    thread_id: ThreadId,
    tools: Option<&[DynamicToolSpec]>,
    stage: &str,
) {
    let Some(ctx) = context else {
        return;
    };
    if let Err(err) = ctx.persist_dynamic_tools(thread_id, tools).await {
        warn!("state db persist_dynamic_tools failed during {stage}: {err}");
    }
}

/// Reconcile rollout items into SQLite, falling back to scanning the rollout file.
#[cfg(feature = "state")]
pub async fn reconcile_rollout(
    context: Option<&jarvis_state::StateRuntime>,
    rollout_path: &Path,
    default_provider: &str,
    builder: Option<&ThreadMetadataBuilder>,
    items: &[RolloutItem],
) {
    let Some(ctx) = context else {
        return;
    };
    if builder.is_some() || !items.is_empty() {
        apply_rollout_items(
            Some(ctx),
            rollout_path,
            default_provider,
            builder,
            items,
            "reconcile_rollout",
        )
        .await;
        return;
    }
    let outcome =
        match metadata::extract_metadata_from_rollout(rollout_path, default_provider, None).await {
            Ok(outcome) => outcome,
            Err(err) => {
                warn!(
                    "state db reconcile_rollout extraction failed {}: {err}",
                    rollout_path.display()
                );
                return;
            }
        };
    if let Err(err) = ctx.upsert_thread(&outcome.metadata).await {
        warn!(
            "state db reconcile_rollout upsert failed {}: {err}",
            rollout_path.display()
        );
        return;
    }
    if let Ok(meta_line) = crate::rollout::list::read_session_meta_line(rollout_path).await {
        persist_dynamic_tools(
            Some(ctx),
            meta_line.meta.id,
            meta_line.meta.dynamic_tools.as_deref(),
            "reconcile_rollout",
        )
        .await;
    } else {
        warn!(
            "state db reconcile_rollout missing session meta {}",
            rollout_path.display()
        );
    }
}

/// Apply rollout items incrementally to SQLite.
#[cfg(feature = "state")]
pub async fn apply_rollout_items(
    context: Option<&jarvis_state::StateRuntime>,
    rollout_path: &Path,
    _default_provider: &str,
    builder: Option<&ThreadMetadataBuilder>,
    items: &[RolloutItem],
    stage: &str,
) {
    let Some(ctx) = context else {
        return;
    };
    let mut builder = match builder {
        Some(builder) => builder.clone(),
        None => match metadata::builder_from_items(items, rollout_path) {
            Some(builder) => builder,
            None => {
                warn!(
                    "state db apply_rollout_items missing builder during {stage}: {}",
                    rollout_path.display()
                );
                record_discrepancy(stage, "missing_builder");
                return;
            }
        },
    };
    builder.rollout_path = rollout_path.to_path_buf();
    if let Err(err) = ctx.apply_rollout_items(&builder, items, None).await {
        warn!(
            "state db apply_rollout_items failed during {stage} for {}: {err}",
            rollout_path.display()
        );
    }
}

/// Record a state discrepancy metric with a stage and reason tag.
#[cfg(feature = "state")]
pub fn record_discrepancy(stage: &str, reason: &str) {
    // We access the global metric because the call sites might not have access to the broader
    // OtelManager.
    tracing::warn!("state db record_discrepancy: {stage}, {reason}");
    if let Some(metric) = jarvis_otel::metrics::global() {
        let _ = metric.counter(
            DB_METRIC_COMPARE_ERROR,
            1,
            &[("stage", stage), ("reason", reason)],
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rollout::list::parse_cursor;
    use pretty_assertions::assert_eq;

    #[cfg(feature = "state")]
    #[test]
    fn cursor_to_anchor_normalizes_timestamp_format() {
        let uuid = Uuid::new_v4();
        let ts_str = "2026-01-27T12-34-56";
        let token = format!("{ts_str}|{uuid}");
        let cursor = parse_cursor(token.as_str()).expect("cursor should parse");
        let anchor = cursor_to_anchor(Some(&cursor)).expect("anchor should parse");

        let naive =
            NaiveDateTime::parse_from_str(ts_str, "%Y-%m-%dT%H-%M-%S").expect("ts should parse");
        let expected_ts = DateTime::<Utc>::from_naive_utc_and_offset(naive, Utc)
            .with_nanosecond(0)
            .expect("nanosecond");

        assert_eq!(anchor.id, uuid);
        assert_eq!(anchor.ts, expected_ts);
    }
}
