//! Local decision engine for the daemon.
//!
//! Provides a rule-based layer that runs **before** calling the LLM in the
//! strategy analyzer. This gives the daemon a fast, deterministic "brain" that:
//!
//! 1. **Pre-screens** gathered data to skip LLM calls when nothing actionable exists
//! 2. **Generates urgent proposals** from hard rules (goal deadlines, cost overruns)
//! 3. **Validates** LLM-generated proposals against safety constraints
//! 4. **Scores** proposals by combining LLM confidence with goal-gap analysis
//!
//! Inspired by `core/src/autonomous/` but specialised for the daemon's domain.

use anyhow::Result;
use jarvis_daemon_common::{DaemonGoal, GoalMetricType, GoalStatus};
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// A locally-generated proposal from the rule engine (before LLM).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleProposal {
    pub action_type: String,
    pub title: String,
    pub description: String,
    pub reasoning: String,
    pub confidence: f64,
    pub risk_level: String,
    pub pipeline_id: Option<String>,
    pub proposed_config: Option<serde_json::Value>,
    /// Which rule generated this proposal.
    pub source_rule: String,
}

/// Summary of the current system state for decision-making.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemSnapshot {
    pub total_published: i64,
    pub total_llm_cost: f64,
    pub total_revenue: f64,
    pub pending_proposals: i64,
    pub goals: Vec<GoalSnapshot>,
    pub pipeline_count: usize,
}

/// Summarised goal data for the decision engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoalSnapshot {
    pub id: String,
    pub name: String,
    pub metric_type: String,
    pub target_value: f64,
    pub current_value: f64,
    pub target_unit: String,
    pub priority: i32,
    pub deadline: Option<i64>,
    pub status: String,
}

impl GoalSnapshot {
    pub fn from_daemon_goal(g: &DaemonGoal) -> Self {
        Self {
            id: g.id.clone(),
            name: g.name.clone(),
            metric_type: g.metric_type.clone(),
            target_value: g.target_value,
            current_value: g.current_value,
            target_unit: g.target_unit.clone(),
            priority: g.priority,
            deadline: g.deadline,
            status: g.status.clone(),
        }
    }

    /// Progress as a percentage (0–100+).
    pub fn progress_pct(&self) -> f64 {
        if self.target_value <= 0.0 {
            return 0.0;
        }
        (self.current_value / self.target_value) * 100.0
    }

    /// Absolute gap remaining.
    pub fn gap(&self) -> f64 {
        (self.target_value - self.current_value).max(0.0)
    }

    /// Whether the goal is at risk (<40% progress or deadline within 7 days).
    pub fn is_at_risk(&self) -> bool {
        if self.progress_pct() < 40.0 {
            return true;
        }
        if let Some(deadline) = self.deadline {
            let now = chrono::Utc::now().timestamp();
            let days_left = (deadline - now) / 86400;
            if days_left < 7 && self.progress_pct() < 80.0 {
                return true;
            }
        }
        false
    }
}

/// Outcome of the local pre-analysis.
#[derive(Debug, Clone)]
pub struct PreAnalysisResult {
    /// Proposals generated purely by rules (no LLM needed).
    pub rule_proposals: Vec<RuleProposal>,
    /// Whether the LLM should still be called for deeper analysis.
    pub should_call_llm: bool,
    /// Brief reason for the decision.
    pub reason: String,
}

// ---------------------------------------------------------------------------
// Decision Engine
// ---------------------------------------------------------------------------

/// Configuration for the local decision engine.
#[derive(Debug, Clone, Deserialize)]
pub struct DecisionEngineConfig {
    /// Cost threshold (USD) above which cost-cutting proposals are generated.
    #[serde(default = "default_cost_alert_threshold")]
    pub cost_alert_threshold: f64,
    /// Minimum data points required before calling the LLM.
    #[serde(default = "default_min_data_points")]
    pub min_data_points_for_llm: i64,
    /// Maximum pending proposals before skipping new analysis.
    #[serde(default = "default_max_pending")]
    pub max_pending_proposals: i64,
}

impl Default for DecisionEngineConfig {
    fn default() -> Self {
        Self {
            cost_alert_threshold: default_cost_alert_threshold(),
            min_data_points_for_llm: default_min_data_points(),
            max_pending_proposals: default_max_pending(),
        }
    }
}

