use crate::config::Config;
use crate::protocol::SandboxPolicy;
use jarvis_protocol::openai_models::ReasoningEffort;
use serde::Deserialize;
use serde::Serialize;

/// Base instructions for the orchestrator role.
const ORCHESTRATOR_PROMPT: &str = include_str!("../../templates/agents/orchestrator.md");
/// Base instructions for the planner role.
const PLANNER_PROMPT: &str = include_str!("../../templates/agents/planner.md");
/// Base instructions for the developer role.
const DEVELOPER_PROMPT: &str = include_str!("../../templates/agents/developer.md");
/// Base instructions for the reviewer role.
const REVIEWER_PROMPT: &str = include_str!("../../templates/agents/reviewer.md");
/// Default model override used.
// TODO(jif) update when we have something smarter.
const EXPLORER_MODEL: &str = "gpt-5.2-Jarvis";

/// Enumerated list of all supported agent roles.
const ALL_ROLES: [AgentRole; 6] = [
    AgentRole::Default,
    AgentRole::Explorer,
    AgentRole::Worker,
    AgentRole::Planner,
    AgentRole::Developer,
    AgentRole::Reviewer,
    // TODO(jif) add when we have stable prompts + models
    // AgentRole::Orchestrator,
];

/// Hard-coded agent role selection used when spawning sub-agents.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentRole {
    /// Inherit the parent agent's configuration unchanged.
    Default,
    /// Coordination-only agent that delegates to workers.
    Orchestrator,
    /// Task-executing agent with a fixed model override.
    Worker,
    /// Task-executing agent with a fixed model override.
    Explorer,
    /// Strategic agent that analyzes requests and creates structured plans.
    Planner,
    /// Implementation agent that writes code according to plans.
    Developer,
    /// Quality assurance agent that reviews code implementations.
    Reviewer,
}

/// Immutable profile data that drives per-agent configuration overrides.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AgentProfile {
    /// Optional base instructions override.
    pub base_instructions: Option<&'static str>,
    /// Optional model override.
    pub model: Option<&'static str>,
    /// Optional reasoning effort override.
    pub reasoning_effort: Option<ReasoningEffort>,
    /// Whether to force a read-only sandbox policy.
    pub read_only: bool,
    /// Description to include in the tool specs.
    pub description: &'static str,
}

