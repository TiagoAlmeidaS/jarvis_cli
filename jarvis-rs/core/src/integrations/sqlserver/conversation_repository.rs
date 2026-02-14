use super::models::MessageRole;
use super::{Conversation, Database, Message, Repository};
use anyhow::{Context, Result};
use async_trait::async_trait;
use tiberius::Row;
use time::OffsetDateTime;
use uuid::Uuid;

/// Repository for Conversation entities
pub struct ConversationRepository {
    db: Database,
}

impl ConversationRepository {
    /// Create a new ConversationRepository
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    /// Find conversations by user ID
    pub async fn find_by_user(
        &self,
        user_id: Uuid,
        limit: Option<i32>,
    ) -> Result<Vec<Conversation>> {
        let mut client = self.db.get_client().await?;

        let query = if let Some(limit) = limit {
            format!(
                "SELECT TOP {} id, user_id, title,
                        FORMAT(created_at, 'yyyy-MM-ddTHH:mm:ss.fffffffzzz') as created_at,
                        FORMAT(updated_at, 'yyyy-MM-ddTHH:mm:ss.fffffffzzz') as updated_at
                 FROM conversations
                 WHERE user_id = @P1
                 ORDER BY updated_at DESC",
                limit
            )
        } else {
            "SELECT id, user_id, title,
                    FORMAT(created_at, 'yyyy-MM-ddTHH:mm:ss.fffffffzzz') as created_at,
                    FORMAT(updated_at, 'yyyy-MM-ddTHH:mm:ss.fffffffzzz') as updated_at
             FROM conversations
             WHERE user_id = @P1
             ORDER BY updated_at DESC"
                .to_string()
        };

        let stream = client
            .query(&query, &[&user_id])
            .await
            .context("Failed to query conversations by user")?;

        let rows = stream.into_first_result().await?;

        let mut conversations = Vec::new();
        for row in rows {
            conversations.push(Self::row_to_conversation(&row)?);
        }

        Ok(conversations)
    }

    /// Find a conversation with all its messages
    pub async fn find_with_messages(
        &self,
        conversation_id: Uuid,
    ) -> Result<Option<(Conversation, Vec<Message>)>> {
        let conversation = self.find_by_id(conversation_id).await?;

        if let Some(conv) = conversation {
            let messages = self.find_messages(conversation_id).await?;
            Ok(Some((conv, messages)))
        } else {
            Ok(None)
        }
    }

    /// Find all messages for a conversation
    pub async fn find_messages(&self, conversation_id: Uuid) -> Result<Vec<Message>> {
        let mut client = self.db.get_client().await?;

        let query = "
            SELECT id, conversation_id, role, content,
                   FORMAT(created_at, 'yyyy-MM-ddTHH:mm:ss.fffffffzzz') as created_at
            FROM messages
            WHERE conversation_id = @P1
            ORDER BY created_at ASC
        ";

        let stream = client
            .query(query, &[&conversation_id])
            .await
            .context("Failed to query messages")?;

        let rows = stream.into_first_result().await?;

        let mut messages = Vec::new();
        for row in rows {
            messages.push(Self::row_to_message(&row)?);
        }

        Ok(messages)
    }

    /// Save a message to a conversation
    pub async fn save_message(&self, message: &Message) -> Result<Message> {
        let mut client = self.db.get_client().await?;

        let query = "
            INSERT INTO messages (id, conversation_id, role, content, created_at)
            VALUES (@P1, @P2, @P3, @P4, CONVERT(datetime2, @P5))
        ";

        let created_at_str = message
            .created_at
            .format(&time::format_description::well_known::Iso8601::DEFAULT)?;

        client
            .execute(
                query,
                &[
                    &message.id,
                    &message.conversation_id,
                    &message.role.to_string(),
                    &message.content,
                    &created_at_str.as_str(),
                ],
            )
            .await
            .context("Failed to insert message")?;

        Ok(message.clone())
    }

    /// Convert a database row to a Conversation
    fn row_to_conversation(row: &Row) -> Result<Conversation> {
        let id: Uuid = row.get(0).context("Missing id column")?;
        let user_id: Uuid = row.get(1).context("Missing user_id column")?;
        let title: &str = row.get(2).context("Missing title column")?;
        let created_at_str: &str = row.get(3).context("Missing created_at column")?;
        let updated_at_str: &str = row.get(4).context("Missing updated_at column")?;

        let created_at = OffsetDateTime::parse(
            created_at_str,
            &time::format_description::well_known::Iso8601::DEFAULT,
        )
        .context("Failed to parse created_at")?;
        let updated_at = OffsetDateTime::parse(
            updated_at_str,
            &time::format_description::well_known::Iso8601::DEFAULT,
        )
        .context("Failed to parse updated_at")?;

        Ok(Conversation {
            id,
            user_id,
            title: title.to_string(),
            created_at,
            updated_at,
        })
    }