fn default_cost_alert_threshold() -> f64 {
    5.0
}

fn default_min_data_points() -> i64 {
    1
}

fn default_max_pending() -> i64 {
    10
}

/// The local decision engine.
pub struct DecisionEngine {
    config: DecisionEngineConfig,
}

impl DecisionEngine {
    pub fn new(config: DecisionEngineConfig) -> Self {
        Self { config }
    }

    /// Run pre-analysis on the current system snapshot.
    ///
    /// Returns rule-based proposals and whether the LLM should be called.
    pub fn pre_analyze(&self, snapshot: &SystemSnapshot) -> PreAnalysisResult {
        let mut rule_proposals = Vec::new();

        // Rule 1: Too many pending proposals — skip everything.
        if snapshot.pending_proposals >= self.config.max_pending_proposals {
            return PreAnalysisResult {
                rule_proposals: vec![],
                should_call_llm: false,
                reason: format!(
                    "Already {} pending proposals (max {}), skipping",
                    snapshot.pending_proposals, self.config.max_pending_proposals
                ),
            };
        }

        // Rule 2: No data to analyze.
        if snapshot.total_published < self.config.min_data_points_for_llm
            && snapshot.goals.is_empty()
        {
            return PreAnalysisResult {
                rule_proposals: vec![],
                should_call_llm: false,
                reason: format!(
                    "Not enough data ({} published, 0 goals) to warrant analysis",
                    snapshot.total_published
                ),
            };
        }

        // Rule 3: Cost alert.
        if snapshot.total_llm_cost > self.config.cost_alert_threshold {
            rule_proposals.push(RuleProposal {
                action_type: "change_model".to_string(),
                title: "LLM cost exceeds threshold".to_string(),
                description: format!(
                    "Total LLM cost is ${:.2} which exceeds the ${:.2} threshold. \
                     Consider switching to a cheaper model or reducing output volume.",
                    snapshot.total_llm_cost, self.config.cost_alert_threshold
                ),
                reasoning: format!(
                    "Cost ${:.2} > threshold ${:.2}. ROI is {:.1}x.",
                    snapshot.total_llm_cost,
                    self.config.cost_alert_threshold,
                    if snapshot.total_llm_cost > 0.0 {
                        snapshot.total_revenue / snapshot.total_llm_cost
                    } else {
                        0.0
                    }
                ),
                confidence: 0.90,
                risk_level: "low".to_string(),
                pipeline_id: None,
                proposed_config: None,
                source_rule: "cost_alert".to_string(),
            });
        }

        // Rule 4: Urgent goal proposals.
        for goal in &snapshot.goals {
            if goal.status != GoalStatus::Active.to_string() {
                continue;
            }
            if goal.is_at_risk() {
                let proposal = self.generate_goal_urgency_proposal(goal);
                if let Some(p) = proposal {
                    rule_proposals.push(p);
                }
            }
        }

        // Rule 5: Zero revenue — suggest scaling up if we have content.
        if snapshot.total_revenue <= 0.0 && snapshot.total_published > 5 {
            rule_proposals.push(RuleProposal {
                action_type: "custom".to_string(),
                title: "No revenue detected despite published content".to_string(),
                description: format!(
                    "{} articles published but $0 revenue. Verify AdSense/affiliate \
                     integration or add monetization to content.",
                    snapshot.total_published
                ),
                reasoning: "Content exists but generates no revenue. \
                            Either monetization is not set up or data sources are not configured."
                    .to_string(),
                confidence: 0.80,
                risk_level: "medium".to_string(),
                pipeline_id: None,
                proposed_config: None,
                source_rule: "zero_revenue".to_string(),
            });
        }

        // Determine if LLM should be called.
        // Always call LLM if there's real data to analyze, unless blocked by pending limit.
        let should_call_llm = snapshot.total_published >= self.config.min_data_points_for_llm
            || !snapshot.goals.is_empty();

        let reason = if should_call_llm {
            format!(
                "Calling LLM: {} published, {} goals, {} rule proposals generated",
                snapshot.total_published,
                snapshot.goals.len(),
                rule_proposals.len()
            )
        } else {
            "Insufficient data for LLM analysis".to_string()
        };

        PreAnalysisResult {
            rule_proposals,
            should_call_llm,
            reason,
        }
    }

