//! Safety layer for autonomous action classification and risk assessment.
//!
//! This module provides functionality to assess the safety of autonomous actions,
//! classify risks, and determine if actions require human approval.

pub mod assessment;
pub mod classifier;
pub mod rules;

pub use assessment::{RiskLevel, SafetyAssessment};
pub use classifier::{ProposedAction, RuleBasedSafetyClassifier, SafetyClassifier};
pub use rules::SafetyRules;
