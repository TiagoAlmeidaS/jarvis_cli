use anyhow::Result;
use chrono::DateTime;
use chrono::Utc;
use jarvis_protocol::ThreadId;
use sqlx::Row;
use sqlx::sqlite::SqliteRow;

/// A single tool operation record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolOperation {
    /// The database ID.
    pub id: i64,
    /// The thread identifier.
    pub thread_id: ThreadId,
    /// The unique call ID for this tool operation.
    pub call_id: String,
    /// The name of the tool.
    pub tool_name: String,
    /// The arguments passed to the tool (JSON string).
    pub arguments: String,
    /// The result returned by the tool (JSON string, optional).
    pub result: Option<String>,
    /// Whether the operation was successful.
    pub success: bool,
    /// Duration in milliseconds.
    pub duration_ms: i64,
    /// The decision made (approved/denied, optional).
    pub decision: Option<String>,
    /// The source of the decision (config/user, optional).
    pub decision_source: Option<String>,
    /// The creation timestamp.
    pub created_at: DateTime<Utc>,
    /// Nanoseconds component for precise ordering.
    pub created_at_nanos: i64,
}

/// Builder for creating tool operation records.
#[derive(Debug, Clone)]
pub struct ToolOperationBuilder {
    /// The thread identifier.
    pub thread_id: ThreadId,
    /// The unique call ID for this tool operation.
    pub call_id: String,
    /// The name of the tool.
    pub tool_name: String,
    /// The arguments passed to the tool (JSON string).
    pub arguments: String,
    /// The result returned by the tool (JSON string, optional).
    pub result: Option<String>,
    /// Whether the operation was successful.
    pub success: bool,
    /// Duration in milliseconds.
    pub duration_ms: i64,
    /// The decision made (approved/denied, optional).
    pub decision: Option<String>,
    /// The source of the decision (config/user, optional).
    pub decision_source: Option<String>,
}

impl ToolOperationBuilder {
    /// Create a new builder with required fields.
    pub fn new(
        thread_id: ThreadId,
        call_id: String,
        tool_name: String,
        arguments: String,
        success: bool,
        duration_ms: i64,
    ) -> Self {
        Self {
            thread_id,
            call_id,
            tool_name,
            arguments,
            result: None,
            success,
            duration_ms,
            decision: None,
            decision_source: None,
        }
    }

    /// Set the result.
    pub fn with_result(mut self, result: Option<String>) -> Self {
        self.result = result;
        self
    }

    /// Set the decision and source.
    pub fn with_decision(mut self, decision: Option<String>, source: Option<String>) -> Self {
        self.decision = decision;
        self.decision_source = source;
        self
    }

    /// Build a tool operation with the current timestamp.
    pub fn build(self) -> ToolOperation {
        let now = Utc::now();
        ToolOperation {
            id: 0, // Will be set by database
            thread_id: self.thread_id,
            call_id: self.call_id,
            tool_name: self.tool_name,
            arguments: self.arguments,
            result: self.result,
            success: self.success,
            duration_ms: self.duration_ms,
            decision: self.decision,
            decision_source: self.decision_source,
            created_at: now,
            created_at_nanos: now.timestamp_subsec_nanos() as i64,
        }
    }
}

impl ToolOperation {
    /// Convert from a database row.
    pub fn from_row(row: &SqliteRow) -> Result<Self> {
        let created_at_ts = row.try_get::<i64, _>("created_at")?;
        let created_at_nanos = row.try_get::<i64, _>("created_at_nanos")?;
        let created_at = DateTime::from_timestamp(created_at_ts, created_at_nanos as u32)
            .unwrap_or_else(|| Utc::now());

        Ok(Self {
            id: row.try_get("id")?,
            thread_id: ThreadId::try_from(row.try_get::<String, _>("thread_id")?)?,
            call_id: row.try_get("call_id")?,
            tool_name: row.try_get("tool_name")?,
            arguments: row.try_get("arguments")?,
            result: row.try_get("result")?,
            success: row.try_get::<i64, _>("success")? != 0,
            duration_ms: row.try_get("duration_ms")?,
            decision: row.try_get("decision")?,
            decision_source: row.try_get("decision_source")?,
            created_at,
            created_at_nanos,
        })
    }