    /// Convert a database row to a Message
    fn row_to_message(row: &Row) -> Result<Message> {
        let id: Uuid = row.get(0).context("Missing id column")?;
        let conversation_id: Uuid = row.get(1).context("Missing conversation_id column")?;
        let role: &str = row.get(2).context("Missing role column")?;
        let content: &str = row.get(3).context("Missing content column")?;
        let created_at_str: &str = row.get(4).context("Missing created_at column")?;

        let created_at = OffsetDateTime::parse(
            created_at_str,
            &time::format_description::well_known::Iso8601::DEFAULT,
        )
        .context("Failed to parse created_at")?;

        Ok(Message {
            id,
            conversation_id,
            role: role.parse()?,
            content: content.to_string(),
            created_at,
        })
    }
}

#[async_trait]
impl Repository<Conversation, Uuid> for ConversationRepository {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Conversation>> {
        let mut client = self.db.get_client().await?;

        let query = "
            SELECT id, user_id, title,
                   FORMAT(created_at, 'yyyy-MM-ddTHH:mm:ss.fffffffzzz') as created_at,
                   FORMAT(updated_at, 'yyyy-MM-ddTHH:mm:ss.fffffffzzz') as updated_at
            FROM conversations
            WHERE id = @P1
        ";

        let stream = client
            .query(query, &[&id])
            .await
            .context("Failed to query conversation by id")?;

        let rows = stream.into_first_result().await?;

        if let Some(row) = rows.into_iter().next() {
            Ok(Some(Self::row_to_conversation(&row)?))
        } else {
            Ok(None)
        }
    }

    async fn find_all(&self) -> Result<Vec<Conversation>> {
        let mut client = self.db.get_client().await?;

        let query = "
            SELECT id, user_id, title,
                   FORMAT(created_at, 'yyyy-MM-ddTHH:mm:ss.fffffffzzz') as created_at,
                   FORMAT(updated_at, 'yyyy-MM-ddTHH:mm:ss.fffffffzzz') as updated_at
            FROM conversations
            ORDER BY updated_at DESC
        ";

        let stream = client
            .query(query, &[])
            .await
            .context("Failed to query all conversations")?;

        let rows = stream.into_first_result().await?;

        let mut conversations = Vec::new();
        for row in rows {
            conversations.push(Self::row_to_conversation(&row)?);
        }

        Ok(conversations)
    }

    async fn save(&self, conversation: &Conversation) -> Result<Conversation> {
        let mut client = self.db.get_client().await?;

        let query = "
            INSERT INTO conversations (id, user_id, title, created_at, updated_at)
            VALUES (@P1, @P2, @P3, CONVERT(datetime2, @P4), CONVERT(datetime2, @P5))
        ";

        let created_at_str = conversation
            .created_at
            .format(&time::format_description::well_known::Iso8601::DEFAULT)?;
        let updated_at_str = conversation
            .updated_at
            .format(&time::format_description::well_known::Iso8601::DEFAULT)?;

        client
            .execute(
                query,
                &[
                    &conversation.id,
                    &conversation.user_id,
                    &conversation.title,
                    &created_at_str.as_str(),
                    &updated_at_str.as_str(),
                ],
            )
            .await
            .context("Failed to insert conversation")?;

        Ok(conversation.clone())
    }

    async fn update(&self, conversation: &Conversation) -> Result<Conversation> {
        let mut client = self.db.get_client().await?;

        let now = OffsetDateTime::now_utc();
        let now_str = now.format(&time::format_description::well_known::Iso8601::DEFAULT)?;

        let query = "
            UPDATE conversations
            SET title = @P2, updated_at = CONVERT(datetime2, @P3)
            WHERE id = @P1
        ";

        client
            .execute(
                query,
                &[&conversation.id, &conversation.title, &now_str.as_str()],
            )
            .await
            .context("Failed to update conversation")?;

        let mut updated = conversation.clone();
        updated.updated_at = now;
        Ok(updated)
    }

    async fn delete(&self, id: Uuid) -> Result<()> {
        let mut client = self.db.get_client().await?;

        let query = "DELETE FROM conversations WHERE id = @P1";

        client
            .execute(query, &[&id])
            .await
            .context("Failed to delete conversation")?;

        Ok(())
    }
}
