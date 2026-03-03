//! Daemon notification system — sends periodic summaries and alerts
//! via Telegram (or other channels in the future).
//!
//! ## Configuration
//!
//! The notifier reads from environment variables:
//! - `JARVIS_TELEGRAM_BOT_TOKEN`: Telegram Bot API token
//! - `JARVIS_TELEGRAM_CHAT_ID`: Chat ID to send notifications to
//! - `JARVIS_NOTIFY_HOUR`: Hour (0-23) for the daily summary (default: 8)
//!
//! If the environment variables are not set, the notifier silently exits.

use anyhow::Result;
use jarvis_daemon_common::DaemonDb;
use jarvis_daemon_common::GoalFilter;
use jarvis_daemon_common::GoalStatus;
use jarvis_daemon_common::JobFilter;
use jarvis_daemon_common::JobStatus;
use jarvis_daemon_common::ProposalFilter;
use jarvis_daemon_common::ProposalStatus;
use jarvis_telegram::client::TelegramClient;
use jarvis_telegram::config::TelegramConfig;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;
use tracing::error;
use tracing::info;
use tracing::warn;

/// Configuration read from environment.
struct NotifierConfig {
    chat_id: String,
    /// Hour of day (0-23) for daily summary.
    notify_hour: u32,
}

/// Try to build a [`TelegramClient`] and [`NotifierConfig`] from environment.
///
/// Returns `None` if the required env vars are not set.
fn from_env() -> Option<(TelegramClient, NotifierConfig)> {
    let bot_token = std::env::var("JARVIS_TELEGRAM_BOT_TOKEN").ok()?;
    let chat_id = std::env::var("JARVIS_TELEGRAM_CHAT_ID").ok()?;
    if bot_token.is_empty() || chat_id.is_empty() {
        return None;
    }
    let notify_hour = std::env::var("JARVIS_NOTIFY_HOUR")
        .ok()
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(8)
        .min(23);

    let telegram = TelegramClient::new(TelegramConfig {
        bot_token,
        webhook_url: None,
        webhook_port: 0,
        webhook_secret: None,
    });

    Some((
        telegram,
        NotifierConfig {
            chat_id,
            notify_hour,
        },
    ))
}

/// Run the daily notifier loop until shutdown is signalled.
///
/// Checks every 5 minutes whether it's time for the daily summary.
/// Also sends immediate alerts for critical events (failed proposals, etc.).
pub async fn run_daily_notifier(db: Arc<DaemonDb>, shutdown: CancellationToken) -> Result<()> {
    let (telegram, config) = match from_env() {
        Some(pair) => pair,
        None => {
            info!(
                "Telegram notifications not configured (set JARVIS_TELEGRAM_BOT_TOKEN and JARVIS_TELEGRAM_CHAT_ID)"
            );
            return Ok(());
        }
    };

    info!(
        "Telegram notifier active (chat_id={}, daily summary at {}:00)",
        config.chat_id, config.notify_hour
    );

    let mut last_summary_date: Option<chrono::NaiveDate> = None;
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(300)); // 5 min
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            _ = interval.tick() => {
                let now = chrono::Utc::now();
                let today = now.date_naive();
                let hour = now.hour();

                // Check for critical alerts (always).
                if let Err(e) = check_and_send_alerts(&telegram, &config, &db).await {
                    error!("Notification alert check failed: {e:#}");
                }

                // Send daily summary at the configured hour, once per day.
                let already_sent = last_summary_date.is_some_and(|d| d == today);
                if hour >= config.notify_hour && !already_sent {
                    match build_daily_summary(&db).await {
                        Ok(message) => {
                            if let Err(e) = telegram.send_text(&config.chat_id, &message).await {
                                error!("Failed to send daily summary: {e:#}");
                            } else {
                                info!("Daily summary sent to Telegram");
                                last_summary_date = Some(today);
                            }
                        }
                        Err(e) => error!("Failed to build daily summary: {e:#}"),
                    }
                }
            }
            _ = shutdown.cancelled() => {
                info!("Telegram notifier shutting down");
                break;
            }
        }
    }

    Ok(())
}

use chrono::Timelike;