    /// Convert to a database row (for insertion).
    pub fn to_insert_params(&self) -> (String, String, String, String, Option<String>, i64, i64, Option<String>, Option<String>, i64, i64) {
        (
            self.thread_id.to_string(),
            self.call_id.clone(),
            self.tool_name.clone(),
            self.arguments.clone(),
            self.result.clone(),
            if self.success { 1 } else { 0 },
            self.duration_ms,
            self.decision.clone(),
            self.decision_source.clone(),
            self.created_at.timestamp(),
            self.created_at_nanos,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn tool_operation_builder_creates_basic_operation() {
        let thread_id = ThreadId::new_v4();
        let builder = ToolOperationBuilder::new(
            thread_id,
            "call-123".to_string(),
            "shell".to_string(),
            r#"{"command": "ls"}"#.to_string(),
            true,
            100,
        );

        let operation = builder.build();

        assert_eq!(operation.thread_id, thread_id);
        assert_eq!(operation.call_id, "call-123");
        assert_eq!(operation.tool_name, "shell");
        assert_eq!(operation.arguments, r#"{"command": "ls"}"#);
        assert!(operation.success);
        assert_eq!(operation.duration_ms, 100);
        assert_eq!(operation.result, None);
        assert_eq!(operation.decision, None);
        assert_eq!(operation.decision_source, None);
        assert_eq!(operation.id, 0);
    }

    #[test]
    fn tool_operation_builder_with_result() {
        let thread_id = ThreadId::new_v4();
        let operation = ToolOperationBuilder::new(
            thread_id,
            "call-123".to_string(),
            "shell".to_string(),
            r#"{"command": "ls"}"#.to_string(),
            true,
            100,
        )
        .with_result(Some("file1.txt\nfile2.txt".to_string()))
        .build();

        assert_eq!(operation.result, Some("file1.txt\nfile2.txt".to_string()));
    }

    #[test]
    fn tool_operation_builder_with_decision() {
        let thread_id = ThreadId::new_v4();
        let operation = ToolOperationBuilder::new(
            thread_id,
            "call-123".to_string(),
            "shell".to_string(),
            r#"{"command": "rm -rf /"}"#.to_string(),
            false,
            50,
        )
        .with_decision(Some("denied".to_string()), Some("user".to_string()))
        .build();

        assert_eq!(operation.decision, Some("denied".to_string()));
        assert_eq!(operation.decision_source, Some("user".to_string()));
    }

    #[test]
    fn tool_operation_builder_chaining() {
        let thread_id = ThreadId::new_v4();
        let operation = ToolOperationBuilder::new(
            thread_id,
            "call-123".to_string(),
            "file_read".to_string(),
            r#"{"path": "test.txt"}"#.to_string(),
            true,
            25,
        )
        .with_result(Some("content".to_string()))
        .with_decision(Some("approved".to_string()), Some("config".to_string()))
        .build();

        assert_eq!(operation.result, Some("content".to_string()));
        assert_eq!(operation.decision, Some("approved".to_string()));
        assert_eq!(operation.decision_source, Some("config".to_string()));
    }

    #[test]
    fn tool_operation_to_insert_params() {
        let thread_id = ThreadId::new_v4();
        let operation = ToolOperationBuilder::new(
            thread_id,
            "call-123".to_string(),
            "shell".to_string(),
            r#"{"command": "ls"}"#.to_string(),
            true,
            100,
        )
        .with_result(Some("output".to_string()))
        .with_decision(Some("approved".to_string()), Some("user".to_string()))
        .build();

        let params = operation.to_insert_params();

        assert_eq!(params.0, thread_id.to_string());
        assert_eq!(params.1, "call-123");
        assert_eq!(params.2, "shell");
        assert_eq!(params.3, r#"{"command": "ls"}"#);
        assert_eq!(params.4, Some("output".to_string()));
        assert_eq!(params.5, 1); // success = true -> 1
        assert_eq!(params.6, 100);
        assert_eq!(params.7, Some("approved".to_string()));
        assert_eq!(params.8, Some("user".to_string()));
    }

    #[test]
    fn tool_operation_to_insert_params_failure() {
        let thread_id = ThreadId::new_v4();
        let operation = ToolOperationBuilder::new(
            thread_id,
            "call-123".to_string(),
            "shell".to_string(),
            r#"{"command": "invalid"}"#.to_string(),
            false,
            10,
        )
        .build();

        let params = operation.to_insert_params();

        assert_eq!(params.5, 0); // success = false -> 0
    }
}
