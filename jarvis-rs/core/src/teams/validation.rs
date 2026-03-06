use std::collections::HashSet;
use std::fmt;

use super::config::TeamDefinition;
use super::config::TeamsConfig;
use super::resolver::resolve_model_spec;

/// A validation error found in a teams configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TeamValidationError {
    /// The team name where the error was found (if applicable).
    pub team: Option<String>,
    /// The teammate name where the error was found (if applicable).
    pub teammate: Option<String>,
    /// Description of the validation error.
    pub message: String,
}

impl fmt::Display for TeamValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (&self.team, &self.teammate) {
            (Some(team), Some(teammate)) => {
                write!(f, "team '{team}', teammate '{teammate}': {}", self.message)
            }
            (Some(team), None) => write!(f, "team '{team}': {}", self.message),
            _ => write!(f, "{}", self.message),
        }
    }
}

/// Validate the semantic integrity of a [`TeamsConfig`].
///
/// This performs the following checks:
/// - Team names are valid identifiers (`[a-z0-9_]+`)
/// - Each team has at least one teammate
/// - The `lead` field references an existing teammate
/// - Each teammate's `model` field is in valid `"provider/model"` format
/// - If `known_providers` is provided, the provider portion must exist in the set
/// - If `known_skills` is provided, each referenced skill must exist in the set
/// - `reasoning_effort` must be one of "low", "medium", "high" if provided
///
/// Returns a list of all validation errors found (empty means valid).
pub fn validate_teams(
    config: &TeamsConfig,
    known_providers: Option<&HashSet<String>>,
    known_skills: Option<&HashSet<String>>,
) -> Vec<TeamValidationError> {
    let mut errors = Vec::new();

    for (team_name, team_def) in &config.teams {
        validate_team_name(team_name, &mut errors);
        validate_team(
            team_name,
            team_def,
            known_providers,
            known_skills,
            &mut errors,
        );
    }

    errors
}

fn validate_team_name(name: &str, errors: &mut Vec<TeamValidationError>) {
    if name.is_empty() {
        errors.push(TeamValidationError {
            team: Some(name.to_string()),
            teammate: None,
            message: "team name cannot be empty".to_string(),
        });
        return;
    }

    if !name
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
    {
        errors.push(TeamValidationError {
            team: Some(name.to_string()),
            teammate: None,
            message: format!(
                "team name '{name}' must contain only lowercase letters, digits, and underscores"
            ),
        });
    }
}