    /// Validate an LLM-proposed action against safety constraints.
    pub fn validate_proposal(
        &self,
        action_type: &str,
        risk_level: &str,
        confidence: f64,
        snapshot: &SystemSnapshot,
    ) -> Result<(), String> {
        // Block high-risk actions when cost is already high.
        if risk_level == "high" && snapshot.total_llm_cost > self.config.cost_alert_threshold {
            return Err("High-risk action blocked: LLM cost already exceeds threshold".to_string());
        }

        // Block pipeline creation if we already have many pipelines.
        if action_type == "create_pipeline" && snapshot.pipeline_count >= 10 {
            return Err(format!(
                "Pipeline creation blocked: already {} pipelines",
                snapshot.pipeline_count
            ));
        }

        // Block very low confidence proposals.
        if confidence < 0.3 {
            return Err(format!(
                "Proposal blocked: confidence {:.0}% is below 30% minimum",
                confidence * 100.0
            ));
        }

        Ok(())
    }

    /// Adjust the confidence score based on goal alignment.
    ///
    /// Proposals that directly address at-risk goals get a confidence boost.
    pub fn adjust_confidence_for_goals(
        &self,
        base_confidence: f64,
        action_type: &str,
        pipeline_id: Option<&str>,
        snapshot: &SystemSnapshot,
    ) -> f64 {
        let mut adjusted = base_confidence;

        // Boost for at-risk goals that this action might help.
        let at_risk_goals: Vec<&GoalSnapshot> =
            snapshot.goals.iter().filter(|g| g.is_at_risk()).collect();

        if !at_risk_goals.is_empty() {
            // Scale-up or add-source actions aligned with content_count goals.
            let content_goals_at_risk = at_risk_goals.iter().any(|g| {
                matches!(
                    g.metric_type.as_str(),
                    "content_count" | "pageviews" | "clicks"
                )
            });
            if content_goals_at_risk
                && matches!(action_type, "scale_up" | "add_source" | "create_pipeline")
            {
                adjusted += 0.05;
            }

            // Revenue goals at risk boost revenue-related actions.
            let revenue_at_risk = at_risk_goals.iter().any(|g| g.metric_type == "revenue");
            if revenue_at_risk && matches!(action_type, "scale_up" | "change_niche") {
                adjusted += 0.05;
            }
        }

        // Penalty for actions on pipelines not associated with any goal.
        if let Some(pid) = pipeline_id {
            let has_goal = snapshot
                .goals
                .iter()
                .any(|g| g.id == pid || g.name.contains(pid));
            if !has_goal && adjusted > 0.7 {
                adjusted -= 0.03;
            }
        }

        adjusted.clamp(0.0, 1.0)
    }

    /// Generate an urgency proposal for an at-risk goal.
    fn generate_goal_urgency_proposal(&self, goal: &GoalSnapshot) -> Option<RuleProposal> {
        let metric: GoalMetricType = goal.metric_type.parse().ok()?;
        let gap = goal.gap();
        let pct = goal.progress_pct();

        let (action_type, title, description) = match metric {
            GoalMetricType::Revenue => (
                "scale_up",
                format!("Urgent: Revenue goal '{}' at risk ({:.0}%)", goal.name, pct),
                format!(
                    "Revenue goal needs ${:.2} more {} to reach target. \
                     Consider increasing content output or adding higher-CPC niches.",
                    gap, goal.target_unit
                ),
            ),
            GoalMetricType::ContentCount => (
                "change_frequency",
                format!("Urgent: Content goal '{}' at risk ({:.0}%)", goal.name, pct),
                format!(
                    "Need {:.0} more {} to reach target. \
                     Consider increasing publishing frequency.",
                    gap, goal.target_unit
                ),
            ),
            GoalMetricType::Pageviews | GoalMetricType::Clicks => (
                "custom",
                format!("Urgent: Traffic goal '{}' at risk ({:.0}%)", goal.name, pct),
                format!(
                    "Need {:.0} more {} to reach target. \
                     Consider SEO optimization or content promotion.",
                    gap, goal.target_unit
                ),
            ),
            GoalMetricType::CostLimit => {
                // Cost limit is inverse — current should be BELOW target.
                if goal.current_value > goal.target_value {
                    (
                        "change_model",
                        format!("Urgent: Cost goal '{}' exceeded ({:.0}%)", goal.name, pct),
                        format!(
                            "LLM cost ${:.2} exceeds limit ${:.2}. \
                             Switch to a cheaper model immediately.",
                            goal.current_value, goal.target_value
                        ),
                    )
                } else {
                    return None;
                }
            }
            _ => return None,
        };

        Some(RuleProposal {
            action_type: action_type.to_string(),
            title,
            description,
            reasoning: format!(
                "Goal '{}' is at risk: {:.1}/{:.1} {} ({:.0}%). Priority: {}.",
                goal.name,
                goal.current_value,
                goal.target_value,
                goal.target_unit,
                pct,
                goal.priority
            ),
            confidence: 0.85,
            risk_level: if pct < 20.0 { "medium" } else { "low" }.to_string(),
            pipeline_id: None,
            proposed_config: None,
            source_rule: "goal_urgency".to_string(),
        })
    }
}

