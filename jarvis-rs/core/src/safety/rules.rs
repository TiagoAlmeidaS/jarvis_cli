//! Safety rules for autonomous action classification.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Safety rules configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyRules {
    /// Actions that are always safe to execute autonomously
    pub autonomous_whitelist: HashSet<String>,
    /// Actions that are prohibited from autonomous execution
    pub prohibited_actions: HashSet<String>,
    /// Risk thresholds for different action types
    pub risk_thresholds: std::collections::HashMap<String, f32>,
}

impl SafetyRules {
    /// Creates default safety rules.
    pub fn default() -> Self {
        let mut autonomous_whitelist = HashSet::new();
        autonomous_whitelist.insert("fix_test_file".to_string());
        autonomous_whitelist.insert("fix_comment".to_string());
        autonomous_whitelist.insert("fix_typo_in_docs".to_string());
        autonomous_whitelist.insert("update_test_assertion".to_string());
        autonomous_whitelist.insert("format_code".to_string());
        autonomous_whitelist.insert("add_comment".to_string());

        let mut prohibited_actions = HashSet::new();
        prohibited_actions.insert("delete_file".to_string());
        prohibited_actions.insert("drop_table".to_string());
        prohibited_actions.insert("change_api_endpoint".to_string());
        prohibited_actions.insert("modify_authentication".to_string());
        prohibited_actions.insert("delete_production_data".to_string());
        prohibited_actions.insert("modify_db_migration".to_string());
        prohibited_actions.insert("change_security_settings".to_string());

        let mut risk_thresholds = std::collections::HashMap::new();
        risk_thresholds.insert("test_file".to_string(), 0.3);
        risk_thresholds.insert("production_code".to_string(), 0.9);
        risk_thresholds.insert("config_file".to_string(), 0.8);
        risk_thresholds.insert("database".to_string(), 0.95);

        Self {
            autonomous_whitelist,
            prohibited_actions,
            risk_thresholds,
        }
    }

    /// Checks if an action is in the whitelist.
    pub fn is_whitelisted(&self, action: &str) -> bool {
        self.autonomous_whitelist.contains(action)
    }

    /// Checks if an action is prohibited.
    pub fn is_prohibited(&self, action: &str) -> bool {
        self.prohibited_actions.contains(action)
    }

    /// Gets risk threshold for a category.
    pub fn get_risk_threshold(&self, category: &str) -> f32 {
        self.risk_thresholds.get(category).copied().unwrap_or(0.7)
    }
}

impl Default for SafetyRules {
    fn default() -> Self {
        Self::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_rules() {
        let rules = SafetyRules::default();
        assert!(rules.is_whitelisted("fix_test_file"));
        assert!(rules.is_prohibited("delete_file"));
    }

    #[test]
    fn test_risk_thresholds() {
        let rules = SafetyRules::default();
        assert!(rules.get_risk_threshold("test_file") < 0.5);
        assert!(rules.get_risk_threshold("production_code") > 0.8);
    }
}
