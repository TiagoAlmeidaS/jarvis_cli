//! Capability registry system for managing and discovering system capabilities.
//!
//! This module provides functionality to register, discover, and manage capabilities
//! in the system, enabling autonomous decision-making about which capabilities to use.

pub mod graph;
pub mod metadata;
pub mod registry;

pub use graph::CapabilityGraph;
pub use graph::CapabilityRelationship;
pub use graph::RelationshipType;
pub use metadata::CapabilityMetadata;
pub use metadata::CapabilityStatus;
pub use metadata::CapabilityType;
pub use metadata::ParameterMetadata;
pub use metadata::PerformanceMetadata;
pub use metadata::PerformanceProfile;
pub use metadata::ReturnMetadata;
pub use registry::CapabilityRegistry;
pub use registry::InMemoryCapabilityRegistry;