fn validate_team(
    team_name: &str,
    team: &TeamDefinition,
    known_providers: Option<&HashSet<String>>,
    known_skills: Option<&HashSet<String>>,
    errors: &mut Vec<TeamValidationError>,
) {
    // Must have at least one teammate
    if team.teammates.is_empty() {
        errors.push(TeamValidationError {
            team: Some(team_name.to_string()),
            teammate: None,
            message: "team must have at least one teammate".to_string(),
        });
    }

    // Lead must reference an existing teammate
    if !team.teammates.contains_key(&team.lead) {
        errors.push(TeamValidationError {
            team: Some(team_name.to_string()),
            teammate: None,
            message: format!(
                "lead '{}' is not a defined teammate (available: {})",
                team.lead,
                team.teammates
                    .keys()
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        });
    }

    // Validate each teammate
    for (mate_name, mate_def) in &team.teammates {
        validate_teammate(
            team_name,
            mate_name,
            mate_def,
            known_providers,
            known_skills,
            errors,
        );
    }
}

fn validate_teammate(
    team_name: &str,
    mate_name: &str,
    mate: &super::config::TeammateDefinition,
    known_providers: Option<&HashSet<String>>,
    known_skills: Option<&HashSet<String>>,
    errors: &mut Vec<TeamValidationError>,
) {
    // Validate model spec format
    match resolve_model_spec(&mate.model) {
        Ok(resolved) => {
            // If we know the available providers, check that the provider exists
            if let Some(providers) = known_providers {
                if !providers.contains(&resolved.provider_id) {
                    errors.push(TeamValidationError {
                        team: Some(team_name.to_string()),
                        teammate: Some(mate_name.to_string()),
                        message: format!(
                            "provider '{}' from model '{}' is not a known provider (available: {})",
                            resolved.provider_id,
                            mate.model,
                            providers.iter().cloned().collect::<Vec<_>>().join(", ")
                        ),
                    });
                }
            }
        }
        Err(e) => {
            errors.push(TeamValidationError {
                team: Some(team_name.to_string()),
                teammate: Some(mate_name.to_string()),
                message: format!("invalid model: {e}"),
            });
        }
    }

    // Validate reasoning_effort if provided
    if let Some(effort) = &mate.reasoning_effort {
        let valid_efforts = ["low", "medium", "high"];
        if !valid_efforts.contains(&effort.as_str()) {
            errors.push(TeamValidationError {
                team: Some(team_name.to_string()),
                teammate: Some(mate_name.to_string()),
                message: format!(
                    "reasoning_effort '{effort}' is not valid (must be one of: low, medium, high)"
                ),
            });
        }
    }

    // Validate skills references if we know available skills
    if let Some(available_skills) = known_skills {
        for skill in &mate.skills {
            if !available_skills.contains(skill) {
                errors.push(TeamValidationError {
                    team: Some(team_name.to_string()),
                    teammate: Some(mate_name.to_string()),
                    message: format!("skill '{skill}' is not a known skill"),
                });
            }
        }
    }

    // Role must not be empty
    if mate.role.trim().is_empty() {
        errors.push(TeamValidationError {
            team: Some(team_name.to_string()),
            teammate: Some(mate_name.to_string()),
            message: "role (system prompt) cannot be empty".to_string(),
        });
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::teams::config::TeamDefinition;
    use crate::teams::config::TeammateDefinition;
    use crate::teams::config::TeamsConfig;

    fn make_teammate(model: &str, role: &str) -> TeammateDefinition {
        TeammateDefinition {
            model: model.to_string(),
            role: role.to_string(),
            skills: vec![],
            read_only: false,
            reasoning_effort: None,
        }
    }

    fn make_team(lead: &str, teammates: Vec<(&str, TeammateDefinition)>) -> TeamDefinition {
        TeamDefinition {
            description: None,
            lead: lead.to_string(),
            shared_memory: false,
            teammates: teammates
                .into_iter()
                .map(|(k, v)| (k.to_string(), v))
                .collect(),
        }
    }

    fn make_config(teams: Vec<(&str, TeamDefinition)>) -> TeamsConfig {
        TeamsConfig {
            teams: teams.into_iter().map(|(k, v)| (k.to_string(), v)).collect(),
        }
    }

    #[test]
    fn valid_config_has_no_errors() {
        let config = make_config(vec![(
            "my_team",
            make_team(
                "lead",
                vec![
                    (
                        "lead",
                        make_teammate("anthropic/claude-3.5-sonnet", "You lead."),
                    ),
                    (
                        "worker",
                        make_teammate("google/gemini-2.0-flash", "You work."),
                    ),
                ],
            ),
        )]);

        let errors = validate_teams(&config, None, None);
        assert!(errors.is_empty(), "expected no errors, got: {errors:?}");
    }

    #[test]
    fn empty_config_is_valid() {
        let config = TeamsConfig {
            teams: HashMap::new(),
        };
        let errors = validate_teams(&config, None, None);
        assert!(errors.is_empty());
    }

    #[test]
    fn lead_not_in_teammates() {
        let config = make_config(vec![(
            "bad_team",
            make_team(
                "ghost",
                vec![(
                    "worker",
                    make_teammate("anthropic/claude-3.5-sonnet", "Work."),
                )],
            ),
        )]);

        let errors = validate_teams(&config, None, None);
        assert_eq!(errors.len(), 1);
        assert!(
            errors[0]
                .message
                .contains("lead 'ghost' is not a defined teammate")
        );
    }

    #[test]
    fn empty_teammates() {
        let config = make_config(vec![(
            "empty_team",
            TeamDefinition {
                description: None,
                lead: "nobody".to_string(),
                shared_memory: false,
                teammates: HashMap::new(),
            },
        )]);

        let errors = validate_teams(&config, None, None);
        // Expect two errors: no teammates + lead not found
        assert!(errors.len() >= 2);
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("at least one teammate"))
        );
        assert!(errors.iter().any(|e| e.message.contains("lead 'nobody'")));
    }

    #[test]
    fn invalid_model_format() {
        let config = make_config(vec![(
            "bad_model",
            make_team(
                "worker",
                vec![("worker", make_teammate("no-slash-here", "Work."))],
            ),
        )]);

        let errors = validate_teams(&config, None, None);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("invalid model"));
    }

    #[test]
    fn unknown_provider() {
        let known = HashSet::from(["anthropic".to_string(), "google".to_string()]);
        let config = make_config(vec![(
            "team",
            make_team(
                "worker",
                vec![("worker", make_teammate("unknown_provider/model", "Work."))],
            ),
        )]);

        let errors = validate_teams(&config, Some(&known), None);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("not a known provider"));
    }

    #[test]
    fn known_provider_passes() {
        let known = HashSet::from(["anthropic".to_string()]);
        let config = make_config(vec![(
            "team",
            make_team(
                "worker",
                vec![(
                    "worker",
                    make_teammate("anthropic/claude-3.5-sonnet", "Work."),
                )],
            ),
        )]);

        let errors = validate_teams(&config, Some(&known), None);
        assert!(errors.is_empty());
    }

    #[test]
    fn unknown_skill() {
        let known_skills = HashSet::from(["shell".to_string(), "read_file".to_string()]);
        let mut teammate = make_teammate("anthropic/claude-3.5-sonnet", "Work.");
        teammate.skills = vec!["shell".to_string(), "nonexistent_skill".to_string()];

        let config = make_config(vec![(
            "team",
            make_team("worker", vec![("worker", teammate)]),
        )]);

        let errors = validate_teams(&config, None, Some(&known_skills));
        assert_eq!(errors.len(), 1);
        assert!(
            errors[0]
                .message
                .contains("'nonexistent_skill' is not a known skill")
        );
    }

    #[test]
    fn invalid_reasoning_effort() {
        let mut teammate = make_teammate("anthropic/claude-3.5-sonnet", "Work.");
        teammate.reasoning_effort = Some("ultra".to_string());

        let config = make_config(vec![(
            "team",
            make_team("worker", vec![("worker", teammate)]),
        )]);

        let errors = validate_teams(&config, None, None);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("reasoning_effort 'ultra'"));
    }

    #[test]
    fn valid_reasoning_efforts() {
        for effort in &["low", "medium", "high"] {
            let mut teammate = make_teammate("anthropic/claude-3.5-sonnet", "Work.");
            teammate.reasoning_effort = Some(effort.to_string());

            let config = make_config(vec![(
                "team",
                make_team("worker", vec![("worker", teammate)]),
            )]);

            let errors = validate_teams(&config, None, None);
            assert!(errors.is_empty(), "effort '{effort}' should be valid");
        }
    }

    #[test]
    fn invalid_team_name() {
        let config = make_config(vec![(
            "Bad-Name",
            make_team(
                "worker",
                vec![(
                    "worker",
                    make_teammate("anthropic/claude-3.5-sonnet", "Work."),
                )],
            ),
        )]);

        let errors = validate_teams(&config, None, None);
        assert!(errors.iter().any(|e| {
            e.message
                .contains("lowercase letters, digits, and underscores")
        }));
    }

    #[test]
    fn empty_role() {
        let config = make_config(vec![(
            "team",
            make_team(
                "worker",
                vec![("worker", make_teammate("anthropic/claude-3.5-sonnet", "  "))],
            ),
        )]);

        let errors = validate_teams(&config, None, None);
        assert_eq!(errors.len(), 1);
        assert!(
            errors[0]
                .message
                .contains("role (system prompt) cannot be empty")
        );
    }

    #[test]
    fn multiple_errors_collected() {
        let mut bad_teammate = make_teammate("no-slash", "");
        bad_teammate.reasoning_effort = Some("invalid".to_string());
        bad_teammate.skills = vec!["ghost_skill".to_string()];

        let known_skills = HashSet::from(["shell".to_string()]);

        let config = make_config(vec![(
            "Bad-Team!",
            TeamDefinition {
                description: None,
                lead: "ghost_lead".to_string(),
                shared_memory: false,
                teammates: [("worker".to_string(), bad_teammate)].into_iter().collect(),
            },
        )]);

        let errors = validate_teams(&config, None, Some(&known_skills));
        // Should have: invalid team name, lead not found, invalid model, invalid reasoning,
        // unknown skill, empty role
        assert!(errors.len() >= 5, "expected >=5 errors, got: {errors:?}");
    }

    #[test]
    fn display_error_with_team_and_teammate() {
        let err = TeamValidationError {
            team: Some("dev".to_string()),
            teammate: Some("coder".to_string()),
            message: "bad model".to_string(),
        };
        assert_eq!(err.to_string(), "team 'dev', teammate 'coder': bad model");
    }

    #[test]
    fn display_error_with_team_only() {
        let err = TeamValidationError {
            team: Some("dev".to_string()),
            teammate: None,
            message: "missing lead".to_string(),
        };
        assert_eq!(err.to_string(), "team 'dev': missing lead");
    }

    #[test]
    fn display_error_no_context() {
        let err = TeamValidationError {
            team: None,
            teammate: None,
            message: "general error".to_string(),
        };
        assert_eq!(err.to_string(), "general error");
    }
}