impl Default for DecisionEngine {
    fn default() -> Self {
        Self::new(DecisionEngineConfig::default())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    fn make_snapshot() -> SystemSnapshot {
        SystemSnapshot {
            total_published: 50,
            total_llm_cost: 2.50,
            total_revenue: 5.00,
            pending_proposals: 0,
            goals: vec![],
            pipeline_count: 2,
        }
    }

    fn make_goal(metric: &str, target: f64, current: f64, priority: i32) -> GoalSnapshot {
        GoalSnapshot {
            id: format!("goal-{metric}"),
            name: format!("Test {metric} goal"),
            metric_type: metric.to_string(),
            target_value: target,
            current_value: current,
            target_unit: "units".to_string(),
            priority,
            deadline: None,
            status: "active".to_string(),
        }
    }

    #[test]
    fn pre_analyze_skips_when_too_many_pending() {
        let engine = DecisionEngine::default();
        let mut snap = make_snapshot();
        snap.pending_proposals = 15;

        let result = engine.pre_analyze(&snap);
        assert!(!result.should_call_llm);
        assert!(result.rule_proposals.is_empty());
        assert!(result.reason.contains("pending"));
    }

    #[test]
    fn pre_analyze_skips_when_no_data() {
        let engine = DecisionEngine::default();
        let snap = SystemSnapshot {
            total_published: 0,
            total_llm_cost: 0.0,
            total_revenue: 0.0,
            pending_proposals: 0,
            goals: vec![],
            pipeline_count: 1,
        };

        let result = engine.pre_analyze(&snap);
        assert!(!result.should_call_llm);
    }

    #[test]
    fn pre_analyze_calls_llm_with_data() {
        let engine = DecisionEngine::default();
        let snap = make_snapshot();

        let result = engine.pre_analyze(&snap);
        assert!(result.should_call_llm);
    }

    #[test]
    fn pre_analyze_generates_cost_alert() {
        let engine = DecisionEngine::default();
        let mut snap = make_snapshot();
        snap.total_llm_cost = 10.0;

        let result = engine.pre_analyze(&snap);
        assert!(result.should_call_llm);
        assert!(
            result
                .rule_proposals
                .iter()
                .any(|p| p.source_rule == "cost_alert")
        );
    }

    #[test]
    fn pre_analyze_generates_zero_revenue_alert() {
        let engine = DecisionEngine::default();
        let mut snap = make_snapshot();
        snap.total_revenue = 0.0;
        snap.total_published = 20;

        let result = engine.pre_analyze(&snap);
        assert!(
            result
                .rule_proposals
                .iter()
                .any(|p| p.source_rule == "zero_revenue")
        );
    }

    #[test]
    fn pre_analyze_generates_goal_urgency() {
        let engine = DecisionEngine::default();
        let mut snap = make_snapshot();
        snap.goals.push(make_goal("revenue", 100.0, 10.0, 1));

        let result = engine.pre_analyze(&snap);
        assert!(
            result
                .rule_proposals
                .iter()
                .any(|p| p.source_rule == "goal_urgency")
        );
    }

    #[test]
    fn goal_snapshot_progress_and_gap() {
        let g = make_goal("revenue", 100.0, 40.0, 1);
        assert!((g.progress_pct() - 40.0).abs() < 0.1);
        assert!((g.gap() - 60.0).abs() < 0.1);
    }

    #[test]
    fn goal_snapshot_is_at_risk_low_progress() {
        let g = make_goal("revenue", 100.0, 20.0, 1);
        assert!(g.is_at_risk());
    }

    #[test]
    fn goal_snapshot_not_at_risk_high_progress() {
        let g = make_goal("revenue", 100.0, 80.0, 1);
        assert!(!g.is_at_risk());
    }

    #[test]
    fn goal_snapshot_at_risk_deadline() {
        let mut g = make_goal("revenue", 100.0, 60.0, 1);
        // Deadline in 3 days, only 60% done.
        g.deadline = Some(chrono::Utc::now().timestamp() + 3 * 86400);
        assert!(g.is_at_risk());
    }

    #[test]
    fn validate_proposal_blocks_high_risk_when_cost_high() {
        let engine = DecisionEngine::default();
        let mut snap = make_snapshot();
        snap.total_llm_cost = 10.0;

        let result = engine.validate_proposal("scale_up", "high", 0.9, &snap);
        assert!(result.is_err());
    }

    #[test]
    fn validate_proposal_blocks_low_confidence() {
        let engine = DecisionEngine::default();
        let snap = make_snapshot();

        let result = engine.validate_proposal("scale_up", "low", 0.2, &snap);
        assert!(result.is_err());
    }

    #[test]
    fn validate_proposal_blocks_too_many_pipelines() {
        let engine = DecisionEngine::default();
        let mut snap = make_snapshot();
        snap.pipeline_count = 12;

        let result = engine.validate_proposal("create_pipeline", "low", 0.9, &snap);
        assert!(result.is_err());
    }

    #[test]
    fn validate_proposal_allows_good_proposal() {
        let engine = DecisionEngine::default();
        let snap = make_snapshot();

        let result = engine.validate_proposal("scale_up", "low", 0.85, &snap);
        assert!(result.is_ok());
    }

    #[test]
    fn adjust_confidence_boosts_for_at_risk_goal() {
        let engine = DecisionEngine::default();
        let mut snap = make_snapshot();
        snap.goals.push(make_goal("content_count", 100.0, 10.0, 1));

        let base = 0.80;
        let adjusted = engine.adjust_confidence_for_goals(base, "scale_up", None, &snap);
        assert!(adjusted > base);
    }

    #[test]
    fn adjust_confidence_clamped_to_1() {
        let engine = DecisionEngine::default();
        let mut snap = make_snapshot();
        snap.goals.push(make_goal("revenue", 100.0, 5.0, 1));
        snap.goals.push(make_goal("content_count", 100.0, 5.0, 1));

        let adjusted = engine.adjust_confidence_for_goals(0.99, "scale_up", None, &snap);
        assert!(adjusted <= 1.0);
    }

    #[test]
    fn config_defaults() {
        let config: DecisionEngineConfig = serde_json::from_value(serde_json::json!({})).unwrap();
        assert!((config.cost_alert_threshold - 5.0).abs() < 0.01);
        assert_eq!(config.min_data_points_for_llm, 1);
        assert_eq!(config.max_pending_proposals, 10);
    }

    #[test]
    fn goal_urgency_cost_limit_exceeded() {
        let engine = DecisionEngine::default();
        let g = GoalSnapshot {
            id: "g1".to_string(),
            name: "Monthly cost limit".to_string(),
            metric_type: "cost_limit".to_string(),
            target_value: 5.0,
            current_value: 8.0,
            target_unit: "USD".to_string(),
            priority: 1,
            deadline: None,
            status: "active".to_string(),
        };

        let proposal = engine.generate_goal_urgency_proposal(&g);
        assert!(proposal.is_some());
        let p = proposal.unwrap();
        assert_eq!(p.action_type, "change_model");
        assert!(p.title.contains("exceeded"));
    }

    #[test]
    fn goal_urgency_content_count() {
        let engine = DecisionEngine::default();
        let g = make_goal("content_count", 90.0, 10.0, 1);

        let proposal = engine.generate_goal_urgency_proposal(&g);
        assert!(proposal.is_some());
        let p = proposal.unwrap();
        assert_eq!(p.action_type, "change_frequency");
    }
}
