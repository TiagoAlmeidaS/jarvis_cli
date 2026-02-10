use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

/// User entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub password_hash: String,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

impl User {
    /// Create a new user with a generated UUID
    pub fn new(username: String, email: String, password_hash: String) -> Self {
        let now = OffsetDateTime::now_utc();
        Self {
            id: Uuid::new_v4(),
            username,
            email,
            password_hash,
            created_at: now,
            updated_at: now,
        }
    }
}

/// Conversation entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    pub id: Uuid,
    pub user_id: Uuid,
    pub title: String,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

impl Conversation {
    /// Create a new conversation
    pub fn new(user_id: Uuid, title: String) -> Self {
        let now = OffsetDateTime::now_utc();
        Self {
            id: Uuid::new_v4(),
            user_id,
            title,
            created_at: now,
            updated_at: now,
        }
    }
}

/// Message entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: Uuid,
    pub conversation_id: Uuid,
    pub role: MessageRole,
    pub content: String,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

impl Message {
    /// Create a new message
    pub fn new(conversation_id: Uuid, role: MessageRole, content: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            conversation_id,
            role,
            content,
            created_at: OffsetDateTime::now_utc(),
        }
    }
}

/// Message role enum
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    User,
    Assistant,
    System,
}

impl std::fmt::Display for MessageRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MessageRole::User => write!(f, "user"),
            MessageRole::Assistant => write!(f, "assistant"),
            MessageRole::System => write!(f, "system"),
        }
    }
}

impl std::str::FromStr for MessageRole {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "user" => Ok(MessageRole::User),
            "assistant" => Ok(MessageRole::Assistant),
            "system" => Ok(MessageRole::System),
            _ => Err(anyhow::anyhow!("Invalid message role: {}", s)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== User Tests ====================

    #[test]
    fn test_user_creation() {
        let user = User::new(
            "testuser".to_string(),
            "test@example.com".to_string(),
            "hashed_password".to_string(),
        );

        assert_eq!(user.username, "testuser");
        assert_eq!(user.email, "test@example.com");
        assert_eq!(user.password_hash, "hashed_password");
        assert!(user.id != Uuid::nil());
        assert!(user.created_at <= OffsetDateTime::now_utc());
        assert_eq!(user.created_at, user.updated_at);
    }

    #[test]
    fn test_user_creation_generates_unique_ids() {
        let user1 = User::new(
            "user1".to_string(),
            "user1@example.com".to_string(),
            "hash1".to_string(),
        );

        let user2 = User::new(
            "user2".to_string(),
            "user2@example.com".to_string(),
            "hash2".to_string(),
        );

        assert_ne!(user1.id, user2.id);
    }

    #[test]
    fn test_user_serialization() {
        let user = User::new(
            "testuser".to_string(),
            "test@example.com".to_string(),
            "hashed_password".to_string(),
        );

        let json = serde_json::to_string(&user).unwrap();
        assert!(json.contains("testuser"));
        assert!(json.contains("test@example.com"));

        let deserialized: User = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.username, user.username);
        assert_eq!(deserialized.email, user.email);
        assert_eq!(deserialized.id, user.id);
    }

    // ==================== Conversation Tests ====================

    #[test]
    fn test_conversation_creation() {
        let user_id = Uuid::new_v4();
        let conversation = Conversation::new(user_id, "Test Conversation".to_string());

        assert_eq!(conversation.user_id, user_id);
        assert_eq!(conversation.title, "Test Conversation");
        assert!(conversation.id != Uuid::nil());
        assert!(conversation.created_at <= OffsetDateTime::now_utc());
        assert_eq!(conversation.created_at, conversation.updated_at);
    }

    #[test]
    fn test_conversation_creation_generates_unique_ids() {
        let user_id = Uuid::new_v4();
        let conv1 = Conversation::new(user_id, "Conv1".to_string());
        let conv2 = Conversation::new(user_id, "Conv2".to_string());

        assert_ne!(conv1.id, conv2.id);
    }

