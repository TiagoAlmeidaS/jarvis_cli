use anyhow::Context;
use anyhow::Result;
use std::sync::Arc;
use tiberius::Client;
use tiberius::Config;
use tokio::net::TcpStream;
use tokio_util::compat::TokioAsyncWriteCompatExt;

/// Database connection manager for SQL Server
#[derive(Debug)]
pub struct Database {
    config: Arc<Config>,
}

impl Database {
    /// Create a new database connection from a connection string
    ///
    /// # Example
    /// ```no_run
    /// let db = Database::new(
    ///     "Server=localhost,1433;Database=jarvis;User Id=sa;Password=Pass123!;TrustServerCertificate=True"
    /// ).await?;
    /// ```
    pub async fn new(connection_string: &str) -> Result<Self> {
        let config = Config::from_ado_string(connection_string)
            .context("Failed to parse SQL Server connection string")?;

        Ok(Self {
            config: Arc::new(config),
        })
    }

    /// Get a new client connection to the database
    ///
    /// Each call creates a new connection. For production use,
    /// consider implementing connection pooling.
    pub async fn get_client(&self) -> Result<Client<tokio_util::compat::Compat<TcpStream>>> {
        let tcp = TcpStream::connect(self.config.get_addr())
            .await
            .context("Failed to connect to SQL Server")?;

        tcp.set_nodelay(true)?;

        let client = Client::connect(self.config.as_ref().clone(), tcp.compat_write())
            .await
            .context("Failed to authenticate with SQL Server")?;

        Ok(client)
    }

    /// Test database connectivity
    pub async fn health_check(&self) -> Result<bool> {
        match self.get_client().await {
            Ok(mut client) => {
                // Try a simple query
                let result = client.simple_query("SELECT 1").await;
                Ok(result.is_ok())
            }
            Err(_) => Ok(false),
        }
    }

    /// Check if database is available
    pub async fn is_available(&self) -> bool {
        self.health_check().await.unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== Connection String Parsing Tests ====================

    #[tokio::test]
    async fn test_database_creation_with_valid_connection_string() {
        let result = Database::new(
            "Server=localhost,1433;Database=jarvis;User Id=sa;Password=Pass123!;TrustServerCertificate=True"
        ).await;

        // Should successfully create Database struct even without connecting
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_database_creation_with_minimal_connection_string() {
        let result = Database::new("Server=localhost;Database=jarvis").await;

        // Should parse minimal connection string
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_database_creation_with_invalid_connection_string() {
        let result = Database::new("invalid connection string").await;

        // Should fail to parse invalid connection string
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Failed to parse"));
    }

    #[tokio::test]
    async fn test_database_creation_with_empty_connection_string() {
        let result = Database::new("").await;

        // Empty connection string is technically valid for parsing (uses defaults)
        // but would fail when trying to connect
        assert!(result.is_ok());

        // Verify it fails when trying to actually connect
        if let Ok(db) = result {
            let client_result = db.get_client().await;
            assert!(client_result.is_err());
        }
    }

    #[tokio::test]
    async fn test_database_creation_with_different_formats() {
        // Test various valid connection string formats
        let formats = vec![
            "Server=localhost;Database=test",
            "Server=localhost,1433;Database=test;User Id=sa;Password=pass",
            "Server=127.0.0.1;Database=test;TrustServerCertificate=True",
            "Server=myserver.example.com;Database=test;Encrypt=False",
        ];

        for conn_str in formats {
            let result = Database::new(conn_str).await;
            assert!(
                result.is_ok(),
                "Failed to parse connection string: {}",
                conn_str
            );
        }
    }

    #[tokio::test]
    async fn test_database_creation_preserves_config() {
        let conn_str = "Server=localhost,1433;Database=jarvis;User Id=sa;Password=Pass123!;TrustServerCertificate=True";

        let db = Database::new(conn_str).await.unwrap();

        // Config should be stored in Arc
        assert!(db.config.get_addr().to_string().contains("localhost"));
    }

    // ==================== Integration Tests (Require SQL Server) ====================

    #[tokio::test]
    #[ignore] // Requires SQL Server running
    async fn test_database_connection_real() {
        let db = Database::new(
            "Server=localhost,1433;Database=jarvis_test;User Id=sa;Password=YourPassword123!;TrustServerCertificate=True"
        ).await;

        assert!(db.is_ok());
    }

    #[tokio::test]
    #[ignore] // Requires SQL Server running
    async fn test_database_get_client() {
        let db = Database::new(
            "Server=localhost,1433;Database=jarvis_test;User Id=sa;Password=YourPassword123!;TrustServerCertificate=True"
        )
        .await
        .unwrap();

        let result = db.get_client().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    #[ignore] // Requires SQL Server running
    async fn test_database_health_check() {
        let db = Database::new(
            "Server=localhost,1433;Database=jarvis_test;User Id=sa;Password=YourPassword123!;TrustServerCertificate=True"
        )
        .await
        .unwrap();

        let result = db.health_check().await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), true);
    }

    #[tokio::test]
    #[ignore] // Requires SQL Server running
    async fn test_database_is_available() {
        let db = Database::new(
            "Server=localhost,1433;Database=jarvis_test;User Id=sa;Password=YourPassword123!;TrustServerCertificate=True"
        )
        .await
        .unwrap();

        let available = db.is_available().await;
        assert_eq!(available, true);
    }

    #[tokio::test]
    #[ignore] // Requires SQL Server running
    async fn test_database_connection_failure() {
        let db = Database::new(
            "Server=nonexistent.example.com,1433;Database=jarvis_test;User Id=sa;Password=Pass123!;TrustServerCertificate=True"
        )
        .await
        .unwrap();

        let result = db.get_client().await;
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Failed to connect")
        );
    }

    #[tokio::test]
    #[ignore] // Requires SQL Server running
    async fn test_database_invalid_credentials() {
        let db = Database::new(
            "Server=localhost,1433;Database=jarvis_test;User Id=invalid_user;Password=wrong_password;TrustServerCertificate=True"
        )
        .await
        .unwrap();

        let result = db.get_client().await;
        assert!(result.is_err());
    }

    #[tokio::test]
    #[ignore] // Requires SQL Server running
    async fn test_database_multiple_clients() {
        let db = Database::new(
            "Server=localhost,1433;Database=jarvis_test;User Id=sa;Password=YourPassword123!;TrustServerCertificate=True"
        )
        .await
        .unwrap();

        // Should be able to create multiple clients
        let client1 = db.get_client().await;
        let client2 = db.get_client().await;

        assert!(client1.is_ok());
        assert!(client2.is_ok());
    }
}
