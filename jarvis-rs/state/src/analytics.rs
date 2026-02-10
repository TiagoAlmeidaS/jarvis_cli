use crate::StateRuntime;
use crate::ToolOperation;
use jarvis_protocol::ThreadId;
use sqlx::QueryBuilder;
use sqlx::Row;
use sqlx::Sqlite;

/// Analytics queries for analyzing agent operation patterns.
impl StateRuntime {
    /// Get tool usage statistics for a specific thread.
    ///
    /// Returns a map of tool names to their usage counts.
    pub async fn tool_usage_stats(
        &self,
        thread_id: ThreadId,
    ) -> anyhow::Result<std::collections::HashMap<String, u64>> {
        let thread_id_str = thread_id.to_string();
        let rows: Vec<sqlx::sqlite::SqliteRow> = sqlx::query(
            r#"
SELECT tool_name, COUNT(*) as count
FROM tool_operations
WHERE thread_id = ?
GROUP BY tool_name
ORDER BY count DESC
            "#,
        )
        .bind(thread_id_str)
        .fetch_all(self.pool.as_ref())
        .await?;

        let mut stats = std::collections::HashMap::new();
        for row in rows {
            let tool_name: String = row.try_get::<String, _>("tool_name")?;
            let count: i64 = row.try_get::<i64, _>("count")?;
            stats.insert(tool_name, count as u64);
        }
        Ok(stats)
    }

    /// Get success rate statistics for tool operations.
    ///
    /// Returns success rate (0.0 to 1.0) per tool name.
    pub async fn tool_success_rates(
        &self,
        thread_id: Option<ThreadId>,
    ) -> anyhow::Result<std::collections::HashMap<String, f64>> {
        let mut builder = QueryBuilder::<Sqlite>::new(
            r#"
SELECT
    tool_name,
    COUNT(*) as total,
    SUM(CASE WHEN success = 1 THEN 1 ELSE 0 END) as successful
FROM tool_operations
            "#,
        );

        if let Some(thread_id) = thread_id {
            builder.push(" WHERE thread_id = ").push_bind(thread_id.to_string());
        }

        builder.push(" GROUP BY tool_name");

        let rows: Vec<sqlx::sqlite::SqliteRow> = builder.build().fetch_all(self.pool.as_ref()).await?;

        let mut rates = std::collections::HashMap::new();
        for row in rows {
            let tool_name: String = row.try_get::<String, _>("tool_name")?;
            let total: i64 = row.try_get::<i64, _>("total")?;
            let successful: i64 = row.try_get::<i64, _>("successful")?;
            if total > 0 {
                let rate = successful as f64 / total as f64;
                rates.insert(tool_name, rate);
            }
        }
        Ok(rates)
    }

    /// Get average duration per tool.
    ///
    /// Returns average duration in milliseconds per tool name.
    pub async fn tool_avg_durations(
        &self,
        thread_id: Option<ThreadId>,
    ) -> anyhow::Result<std::collections::HashMap<String, f64>> {
        let mut builder = QueryBuilder::<Sqlite>::new(
            r#"
SELECT
    tool_name,
    AVG(duration_ms) as avg_duration
FROM tool_operations
            "#,
        );

        if let Some(thread_id) = thread_id {
            builder.push(" WHERE thread_id = ").push_bind(thread_id.to_string());
        }

        builder.push(" GROUP BY tool_name");

        let rows: Vec<sqlx::sqlite::SqliteRow> = builder.build().fetch_all(self.pool.as_ref()).await?;

        let mut durations = std::collections::HashMap::new();
        for row in rows {
            let tool_name: String = row.try_get::<String, _>("tool_name")?;
            let avg_duration: Option<f64> = row.try_get::<Option<f64>, _>("avg_duration")?;
            if let Some(duration) = avg_duration {
                durations.insert(tool_name, duration);
            }
        }
        Ok(durations)
    }