    #[test]
    fn test_conversation_serialization() {
        let user_id = Uuid::new_v4();
        let conversation = Conversation::new(user_id, "Test Conversation".to_string());

        let json = serde_json::to_string(&conversation).unwrap();
        assert!(json.contains("Test Conversation"));

        let deserialized: Conversation = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.title, conversation.title);
        assert_eq!(deserialized.user_id, conversation.user_id);
        assert_eq!(deserialized.id, conversation.id);
    }

    // ==================== Message Tests ====================

    #[test]
    fn test_message_creation() {
        let conversation_id = Uuid::new_v4();
        let message = Message::new(
            conversation_id,
            MessageRole::User,
            "Hello, world!".to_string(),
        );

        assert_eq!(message.conversation_id, conversation_id);
        assert_eq!(message.role, MessageRole::User);
        assert_eq!(message.content, "Hello, world!");
        assert!(message.id != Uuid::nil());
        assert!(message.created_at <= OffsetDateTime::now_utc());
    }

    #[test]
    fn test_message_creation_all_roles() {
        let conversation_id = Uuid::new_v4();

        let user_msg = Message::new(conversation_id, MessageRole::User, "User message".to_string());
        assert_eq!(user_msg.role, MessageRole::User);

        let assistant_msg = Message::new(conversation_id, MessageRole::Assistant, "Assistant message".to_string());
        assert_eq!(assistant_msg.role, MessageRole::Assistant);

        let system_msg = Message::new(conversation_id, MessageRole::System, "System message".to_string());
        assert_eq!(system_msg.role, MessageRole::System);
    }

    #[test]
    fn test_message_creation_generates_unique_ids() {
        let conversation_id = Uuid::new_v4();
        let msg1 = Message::new(conversation_id, MessageRole::User, "Message 1".to_string());
        let msg2 = Message::new(conversation_id, MessageRole::User, "Message 2".to_string());

        assert_ne!(msg1.id, msg2.id);
    }

    #[test]
    fn test_message_serialization() {
        let conversation_id = Uuid::new_v4();
        let message = Message::new(
            conversation_id,
            MessageRole::Assistant,
            "Hello!".to_string(),
        );

        let json = serde_json::to_string(&message).unwrap();
        assert!(json.contains("Hello!"));
        assert!(json.contains("assistant"));

        let deserialized: Message = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.content, message.content);
        assert_eq!(deserialized.role, message.role);
        assert_eq!(deserialized.id, message.id);
    }

    // ==================== MessageRole Tests ====================

    #[test]
    fn test_message_role_parsing() {
        let role: MessageRole = "user".parse().unwrap();
        assert_eq!(role, MessageRole::User);

        let role: MessageRole = "assistant".parse().unwrap();
        assert_eq!(role, MessageRole::Assistant);

        let role: MessageRole = "system".parse().unwrap();
        assert_eq!(role, MessageRole::System);
    }

    #[test]
    fn test_message_role_parsing_case_insensitive() {
        assert_eq!("USER".parse::<MessageRole>().unwrap(), MessageRole::User);
        assert_eq!("User".parse::<MessageRole>().unwrap(), MessageRole::User);
        assert_eq!("ASSISTANT".parse::<MessageRole>().unwrap(), MessageRole::Assistant);
        assert_eq!("Assistant".parse::<MessageRole>().unwrap(), MessageRole::Assistant);
        assert_eq!("SYSTEM".parse::<MessageRole>().unwrap(), MessageRole::System);
        assert_eq!("System".parse::<MessageRole>().unwrap(), MessageRole::System);
    }

    #[test]
    fn test_message_role_parsing_invalid() {
        let result = "invalid".parse::<MessageRole>();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid message role"));
    }

    #[test]
    fn test_message_role_display() {
        assert_eq!(MessageRole::User.to_string(), "user");
        assert_eq!(MessageRole::Assistant.to_string(), "assistant");
        assert_eq!(MessageRole::System.to_string(), "system");
    }

    #[test]
    fn test_message_role_serialization() {
        // Serialize
        let role = MessageRole::User;
        let json = serde_json::to_string(&role).unwrap();
        assert_eq!(json, "\"user\"");

        let role = MessageRole::Assistant;
        let json = serde_json::to_string(&role).unwrap();
        assert_eq!(json, "\"assistant\"");

        let role = MessageRole::System;
        let json = serde_json::to_string(&role).unwrap();
        assert_eq!(json, "\"system\"");

        // Deserialize
        let role: MessageRole = serde_json::from_str("\"user\"").unwrap();
        assert_eq!(role, MessageRole::User);

        let role: MessageRole = serde_json::from_str("\"assistant\"").unwrap();
        assert_eq!(role, MessageRole::Assistant);

        let role: MessageRole = serde_json::from_str("\"system\"").unwrap();
        assert_eq!(role, MessageRole::System);
    }

    #[test]
    fn test_message_role_equality() {
        assert_eq!(MessageRole::User, MessageRole::User);
        assert_eq!(MessageRole::Assistant, MessageRole::Assistant);
        assert_eq!(MessageRole::System, MessageRole::System);

        assert_ne!(MessageRole::User, MessageRole::Assistant);
        assert_ne!(MessageRole::User, MessageRole::System);
        assert_ne!(MessageRole::Assistant, MessageRole::System);
    }
}
