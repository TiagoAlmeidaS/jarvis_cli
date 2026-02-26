//! Agent session management for maintaining context across agent operations.

use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use std::collections::HashSet;
use std::path::PathBuf;
use uuid::Uuid;

/// Represents an agent session with context and history.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSession {
    /// Unique session identifier
    pub session_id: String,
    /// Agent type (explore, plan, etc.)
    pub agent_type: String,
    /// Conversation history
    pub history: Vec<SessionMessage>,
    /// Files read during the session
    pub files_read: HashSet<PathBuf>,
    /// Knowledge base accumulated during session
    pub knowledge_base: HashMap<String, String>,
    /// Tools used during session
    pub tools_used: Vec<String>,
    /// Current context/state
    pub context: SessionContext,
    /// Timestamp when session was created
    pub created_at: i64,
    /// Timestamp when session was last updated
    pub updated_at: i64,
}

/// Message in the session history.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMessage {
    /// Role (user, assistant, system, tool)
    pub role: String,
    /// Message content
    pub content: String,
    /// Timestamp
    pub timestamp: i64,
}

/// Session context information.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SessionContext {
    /// Current working directory
    pub cwd: Option<PathBuf>,
    /// Current task/goal
    pub current_task: Option<String>,
    /// Progress information
    pub progress: HashMap<String, String>,
    /// Metadata
    pub metadata: HashMap<String, String>,
}

/// Trait for agent session management.
#[async_trait::async_trait]
pub trait AgentSessionManager: Send + Sync {
    /// Creates a new agent session.
    async fn create_session(&self, agent_type: &str) -> Result<AgentSession, SessionError>;

    /// Retrieves a session by ID.
    async fn get_session(&self, session_id: &str) -> Result<Option<AgentSession>, SessionError>;

    /// Updates a session.
    async fn update_session(&self, session: &AgentSession) -> Result<(), SessionError>;

    /// Adds a message to session history.
    async fn add_message(
        &self,
        session_id: &str,
        role: &str,
        content: &str,
    ) -> Result<(), SessionError>;

    /// Records a file as read.
    async fn record_file_read(
        &self,
        session_id: &str,
        file_path: &PathBuf,
    ) -> Result<(), SessionError>;

    /// Adds knowledge to the knowledge base.
    async fn add_knowledge(
        &self,
        session_id: &str,
        key: &str,
        value: &str,
    ) -> Result<(), SessionError>;

    /// Records tool usage.
    async fn record_tool_usage(
        &self,
        session_id: &str,
        tool_name: &str,
    ) -> Result<(), SessionError>;

    /// Resumes a session (loads and returns it).
    async fn resume_session(&self, session_id: &str) -> Result<AgentSession, SessionError>;
}

/// Error types for session management.
#[derive(Debug, thiserror::Error)]
pub enum SessionError {
    #[error("Session not found: {0}")]
    NotFound(String),
    #[error("Session storage error: {0}")]
    StorageError(String),
    #[error("Invalid session data: {0}")]
    InvalidData(String),
}

/// In-memory implementation of agent session manager.
///
/// This is a simple implementation that stores sessions in memory.
/// For production use, consider implementing persistence (e.g., using state_db).
pub struct InMemoryAgentSessionManager {
    sessions: std::sync::Arc<tokio::sync::RwLock<HashMap<String, AgentSession>>>,
}

impl InMemoryAgentSessionManager {
    /// Creates a new in-memory session manager.
    pub fn new() -> Self {
        Self {
            sessions: std::sync::Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        }
    }

    fn current_timestamp() -> i64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64
    }
}

impl Default for InMemoryAgentSessionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl AgentSessionManager for InMemoryAgentSessionManager {
    async fn create_session(&self, agent_type: &str) -> Result<AgentSession, SessionError> {
        let session_id = Uuid::new_v4().to_string();
        let timestamp = Self::current_timestamp();

        let session = AgentSession {
            session_id: session_id.clone(),
            agent_type: agent_type.to_string(),
            history: vec![],
            files_read: HashSet::new(),
            knowledge_base: HashMap::new(),
            tools_used: vec![],
            context: SessionContext::default(),
            created_at: timestamp,
            updated_at: timestamp,
        };

        let mut sessions = self.sessions.write().await;
        sessions.insert(session_id.clone(), session.clone());

        Ok(session)
    }

