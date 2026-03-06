//! Builds the orchestrator system prompt for a team's lead agent.
//!
//! When `jarvis teams run` is invoked, the lead agent receives its own `role`
//! prompt **plus** an auto-generated section describing the teammates it can
//! delegate to via `spawn_agent(teammate_name: "...")`.

use crate::teams::config::TeamDefinition;

/// Build the full system prompt for the lead agent of a team.
///
/// The result is: the lead's own `role` text, followed by an auto-generated
/// "## Your Team" section listing every teammate (excluding the lead itself)
/// with their model, role summary, skills, and capabilities.
pub fn build_lead_prompt(team_name: &str, team: &TeamDefinition) -> String {
    let lead_def = team
        .teammates
        .get(&team.lead)
        .expect("lead must exist in teammates (validated earlier)");

    let mut prompt = lead_def.role.clone();
    prompt.push_str("\n\n");

    // Build the team roster section.
    prompt.push_str("## Your Team\n\n");
    prompt.push_str(&format!("You are the **lead** of team `{team_name}`. ",));
    prompt.push_str(
        "Use `spawn_agent` with the `teammate_name` parameter to delegate tasks to your teammates. ",
    );
    prompt.push_str(&format!(
        "Always pass `team_name: \"{team_name}\"` to avoid ambiguity.\n\n",
    ));

    // Describe the team if a description is available.
    if let Some(ref desc) = team.description {
        prompt.push_str(&format!("**Team purpose:** {desc}\n\n"));
    }

    prompt.push_str("### Available Teammates\n\n");

    // Sort teammates alphabetically for deterministic output, skip the lead.
    let mut teammates: Vec<(&String, &crate::teams::config::TeammateDefinition)> = team
        .teammates
        .iter()
        .filter(|(name, _)| name.as_str() != team.lead)
        .collect();
    teammates.sort_by_key(|(name, _)| name.to_owned());

    if teammates.is_empty() {
        prompt.push_str("No other teammates configured. You are working solo.\n");
    } else {
        for (name, def) in &teammates {
            prompt.push_str(&format!("**`{name}`**\n"));
            prompt.push_str(&format!("- Model: `{}`\n", def.model));

            // Show a one-line summary of the role (first non-empty line).
            let role_summary = def
                .role
                .lines()
                .find(|l| !l.trim().is_empty())
                .unwrap_or("(no role description)");
            prompt.push_str(&format!("- Role: {role_summary}\n"));

            if !def.skills.is_empty() {
                prompt.push_str(&format!("- Skills: {}\n", def.skills.join(", ")));
            }
            if def.read_only {
                prompt.push_str("- Access: read-only\n");
            }
            if let Some(ref effort) = def.reasoning_effort {
                prompt.push_str(&format!("- Reasoning: {effort}\n"));
            }
            prompt.push('\n');
        }
    }

    prompt.push_str("### Delegation Guide\n\n");
    prompt.push_str("To delegate a task, use:\n");
    prompt.push_str("```\n");
    prompt.push_str(&format!(
        "spawn_agent(message: \"<task>\", teammate_name: \"<name>\", team_name: \"{team_name}\")\n",
    ));
    prompt.push_str("```\n");
    prompt.push_str("Use `wait` to collect results and `send_input` for follow-up instructions.\n");

    prompt
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::teams::config::{TeamDefinition, TeammateDefinition};
    use std::collections::HashMap;

    fn make_teammate(model: &str, role: &str) -> TeammateDefinition {
        TeammateDefinition {
            model: model.to_string(),
            role: role.to_string(),
            skills: vec![],
            read_only: false,
            reasoning_effort: None,
        }
    }

    #[test]
    fn basic_lead_prompt() {
        let mut teammates = HashMap::new();
        teammates.insert(
            "architect".to_string(),
            make_teammate(
                "anthropic/claude-sonnet-4-20250514",
                "You are the team lead and architect.",
            ),
        );
        teammates.insert(
            "coder".to_string(),
            make_teammate(
                "openrouter/deepseek/deepseek-r1:free",
                "You write production code.\nFollow best practices.",
            ),
        );

        let team = TeamDefinition {
            description: Some("Development squad".to_string()),
            lead: "architect".to_string(),
            shared_memory: false,
            teammates,
        };

        let prompt = build_lead_prompt("dev_squad", &team);

        // Lead's own role is at the start.
        assert!(prompt.starts_with("You are the team lead and architect."));

        // Team section is present.
        assert!(prompt.contains("## Your Team"));
        assert!(prompt.contains("lead** of team `dev_squad`"));
        assert!(prompt.contains("Team purpose:** Development squad"));

        // Teammate listing (only coder, not architect itself).
        assert!(prompt.contains("**`coder`**"));
        assert!(prompt.contains("openrouter/deepseek/deepseek-r1:free"));
        assert!(prompt.contains("You write production code."));

        // Lead should NOT appear in the teammates list.
        assert!(!prompt.contains("**`architect`**"));

        // Delegation guide.
        assert!(prompt.contains("spawn_agent"));
        assert!(prompt.contains("team_name: \"dev_squad\""));
    }

    #[test]
    fn solo_team_shows_solo_message() {
        let mut teammates = HashMap::new();
        teammates.insert(
            "solo_agent".to_string(),
            make_teammate("anthropic/claude-sonnet-4-20250514", "I work alone."),
        );

        let team = TeamDefinition {
            description: None,
            lead: "solo_agent".to_string(),
            shared_memory: false,
            teammates,
        };

        let prompt = build_lead_prompt("solo", &team);

        assert!(prompt.contains("working solo"));
        assert!(!prompt.contains("Team purpose:"));
    }

    #[test]
    fn skills_and_read_only_shown() {
        let mut teammates = HashMap::new();
        teammates.insert(
            "lead".to_string(),
            make_teammate("anthropic/claude-sonnet-4-20250514", "Lead agent."),
        );
        teammates.insert(
            "reviewer".to_string(),
            TeammateDefinition {
                model: "google/gemini-2.5-pro".to_string(),
                role: "Review code for quality.".to_string(),
                skills: vec!["rust".to_string(), "testing".to_string()],
                read_only: true,
                reasoning_effort: Some("high".to_string()),
            },
        );

        let team = TeamDefinition {
            description: None,
            lead: "lead".to_string(),
            shared_memory: false,
            teammates,
        };

        let prompt = build_lead_prompt("qa_team", &team);

        assert!(prompt.contains("Skills: rust, testing"));
        assert!(prompt.contains("Access: read-only"));
        assert!(prompt.contains("Reasoning: high"));
    }

    #[test]
    fn teammates_sorted_alphabetically() {
        let mut teammates = HashMap::new();
        teammates.insert(
            "lead".to_string(),
            make_teammate("anthropic/claude-sonnet-4-20250514", "Lead."),
        );
        teammates.insert(
            "zulu".to_string(),
            make_teammate("anthropic/claude-sonnet-4-20250514", "Zulu agent."),
        );
        teammates.insert(
            "alpha".to_string(),
            make_teammate("anthropic/claude-sonnet-4-20250514", "Alpha agent."),
        );

        let team = TeamDefinition {
            description: None,
            lead: "lead".to_string(),
            shared_memory: false,
            teammates,
        };

        let prompt = build_lead_prompt("test", &team);

        let alpha_pos = prompt.find("**`alpha`**").unwrap();
        let zulu_pos = prompt.find("**`zulu`**").unwrap();
        assert!(alpha_pos < zulu_pos, "alpha should appear before zulu");
    }
}
