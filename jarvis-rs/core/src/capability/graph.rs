//! Knowledge graph for mapping capability dependencies and relationships.

use crate::capability::metadata::CapabilityMetadata;
use crate::capability::registry::CapabilityRegistry;
use anyhow::Result;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use std::collections::HashSet;

/// Represents a relationship between capabilities.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityRelationship {
    /// Source capability ID
    pub from: String,
    /// Target capability ID
    pub to: String,
    /// Type of relationship
    pub relationship_type: RelationshipType,
    /// Strength/weight of the relationship (0.0 to 1.0)
    pub weight: f32,
}

/// Type of relationship between capabilities.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelationshipType {
    /// One capability depends on another
    DependsOn,
    /// One capability is used by another
    UsedBy,
    /// Capabilities are similar/related
    Similar,
    /// One capability extends another
    Extends,
}

/// Knowledge graph for capabilities.
pub struct CapabilityGraph {
    /// Relationships between capabilities
    relationships: HashMap<String, Vec<CapabilityRelationship>>,
    /// Reverse relationships (for UsedBy)
    reverse_relationships: HashMap<String, Vec<CapabilityRelationship>>,
}

impl CapabilityGraph {
    /// Creates a new capability graph.
    pub fn new() -> Self {
        Self {
            relationships: HashMap::new(),
            reverse_relationships: HashMap::new(),
        }
    }

    /// Adds a relationship to the graph.
    pub fn add_relationship(&mut self, relationship: CapabilityRelationship) {
        self.relationships
            .entry(relationship.from.clone())
            .or_insert_with(Vec::new)
            .push(relationship.clone());

        // Add reverse relationship
        let reverse = CapabilityRelationship {
            from: relationship.to.clone(),
            to: relationship.from.clone(),
            relationship_type: match relationship.relationship_type {
                RelationshipType::DependsOn => RelationshipType::UsedBy,
                RelationshipType::UsedBy => RelationshipType::DependsOn,
                RelationshipType::Similar => RelationshipType::Similar,
                RelationshipType::Extends => RelationshipType::Extends,
            },
            weight: relationship.weight,
        };

        self.reverse_relationships
            .entry(reverse.from.clone())
            .or_insert_with(Vec::new)
            .push(reverse);
    }

    /// Builds graph from capability registry.
    pub async fn build_from_registry(&mut self, registry: &dyn CapabilityRegistry) -> Result<()> {
        let capabilities = registry.list().await?;

        for capability in capabilities {
            // Add dependency relationships
            for dep_id in &capability.dependencies {
                let relationship = CapabilityRelationship {
                    from: capability.id.clone(),
                    to: dep_id.clone(),
                    relationship_type: RelationshipType::DependsOn,
                    weight: 1.0,
                };
                self.add_relationship(relationship);
            }
        }

        Ok(())
    }

    /// Gets all dependencies for a capability (transitive).
    pub fn get_all_dependencies(&self, capability_id: &str) -> HashSet<String> {
        let mut dependencies = HashSet::new();
        let mut visited = HashSet::new();
        self.collect_dependencies(capability_id, &mut dependencies, &mut visited);
        dependencies
    }

    /// Recursively collects dependencies.
    fn collect_dependencies(
        &self,
        capability_id: &str,
        dependencies: &mut HashSet<String>,
        visited: &mut HashSet<String>,
    ) {
        if visited.contains(capability_id) {
            return;
        }
        visited.insert(capability_id.to_string());

        if let Some(relationships) = self.relationships.get(capability_id) {
            for rel in relationships {
                if matches!(rel.relationship_type, RelationshipType::DependsOn) {
                    dependencies.insert(rel.to.clone());
                    self.collect_dependencies(&rel.to, dependencies, visited);
                }
            }
        }
    }

    /// Gets all capabilities that use a given capability.
    pub fn get_dependents(&self, capability_id: &str) -> Vec<String> {
        self.reverse_relationships
            .get(capability_id)
            .map(|rels| {
                rels.iter()
                    .filter(|r| matches!(r.relationship_type, RelationshipType::UsedBy))
                    .map(|r| r.to.clone())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Finds a path between two capabilities.
    pub fn find_path(&self, from: &str, to: &str) -> Option<Vec<String>> {
        let mut visited = HashSet::new();
        let mut path = Vec::new();
        if self.dfs_path(from, to, &mut visited, &mut path) {
            Some(path)
        } else {
            None
        }
    }

    /// Depth-first search for path finding.
    fn dfs_path(
        &self,
        current: &str,
        target: &str,
        visited: &mut HashSet<String>,
        path: &mut Vec<String>,
    ) -> bool {
        if current == target {
            path.push(current.to_string());
            return true;
        }

        if visited.contains(current) {
            return false;
        }
        visited.insert(current.to_string());
        path.push(current.to_string());

        if let Some(relationships) = self.relationships.get(current) {
            for rel in relationships {
                if matches!(rel.relationship_type, RelationshipType::DependsOn) {
                    if self.dfs_path(&rel.to, target, visited, path) {
                        return true;
                    }
                }
            }
        }

        path.pop();
        false
    }

    /// Detects circular dependencies.
    pub fn detect_cycles(&self) -> Vec<Vec<String>> {
        let mut cycles = Vec::new();
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();
        let mut path = Vec::new();

        for capability_id in self.relationships.keys() {
            if !visited.contains(capability_id) {
                self.detect_cycle_dfs(
                    capability_id,
                    &mut visited,
                    &mut rec_stack,
                    &mut path,
                    &mut cycles,
                );
            }
        }

        cycles
    }

    /// DFS for cycle detection.
    fn detect_cycle_dfs(
        &self,
        current: &str,
        visited: &mut HashSet<String>,
        rec_stack: &mut HashSet<String>,
        path: &mut Vec<String>,
        cycles: &mut Vec<Vec<String>>,
    ) {
        visited.insert(current.to_string());
        rec_stack.insert(current.to_string());
        path.push(current.to_string());

        if let Some(relationships) = self.relationships.get(current) {
            for rel in relationships {
                if matches!(rel.relationship_type, RelationshipType::DependsOn) {
                    if !visited.contains(&rel.to) {
                        self.detect_cycle_dfs(&rel.to, visited, rec_stack, path, cycles);
                    } else if rec_stack.contains(&rel.to) {
                        // Found a cycle
                        let cycle_start = path.iter().position(|x| x == &rel.to).unwrap();
                        cycles.push(path[cycle_start..].to_vec());
                    }
                }
            }
        }

        rec_stack.remove(current);
        path.pop();
    }
}

impl Default for CapabilityGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_relationship() {
        let mut graph = CapabilityGraph::new();
        let rel = CapabilityRelationship {
            from: "a".to_string(),
            to: "b".to_string(),
            relationship_type: RelationshipType::DependsOn,
            weight: 1.0,
        };

        graph.add_relationship(rel);
        let deps = graph.get_all_dependencies("a");
        assert!(deps.contains("b"));
    }

    #[test]
    fn test_circular_dependency_detection() {
        let mut graph = CapabilityGraph::new();

        graph.add_relationship(CapabilityRelationship {
            from: "a".to_string(),
            to: "b".to_string(),
            relationship_type: RelationshipType::DependsOn,
            weight: 1.0,
        });

        graph.add_relationship(CapabilityRelationship {
            from: "b".to_string(),
            to: "a".to_string(),
            relationship_type: RelationshipType::DependsOn,
            weight: 1.0,
        });

        let cycles = graph.detect_cycles();
        assert!(!cycles.is_empty());
    }
}
