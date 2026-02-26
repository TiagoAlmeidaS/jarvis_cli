use super::Database;
use super::Repository;
use super::User;
use anyhow::Context;
use anyhow::Result;
use async_trait::async_trait;
use tiberius::Row;
use time::OffsetDateTime;
use uuid::Uuid;

/// Repository for User entities
pub struct UserRepository {
    db: Database,
}

impl UserRepository {
    /// Create a new UserRepository
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    /// Find a user by email
    pub async fn find_by_email(&self, email: &str) -> Result<Option<User>> {
        let mut client = self.db.get_client().await?;

        let query = "
            SELECT id, username, email, password_hash,
                   FORMAT(created_at, 'yyyy-MM-ddTHH:mm:ss.fffffffzzz') as created_at,
                   FORMAT(updated_at, 'yyyy-MM-ddTHH:mm:ss.fffffffzzz') as updated_at
            FROM users
            WHERE email = @P1
        ";

        let stream = client
            .query(query, &[&email])
            .await
            .context("Failed to query user by email")?;

        let rows = stream.into_first_result().await?;

        if let Some(row) = rows.into_iter().next() {
            Ok(Some(Self::row_to_user(&row)?))
        } else {
            Ok(None)
        }
    }

    /// Find a user by username
    pub async fn find_by_username(&self, username: &str) -> Result<Option<User>> {
        let mut client = self.db.get_client().await?;

        let query = "
            SELECT id, username, email, password_hash,
                   FORMAT(created_at, 'yyyy-MM-ddTHH:mm:ss.fffffffzzz') as created_at,
                   FORMAT(updated_at, 'yyyy-MM-ddTHH:mm:ss.fffffffzzz') as updated_at
            FROM users
            WHERE username = @P1
        ";

        let stream = client
            .query(query, &[&username])
            .await
            .context("Failed to query user by username")?;

        let rows = stream.into_first_result().await?;

        if let Some(row) = rows.into_iter().next() {
            Ok(Some(Self::row_to_user(&row)?))
        } else {
            Ok(None)
        }
    }

    /// Convert a database row to a User
    fn row_to_user(row: &Row) -> Result<User> {
        let id: Uuid = row.get(0).context("Missing id column")?;
        let username: &str = row.get(1).context("Missing username column")?;
        let email: &str = row.get(2).context("Missing email column")?;
        let password_hash: &str = row.get(3).context("Missing password_hash column")?;
        let created_at_str: &str = row.get(4).context("Missing created_at column")?;
        let updated_at_str: &str = row.get(5).context("Missing updated_at column")?;

        // Parse ISO 8601 strings
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

        Ok(User {
            id,
            username: username.to_string(),
            email: email.to_string(),
            password_hash: password_hash.to_string(),
            created_at,
            updated_at,
        })
    }
}

#[async_trait]
impl Repository<User, Uuid> for UserRepository {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<User>> {
        let mut client = self.db.get_client().await?;

        let query = "
            SELECT id, username, email, password_hash,
                   FORMAT(created_at, 'yyyy-MM-ddTHH:mm:ss.fffffffzzz') as created_at,
                   FORMAT(updated_at, 'yyyy-MM-ddTHH:mm:ss.fffffffzzz') as updated_at
            FROM users
            WHERE id = @P1
        ";

        let stream = client
            .query(query, &[&id])
            .await
            .context("Failed to query user by id")?;

        let rows = stream.into_first_result().await?;

        if let Some(row) = rows.into_iter().next() {
            Ok(Some(Self::row_to_user(&row)?))
        } else {
            Ok(None)
        }
    }

    async fn find_all(&self) -> Result<Vec<User>> {
        let mut client = self.db.get_client().await?;

        let query = "
            SELECT id, username, email, password_hash,
                   FORMAT(created_at, 'yyyy-MM-ddTHH:mm:ss.fffffffzzz') as created_at,
                   FORMAT(updated_at, 'yyyy-MM-ddTHH:mm:ss.fffffffzzz') as updated_at
            FROM users
            ORDER BY created_at DESC
        ";

        let stream = client
            .query(query, &[])
            .await
            .context("Failed to query all users")?;

        let rows = stream.into_first_result().await?;

        let mut users = Vec::new();
        for row in rows {
            users.push(Self::row_to_user(&row)?);
        }

        Ok(users)
    }

    async fn save(&self, user: &User) -> Result<User> {
        let mut client = self.db.get_client().await?;

        let query = "
            INSERT INTO users (id, username, email, password_hash, created_at, updated_at)
            VALUES (@P1, @P2, @P3, @P4, CONVERT(datetime2, @P5), CONVERT(datetime2, @P6))
        ";

        // Convert to ISO 8601 strings
        let created_at_str = user
            .created_at
            .format(&time::format_description::well_known::Iso8601::DEFAULT)?;
        let updated_at_str = user
            .updated_at
            .format(&time::format_description::well_known::Iso8601::DEFAULT)?;

        client
            .execute(
                query,
                &[
                    &user.id,
                    &user.username,
                    &user.email,
                    &user.password_hash,
                    &created_at_str.as_str(),
                    &updated_at_str.as_str(),
                ],
            )
            .await
            .context("Failed to insert user")?;

        Ok(user.clone())
    }

    async fn update(&self, user: &User) -> Result<User> {
        let mut client = self.db.get_client().await?;

        let now = OffsetDateTime::now_utc();
        let now_str = now.format(&time::format_description::well_known::Iso8601::DEFAULT)?;

        let query = "
            UPDATE users
            SET username = @P2, email = @P3, password_hash = @P4, updated_at = CONVERT(datetime2, @P5)
            WHERE id = @P1
        ";

        client
            .execute(
                query,
                &[
                    &user.id,
                    &user.username,
                    &user.email,
                    &user.password_hash,
                    &now_str.as_str(),
                ],
            )
            .await
            .context("Failed to update user")?;

        let mut updated = user.clone();
        updated.updated_at = now;
        Ok(updated)
    }

    async fn delete(&self, id: Uuid) -> Result<()> {
        let mut client = self.db.get_client().await?;

        let query = "DELETE FROM users WHERE id = @P1";

        client
            .execute(query, &[&id])
            .await
            .context("Failed to delete user")?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires SQL Server running
    async fn test_user_crud() {
        let db = Database::new(
            "Server=localhost,1433;Database=jarvis;User Id=sa;Password=YourPassword123!;TrustServerCertificate=True"
        )
        .await
        .unwrap();

        let repo = UserRepository::new(db);

        // Create
        let user = User::new(
            "testuser".to_string(),
            "test@example.com".to_string(),
            "hashed_password".to_string(),
        );
        let saved = repo.save(&user).await.unwrap();
        assert_eq!(saved.username, "testuser");

        // Read
        let found = repo.find_by_id(user.id).await.unwrap();
        assert!(found.is_some());

        // Delete
        repo.delete(user.id).await.unwrap();
        let found = repo.find_by_id(user.id).await.unwrap();
        assert!(found.is_none());
    }
}
