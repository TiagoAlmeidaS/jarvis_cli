use crate::skills::model::SkillMetadata;

/// Filter a list of skills to only include those whose names appear in the
/// allow-list. Comparison is case-insensitive. When `allowed` is empty, all
/// skills are returned unchanged (an empty allow-list means "no restriction").
pub(crate) fn filter_skills_by_allowed(
    skills: Vec<SkillMetadata>,
    allowed: &[String],
) -> Vec<SkillMetadata> {
    if allowed.is_empty() {
        return skills;
    }
    let allowed_lower: Vec<String> = allowed.iter().map(|s| s.to_ascii_lowercase()).collect();
    skills
        .into_iter()
        .filter(|skill| allowed_lower.contains(&skill.name.to_ascii_lowercase()))
        .collect()
}

/// Filter a slice of skills, returning a new Vec with only the allowed skills.
/// Like [`filter_skills_by_allowed`] but operates on a borrowed slice.
pub(crate) fn filter_skills_ref_by_allowed(
    skills: &[SkillMetadata],
    allowed: &[String],
) -> Vec<SkillMetadata> {
    if allowed.is_empty() {
        return skills.to_vec();
    }
    let allowed_lower: Vec<String> = allowed.iter().map(|s| s.to_ascii_lowercase()).collect();
    skills
        .iter()
        .filter(|skill| allowed_lower.contains(&skill.name.to_ascii_lowercase()))
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use jarvis_protocol::protocol::SkillScope;
    use pretty_assertions::assert_eq;
    use std::path::PathBuf;

    fn make_skill(name: &str) -> SkillMetadata {
        SkillMetadata {
            name: name.to_string(),
            description: format!("{name} skill"),
            short_description: None,
            interface: None,
            dependencies: None,
            path: PathBuf::from(format!("/skills/{name}/SKILL.md")),
            scope: SkillScope::User,
        }
    }

    #[test]
    fn empty_allowed_returns_all_skills() {
        let skills = vec![make_skill("alpha"), make_skill("beta"), make_skill("gamma")];
        let result = filter_skills_by_allowed(skills.clone(), &[]);
        assert_eq!(result, skills);
    }

    #[test]
    fn filters_to_only_allowed_skills() {
        let skills = vec![make_skill("alpha"), make_skill("beta"), make_skill("gamma")];
        let allowed = vec!["alpha".to_string(), "gamma".to_string()];
        let result = filter_skills_by_allowed(skills, &allowed);
        assert_eq!(result, vec![make_skill("alpha"), make_skill("gamma")]);
    }

    #[test]
    fn case_insensitive_matching() {
        let skills = vec![make_skill("CodeReview"), make_skill("testing")];
        let allowed = vec!["codereview".to_string(), "TESTING".to_string()];
        let result = filter_skills_by_allowed(skills.clone(), &allowed);
        assert_eq!(result, skills);
    }

    #[test]
    fn unknown_allowed_names_are_ignored() {
        let skills = vec![make_skill("alpha"), make_skill("beta")];
        let allowed = vec!["alpha".to_string(), "nonexistent".to_string()];
        let result = filter_skills_by_allowed(skills, &allowed);
        assert_eq!(result, vec![make_skill("alpha")]);
    }

    #[test]
    fn preserves_order() {
        let skills = vec![make_skill("gamma"), make_skill("alpha"), make_skill("beta")];
        let allowed = vec!["beta".to_string(), "gamma".to_string()];
        let result = filter_skills_by_allowed(skills, &allowed);
        assert_eq!(result, vec![make_skill("gamma"), make_skill("beta")]);
    }

    #[test]
    fn ref_variant_empty_allowed_returns_all() {
        let skills = vec![make_skill("alpha"), make_skill("beta")];
        let result = filter_skills_ref_by_allowed(&skills, &[]);
        assert_eq!(result, skills);
    }

    #[test]
    fn ref_variant_filters_correctly() {
        let skills = vec![make_skill("alpha"), make_skill("beta"), make_skill("gamma")];
        let allowed = vec!["beta".to_string()];
        let result = filter_skills_ref_by_allowed(&skills, &allowed);
        assert_eq!(result, vec![make_skill("beta")]);
    }
}
