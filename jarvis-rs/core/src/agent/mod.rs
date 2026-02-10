pub(crate) mod control;
pub mod explore;
mod guards;
pub mod plan;
pub(crate) mod role;
pub mod session;
pub mod session_persistent;
pub(crate) mod status;

pub(crate) use jarvis_protocol::protocol::AgentStatus;
pub(crate) use control::AgentControl;
pub use explore::{ExploreAgent, ExploreAgentResult, RuleBasedExploreAgent, Thoroughness};
pub(crate) use guards::MAX_THREAD_SPAWN_DEPTH;
pub(crate) use guards::exceeds_thread_spawn_depth_limit;
pub(crate) use guards::next_thread_spawn_depth;
pub use plan::{PlanAgent, PlanAgentResult, RuleBasedPlanAgent};
pub(crate) use role::AgentRole;
pub use session::{AgentSession, AgentSessionManager, InMemoryAgentSessionManager, SessionError};
pub(crate) use status::agent_status_from_event;
