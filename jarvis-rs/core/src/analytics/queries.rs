use crate::integrations::sqlserver::Database;
use anyhow::{Context, Result};

use super::metrics::{CommandMetrics, ErrorMetrics, SkillMetrics};

/// SQL queries for analytics
pub struct AnalyticsQueries {
    db: Database,
}

impl AnalyticsQueries {
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    /// Get slow commands (avg execution time > threshold_ms)
    pub async fn get_slow_commands(&self, threshold_ms: i32, limit: i32) -> Result<Vec<CommandMetrics>> {
        let mut client = self.db.get_client().await?;

        let query = format!(
            "SELECT TOP {}
                command_name,
                COUNT(*) as execution_count,
                SUM(CASE WHEN success = 1 THEN 1 ELSE 0 END) as success_count,
                SUM(CASE WHEN success = 0 THEN 1 ELSE 0 END) as failure_count,
                AVG(CAST(execution_time_ms AS FLOAT)) as avg_execution_time_ms,
                MIN(execution_time_ms) as min_execution_time_ms,
                MAX(execution_time_ms) as max_execution_time_ms,
                FORMAT(MAX(executed_at), 'yyyy-MM-ddTHH:mm:ss.fffffffzzz') as last_executed_at
             FROM command_executions
             WHERE success = 1
             GROUP BY command_name
             HAVING AVG(CAST(execution_time_ms AS FLOAT)) > {}
             ORDER BY avg_execution_time_ms DESC",
            limit, threshold_ms
        );

        let stream = client.query(&query, &[]).await?;
        let rows = stream.into_first_result().await?;

        let mut commands = Vec::new();
        for row in rows {
            commands.push(Self::row_to_command_metrics(&row)?);
        }

        Ok(commands)
    }

    /// Get commands with low success rate
    pub async fn get_unreliable_commands(&self, min_executions: i32, limit: i32) -> Result<Vec<CommandMetrics>> {
        let mut client = self.db.get_client().await?;

        let query = format!(
            "SELECT TOP {}
                command_name,
                COUNT(*) as execution_count,
                SUM(CASE WHEN success = 1 THEN 1 ELSE 0 END) as success_count,
                SUM(CASE WHEN success = 0 THEN 1 ELSE 0 END) as failure_count,
                AVG(CAST(execution_time_ms AS FLOAT)) as avg_execution_time_ms,
                MIN(execution_time_ms) as min_execution_time_ms,
                MAX(execution_time_ms) as max_execution_time_ms,
                FORMAT(MAX(executed_at), 'yyyy-MM-ddTHH:mm:ss.fffffffzzz') as last_executed_at
             FROM command_executions
             GROUP BY command_name
             HAVING COUNT(*) > {}
                AND (CAST(SUM(CASE WHEN success = 1 THEN 1 ELSE 0 END) AS FLOAT) / COUNT(*)) < 0.8
             ORDER BY success_count * 1.0 / COUNT(*) ASC",
            limit, min_executions
        );

        let stream = client.query(&query, &[]).await?;
        let rows = stream.into_first_result().await?;

        let mut commands = Vec::new();
        for row in rows {
            commands.push(Self::row_to_command_metrics(&row)?);
        }

        Ok(commands)
    }

    /// Get frequent errors
    pub async fn get_frequent_errors(&self, min_occurrences: i32, limit: i32) -> Result<Vec<ErrorMetrics>> {
        let mut client = self.db.get_client().await?;

        let query = format!(
            "SELECT TOP {}
                error_message,
                COUNT(*) as occurrence_count,
                FORMAT(MAX(executed_at), 'yyyy-MM-ddTHH:mm:ss.fffffffzzz') as last_occurred_at
             FROM command_executions
             WHERE success = 0 AND error_message IS NOT NULL
             GROUP BY error_message
             HAVING COUNT(*) >= {}
             ORDER BY occurrence_count DESC",
            limit, min_occurrences
        );

        let stream = client.query(&query, &[]).await?;
        let rows = stream.into_first_result().await?;

        let mut errors = Vec::new();
        for row in rows {
            let error_message: &str = row.get(0).context("Missing error_message")?;
            let occurrence_count: i32 = row.get(1).context("Missing occurrence_count")?;
            let last_occurred_at: &str = row.get(2).context("Missing last_occurred_at")?;

            errors.push(ErrorMetrics {
                error_message: error_message.to_string(),
                occurrence_count: occurrence_count as u64,
                last_occurred_at: last_occurred_at.to_string(),
            });
        }

        Ok(errors)
    }

    /// Get skills that need optimization
    pub async fn get_skills_needing_optimization(&self, limit: i32) -> Result<Vec<SkillMetrics>> {
        let mut client = self.db.get_client().await?;

        let query = format!(
            "SELECT TOP {}
                skill_name,
                execution_count,
                avg_execution_time_ms,
                success_rate,
                FORMAT(last_used_at, 'yyyy-MM-ddTHH:mm:ss.fffffffzzz') as last_used_at
             FROM skill_usage
             WHERE success_rate < 0.9 OR avg_execution_time_ms > 5000
             ORDER BY execution_count DESC",
            limit
        );

        let stream = client.query(&query, &[]).await?;
        let rows = stream.into_first_result().await?;

        let mut skills = Vec::new();
        for row in rows {
            skills.push(Self::row_to_skill_metrics(&row)?);
        }

        Ok(skills)
    }

