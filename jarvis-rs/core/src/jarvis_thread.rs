use crate::agent::AgentStatus;
use crate::Jarvis;
use crate::error::Result as CodexResult;
use crate::protocol::Event;
use crate::protocol::Op;
use crate::protocol::Submission;
use jarvis_protocol::config_types::Personality;
use jarvis_protocol::openai_models::ReasoningEffort;
use jarvis_protocol::protocol::AskForApproval;
use jarvis_protocol::protocol::SandboxPolicy;
use jarvis_protocol::protocol::SessionSource;
use std::path::PathBuf;
use tokio::sync::watch;

use crate::state_db::StateDbHandle;

#[derive(Clone, Debug)]
pub struct ThreadConfigSnapshot {
    pub model: String,
    pub model_provider_id: String,
    pub approval_policy: AskForApproval,
    pub sandbox_policy: SandboxPolicy,
    pub cwd: PathBuf,
    pub reasoning_effort: Option<ReasoningEffort>,
    pub personality: Option<Personality>,
    pub session_source: SessionSource,
}

pub struct JarvisThread {
    Jarvis: Jarvis,
    rollout_path: Option<PathBuf>,
}

/// Conduit for the bidirectional stream of messages that compose a thread
/// (formerly called a conversation) in Jarvis.
impl JarvisThread {
    pub(crate) fn new(Jarvis: Jarvis, rollout_path: Option<PathBuf>) -> Self {
        Self {
            Jarvis,
            rollout_path,
        }
    }

    pub async fn submit(&self, op: Op) -> CodexResult<String> {
        self.Jarvis.submit(op).await
    }

    /// Use sparingly: this is intended to be removed soon.
    pub async fn submit_with_id(&self, sub: Submission) -> CodexResult<()> {
        self.Jarvis.submit_with_id(sub).await
    }

    pub async fn next_event(&self) -> CodexResult<Event> {
        self.Jarvis.next_event().await
    }

    pub async fn agent_status(&self) -> AgentStatus {
        self.Jarvis.agent_status().await
    }

    pub(crate) fn subscribe_status(&self) -> watch::Receiver<AgentStatus> {
        self.Jarvis.agent_status.clone()
    }

    pub fn rollout_path(&self) -> Option<PathBuf> {
        self.rollout_path.clone()
    }

    pub fn state_db(&self) -> Option<StateDbHandle> {
        self.Jarvis.state_db()
    }

    pub async fn config_snapshot(&self) -> ThreadConfigSnapshot {
        self.Jarvis.thread_config_snapshot().await
    }
}
