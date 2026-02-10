use serde::{Deserialize, Serialize};

/// Priority level for improvements
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImprovementPriority {
    Critical,
    High,
    Medium,
    Low,
}

impl std::fmt::Display for ImprovementPriority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ImprovementPriority::Critical => write!(f, "🔴 CRÍTICO"),
            ImprovementPriority::High => write!(f, "🟠 ALTA"),
            ImprovementPriority::Medium => write!(f, "🟡 MÉDIA"),
            ImprovementPriority::Low => write!(f, "🟢 BAIXA"),
        }
    }
}

/// Category of improvement
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImprovementCategory {
    Performance,
    Reliability,
    Cache,
    Skills,
    Database,
    General,
}

impl std::fmt::Display for ImprovementCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ImprovementCategory::Performance => write!(f, "⚡ Performance"),
            ImprovementCategory::Reliability => write!(f, "🛡️ Confiabilidade"),
            ImprovementCategory::Cache => write!(f, "💾 Cache"),
            ImprovementCategory::Skills => write!(f, "🎯 Skills"),
            ImprovementCategory::Database => write!(f, "🗄️ Database"),
            ImprovementCategory::General => write!(f, "📋 Geral"),
        }
    }
}

/// Suggested improvement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Improvement {
    pub priority: ImprovementPriority,
    pub category: ImprovementCategory,
    pub title: String,
    pub description: String,
    pub action: Option<String>,
    pub impact: Option<String>,
}

impl Improvement {
    /// Create a critical improvement
    pub fn critical(category: ImprovementCategory, title: String, description: String) -> Self {
        Self {
            priority: ImprovementPriority::Critical,
            category,
            title,
            description,
            action: None,
            impact: None,
        }
    }

    /// Create a high priority improvement
    pub fn high(category: ImprovementCategory, title: String, description: String) -> Self {
        Self {
            priority: ImprovementPriority::High,
            category,
            title,
            description,
            action: None,
            impact: None,
        }
    }

    /// Create a medium priority improvement
    pub fn medium(category: ImprovementCategory, title: String, description: String) -> Self {
        Self {
            priority: ImprovementPriority::Medium,
            category,
            title,
            description,
            action: None,
            impact: None,
        }
    }

    /// Create a low priority improvement
    pub fn low(category: ImprovementCategory, title: String, description: String) -> Self {
        Self {
            priority: ImprovementPriority::Low,
            category,
            title,
            description,
            action: None,
            impact: None,
        }
    }

    /// Add suggested action
    pub fn with_action(mut self, action: String) -> Self {
        self.action = Some(action);
        self
    }

    /// Add impact description
    pub fn with_impact(mut self, impact: String) -> Self {
        self.impact = Some(impact);
        self
    }
}

