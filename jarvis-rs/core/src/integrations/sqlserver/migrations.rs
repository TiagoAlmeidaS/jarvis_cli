use super::Database;
use anyhow::{Context, Result};

/// Database migration manager
pub struct Migrator {
    db: Database,
}

impl Migrator {
    /// Create a new Migrator
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    /// Run all migrations
    pub async fn migrate(&self) -> Result<()> {
        self.create_migrations_table().await?;
        self.run_migration("001_create_users", Self::migration_001_create_users)
            .await?;
        self.run_migration(
            "002_create_conversations",
            Self::migration_002_create_conversations,
        )
        .await?;
        self.run_migration("003_create_messages", Self::migration_003_create_messages)
            .await?;
        self.run_migration("004_create_analytics", Self::migration_004_create_analytics)
            .await?;

        Ok(())
    }

    /// Create migrations tracking table
    async fn create_migrations_table(&self) -> Result<()> {
        let mut client = self.db.get_client().await?;

        let query = "
            IF NOT EXISTS (SELECT * FROM sys.tables WHERE name = 'migrations')
            BEGIN
                CREATE TABLE migrations (
                    id INT IDENTITY(1,1) PRIMARY KEY,
                    name NVARCHAR(255) NOT NULL UNIQUE,
                    executed_at DATETIME2 NOT NULL DEFAULT GETUTCDATE()
                )
            END
        ";

        client
            .execute(query, &[])
            .await
            .context("Failed to create migrations table")?;

        Ok(())
    }

    /// Run a single migration if not already executed
    async fn run_migration<F>(&self, name: &str, migration_fn: F) -> Result<()>
    where
        F: FnOnce() -> &'static str,
    {
        let mut client = self.db.get_client().await?;

        // Check if migration already executed
        let check_query = "SELECT COUNT(*) FROM migrations WHERE name = @P1";
        let mut result = client.query(check_query, &[&name]).await?;

        if let Some(row) = result.into_row().await? {
            let count: i32 = row.get(0).unwrap_or(0);
            if count > 0 {
                tracing::info!("Migration '{}' already executed, skipping", name);
                return Ok(());
            }
        }

        // Execute migration
        tracing::info!("Running migration '{}'", name);
        let sql = migration_fn();
        client
            .execute(sql, &[])
            .await
            .with_context(|| format!("Failed to execute migration '{}'", name))?;

        // Record migration
        let record_query = "INSERT INTO migrations (name) VALUES (@P1)";
        client.execute(record_query, &[&name]).await?;

        tracing::info!("Migration '{}' completed successfully", name);
        Ok(())
    }

    /// Migration 001: Create users table
    fn migration_001_create_users() -> &'static str {
        "
        CREATE TABLE users (
            id UNIQUEIDENTIFIER PRIMARY KEY,
            username NVARCHAR(255) NOT NULL UNIQUE,
            email NVARCHAR(255) NOT NULL UNIQUE,
            password_hash NVARCHAR(255) NOT NULL,
            created_at DATETIME2 NOT NULL DEFAULT GETUTCDATE(),
            updated_at DATETIME2 NOT NULL DEFAULT GETUTCDATE()
        );

