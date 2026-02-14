//! Knowledge base and learning system.
//!
//! This module provides functionality for accumulating contextual knowledge,
//! learning from interactions, and pattern recognition.

pub mod base;
pub mod learning;
pub mod persistent;

pub use base::{InMemoryKnowledgeBase, Knowledge, KnowledgeBase, KnowledgeError, KnowledgeType};
pub use learning::{
    Interaction, LearningPattern, LearningSystem, Outcome, PatternType, RuleBasedLearningSystem,
};
pub use persistent::PersistentKnowledgeBase;
