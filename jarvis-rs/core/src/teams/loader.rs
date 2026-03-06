use std::fmt;
use std::path::Path;
use std::path::PathBuf;

use tracing::debug;
use tracing::warn;

use super::config::TeamsConfig;

/// The filename we look for when discovering teams configuration.
const TEAMS_FILE_NAME: &str = "teams.yaml";

/// The config directory name under project root.
const CONFIG_DIR_NAME: &str = ".jarvis";

/// Error type for teams loading operations.
#[derive(Debug)]
pub enum TeamLoadError {
    /// Failed to read the file from disk.
    Read {
        path: PathBuf,
        source: std::io::Error,
    },
    /// Failed to parse the YAML content.
    InvalidYaml {
        path: PathBuf,
        source: serde_yaml::Error,
    },
}

impl fmt::Display for TeamLoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TeamLoadError::Read { path, source } => {
                write!(f, "failed to read '{}': {source}", path.display())
            }
            TeamLoadError::InvalidYaml { path, source } => {
                write!(f, "invalid YAML in '{}': {source}", path.display())
            }
        }
    }
}

impl std::error::Error for TeamLoadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            TeamLoadError::Read { source, .. } => Some(source),
            TeamLoadError::InvalidYaml { source, .. } => Some(source),
        }
    }
}

/// Result of loading teams configuration from disk.
#[derive(Debug, Clone, Default)]
pub struct TeamLoadOutcome {
    /// The merged teams configuration.
    pub config: TeamsConfig,

    /// Paths that were successfully loaded.
    pub loaded_from: Vec<PathBuf>,

    /// Errors encountered during loading (non-fatal: other roots may still succeed).
    pub errors: Vec<String>,
}

/// Load and merge teams configuration from multiple roots.
///
/// Searches for `teams.yaml` in the following locations (highest precedence first):
/// 1. `<cwd>/.jarvis/teams.yaml` (project-level)
/// 2. `<jarvis_home>/teams.yaml` (user-level)
///
/// Teams defined at the project level override user-level teams with the same name.
/// Teams with different names are merged together.
pub fn load_teams(jarvis_home: &Path, cwd: &Path) -> TeamLoadOutcome {
    let mut outcome = TeamLoadOutcome::default();

    // Collect candidate paths (lowest precedence first so higher-precedence
    // entries overwrite when we insert into the HashMap).
    let candidates = vec![
        // User-level (lowest precedence)
        jarvis_home.join(TEAMS_FILE_NAME),
        // Project-level (highest precedence)
        cwd.join(CONFIG_DIR_NAME).join(TEAMS_FILE_NAME),
    ];

    for path in candidates {
        if !path.is_file() {
            debug!("teams config not found at {}", path.display());
            continue;
        }

        match load_teams_file(&path) {
            Ok(file_config) => {
                debug!(
                    "loaded {} team(s) from {}",
                    file_config.teams.len(),
                    path.display()
                );
                outcome.loaded_from.push(path);

                // Merge: later entries (higher precedence) overwrite earlier ones.
                for (name, team) in file_config.teams {
                    outcome.config.teams.insert(name, team);
                }
            }
            Err(e) => {
                warn!("failed to load teams from {}: {e}", path.display());
                outcome.errors.push(e.to_string());
            }
        }
    }

    outcome
}