    async fn get_session(&self, session_id: &str) -> Result<Option<AgentSession>, SessionError> {
        let sessions = self.sessions.read().await;
        Ok(sessions.get(session_id).cloned())
    }

    async fn update_session(&self, session: &AgentSession) -> Result<(), SessionError> {
        let mut sessions = self.sessions.write().await;
        if sessions.contains_key(&session.session_id) {
            let mut updated = session.clone();
            updated.updated_at = Self::current_timestamp();
            sessions.insert(session.session_id.clone(), updated);
            Ok(())
        } else {
            Err(SessionError::NotFound(session.session_id.clone()))
        }
    }

    async fn add_message(
        &self,
        session_id: &str,
        role: &str,
        content: &str,
    ) -> Result<(), SessionError> {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(session_id) {
            let message = SessionMessage {
                role: role.to_string(),
                content: content.to_string(),
                timestamp: Self::current_timestamp(),
            };
            session.history.push(message);
            session.updated_at = Self::current_timestamp();
            Ok(())
        } else {
            Err(SessionError::NotFound(session_id.to_string()))
        }
    }

    async fn record_file_read(
        &self,
        session_id: &str,
        file_path: &PathBuf,
    ) -> Result<(), SessionError> {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(session_id) {
            session.files_read.insert(file_path.clone());
            session.updated_at = Self::current_timestamp();
            Ok(())
        } else {
            Err(SessionError::NotFound(session_id.to_string()))
        }
    }

    async fn add_knowledge(
        &self,
        session_id: &str,
        key: &str,
        value: &str,
    ) -> Result<(), SessionError> {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(session_id) {
            session
                .knowledge_base
                .insert(key.to_string(), value.to_string());
            session.updated_at = Self::current_timestamp();
            Ok(())
        } else {
            Err(SessionError::NotFound(session_id.to_string()))
        }
    }

    async fn record_tool_usage(
        &self,
        session_id: &str,
        tool_name: &str,
    ) -> Result<(), SessionError> {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(session_id) {
            if !session.tools_used.contains(&tool_name.to_string()) {
                session.tools_used.push(tool_name.to_string());
            }
            session.updated_at = Self::current_timestamp();
            Ok(())
        } else {
            Err(SessionError::NotFound(session_id.to_string()))
        }
    }

    async fn resume_session(&self, session_id: &str) -> Result<AgentSession, SessionError> {
        let sessions = self.sessions.read().await;
        sessions
            .get(session_id)
            .cloned()
            .ok_or_else(|| SessionError::NotFound(session_id.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_and_get_session() {
        let manager = InMemoryAgentSessionManager::new();
        let session = manager.create_session("explore").await.unwrap();

        assert_eq!(session.agent_type, "explore");
        assert!(!session.session_id.is_empty());

        let retrieved = manager.get_session(&session.session_id).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().agent_type, "explore");
    }

    #[tokio::test]
    async fn test_add_message() {
        let manager = InMemoryAgentSessionManager::new();
        let session = manager.create_session("explore").await.unwrap();

        manager
            .add_message(&session.session_id, "user", "Hello")
            .await
            .unwrap();

        let updated = manager
            .get_session(&session.session_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(updated.history.len(), 1);
        assert_eq!(updated.history[0].role, "user");
        assert_eq!(updated.history[0].content, "Hello");
    }

    #[tokio::test]
    async fn test_record_file_read() {
        let manager = InMemoryAgentSessionManager::new();
        let session = manager.create_session("explore").await.unwrap();
        let file_path = PathBuf::from("test.rs");

        manager
            .record_file_read(&session.session_id, &file_path)
            .await
            .unwrap();

        let updated = manager
            .get_session(&session.session_id)
            .await
            .unwrap()
            .unwrap();
        assert!(updated.files_read.contains(&file_path));
    }
}
