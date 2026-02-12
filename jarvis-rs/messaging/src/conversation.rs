use std::collections::HashMap;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Contexto de uma conversa
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationContext {
    pub chat_id: String,
    pub platform: crate::message::Platform,
    pub created_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
    pub message_count: u64,
    pub metadata: HashMap<String, String>,
}

impl ConversationContext {
    pub fn new(chat_id: String, platform: crate::message::Platform) -> Self {
        let now = Utc::now();
        Self {
            chat_id,
            platform,
            created_at: now,
            last_activity: now,
            message_count: 0,
            metadata: HashMap::new(),
        }
    }

    pub fn update_activity(&mut self) {
        self.last_activity = Utc::now();
        self.message_count += 1;
    }
}

/// Gerenciador de conversas
pub struct ConversationManager {
    conversations: HashMap<String, ConversationContext>,
}

impl ConversationManager {
    pub fn new() -> Self {
        Self {
            conversations: HashMap::new(),
        }
    }

    pub fn get_or_create(
        &mut self,
        chat_id: &str,
        platform: crate::message::Platform,
    ) -> &mut ConversationContext {
        let key = format!("{}:{}", platform.as_str(), chat_id);
        if !self.conversations.contains_key(&key) {
            let context = ConversationContext::new(chat_id.to_string(), platform);
            self.conversations.insert(key.clone(), context);
        }
        self.conversations.get_mut(&key).unwrap()
    }

    pub fn get(&self, chat_id: &str, platform: crate::message::Platform) -> Option<&ConversationContext> {
        let key = format!("{}:{}", platform.as_str(), chat_id);
        self.conversations.get(&key)
    }

    pub fn remove(&mut self, chat_id: &str, platform: crate::message::Platform) {
        let key = format!("{}:{}", platform.as_str(), chat_id);
        self.conversations.remove(&key);
    }
}

impl Default for ConversationManager {
    fn default() -> Self {
        Self::new()
    }
}
