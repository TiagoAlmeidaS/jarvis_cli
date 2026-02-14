//! Persistent agent session management using file-based storage.

use crate::agent::session::{AgentSession, AgentSessionManager, SessionError};
use std::path::{Path, PathBuf};
use tokio::fs;

/// Persistent agent session manager using JSON file storage.
pub struct PersistentAgentSessionManager {
    /// Storage directory
    storage_dir: PathBuf,
    /// In-memory fallback
    fallback: crate::agent::session::InMemoryAgentSessionManager,
}

impl PersistentAgentSessionManager {
    /// Creates a new persistent session manager.
    pub fn new(storage_dir: PathBuf) -> Self {
        Self {
            storage_dir,
            fallback: crate::agent::session::InMemoryAgentSessionManager::new(),
        }
    }

    /// Gets the file path for a session.
    fn session_path(&self, session_id: &str) -> PathBuf {
        self.storage_dir
            .join(format!("session_{}.json", session_id))
    }

    /// Stores session to file.
    async fn store_session(&self, session: &AgentSession) -> Result<(), SessionError> {
        // Ensure directory exists
        if let Err(e) = fs::create_dir_all(&self.storage_dir).await {
            return Err(SessionError::StorageError(format!(
                "Failed to create storage dir: {}",
                e
            )));
        }

        let path = self.session_path(&session.session_id);
        let data = serde_json::to_string_pretty(session)
            .map_err(|e| SessionError::StorageError(format!("Serialization error: {}", e)))?;

        fs::write(&path, data).await.map_err(|e| {
            SessionError::StorageError(format!("Failed to write session file: {}", e))
        })?;

        Ok(())
    }

    /// Loads session from file.
    async fn load_session(&self, session_id: &str) -> Result<Option<AgentSession>, SessionError> {
        let path = self.session_path(session_id);

        if !path.exists() {
            return Ok(None);
        }

        let data = fs::read_to_string(&path).await.map_err(|e| {
            SessionError::StorageError(format!("Failed to read session file: {}", e))
        })?;

        let session: AgentSession = serde_json::from_str(&data)
            .map_err(|e| SessionError::InvalidData(format!("Deserialization error: {}", e)))?;

        Ok(Some(session))
    }

    /// Lists all session IDs.
    async fn list_sessions(&self) -> Result<Vec<String>, SessionError> {
        if !self.storage_dir.exists() {
            return Ok(vec![]);
        }

        let mut sessions = Vec::new();
        let mut entries = fs::read_dir(&self.storage_dir).await.map_err(|e| {
            SessionError::StorageError(format!("Failed to read storage dir: {}", e))
        })?;

        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(|e| SessionError::StorageError(format!("Failed to read dir entry: {}", e)))?
        {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Some(file_name) = path.file_stem().and_then(|s| s.to_str()) {
                    if let Some(session_id) = file_name.strip_prefix("session_") {
                        sessions.push(session_id.to_string());
                    }
                }
            }
        }

        Ok(sessions)
    }
}

#[async_trait::async_trait]
impl AgentSessionManager for PersistentAgentSessionManager {
    async fn create_session(&self, agent_type: &str) -> Result<AgentSession, SessionError> {
        // Create in fallback first
        let mut session = self.fallback.create_session(agent_type).await?;

        // Persist immediately
        self.store_session(&session).await?;

        Ok(session)
    }

    async fn get_session(&self, session_id: &str) -> Result<Option<AgentSession>, SessionError> {
        // Try to load from file first
        if let Ok(Some(session)) = self.load_session(session_id).await {
            // Also update fallback for consistency
            let _ = self.fallback.update_session(&session).await;
            return Ok(Some(session));
        }

        // Fallback to in-memory
        self.fallback.get_session(session_id).await
    }

    async fn update_session(&self, session: &AgentSession) -> Result<(), SessionError> {
        // Update fallback
        self.fallback.update_session(session).await?;

        // Persist
        self.store_session(session).await?;

        Ok(())
    }

    async fn add_message(
        &self,
        session_id: &str,
        role: &str,
        content: &str,
    ) -> Result<(), SessionError> {
        // Get current session
        let mut session = self
            .get_session(session_id)
            .await?
            .ok_or_else(|| SessionError::NotFound(session_id.to_string()))?;

        // Add message
        session.history.push(crate::agent::session::SessionMessage {
            role: role.to_string(),
            content: content.to_string(),
            timestamp: Self::current_timestamp(),
        });
        session.updated_at = Self::current_timestamp();

        // Update both
        self.fallback.update_session(&session).await?;
        self.store_session(&session).await?;

        Ok(())
    }

    async fn record_file_read(
        &self,
        session_id: &str,
        file_path: &PathBuf,
    ) -> Result<(), SessionError> {
        // Get current session
        let mut session = self
            .get_session(session_id)
            .await?
            .ok_or_else(|| SessionError::NotFound(session_id.to_string()))?;

        // Record file
        session.files_read.insert(file_path.clone());
        session.updated_at = Self::current_timestamp();

        // Update both
        self.fallback.update_session(&session).await?;
        self.store_session(&session).await?;

        Ok(())
    }

    async fn add_knowledge(
        &self,
        session_id: &str,
        key: &str,
        value: &str,
    ) -> Result<(), SessionError> {
        // Get current session
        let mut session = self
            .get_session(session_id)
            .await?
            .ok_or_else(|| SessionError::NotFound(session_id.to_string()))?;

        // Add knowledge
        session
            .knowledge_base
            .insert(key.to_string(), value.to_string());
        session.updated_at = Self::current_timestamp();

        // Update both
        self.fallback.update_session(&session).await?;
        self.store_session(&session).await?;

        Ok(())
    }

    async fn record_tool_usage(
        &self,
        session_id: &str,
        tool_name: &str,
    ) -> Result<(), SessionError> {
        // Get current session
        let mut session = self
            .get_session(session_id)
            .await?
            .ok_or_else(|| SessionError::NotFound(session_id.to_string()))?;

        // Record tool
        session.tools_used.push(tool_name.to_string());
        session.updated_at = Self::current_timestamp();

        // Update both
        self.fallback.update_session(&session).await?;
        self.store_session(&session).await?;

        Ok(())
    }

    async fn resume_session(&self, session_id: &str) -> Result<AgentSession, SessionError> {
        // Try file first
        if let Ok(Some(session)) = self.load_session(session_id).await {
            // Update fallback
            let _ = self.fallback.update_session(&session).await;
            return Ok(session);
        }

        // Fallback to in-memory
        self.fallback.resume_session(session_id).await
    }
}

impl PersistentAgentSessionManager {
    fn current_timestamp() -> i64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_persistent_session() {
        let temp_dir = TempDir::new().unwrap();
        let manager = PersistentAgentSessionManager::new(temp_dir.path().to_path_buf());

        // Create session
        let session = manager.create_session("test").await.unwrap();
        let session_id = session.session_id.clone();

        // Get session
        let retrieved = manager.get_session(&session_id).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().agent_type, "test");

        // Add message
        manager
            .add_message(&session_id, "user", "Hello")
            .await
            .unwrap();

        // Verify persistence
        let new_manager = PersistentAgentSessionManager::new(temp_dir.path().to_path_buf());
        let session = new_manager.get_session(&session_id).await.unwrap().unwrap();
        assert_eq!(session.history.len(), 1);
    }
}
