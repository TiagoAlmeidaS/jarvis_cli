//! Knowledge base and learning system.
//!
//! This module provides functionality for accumulating contextual knowledge,
//! learning from interactions, and pattern recognition.

pub mod base;
pub mod learning;
pub mod persistent;

pub use base::InMemoryKnowledgeBase;
pub use base::Knowledge;
pub use base::KnowledgeBase;
pub use base::KnowledgeError;
pub use base::KnowledgeType;
pub use learning::Interaction;
pub use learning::LearningPattern;
pub use learning::LearningSystem;
pub use learning::Outcome;
pub use learning::PatternType;
pub use learning::RuleBasedLearningSystem;
pub use persistent::PersistentKnowledgeBase;