/// Group improvements by priority
pub fn group_by_priority(improvements: Vec<Improvement>) -> Vec<(ImprovementPriority, Vec<Improvement>)> {
    let mut critical = Vec::new();
    let mut high = Vec::new();
    let mut medium = Vec::new();
    let mut low = Vec::new();

    for improvement in improvements {
        match improvement.priority {
            ImprovementPriority::Critical => critical.push(improvement),
            ImprovementPriority::High => high.push(improvement),
            ImprovementPriority::Medium => medium.push(improvement),
            ImprovementPriority::Low => low.push(improvement),
        }
    }

    let mut result = Vec::new();
    if !critical.is_empty() {
        result.push((ImprovementPriority::Critical, critical));
    }
    if !high.is_empty() {
        result.push((ImprovementPriority::High, high));
    }
    if !medium.is_empty() {
        result.push((ImprovementPriority::Medium, medium));
    }
    if !low.is_empty() {
        result.push((ImprovementPriority::Low, low));
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_improvement_creation() {
        let improvement = Improvement::high(
            ImprovementCategory::Performance,
            "Slow command".to_string(),
            "Command takes too long".to_string(),
        )
        .with_action("Optimize algorithm".to_string())
        .with_impact("50% faster execution".to_string());

        assert_eq!(improvement.priority, ImprovementPriority::High);
        assert_eq!(improvement.category, ImprovementCategory::Performance);
        assert!(improvement.action.is_some());
        assert!(improvement.impact.is_some());
    }

    #[test]
    fn test_group_by_priority() {
        let improvements = vec![
            Improvement::low(ImprovementCategory::General, "A".into(), "B".into()),
            Improvement::critical(ImprovementCategory::Reliability, "C".into(), "D".into()),
            Improvement::high(ImprovementCategory::Performance, "E".into(), "F".into()),
        ];

        let grouped = group_by_priority(improvements);
        assert_eq!(grouped.len(), 3);
        assert_eq!(grouped[0].0, ImprovementPriority::Critical);
        assert_eq!(grouped[1].0, ImprovementPriority::High);
        assert_eq!(grouped[2].0, ImprovementPriority::Low);
    }

    #[test]
    fn test_improvement_all_priority_levels() {
        let critical = Improvement::critical(
            ImprovementCategory::Reliability,
            "Critical issue".into(),
            "System failing".into(),
        );
        assert_eq!(critical.priority, ImprovementPriority::Critical);

        let high = Improvement::high(
            ImprovementCategory::Performance,
            "High issue".into(),
            "Slow performance".into(),
        );
        assert_eq!(high.priority, ImprovementPriority::High);

        let medium = Improvement::medium(
            ImprovementCategory::Cache,
            "Medium issue".into(),
            "Low hit rate".into(),
        );
        assert_eq!(medium.priority, ImprovementPriority::Medium);

        let low = Improvement::low(
            ImprovementCategory::Skills,
            "Low issue".into(),
            "Minor optimization".into(),
        );
        assert_eq!(low.priority, ImprovementPriority::Low);
    }

    #[test]
    fn test_improvement_builder_pattern() {
        let improvement = Improvement::high(
            ImprovementCategory::Database,
            "Test".into(),
            "Description".into(),
        )
        .with_action("Fix it".into())
        .with_impact("Much better".into());

        assert_eq!(improvement.action.as_ref().unwrap(), "Fix it");
        assert_eq!(improvement.impact.as_ref().unwrap(), "Much better");
    }

    #[test]
    fn test_priority_display() {
        assert_eq!(
            ImprovementPriority::Critical.to_string(),
            "🔴 CRÍTICO"
        );
        assert_eq!(
            ImprovementPriority::High.to_string(),
            "🟠 ALTA"
        );
        assert_eq!(
            ImprovementPriority::Medium.to_string(),
            "🟡 MÉDIA"
        );
        assert_eq!(
            ImprovementPriority::Low.to_string(),
            "🟢 BAIXA"
        );
    }

    #[test]
    fn test_category_display() {
        assert_eq!(
            ImprovementCategory::Performance.to_string(),
            "⚡ Performance"
        );
        assert_eq!(
            ImprovementCategory::Reliability.to_string(),
            "🛡️ Confiabilidade"
        );
        assert_eq!(
            ImprovementCategory::Cache.to_string(),
            "💾 Cache"
        );
        assert_eq!(
            ImprovementCategory::Skills.to_string(),
            "🎯 Skills"
        );
        assert_eq!(
            ImprovementCategory::Database.to_string(),
            "🗄️ Database"
        );
        assert_eq!(
            ImprovementCategory::General.to_string(),
            "📋 Geral"
        );
    }

    #[test]
    fn test_group_by_priority_empty() {
        let improvements: Vec<Improvement> = vec![];
        let grouped = group_by_priority(improvements);
        assert!(grouped.is_empty());
    }

    #[test]
    fn test_group_by_priority_single_priority() {
        let improvements = vec![
            Improvement::critical(ImprovementCategory::Reliability, "A".into(), "B".into()),
            Improvement::critical(ImprovementCategory::Performance, "C".into(), "D".into()),
        ];

        let grouped = group_by_priority(improvements);
        assert_eq!(grouped.len(), 1);
        assert_eq!(grouped[0].0, ImprovementPriority::Critical);
        assert_eq!(grouped[0].1.len(), 2);
    }

    #[test]
    fn test_group_by_priority_ordering() {
        // Add in random order
        let improvements = vec![
            Improvement::low(ImprovementCategory::General, "1".into(), "Low".into()),
            Improvement::critical(ImprovementCategory::Reliability, "2".into(), "Critical".into()),
            Improvement::medium(ImprovementCategory::Cache, "3".into(), "Medium".into()),
            Improvement::high(ImprovementCategory::Performance, "4".into(), "High".into()),
            Improvement::low(ImprovementCategory::Skills, "5".into(), "Low2".into()),
        ];

        let grouped = group_by_priority(improvements);

        // Should be ordered: Critical, High, Medium, Low
        assert_eq!(grouped.len(), 4);
        assert_eq!(grouped[0].0, ImprovementPriority::Critical);
        assert_eq!(grouped[1].0, ImprovementPriority::High);
        assert_eq!(grouped[2].0, ImprovementPriority::Medium);
        assert_eq!(grouped[3].0, ImprovementPriority::Low);

        // Check counts
        assert_eq!(grouped[0].1.len(), 1); // 1 critical
        assert_eq!(grouped[1].1.len(), 1); // 1 high
        assert_eq!(grouped[2].1.len(), 1); // 1 medium
        assert_eq!(grouped[3].1.len(), 2); // 2 low
    }
}
