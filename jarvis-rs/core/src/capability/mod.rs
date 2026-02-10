//! Capability registry system for managing and discovering system capabilities.
//!
//! This module provides functionality to register, discover, and manage capabilities
//! in the system, enabling autonomous decision-making about which capabilities to use.

pub mod graph;
pub mod metadata;
pub mod registry;

pub use graph::{CapabilityGraph, CapabilityRelationship, RelationshipType};
pub use metadata::{
    CapabilityMetadata, CapabilityStatus, CapabilityType, ParameterMetadata,
    PerformanceMetadata, PerformanceProfile, ReturnMetadata,
};
pub use registry::{CapabilityRegistry, InMemoryCapabilityRegistry};
