pub mod config;
pub mod loader;
pub mod orchestrator;
pub mod resolver;
pub mod validation;

pub use config::TeamDefinition;
pub use config::TeammateDefinition;
pub use config::TeamsConfig;
pub use loader::TeamLoadError;
pub use loader::TeamLoadOutcome;
pub use loader::load_teams;
pub use orchestrator::build_lead_prompt;
pub use resolver::ModelSpecError;
pub use resolver::ResolvedModel;
pub use resolver::resolve_model_spec;
pub use validation::TeamValidationError;
pub use validation::validate_teams;
