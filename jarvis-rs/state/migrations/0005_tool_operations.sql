CREATE TABLE tool_operations (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    thread_id TEXT NOT NULL,
    call_id TEXT NOT NULL,
    tool_name TEXT NOT NULL,
    arguments TEXT NOT NULL,
    result TEXT,
    success INTEGER NOT NULL DEFAULT 0,
    duration_ms INTEGER NOT NULL DEFAULT 0,
    decision TEXT,
    decision_source TEXT,
    created_at INTEGER NOT NULL,
    created_at_nanos INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY(thread_id) REFERENCES threads(id) ON DELETE CASCADE
);

CREATE INDEX idx_tool_operations_thread_id ON tool_operations(thread_id);
CREATE INDEX idx_tool_operations_tool_name ON tool_operations(tool_name);
CREATE INDEX idx_tool_operations_created_at ON tool_operations(created_at DESC, created_at_nanos DESC, id DESC);
CREATE INDEX idx_tool_operations_success ON tool_operations(success);
CREATE INDEX idx_tool_operations_call_id ON tool_operations(call_id);