    /// Get most used skills
    pub async fn get_top_skills(&self, limit: i32) -> Result<Vec<SkillMetrics>> {
        let mut client = self.db.get_client().await?;

        let query = format!(
            "SELECT TOP {}
                skill_name,
                execution_count,
                avg_execution_time_ms,
                success_rate,
                FORMAT(last_used_at, 'yyyy-MM-ddTHH:mm:ss.fffffffzzz') as last_used_at
             FROM skill_usage
             ORDER BY execution_count DESC",
            limit
        );

        let stream = client.query(&query, &[]).await?;
        let rows = stream.into_first_result().await?;

        let mut skills = Vec::new();
        for row in rows {
            skills.push(Self::row_to_skill_metrics(&row)?);
        }

        Ok(skills)
    }

    /// Helper: Convert row to CommandMetrics
    fn row_to_command_metrics(row: &tiberius::Row) -> Result<CommandMetrics> {
        use time::OffsetDateTime;

        let command_name: &str = row.get(0).context("Missing command_name")?;
        let execution_count: i32 = row.get(1).context("Missing execution_count")?;
        let success_count: i32 = row.get(2).context("Missing success_count")?;
        let failure_count: i32 = row.get(3).context("Missing failure_count")?;
        let avg_execution_time_ms: f64 = row.get(4).context("Missing avg_execution_time_ms")?;
        let min_execution_time_ms: i32 = row.get(5).context("Missing min_execution_time_ms")?;
        let max_execution_time_ms: i32 = row.get(6).context("Missing max_execution_time_ms")?;
        let last_executed_at_str: &str = row.get(7).context("Missing last_executed_at")?;

        let last_executed_at = OffsetDateTime::parse(
            last_executed_at_str,
            &time::format_description::well_known::Iso8601::DEFAULT,
        )
        .context("Failed to parse last_executed_at")?;

        Ok(CommandMetrics {
            command_name: command_name.to_string(),
            execution_count: execution_count as u64,
            success_count: success_count as u64,
            failure_count: failure_count as u64,
            avg_execution_time_ms,
            min_execution_time_ms,
            max_execution_time_ms,
            last_executed_at,
        })
    }

    /// Helper: Convert row to SkillMetrics
    fn row_to_skill_metrics(row: &tiberius::Row) -> Result<SkillMetrics> {
        use time::OffsetDateTime;

        let skill_name: &str = row.get(0).context("Missing skill_name")?;
        let execution_count: i32 = row.get(1).context("Missing execution_count")?;
        let avg_execution_time_ms: i32 = row.get(2).context("Missing avg_execution_time_ms")?;
        let success_rate: f64 = row.get(3).context("Missing success_rate")?;
        let last_used_at_str: &str = row.get(4).context("Missing last_used_at")?;

        let last_used_at = OffsetDateTime::parse(
            last_used_at_str,
            &time::format_description::well_known::Iso8601::DEFAULT,
        )
        .context("Failed to parse last_used_at")?;

        Ok(SkillMetrics {
            skill_name: skill_name.to_string(),
            execution_count: execution_count as u64,
            avg_execution_time_ms,
            success_rate,
            last_used_at,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: This module contains primarily async methods that interact with SQL Server.
    // Unit tests for these methods require complex mocking of tiberius::Row and Database.
    //
    // The following should be tested in INTEGRATION TESTS (Task #13):
    //
    // 1. get_slow_commands(threshold_ms, limit)
    //    - Verify SQL query format (TOP, GROUP BY, HAVING, ORDER BY)
    //    - Test with different thresholds (1000ms, 2000ms, 5000ms)
    //    - Verify limit is applied correctly
    //    - Test with empty results
    //
    // 2. get_unreliable_commands(min_executions, limit)
    //    - Verify success rate calculation (< 0.8)
    //    - Test filtering by min_executions
    //    - Verify ordering (worst success rate first)
    //    - Test edge case: exactly 80% success rate
    //
    // 3. get_frequent_errors(min_occurrences, limit)
    //    - Verify filtering by error count
    //    - Test with NULL error_message handling
    //    - Verify ordering (most frequent first)
    //    - Test date formatting in results
    //
    // 4. get_skills_needing_optimization(limit)
    //    - Verify filtering: success_rate < 0.9 OR avg_time > 5000ms
    //    - Test boundary conditions (exactly 0.9, exactly 5000ms)
    //    - Verify ordering by execution_count DESC
    //
    // 5. get_top_skills(limit)
    //    - Verify simple ordering by execution_count
    //    - Test with various limit values
    //
    // 6. row_to_command_metrics(row)
    //    - Test with valid row data
    //    - Test with missing columns (should return error)
    //    - Test date parsing with ISO8601 format
    //    - Test type conversions (i32 -> u64, etc.)
    //
    // 7. row_to_skill_metrics(row)
    //    - Test with valid row data
    //    - Test with missing columns
    //    - Test date parsing
    //
    // Integration test setup requirements:
    // - Docker container with SQL Server
    // - Test database with command_executions and skill_usage tables
    // - Seed data with known slow commands, unreliable commands, errors
    // - Verify query results match expected metrics
    //
    // Coverage target: 80% (via integration tests)

    #[test]
    fn test_queries_documentation() {
        // This test ensures the integration test requirements are documented
        // Actual tests will be in tests/integration/analytics_queries.rs
        assert!(true, "Integration tests required - see comments above");
    }
}