        CREATE INDEX idx_users_email ON users(email);
        CREATE INDEX idx_users_username ON users(username);
        "
    }

    /// Migration 002: Create conversations table
    fn migration_002_create_conversations() -> &'static str {
        "
        CREATE TABLE conversations (
            id UNIQUEIDENTIFIER PRIMARY KEY,
            user_id UNIQUEIDENTIFIER NOT NULL,
            title NVARCHAR(500),
            created_at DATETIME2 NOT NULL DEFAULT GETUTCDATE(),
            updated_at DATETIME2 NOT NULL DEFAULT GETUTCDATE(),
            FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
        );

        CREATE INDEX idx_conversations_user_id ON conversations(user_id);
        CREATE INDEX idx_conversations_updated_at ON conversations(updated_at);
        "
    }

    /// Migration 003: Create messages table
    fn migration_003_create_messages() -> &'static str {
        "
        CREATE TABLE messages (
            id UNIQUEIDENTIFIER PRIMARY KEY,
            conversation_id UNIQUEIDENTIFIER NOT NULL,
            role NVARCHAR(50) NOT NULL,
            content NVARCHAR(MAX) NOT NULL,
            created_at DATETIME2 NOT NULL DEFAULT GETUTCDATE(),
            FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE CASCADE
        );

        CREATE INDEX idx_messages_conversation_id ON messages(conversation_id);
        CREATE INDEX idx_messages_created_at ON messages(created_at);
        "
    }

    /// Migration 004: Create analytics tables
    fn migration_004_create_analytics() -> &'static str {
        "
        -- Command executions tracking
        CREATE TABLE command_executions (
            id UNIQUEIDENTIFIER PRIMARY KEY,
            command_name NVARCHAR(255) NOT NULL,
            execution_time_ms INT NOT NULL,
            success BIT NOT NULL,
            error_message NVARCHAR(MAX),
            executed_at DATETIME2 NOT NULL DEFAULT GETUTCDATE()
        );

        CREATE INDEX idx_command_executions_name ON command_executions(command_name);
        CREATE INDEX idx_command_executions_executed_at ON command_executions(executed_at);

        -- Response quality tracking
        CREATE TABLE response_quality (
            id UNIQUEIDENTIFIER PRIMARY KEY,
            conversation_id UNIQUEIDENTIFIER,
            message_id UNIQUEIDENTIFIER,
            user_rating INT CHECK (user_rating >= 1 AND user_rating <= 5),
            feedback NVARCHAR(MAX),
            created_at DATETIME2 NOT NULL DEFAULT GETUTCDATE()
        );

        CREATE INDEX idx_response_quality_conversation ON response_quality(conversation_id);
        CREATE INDEX idx_response_quality_rating ON response_quality(user_rating);

        -- Skill usage tracking
        CREATE TABLE skill_usage (
            id UNIQUEIDENTIFIER PRIMARY KEY,
            skill_name NVARCHAR(255) NOT NULL UNIQUE,
            execution_count INT NOT NULL DEFAULT 0,
            avg_execution_time_ms INT NOT NULL DEFAULT 0,
            success_rate FLOAT NOT NULL DEFAULT 1.0,
            last_used_at DATETIME2 NOT NULL DEFAULT GETUTCDATE()
        );

        CREATE INDEX idx_skill_usage_name ON skill_usage(skill_name);
        CREATE INDEX idx_skill_usage_last_used ON skill_usage(last_used_at);
        "
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== SQL Generation Tests ====================

    #[test]
    fn test_migration_001_creates_users_table() {
        let sql = Migrator::migration_001_create_users();

        // Verify table creation
        assert!(sql.contains("CREATE TABLE users"));

        // Verify columns
        assert!(sql.contains("id UNIQUEIDENTIFIER PRIMARY KEY"));
        assert!(sql.contains("username NVARCHAR(255) NOT NULL UNIQUE"));
        assert!(sql.contains("email NVARCHAR(255) NOT NULL UNIQUE"));
        assert!(sql.contains("password_hash NVARCHAR(255) NOT NULL"));
        assert!(sql.contains("created_at DATETIME2 NOT NULL DEFAULT GETUTCDATE()"));
        assert!(sql.contains("updated_at DATETIME2 NOT NULL DEFAULT GETUTCDATE()"));

        // Verify indexes
        assert!(sql.contains("CREATE INDEX idx_users_email ON users(email)"));
        assert!(sql.contains("CREATE INDEX idx_users_username ON users(username)"));
    }

    #[test]
    fn test_migration_002_creates_conversations_table() {
        let sql = Migrator::migration_002_create_conversations();

        // Verify table creation
        assert!(sql.contains("CREATE TABLE conversations"));

        // Verify columns
        assert!(sql.contains("id UNIQUEIDENTIFIER PRIMARY KEY"));
        assert!(sql.contains("user_id UNIQUEIDENTIFIER NOT NULL"));
        assert!(sql.contains("title NVARCHAR(500)"));
        assert!(sql.contains("created_at DATETIME2 NOT NULL DEFAULT GETUTCDATE()"));
        assert!(sql.contains("updated_at DATETIME2 NOT NULL DEFAULT GETUTCDATE()"));

        // Verify foreign key
        assert!(sql.contains("FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE"));

        // Verify indexes
        assert!(sql.contains("CREATE INDEX idx_conversations_user_id ON conversations(user_id)"));
        assert!(sql.contains("CREATE INDEX idx_conversations_updated_at ON conversations(updated_at)"));
    }

    #[test]
    fn test_migration_003_creates_messages_table() {
        let sql = Migrator::migration_003_create_messages();

        // Verify table creation
        assert!(sql.contains("CREATE TABLE messages"));

        // Verify columns
        assert!(sql.contains("id UNIQUEIDENTIFIER PRIMARY KEY"));
        assert!(sql.contains("conversation_id UNIQUEIDENTIFIER NOT NULL"));
        assert!(sql.contains("role NVARCHAR(50) NOT NULL"));
        assert!(sql.contains("content NVARCHAR(MAX) NOT NULL"));
        assert!(sql.contains("created_at DATETIME2 NOT NULL DEFAULT GETUTCDATE()"));

        // Verify foreign key
        assert!(sql.contains("FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE CASCADE"));

        // Verify indexes
        assert!(sql.contains("CREATE INDEX idx_messages_conversation_id ON messages(conversation_id)"));
        assert!(sql.contains("CREATE INDEX idx_messages_created_at ON messages(created_at)"));
    }

    #[test]
    fn test_migration_004_creates_analytics_tables() {
        let sql = Migrator::migration_004_create_analytics();

        // Verify command_executions table
        assert!(sql.contains("CREATE TABLE command_executions"));
        assert!(sql.contains("command_name NVARCHAR(255) NOT NULL"));
        assert!(sql.contains("execution_time_ms INT NOT NULL"));
        assert!(sql.contains("success BIT NOT NULL"));
        assert!(sql.contains("error_message NVARCHAR(MAX)"));
        assert!(sql.contains("CREATE INDEX idx_command_executions_name"));

        // Verify response_quality table
        assert!(sql.contains("CREATE TABLE response_quality"));
        assert!(sql.contains("conversation_id UNIQUEIDENTIFIER"));
        assert!(sql.contains("message_id UNIQUEIDENTIFIER"));
        assert!(sql.contains("user_rating INT CHECK (user_rating >= 1 AND user_rating <= 5)"));
        assert!(sql.contains("feedback NVARCHAR(MAX)"));
        assert!(sql.contains("CREATE INDEX idx_response_quality_conversation"));

        // Verify skill_usage table
        assert!(sql.contains("CREATE TABLE skill_usage"));
        assert!(sql.contains("skill_name NVARCHAR(255) NOT NULL UNIQUE"));
        assert!(sql.contains("execution_count INT NOT NULL DEFAULT 0"));
        assert!(sql.contains("avg_execution_time_ms INT NOT NULL DEFAULT 0"));
        assert!(sql.contains("success_rate FLOAT NOT NULL DEFAULT 1.0"));
        assert!(sql.contains("last_used_at DATETIME2 NOT NULL DEFAULT GETUTCDATE()"));
        assert!(sql.contains("CREATE INDEX idx_skill_usage_name"));
    }

    #[test]
    fn test_migrations_have_cascade_deletes() {
        // Verify conversations CASCADE on user delete
        let conv_sql = Migrator::migration_002_create_conversations();
        assert!(conv_sql.contains("ON DELETE CASCADE"));

        // Verify messages CASCADE on conversation delete
        let msg_sql = Migrator::migration_003_create_messages();
        assert!(msg_sql.contains("ON DELETE CASCADE"));
    }

    #[test]
    fn test_migrations_have_proper_indexes() {
        // Users should have indexes on email and username
        let users_sql = Migrator::migration_001_create_users();
        assert!(users_sql.contains("idx_users_email"));
        assert!(users_sql.contains("idx_users_username"));

        // Conversations should have indexes on user_id and updated_at
        let conv_sql = Migrator::migration_002_create_conversations();
        assert!(conv_sql.contains("idx_conversations_user_id"));
        assert!(conv_sql.contains("idx_conversations_updated_at"));

        // Messages should have indexes on conversation_id and created_at
        let msg_sql = Migrator::migration_003_create_messages();
        assert!(msg_sql.contains("idx_messages_conversation_id"));
        assert!(msg_sql.contains("idx_messages_created_at"));
    }

    #[test]
    fn test_all_migrations_use_uniqueidentifier() {
        // All primary keys should be UNIQUEIDENTIFIER
        let migrations = vec![
            Migrator::migration_001_create_users(),
            Migrator::migration_002_create_conversations(),
            Migrator::migration_003_create_messages(),
            Migrator::migration_004_create_analytics(),
        ];

        for sql in migrations {
            assert!(sql.contains("UNIQUEIDENTIFIER"));
        }
    }

    #[test]
    fn test_all_migrations_use_utc_timestamps() {
        // All timestamps should use GETUTCDATE()
        let migrations = vec![
            Migrator::migration_001_create_users(),
            Migrator::migration_002_create_conversations(),
            Migrator::migration_003_create_messages(),
            Migrator::migration_004_create_analytics(),
        ];

        for sql in migrations {
            assert!(sql.contains("GETUTCDATE()"));
        }
    }

    #[test]
    fn test_analytics_tables_have_proper_constraints() {
        let sql = Migrator::migration_004_create_analytics();

        // response_quality should have rating constraint
        assert!(sql.contains("CHECK (user_rating >= 1 AND user_rating <= 5)"));

        // skill_usage should have default values
        assert!(sql.contains("execution_count INT NOT NULL DEFAULT 0"));
        assert!(sql.contains("avg_execution_time_ms INT NOT NULL DEFAULT 0"));
        assert!(sql.contains("success_rate FLOAT NOT NULL DEFAULT 1.0"));
    }

    // ==================== Integration Tests (Require SQL Server) ====================

    #[tokio::test]
    #[ignore] // Requires SQL Server running
    async fn test_migrations_full_run() {
        let db = Database::new(
            "Server=localhost,1433;Database=jarvis_test;User Id=sa;Password=YourPassword123!;TrustServerCertificate=True"
        )
        .await
        .unwrap();

        let migrator = Migrator::new(db);
        let result = migrator.migrate().await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    #[ignore] // Requires SQL Server running
    async fn test_migrations_idempotent() {
        // Migrations should be safe to run multiple times
        let db = Database::new(
            "Server=localhost,1433;Database=jarvis_test;User Id=sa;Password=YourPassword123!;TrustServerCertificate=True"
        )
        .await
        .unwrap();

        let migrator = Migrator::new(db);

        // Run migrations twice
        let result1 = migrator.migrate().await;
        assert!(result1.is_ok());

        let result2 = migrator.migrate().await;
        assert!(result2.is_ok());
    }
}