impl AgentRole {
    /// Returns the string values used by JSON schema enums.
    pub fn enum_values() -> Vec<String> {
        ALL_ROLES
            .iter()
            .filter_map(|role| {
                let description = role.profile().description;
                serde_json::to_string(role)
                    .map(|role| {
                        let description = if !description.is_empty() {
                            format!(r#", "description": {description}"#)
                        } else {
                            String::new()
                        };
                        format!(r#"{{ "name": {role}{description}}}"#)
                    })
                    .ok()
            })
            .collect()
    }

    /// Returns the hard-coded profile for this role.
    pub fn profile(self) -> AgentProfile {
        match self {
            AgentRole::Default => AgentProfile::default(),
            AgentRole::Orchestrator => AgentProfile {
                base_instructions: Some(ORCHESTRATOR_PROMPT),
                ..Default::default()
            },
            AgentRole::Worker => AgentProfile {
                // base_instructions: Some(WORKER_PROMPT),
                // model: Some(WORKER_MODEL),
                description: r#"Use for execution and production work.
Typical tasks:
- Implement part of a feature
- Fix tests or bugs
- Split large refactors into independent chunks
Rules:
- Explicitly assign **ownership** of the task (files / responsibility).
- Always tell workers they are **not alone in the codebase**, and they should ignore edits made by others without touching them"#,
                ..Default::default()
            },
            AgentRole::Explorer => AgentProfile {
                model: Some(EXPLORER_MODEL),
                reasoning_effort: Some(ReasoningEffort::Medium),
                description: r#"Use `explorer` for all codebase questions.
Explorers are fast and authoritative.
Always prefer them over manual search or file reading.
Rules:
- Ask explorers first and precisely.
- Do not re-read or re-search code they cover.
- Trust explorer results without verification.
- Run explorers in parallel when useful.
- Reuse existing explorers for related questions.
                "#,
                ..Default::default()
            },
            AgentRole::Planner => AgentProfile {
                base_instructions: Some(PLANNER_PROMPT),
                description: r#"Use `planner` for strategic analysis and plan creation.
Planners analyze user requests and create structured, actionable plans.
Typical tasks:
- Break down complex tasks into clear steps
- Create detailed plans with checklists
- Identify dependencies and risks
- Coordinate between different agents
Rules:
- Create clear, numbered steps that are actionable
- Include dependencies between steps
- Transfer to Developer when plan is complete and approved
- Ask for user confirmation before transferring"#,
                ..Default::default()
            },
            AgentRole::Developer => AgentProfile {
                base_instructions: Some(DEVELOPER_PROMPT),
                description: r#"Use `developer` for code implementation.
Developers implement code following plans created by Planner.
Typical tasks:
- Implement features according to plan
- Write clean, maintainable code
- Follow existing codebase patterns
- Create or update tests
Rules:
- Follow the plan step by step
- Match existing codebase style
- Write production-ready code
- Transfer to Reviewer when implementation is complete"#,
                ..Default::default()
            },
            AgentRole::Reviewer => AgentProfile {
                base_instructions: Some(REVIEWER_PROMPT),
                description: r#"Use `reviewer` for code quality assurance.
Reviewers review code implementations and ensure quality standards.
Typical tasks:
- Review code for correctness and quality
- Identify bugs, risks, and improvements
- Ensure code follows best practices
- Verify implementation matches plan
Rules:
- Prioritize findings by severity
- Include file and line references
- Be constructive and specific
- Approve when code meets standards"#,
                ..Default::default()
            },
        }
    }

    /// Applies this role's profile onto the provided config.
    pub fn apply_to_config(self, config: &mut Config) -> Result<(), String> {
        let profile = self.profile();
        if let Some(base_instructions) = profile.base_instructions {
            config.base_instructions = Some(base_instructions.to_string());
        }
        if let Some(model) = profile.model {
            config.model = Some(model.to_string());
        }
        if let Some(reasoning_effort) = profile.reasoning_effort {
            config.model_reasoning_effort = Some(reasoning_effort)
        }
        if profile.read_only {
            config
                .sandbox_policy
                .set(SandboxPolicy::new_read_only_policy())
                .map_err(|err| format!("sandbox_policy is invalid: {err}"))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::test_config;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_planner_profile() {
        let profile = AgentRole::Planner.profile();
        assert!(profile.base_instructions.is_some());
        assert_eq!(profile.base_instructions.unwrap(), PLANNER_PROMPT);
        assert!(profile.model.is_none());
        assert!(profile.reasoning_effort.is_none());
        assert!(!profile.read_only);
        assert!(!profile.description.is_empty());
        assert!(profile.description.contains("planner"));
        assert!(profile.description.contains("strategic"));
    }

    #[test]
    fn test_developer_profile() {
        let profile = AgentRole::Developer.profile();
        assert!(profile.base_instructions.is_some());
        assert_eq!(profile.base_instructions.unwrap(), DEVELOPER_PROMPT);
        assert!(profile.model.is_none());
        assert!(profile.reasoning_effort.is_none());
        assert!(!profile.read_only);
        assert!(!profile.description.is_empty());
        assert!(profile.description.contains("developer"));
        assert!(profile.description.contains("implementation"));
    }

    #[test]
    fn test_reviewer_profile() {
        let profile = AgentRole::Reviewer.profile();
        assert!(profile.base_instructions.is_some());
        assert_eq!(profile.base_instructions.unwrap(), REVIEWER_PROMPT);
        assert!(profile.model.is_none());
        assert!(profile.reasoning_effort.is_none());
        assert!(!profile.read_only);
        assert!(!profile.description.is_empty());
        assert!(profile.description.contains("reviewer"));
        assert!(profile.description.contains("quality"));
    }

    #[test]
    fn test_planner_apply_to_config() {
        let mut config = test_config();
        let original_instructions = config.base_instructions.clone();

        AgentRole::Planner
            .apply_to_config(&mut config)
            .expect("apply planner role");

        assert!(config.base_instructions.is_some());
        assert_ne!(config.base_instructions, original_instructions);
        assert_eq!(config.base_instructions.as_ref().unwrap(), PLANNER_PROMPT);
    }

    #[test]
    fn test_developer_apply_to_config() {
        let mut config = test_config();
        let original_instructions = config.base_instructions.clone();

        AgentRole::Developer
            .apply_to_config(&mut config)
            .expect("apply developer role");

        assert!(config.base_instructions.is_some());
        assert_ne!(config.base_instructions, original_instructions);
        assert_eq!(config.base_instructions.as_ref().unwrap(), DEVELOPER_PROMPT);
    }

    #[test]
    fn test_reviewer_apply_to_config() {
        let mut config = test_config();
        let original_instructions = config.base_instructions.clone();

        AgentRole::Reviewer
            .apply_to_config(&mut config)
            .expect("apply reviewer role");

        assert!(config.base_instructions.is_some());
        assert_ne!(config.base_instructions, original_instructions);
        assert_eq!(config.base_instructions.as_ref().unwrap(), REVIEWER_PROMPT);
    }

    #[test]
    fn test_new_roles_in_enum_values() {
        let enum_values = AgentRole::enum_values();
        let enum_values_str = enum_values.join(" ");

        assert!(enum_values_str.contains("planner"));
        assert!(enum_values_str.contains("developer"));
        assert!(enum_values_str.contains("reviewer"));
    }

    #[test]
    fn test_new_roles_in_all_roles() {
        assert!(ALL_ROLES.contains(&AgentRole::Planner));
        assert!(ALL_ROLES.contains(&AgentRole::Developer));
        assert!(ALL_ROLES.contains(&AgentRole::Reviewer));
    }

    #[test]
    fn test_planner_serialization() {
        let role = AgentRole::Planner;
        let serialized = serde_json::to_string(&role).expect("serialize planner");
        assert_eq!(serialized, "\"planner\"");
        let deserialized: AgentRole =
            serde_json::from_str(&serialized).expect("deserialize planner");
        assert_eq!(deserialized, role);
    }

    #[test]
    fn test_developer_serialization() {
        let role = AgentRole::Developer;
        let serialized = serde_json::to_string(&role).expect("serialize developer");
        assert_eq!(serialized, "\"developer\"");
        let deserialized: AgentRole =
            serde_json::from_str(&serialized).expect("deserialize developer");
        assert_eq!(deserialized, role);
    }

    #[test]
    fn test_reviewer_serialization() {
        let role = AgentRole::Reviewer;
        let serialized = serde_json::to_string(&role).expect("serialize reviewer");
        assert_eq!(serialized, "\"reviewer\"");
        let deserialized: AgentRole =
            serde_json::from_str(&serialized).expect("deserialize reviewer");
        assert_eq!(deserialized, role);
    }

    #[test]
    fn test_planner_prompt_not_empty() {
        assert!(!PLANNER_PROMPT.is_empty());
        assert!(PLANNER_PROMPT.contains("Planner"));
        assert!(PLANNER_PROMPT.contains("plan"));
    }

    #[test]
    fn test_developer_prompt_not_empty() {
        assert!(!DEVELOPER_PROMPT.is_empty());
        assert!(DEVELOPER_PROMPT.contains("Developer"));
        assert!(DEVELOPER_PROMPT.contains("implement"));
    }

    #[test]
    fn test_reviewer_prompt_not_empty() {
        assert!(!REVIEWER_PROMPT.is_empty());
        assert!(REVIEWER_PROMPT.contains("Reviewer"));
        assert!(REVIEWER_PROMPT.contains("review"));
    }

    #[test]
    fn test_all_new_roles_have_descriptions() {
        let planner_desc = AgentRole::Planner.profile().description;
        let developer_desc = AgentRole::Developer.profile().description;
        let reviewer_desc = AgentRole::Reviewer.profile().description;

        assert!(!planner_desc.is_empty());
        assert!(!developer_desc.is_empty());
        assert!(!reviewer_desc.is_empty());

        assert!(planner_desc.len() > 50); // Should have meaningful description
        assert!(developer_desc.len() > 50);
        assert!(reviewer_desc.len() > 50);
    }

    #[test]
    fn test_role_equality() {
        assert_eq!(AgentRole::Planner, AgentRole::Planner);
        assert_eq!(AgentRole::Developer, AgentRole::Developer);
        assert_eq!(AgentRole::Reviewer, AgentRole::Reviewer);
        assert_ne!(AgentRole::Planner, AgentRole::Developer);
        assert_ne!(AgentRole::Planner, AgentRole::Reviewer);
        assert_ne!(AgentRole::Developer, AgentRole::Reviewer);
    }

    #[test]
    fn test_role_clone() {
        let planner = AgentRole::Planner;
        let cloned = planner;
        assert_eq!(planner, cloned);

        let developer = AgentRole::Developer;
        let cloned_dev = developer;
        assert_eq!(developer, cloned_dev);

        let reviewer = AgentRole::Reviewer;
        let cloned_rev = reviewer;
        assert_eq!(reviewer, cloned_rev);
    }
}
