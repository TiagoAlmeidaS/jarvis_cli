//! Intent detection system for analyzing user input and determining user intent.
//!
//! This module provides functionality to detect user intentions from natural language input,
//! enabling the system to route requests to appropriate handlers autonomously.

pub mod detector;
pub mod types;

pub use detector::{IntentDetector, RuleBasedIntentDetector};
pub use types::{Intent, IntentParameters, IntentType};