    /// Get tool chain patterns.
    ///
    /// Returns sequences of tools used together, helping identify common workflows.
    pub async fn tool_chain_patterns(
        &self,
        thread_id: ThreadId,
        min_chain_length: usize,
    ) -> anyhow::Result<Vec<Vec<String>>> {
        let operations: Vec<crate::ToolOperation> = self.query_tool_operations(thread_id, None).await?;
        
        if operations.len() < min_chain_length {
            return Ok(Vec::new());
        }

        let mut chains = Vec::new();
        let mut current_chain = Vec::new();
        
        for op in operations {
            current_chain.push(op.tool_name.clone());
            
            // Consider a chain complete when we have enough tools
            // or when there's a significant time gap (simplified: just use min_chain_length)
            if current_chain.len() >= min_chain_length {
                chains.push(current_chain.clone());
                // Start a new chain from the last tool
                current_chain = vec![op.tool_name];
            }
        }
        
        // Add the last chain if it's long enough
        if current_chain.len() >= min_chain_length {
            chains.push(current_chain);
        }

        Ok(chains)
    }

    /// Get decision statistics (approved vs denied).
    pub async fn decision_stats(
        &self,
        thread_id: Option<ThreadId>,
    ) -> anyhow::Result<std::collections::HashMap<String, u64>> {
        let mut builder = QueryBuilder::<Sqlite>::new(
            r#"
SELECT decision, COUNT(*) as count
FROM tool_operations
WHERE decision IS NOT NULL
            "#,
        );

        if let Some(thread_id) = thread_id {
            builder.push(" AND thread_id = ").push_bind(thread_id.to_string());
        }

        builder.push(" GROUP BY decision");

        let rows: Vec<sqlx::sqlite::SqliteRow> = builder.build().fetch_all(self.pool.as_ref()).await?;

        let mut stats = std::collections::HashMap::new();
        for row in rows {
            let decision: String = row.try_get::<String, _>("decision")?;
            let count: i64 = row.try_get::<i64, _>("count")?;
            stats.insert(decision, count as u64);
        }
        Ok(stats)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ToolOperationBuilder;
    use pretty_assertions::assert_eq;

    fn temp_dir() -> std::path::PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("jarvis-state-analytics-test-{nanos}"))
    }

    #[tokio::test]
    async fn tool_usage_stats_empty() {
        let jarvis_home = temp_dir();
        let runtime = StateRuntime::init(jarvis_home.clone(), "test-provider".to_string(), None)
            .await
            .expect("init runtime");

        let thread_id = ThreadId::new_v4();
        let stats = runtime
            .tool_usage_stats(thread_id)
            .await
            .expect("get stats");

        assert_eq!(stats.len(), 0);
    }

    #[tokio::test]
    async fn tool_usage_stats_multiple_tools() {
        let jarvis_home = temp_dir();
        let runtime = StateRuntime::init(jarvis_home.clone(), "test-provider".to_string(), None)
            .await
            .expect("init runtime");

        let thread_id = ThreadId::new_v4();

        // Insert shell operations
        for i in 0..5 {
            let op = ToolOperationBuilder::new(
                thread_id,
                format!("shell-{i}"),
                "shell".to_string(),
                format!(r#"{{"cmd": "echo {i}"}}"#),
                true,
                50,
            )
            .build();
            runtime.insert_tool_operation(&op).await.unwrap();
        }

        // Insert file operations
        for i in 0..3 {
            let op = ToolOperationBuilder::new(
                thread_id,
                format!("file-{i}"),
                "file_read".to_string(),
                format!(r#"{{"path": "file{i}.txt"}}"#),
                true,
                25,
            )
            .build();
            runtime.insert_tool_operation(&op).await.unwrap();
        }

        let stats = runtime
            .tool_usage_stats(thread_id)
            .await
            .expect("get stats");

        assert_eq!(stats.get("shell"), Some(&5));
        assert_eq!(stats.get("file_read"), Some(&3));
        assert_eq!(stats.len(), 2);
    }

    #[tokio::test]
    async fn tool_success_rates_all_successful() {
        let jarvis_home = temp_dir();
        let runtime = StateRuntime::init(jarvis_home.clone(), "test-provider".to_string(), None)
            .await
            .expect("init runtime");

        let thread_id = ThreadId::new_v4();

        for i in 0..10 {
            let op = ToolOperationBuilder::new(
                thread_id,
                format!("call-{i}"),
                "shell".to_string(),
                format!(r#"{{"cmd": "echo {i}"}}"#),
                true,
                50,
            )
            .build();
            runtime.insert_tool_operation(&op).await.unwrap();
        }

        let rates = runtime
            .tool_success_rates(Some(thread_id))
            .await
            .expect("get rates");

        assert_eq!(rates.get("shell"), Some(&1.0));
    }

    #[tokio::test]
    async fn tool_success_rates_mixed() {
        let jarvis_home = temp_dir();
        let runtime = StateRuntime::init(jarvis_home.clone(), "test-provider".to_string(), None)
            .await
            .expect("init runtime");

        let thread_id = ThreadId::new_v4();

        // 8 successful
        for i in 0..8 {
            let op = ToolOperationBuilder::new(
                thread_id,
                format!("success-{i}"),
                "shell".to_string(),
                format!(r#"{{"cmd": "echo {i}"}}"#),
                true,
                50,
            )
            .build();
            runtime.insert_tool_operation(&op).await.unwrap();
        }

        // 2 failed
        for i in 0..2 {
            let op = ToolOperationBuilder::new(
                thread_id,
                format!("fail-{i}"),
                "shell".to_string(),
                format!(r#"{{"cmd": "invalid"}}"#),
                false,
                10,
            )
            .build();
            runtime.insert_tool_operation(&op).await.unwrap();
        }

        let rates = runtime
            .tool_success_rates(Some(thread_id))
            .await
            .expect("get rates");

        let shell_rate = rates.get("shell").expect("shell rate");
        assert!((shell_rate - 0.8).abs() < 0.01); // 8/10 = 0.8
    }

    #[tokio::test]
    async fn tool_success_rates_all_threads() {
        let jarvis_home = temp_dir();
        let runtime = StateRuntime::init(jarvis_home.clone(), "test-provider".to_string(), None)
            .await
            .expect("init runtime");

        let thread1 = ThreadId::new_v4();
        let thread2 = ThreadId::new_v4();

        // Thread 1: 5 successful
        for i in 0..5 {
            let op = ToolOperationBuilder::new(
                thread1,
                format!("t1-{i}"),
                "shell".to_string(),
                format!(r#"{{"cmd": "echo {i}"}}"#),
                true,
                50,
            )
            .build();
            runtime.insert_tool_operation(&op).await.unwrap();
        }

        // Thread 2: 3 successful, 2 failed
        for i in 0..3 {
            let op = ToolOperationBuilder::new(
                thread2,
                format!("t2-success-{i}"),
                "shell".to_string(),
                format!(r#"{{"cmd": "echo {i}"}}"#),
                true,
                50,
            )
            .build();
            runtime.insert_tool_operation(&op).await.unwrap();
        }
        for i in 0..2 {
            let op = ToolOperationBuilder::new(
                thread2,
                format!("t2-fail-{i}"),
                "shell".to_string(),
                format!(r#"{{"cmd": "invalid"}}"#),
                false,
                10,
            )
            .build();
            runtime.insert_tool_operation(&op).await.unwrap();
        }

        let rates = runtime
            .tool_success_rates(None)
            .await
            .expect("get rates");

        let shell_rate = rates.get("shell").expect("shell rate");
        // Total: 8 successful out of 10 = 0.8
        assert!((shell_rate - 0.8).abs() < 0.01);
    }

    #[tokio::test]
    async fn tool_avg_durations() {
        let jarvis_home = temp_dir();
        let runtime = StateRuntime::init(jarvis_home.clone(), "test-provider".to_string(), None)
            .await
            .expect("init runtime");

        let thread_id = ThreadId::new_v4();

        // Insert operations with different durations
        let durations = vec![100, 200, 300, 400, 500];
        for (i, duration) in durations.iter().enumerate() {
            let op = ToolOperationBuilder::new(
                thread_id,
                format!("call-{i}"),
                "shell".to_string(),
                format!(r#"{{"cmd": "echo {i}"}}"#),
                true,
                *duration,
            )
            .build();
            runtime.insert_tool_operation(&op).await.unwrap();
        }

        let avg_durations = runtime
            .tool_avg_durations(Some(thread_id))
            .await
            .expect("get avg durations");

        let shell_avg = avg_durations.get("shell").expect("shell avg");
        // Average of [100, 200, 300, 400, 500] = 300
        assert!((shell_avg - 300.0).abs() < 0.1);
    }

    #[tokio::test]
    async fn tool_chain_patterns_empty() {
        let jarvis_home = temp_dir();
        let runtime = StateRuntime::init(jarvis_home.clone(), "test-provider".to_string(), None)
            .await
            .expect("init runtime");

        let thread_id = ThreadId::new_v4();

        let chains = runtime
            .tool_chain_patterns(thread_id, 3)
            .await
            .expect("get chains");

        assert_eq!(chains.len(), 0);
    }

    #[tokio::test]
    async fn tool_chain_patterns_single_chain() {
        let jarvis_home = temp_dir();
        let runtime = StateRuntime::init(jarvis_home.clone(), "test-provider".to_string(), None)
            .await
            .expect("init runtime");

        let thread_id = ThreadId::new_v4();

        // Create a chain: shell -> file_read -> file_write
        let tools = vec!["shell", "file_read", "file_write", "shell"];
        for (i, tool) in tools.iter().enumerate() {
            let op = ToolOperationBuilder::new(
                thread_id,
                format!("call-{i}"),
                tool.to_string(),
                format!(r#"{{"arg": "{i}"}}"#),
                true,
                50,
            )
            .build();
            runtime.insert_tool_operation(&op).await.unwrap();
        }

        let chains = runtime
            .tool_chain_patterns(thread_id, 3)
            .await
            .expect("get chains");

        assert_eq!(chains.len(), 1);
        assert_eq!(chains[0], vec!["shell", "file_read", "file_write"]);
    }

    #[tokio::test]
    async fn decision_stats() {
        let jarvis_home = temp_dir();
        let runtime = StateRuntime::init(jarvis_home.clone(), "test-provider".to_string(), None)
            .await
            .expect("init runtime");

        let thread_id = ThreadId::new_v4();

        // Insert approved operations
        for i in 0..5 {
            let op = ToolOperationBuilder::new(
                thread_id,
                format!("approved-{i}"),
                "shell".to_string(),
                format!(r#"{{"cmd": "echo {i}"}}"#),
                true,
                50,
            )
            .with_decision(Some("approved".to_string()), Some("user".to_string()))
            .build();
            runtime.insert_tool_operation(&op).await.unwrap();
        }

        // Insert denied operations
        for i in 0..2 {
            let op = ToolOperationBuilder::new(
                thread_id,
                format!("denied-{i}"),
                "shell".to_string(),
                format!(r#"{{"cmd": "rm -rf /"}}"#),
                false,
                10,
            )
            .with_decision(Some("denied".to_string()), Some("user".to_string()))
            .build();
            runtime.insert_tool_operation(&op).await.unwrap();
        }

        let stats = runtime
            .decision_stats(Some(thread_id))
            .await
            .expect("get decision stats");

        assert_eq!(stats.get("approved"), Some(&5));
        assert_eq!(stats.get("denied"), Some(&2));
    }

    #[tokio::test]
    async fn decision_stats_no_decisions() {
        let jarvis_home = temp_dir();
        let runtime = StateRuntime::init(jarvis_home.clone(), "test-provider".to_string(), None)
            .await
            .expect("init runtime");

        let thread_id = ThreadId::new_v4();

        // Insert operations without decisions
        for i in 0..5 {
            let op = ToolOperationBuilder::new(
                thread_id,
                format!("call-{i}"),
                "shell".to_string(),
                format!(r#"{{"cmd": "echo {i}"}}"#),
                true,
                50,
            )
            .build();
            runtime.insert_tool_operation(&op).await.unwrap();
        }

        let stats = runtime
            .decision_stats(Some(thread_id))
            .await
            .expect("get decision stats");

        assert_eq!(stats.len(), 0);
    }
}