/// Build the daily summary message.
async fn build_daily_summary(db: &DaemonDb) -> Result<String> {
    let now = chrono::Utc::now();
    let today_str = now.format("%Y-%m-%d").to_string();

    // Revenue summary (last 30 days).
    let revenue = db.revenue_summary(30).await?;

    // Content counts (last 24h).
    let content_filter = jarvis_daemon_common::ContentFilter {
        status: Some(jarvis_daemon_common::ContentStatus::Published),
        since_days: Some(1),
        ..Default::default()
    };
    let recent_content = db.list_content(&content_filter).await?;

    // Active goals progress.
    let goals = db
        .list_goals(&GoalFilter {
            status: Some(GoalStatus::Active),
            ..Default::default()
        })
        .await?;

    // Pending proposals.
    let pending_proposals = db
        .list_proposals(&ProposalFilter {
            status: Some(ProposalStatus::Pending),
            ..Default::default()
        })
        .await?;

    // Jobs in the last 24h.
    let jobs_filter = JobFilter {
        status: Some(JobStatus::Completed),
        limit: Some(100),
        ..Default::default()
    };
    let completed_jobs = db.list_jobs(&jobs_filter).await?;

    let failed_jobs_filter = JobFilter {
        status: Some(JobStatus::Failed),
        limit: Some(100),
        ..Default::default()
    };
    let failed_jobs = db.list_jobs(&failed_jobs_filter).await?;

    // Build message.
    let mut msg = format!("📊 *Jarvis Daily Report — {today_str}*\n\n");

    // Revenue.
    msg.push_str(&format!("💰 *Revenue (30d)*: ${:.2}\n", revenue.total_usd));

    // Content.
    msg.push_str(&format!(
        "📝 *Published (24h)*: {} articles\n",
        recent_content.len()
    ));

    // Jobs.
    msg.push_str(&format!(
        "⚙️ *Jobs*: {} completed, {} failed\n",
        completed_jobs.len(),
        failed_jobs.len()
    ));

    // Goals.
    if !goals.is_empty() {
        msg.push_str("\n🎯 *Goals Progress*:\n");
        for goal in &goals {
            let pct = if goal.target_value > 0.0 {
                (goal.current_value / goal.target_value * 100.0).min(999.0)
            } else {
                0.0
            };
            let bar = progress_bar(pct);
            msg.push_str(&format!(
                "  {bar} {:.0}% — {} ({:.1}/{:.1} {})\n",
                pct, goal.name, goal.current_value, goal.target_value, goal.target_unit
            ));
        }
    }

    // Proposals needing attention.
    if !pending_proposals.is_empty() {
        msg.push_str(&format!(
            "\n⏳ *{} pending proposals* awaiting review\n",
            pending_proposals.len()
        ));
        for p in pending_proposals.iter().take(5) {
            msg.push_str(&format!(
                "  • _{}_  (conf: {:.0}%, risk: {})\n",
                p.title,
                p.confidence * 100.0,
                p.risk_level
            ));
        }
        if pending_proposals.len() > 5 {
            msg.push_str(&format!("  ...and {} more\n", pending_proposals.len() - 5));
        }
    }

    Ok(msg)
}

/// Check for critical conditions and send immediate alerts.
async fn check_and_send_alerts(
    telegram: &TelegramClient,
    config: &NotifierConfig,
    db: &DaemonDb,
) -> Result<()> {
    // Alert 1: Failed jobs in the last hour.
    let one_hour_ago = chrono::Utc::now().timestamp() - 3600;
    let failed_filter = JobFilter {
        status: Some(JobStatus::Failed),
        limit: Some(10),
        ..Default::default()
    };
    let failed_jobs = db.list_jobs(&failed_filter).await?;
    let recent_failures: Vec<_> = failed_jobs
        .iter()
        .filter(|j| j.created_at > one_hour_ago)
        .collect();

    if !recent_failures.is_empty() {
        let msg = format!(
            "🚨 *Alert*: {} job(s) failed in the last hour!\n{}",
            recent_failures.len(),
            recent_failures
                .iter()
                .take(3)
                .map(|j| format!("  • Pipeline: `{}`, Job: `{}`", j.pipeline_id, j.id))
                .collect::<Vec<_>>()
                .join("\n")
        );
        if let Err(e) = telegram.send_text(&config.chat_id, &msg).await {
            warn!("Failed to send failure alert: {e}");
        }
    }

    // Alert 2: Goals close to deadline with low progress.
    let goals = db
        .list_goals(&GoalFilter {
            status: Some(GoalStatus::Active),
            ..Default::default()
        })
        .await?;

    for goal in &goals {
        if let Some(deadline) = goal.deadline {
            let remaining_days = (deadline - chrono::Utc::now().timestamp()) / 86400;
            let progress_pct = if goal.target_value > 0.0 {
                goal.current_value / goal.target_value * 100.0
            } else {
                0.0
            };

            // Alert if <7 days remaining and <50% progress.
            if remaining_days < 7 && remaining_days > 0 && progress_pct < 50.0 {
                let msg = format!(
                    "⚠️ *Goal at risk*: {} — {:.0}% complete with only {} days left!",
                    goal.name, progress_pct, remaining_days
                );
                if let Err(e) = telegram.send_text(&config.chat_id, &msg).await {
                    warn!("Failed to send goal alert: {e}");
                }
            }
        }
    }

    Ok(())
}

/// Generate a simple ASCII progress bar for Telegram.
fn progress_bar(pct: f64) -> String {
    let filled = ((pct / 100.0) * 10.0).round() as usize;
    let filled = filled.min(10);
    let empty = 10 - filled;
    format!("[{}{}]", "█".repeat(filled), "░".repeat(empty))
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_progress_bar_empty() {
        assert_eq!(progress_bar(0.0), "[░░░░░░░░░░]");
    }

    #[test]
    fn test_progress_bar_50() {
        assert_eq!(progress_bar(50.0), "[█████░░░░░]");
    }

    #[test]
    fn test_progress_bar_full() {
        assert_eq!(progress_bar(100.0), "[██████████]");
    }

    #[test]
    fn test_progress_bar_over_100() {
        assert_eq!(progress_bar(150.0), "[██████████]");
    }

    #[test]
    fn test_notifier_config_missing_env() {
        // Ensure it returns None when env vars are not set.
        // We can't easily test positive cases without setting env, but
        // the important thing is that it doesn't panic.
        let result = from_env();
        // If JARVIS_TELEGRAM_BOT_TOKEN is not set in the test env, this is None.
        // We just verify it doesn't panic.
        let _ = result;
    }
}
