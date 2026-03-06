use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;

/// Top-level teams configuration, typically loaded from `teams.yaml`.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct TeamsConfig {
    /// Map of team name to team definition.
    #[serde(default)]
    pub teams: HashMap<String, TeamDefinition>,
}

/// Definition of a single team, composed of a lead and one or more teammates.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TeamDefinition {
    /// Human-readable description of the team's purpose.
    pub description: Option<String>,

    /// Name of the teammate that acts as the orchestrator/lead.
    /// Must be a key in [`teammates`].
    pub lead: String,

    /// Whether to enable shared memory (SQLite) between teammates.
    #[serde(default)]
    pub shared_memory: bool,

    /// Map of teammate name to teammate definition.
    pub teammates: HashMap<String, TeammateDefinition>,
}

/// Definition of a single teammate within a team.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TeammateDefinition {
    /// Model specification in "provider/model" format.
    /// Examples: "anthropic/claude-3.5-sonnet", "openrouter/deepseek/deepseek-r1:free"
    pub model: String,

    /// System prompt / role instructions for this teammate.
    pub role: String,

    /// List of skill names this teammate is allowed to use.
    /// An empty list means the teammate can use all available skills.
    #[serde(default)]
    pub skills: Vec<String>,

    /// Whether this teammate should operate in read-only mode (no file writes, no shell mutations).
    #[serde(default)]
    pub read_only: bool,

    /// Reasoning effort level: "low", "medium", or "high".
    pub reasoning_effort: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_full_teams_yaml() {
        let yaml = r#"
teams:
  dev_squad:
    description: "Full-stack development team"
    lead: architect
    shared_memory: true
    teammates:
      architect:
        model: "anthropic/claude-3.5-sonnet"
        role: "You are the Lead Architect. Break down requests and delegate."
        skills: []
        read_only: false
        reasoning_effort: "high"
      coder:
        model: "openrouter/deepseek/deepseek-r1:free"
        role: "You write clean, testable code."
        skills:
          - file_writer
          - git_commit
          - shell
        read_only: false
      reviewer:
        model: "google/gemini-2.0-flash"
        role: "You review code for bugs and security issues."
        skills:
          - read_file
          - grep_files
        read_only: true

  content_factory:
    description: "Content creation team for SEO"
    lead: strategist
    teammates:
      strategist:
        model: "anthropic/claude-3.5-sonnet"
        role: "Plan content strategy and delegate."
      writer:
        model: "openrouter/deepseek/deepseek-r1:free"
        role: "Write SEO-optimized articles."
        skills:
          - file_writer
"#;

        let config: TeamsConfig = serde_yaml::from_str(yaml).unwrap();

        assert_eq!(config.teams.len(), 2);

        // Verify dev_squad
        let dev_squad = &config.teams["dev_squad"];
        assert_eq!(
            dev_squad.description.as_deref(),
            Some("Full-stack development team")
        );
        assert_eq!(dev_squad.lead, "architect");
        assert!(dev_squad.shared_memory);
        assert_eq!(dev_squad.teammates.len(), 3);

        let architect = &dev_squad.teammates["architect"];
        assert_eq!(architect.model, "anthropic/claude-3.5-sonnet");
        assert!(architect.skills.is_empty());
        assert!(!architect.read_only);
        assert_eq!(architect.reasoning_effort.as_deref(), Some("high"));

        let coder = &dev_squad.teammates["coder"];
        assert_eq!(coder.model, "openrouter/deepseek/deepseek-r1:free");
        assert_eq!(coder.skills, vec!["file_writer", "git_commit", "shell"]);
        assert!(!coder.read_only);

        let reviewer = &dev_squad.teammates["reviewer"];
        assert_eq!(reviewer.model, "google/gemini-2.0-flash");
        assert!(reviewer.read_only);

        // Verify content_factory
        let content = &config.teams["content_factory"];
        assert_eq!(content.lead, "strategist");
        assert!(!content.shared_memory);
        assert_eq!(content.teammates.len(), 2);
    }

    #[test]
    fn parse_minimal_yaml() {
        let yaml = r#"
teams:
  minimal:
    lead: worker
    teammates:
      worker:
        model: "ollama/llama3"
        role: "Do the work."
"#;

        let config: TeamsConfig = serde_yaml::from_str(yaml).unwrap();

        assert_eq!(config.teams.len(), 1);
        let team = &config.teams["minimal"];
        assert_eq!(team.description, None);
        assert!(!team.shared_memory);

        let worker = &team.teammates["worker"];
        assert!(worker.skills.is_empty());
        assert!(!worker.read_only);
        assert_eq!(worker.reasoning_effort, None);
    }

    #[test]
    fn parse_empty_teams() {
        let yaml = "teams: {}";
        let config: TeamsConfig = serde_yaml::from_str(yaml).unwrap();
        assert!(config.teams.is_empty());
    }

    #[test]
    fn parse_empty_document() {
        let yaml = "";
        let config: TeamsConfig = serde_yaml::from_str(yaml).unwrap();
        assert!(config.teams.is_empty());
    }

    #[test]
    fn parse_invalid_yaml_is_error() {
        let yaml = "teams:\n  broken:\n    lead: [invalid";
        let result = serde_yaml::from_str::<TeamsConfig>(yaml);
        assert!(result.is_err());
    }

    #[test]
    fn parse_missing_required_field_is_error() {
        // Missing `lead` field
        let yaml = r#"
teams:
  bad_team:
    teammates:
      worker:
        model: "ollama/llama3"
        role: "Do work."
"#;
        let result = serde_yaml::from_str::<TeamsConfig>(yaml);
        assert!(result.is_err());
    }

    #[test]
    fn parse_missing_model_is_error() {
        let yaml = r#"
teams:
  bad_team:
    lead: worker
    teammates:
      worker:
        role: "Do work."
"#;
        let result = serde_yaml::from_str::<TeamsConfig>(yaml);
        assert!(result.is_err());
    }

    #[test]
    fn roundtrip_serialization() {
        let mut teammates = HashMap::new();
        teammates.insert(
            "agent1".to_string(),
            TeammateDefinition {
                model: "anthropic/claude-3.5-sonnet".to_string(),
                role: "You are agent 1.".to_string(),
                skills: vec!["shell".to_string()],
                read_only: false,
                reasoning_effort: Some("high".to_string()),
            },
        );

        let mut teams = HashMap::new();
        teams.insert(
            "test_team".to_string(),
            TeamDefinition {
                description: Some("Test team".to_string()),
                lead: "agent1".to_string(),
                shared_memory: false,
                teammates,
            },
        );

        let config = TeamsConfig { teams };

        let yaml = serde_yaml::to_string(&config).unwrap();
        let parsed: TeamsConfig = serde_yaml::from_str(&yaml).unwrap();

        assert_eq!(config, parsed);
    }
}
