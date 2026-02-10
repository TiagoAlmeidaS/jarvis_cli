//! Capability metadata structures for the capability registry.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Represents metadata for a capability in the system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityMetadata {
    /// Unique identifier for the capability
    pub id: String,
    /// Name of the capability
    pub name: String,
    /// Type of capability (tool, handler, service, skill)
    pub capability_type: CapabilityType,
    /// Description of what the capability does
    pub description: String,
    /// Category of the capability
    pub category: Option<String>,
    /// Parameters required by the capability
    pub parameters: Vec<ParameterMetadata>,
    /// Return type information
    pub returns: Option<ReturnMetadata>,
    /// Dependencies on other capabilities
    pub dependencies: Vec<String>,
    /// Prerequisites for using this capability
    pub prerequisites: Vec<String>,
    /// Tags for searchability
    pub tags: Vec<String>,
    /// Version of the capability
    pub version: String,
    /// Status of the capability
    pub status: CapabilityStatus,
    /// Performance and reliability metadata
    pub performance_metadata: PerformanceMetadata,
    /// Timestamp when capability was last updated
    pub last_updated: i64,
    /// Usage count
    pub usage_count: u64,
}

/// Type of capability.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityType {
    /// A tool that can be executed
    Tool,
    /// A handler for processing requests
    Handler,
    /// A service providing functionality
    Service,
    /// A skill (autonomous capability)
    Skill,
}

/// Status of a capability.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityStatus {
    /// Capability is active and available
    Active,
    /// Capability is deprecated but still available
    Deprecated,
    /// Capability is experimental
    Experimental,
    /// Capability is disabled
    Disabled,
}

/// Metadata for a parameter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterMetadata {
    /// Parameter name
    pub name: String,
    /// Parameter type
    pub param_type: String,
    /// Whether parameter is required
    pub required: bool,
    /// Description of the parameter
    pub description: String,
    /// Default value if any
    pub default_value: Option<String>,
}

/// Metadata for return value.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReturnMetadata {
    /// Return type
    pub return_type: String,
    /// Description of return value
    pub description: String,
}

/// Performance and reliability metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetadata {
    /// Performance profile (fast, medium, slow)
    pub performance_profile: PerformanceProfile,
    /// Reliability score (0-100)
    pub reliability_score: u8,
    /// Average execution time in milliseconds
    pub avg_execution_time_ms: Option<u64>,
    /// Success rate (0.0 to 1.0)
    pub success_rate: Option<f32>,
}

/// Performance profile.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PerformanceProfile {
    /// Fast execution
    Fast,
    /// Medium speed
    Medium,
    /// Slow execution
    Slow,
}

impl CapabilityMetadata {
    /// Creates a new capability metadata.
    pub fn new(
        id: String,
        name: String,
        capability_type: CapabilityType,
        description: String,
    ) -> Self {
        Self {
            id,
            name,
            capability_type,
            description,
            category: None,
            parameters: vec![],
            returns: None,
            dependencies: vec![],
            prerequisites: vec![],
            tags: vec![],
            version: "1.0.0".to_string(),
            status: CapabilityStatus::Active,
            performance_metadata: PerformanceMetadata {
                performance_profile: PerformanceProfile::Medium,
                reliability_score: 80,
                avg_execution_time_ms: None,
                success_rate: None,
            },
            last_updated: Self::current_timestamp(),
            usage_count: 0,
        }
    }

    /// Returns current timestamp.
    fn current_timestamp() -> i64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64
    }

    /// Checks if capability is available for use.
    pub fn is_available(&self) -> bool {
        matches!(self.status, CapabilityStatus::Active | CapabilityStatus::Experimental)
    }

    /// Increments usage count.
    pub fn increment_usage(&mut self) {
        self.usage_count += 1;
        self.last_updated = Self::current_timestamp();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capability_metadata_creation() {
        let metadata = CapabilityMetadata::new(
            "test-id".to_string(),
            "test-capability".to_string(),
            CapabilityType::Tool,
            "A test capability".to_string(),
        );

        assert_eq!(metadata.id, "test-id");
        assert_eq!(metadata.name, "test-capability");
        assert_eq!(metadata.capability_type, CapabilityType::Tool);
        assert!(metadata.is_available());
    }

    #[test]
    fn test_capability_availability() {
        let mut metadata = CapabilityMetadata::new(
            "test-id".to_string(),
            "test".to_string(),
            CapabilityType::Tool,
            "Test".to_string(),
        );

        assert!(metadata.is_available());

        metadata.status = CapabilityStatus::Deprecated;
        assert!(metadata.is_available());

        metadata.status = CapabilityStatus::Disabled;
        assert!(!metadata.is_available());
    }

    #[test]
    fn test_increment_usage() {
        let mut metadata = CapabilityMetadata::new(
            "test-id".to_string(),
            "test".to_string(),
            CapabilityType::Tool,
            "Test".to_string(),
        );

        assert_eq!(metadata.usage_count, 0);
        metadata.increment_usage();
        assert_eq!(metadata.usage_count, 1);
    }
}