/// Load a single teams YAML file from disk.
fn load_teams_file(path: &Path) -> Result<TeamsConfig, TeamLoadError> {
    let contents = std::fs::read_to_string(path).map_err(|source| TeamLoadError::Read {
        path: path.to_path_buf(),
        source,
    })?;

    serde_yaml::from_str(&contents).map_err(|source| TeamLoadError::InvalidYaml {
        path: path.to_path_buf(),
        source,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn setup_dirs() -> (TempDir, TempDir) {
        let home = TempDir::new().unwrap();
        let project = TempDir::new().unwrap();
        (home, project)
    }

    fn write_user_teams(home: &Path, content: &str) {
        fs::write(home.join(TEAMS_FILE_NAME), content).unwrap();
    }

    fn write_project_teams(project: &Path, content: &str) {
        let config_dir = project.join(CONFIG_DIR_NAME);
        fs::create_dir_all(&config_dir).unwrap();
        fs::write(config_dir.join(TEAMS_FILE_NAME), content).unwrap();
    }

    #[test]
    fn load_from_user_level() {
        let (home, project) = setup_dirs();

        write_user_teams(
            home.path(),
            r#"
teams:
  my_team:
    lead: worker
    teammates:
      worker:
        model: "anthropic/claude-3.5-sonnet"
        role: "Work."
"#,
        );

        let outcome = load_teams(home.path(), project.path());

        assert!(outcome.errors.is_empty());
        assert_eq!(outcome.loaded_from.len(), 1);
        assert_eq!(outcome.config.teams.len(), 1);
        assert!(outcome.config.teams.contains_key("my_team"));
    }

    #[test]
    fn load_from_project_level() {
        let (home, project) = setup_dirs();

        write_project_teams(
            project.path(),
            r#"
teams:
  project_team:
    lead: dev
    teammates:
      dev:
        model: "google/gemini-2.0-flash"
        role: "Develop."
"#,
        );

        let outcome = load_teams(home.path(), project.path());

        assert!(outcome.errors.is_empty());
        assert_eq!(outcome.loaded_from.len(), 1);
        assert_eq!(outcome.config.teams.len(), 1);
        assert!(outcome.config.teams.contains_key("project_team"));
    }

    #[test]
    fn project_overrides_user() {
        let (home, project) = setup_dirs();

        write_user_teams(
            home.path(),
            r#"
teams:
  shared_team:
    lead: user_worker
    teammates:
      user_worker:
        model: "anthropic/claude-3.5-sonnet"
        role: "User version."
  user_only:
    lead: helper
    teammates:
      helper:
        model: "ollama/llama3"
        role: "Help."
"#,
        );

        write_project_teams(
            project.path(),
            r#"
teams:
  shared_team:
    lead: project_worker
    teammates:
      project_worker:
        model: "google/gemini-2.0-flash"
        role: "Project version."
"#,
        );

        let outcome = load_teams(home.path(), project.path());

        assert!(outcome.errors.is_empty());
        assert_eq!(outcome.loaded_from.len(), 2);
        assert_eq!(outcome.config.teams.len(), 2);

        // shared_team should be the project version
        let shared = &outcome.config.teams["shared_team"];
        assert_eq!(shared.lead, "project_worker");

        // user_only should still be present
        assert!(outcome.config.teams.contains_key("user_only"));
    }

    #[test]
    fn no_files_returns_empty() {
        let (home, project) = setup_dirs();

        let outcome = load_teams(home.path(), project.path());

        assert!(outcome.errors.is_empty());
        assert!(outcome.loaded_from.is_empty());
        assert!(outcome.config.teams.is_empty());
    }

    #[test]
    fn invalid_yaml_records_error() {
        let (home, project) = setup_dirs();

        write_user_teams(home.path(), "teams:\n  broken:\n    lead: [invalid");

        let outcome = load_teams(home.path(), project.path());

        assert!(!outcome.errors.is_empty());
        assert!(outcome.loaded_from.is_empty());
        assert!(outcome.config.teams.is_empty());
    }

    #[test]
    fn invalid_user_does_not_block_project() {
        let (home, project) = setup_dirs();

        // User file is broken
        write_user_teams(home.path(), "not: valid: yaml: [");

        // Project file is valid
        write_project_teams(
            project.path(),
            r#"
teams:
  good_team:
    lead: worker
    teammates:
      worker:
        model: "anthropic/claude-3.5-sonnet"
        role: "Work."
"#,
        );

        let outcome = load_teams(home.path(), project.path());

        // Error from user file recorded but project still loaded
        assert_eq!(outcome.errors.len(), 1);
        assert_eq!(outcome.loaded_from.len(), 1);
        assert_eq!(outcome.config.teams.len(), 1);
        assert!(outcome.config.teams.contains_key("good_team"));
    }

    #[test]
    fn merge_disjoint_teams() {
        let (home, project) = setup_dirs();

        write_user_teams(
            home.path(),
            r#"
teams:
  team_a:
    lead: a
    teammates:
      a:
        model: "anthropic/claude-3.5-sonnet"
        role: "A."
"#,
        );

        write_project_teams(
            project.path(),
            r#"
teams:
  team_b:
    lead: b
    teammates:
      b:
        model: "google/gemini-2.0-flash"
        role: "B."
"#,
        );

        let outcome = load_teams(home.path(), project.path());

        assert!(outcome.errors.is_empty());
        assert_eq!(outcome.loaded_from.len(), 2);
        assert_eq!(outcome.config.teams.len(), 2);
        assert!(outcome.config.teams.contains_key("team_a"));
        assert!(outcome.config.teams.contains_key("team_b"));
    }

    #[test]
    fn display_read_error() {
        let err = TeamLoadError::Read {
            path: PathBuf::from("/foo/teams.yaml"),
            source: std::io::Error::new(std::io::ErrorKind::NotFound, "not found"),
        };
        let msg = err.to_string();
        assert!(msg.contains("failed to read"));
        assert!(msg.contains("teams.yaml"));
    }

    #[test]
    fn display_yaml_error() {
        let yaml_err = serde_yaml::from_str::<TeamsConfig>("invalid: [yaml").unwrap_err();
        let err = TeamLoadError::InvalidYaml {
            path: PathBuf::from("/foo/teams.yaml"),
            source: yaml_err,
        };
        let msg = err.to_string();
        assert!(msg.contains("invalid YAML"));
        assert!(msg.contains("teams.yaml"));
    }
}
