# RAG Integration Example for `jarvis exec`

## Overview

This document shows **exactly** how to integrate RAG into the `jarvis exec` command flow.

## File to Modify

**Primary file**: `jarvis-rs/exec/src/lib.rs`

## Step-by-Step Integration

### 1. Add RAG Injector to Dependencies

At the top of `lib.rs`, add the RAG imports:

```rust
// Add to existing imports:
use jarvis_core::rag::{create_rag_injector, inject_rag_context, RagContextConfig, RagContextInjector};
use std::sync::Arc;
```

### 2. Initialize RAG in `run_main`

Around line 89 in `run_main`, after config loading but before the event loop:

```rust
pub async fn run_main(cli: Cli, jarvis_linux_sandbox_exe: Option<PathBuf>) -> anyhow::Result<()> {
    // ... existing code ...

    // NEW: Initialize RAG context injector
    let rag_injector = create_rag_injector().await;

    // Optional: Show RAG status to user (only in human-friendly mode, not JSON)
    if !json_mode {
        use jarvis_core::rag::is_rag_ready;
        if is_rag_ready(&rag_injector).await {
            let stats = rag_injector.get_context_stats().await.unwrap_or_default();
            if stats.total_documents > 0 {
                eprintln!("🔮 RAG Context: {} documents, {} chunks",
                    stats.total_documents, stats.total_chunks);
            }
        }
    }

    // ... rest of existing code ...
```

### 3. Inject RAG Context Before Sending Messages

Find the section around **line 400-430** where `InitialOperation::UserTurn` is created.

There are two places where `UserInput::Text` is pushed:

#### Location 1: Resume command (around line 400)

**BEFORE:**
```rust
items.push(UserInput::Text {
    text: prompt_text.clone(),
    text_elements: Vec::new(),
});
```

**AFTER:**
```rust
// Inject RAG context
let config = RagContextConfig {
    max_chunks: 5,
    min_score: 0.7,
    enabled: true,
};

let enhanced_text = inject_rag_context(&prompt_text, &rag_injector, &config)
    .await
    .unwrap_or_else(|e| {
        tracing::warn!("Failed to inject RAG context: {}", e);
        prompt_text.clone()
    });

items.push(UserInput::Text {
    text: enhanced_text,
    text_elements: Vec::new(),
});
```

#### Location 2: Normal prompt (around line 420)

**BEFORE:**
```rust
items.push(UserInput::Text {
    text: prompt_text.clone(),
    text_elements: Vec::new(),
});
```

**AFTER:**
```rust
// Inject RAG context
let config = RagContextConfig::default();

let enhanced_text = inject_rag_context(&prompt_text, &rag_injector, &config)
    .await
    .unwrap_or_else(|e| {
        tracing::warn!("Failed to inject RAG context: {}", e);
        prompt_text.clone()
    });

items.push(UserInput::Text {
    text: enhanced_text,
    text_elements: Vec::new(),
});
```

### Complete Modified Section

Here's what the complete section should look like after modifications:

```rust
// Around line 390-440 in lib.rs

let (initial_operation, prompt_summary) = match (command.as_ref(), prompt, images) {
    (Some(ExecCommand::Resume(resume_args)), None, imgs) => {
        // ... existing resume logic ...

        // RAG INTEGRATION:
        let config = RagContextConfig {
            max_chunks: 5,
            min_score: 0.7,
            enabled: true,
        };

        let enhanced_text = inject_rag_context(&prompt_text, &rag_injector, &config)
            .await
            .unwrap_or_else(|e| {
                tracing::warn!("Failed to inject RAG context: {}", e);
                prompt_text.clone()
            });

        items.push(UserInput::Text {
            text: enhanced_text,
            text_elements: Vec::new(),
        });

        // ... rest of resume code ...
    }
    (None, root_prompt, imgs) => {
        let prompt_text = resolve_prompt(root_prompt);
        let mut items: Vec<UserInput> = imgs
            .into_iter()
            .map(|path| UserInput::LocalImage { path })
            .collect();

        // RAG INTEGRATION:
        let config = RagContextConfig::default();

        let enhanced_text = inject_rag_context(&prompt_text, &rag_injector, &config)
            .await
            .unwrap_or_else(|e| {
                tracing::warn!("Failed to inject RAG context: {}", e);
                prompt_text.clone()
            });

        items.push(UserInput::Text {
            text: enhanced_text,
            text_elements: Vec::new(),
        });

        let output_schema = load_output_schema(output_schema_path);
        (
            InitialOperation::UserTurn {
                items,
                output_schema,
            },
            prompt_text,
        )
    }
};
```

## Optional: Add CLI Flags for RAG Control

In `jarvis-rs/exec/src/cli.rs`, add RAG control flags:

```rust
#[derive(Debug, Parser)]
pub struct Cli {
    // ... existing fields ...

    /// Disable RAG context injection
    #[arg(long = "no-rag")]
    pub disable_rag: bool,

    /// Maximum number of RAG context chunks to retrieve
    #[arg(long = "rag-max-chunks", default_value = "5")]
    pub rag_max_chunks: usize,

    /// Minimum similarity score for RAG chunks (0.0 to 1.0)
    #[arg(long = "rag-min-score", default_value = "0.7")]
    pub rag_min_score: f32,
}
```

Then use these flags when creating the config:

```rust
let config = RagContextConfig {
    max_chunks: cli.rag_max_chunks,
    min_score: cli.rag_min_score,
    enabled: !cli.disable_rag,
};
```

## Testing the Integration

### 1. Build the Project

```bash
cd jarvis-rs
cargo build --release --features qdrant,postgres
```

### 2. Add Test Documents

```bash
# Add some project documentation
./target/release/jarvis context add ../README.md
./target/release/jarvis context add ../docs/rag-integration-guide.md

# Verify
./target/release/jarvis context list
```

### 3. Test RAG in Exec

```bash
# Without RAG
./target/release/jarvis exec "What is this project about?" --no-rag

# With RAG (default)
./target/release/jarvis exec "What is this project about?"
```

You should see the LLM's response include information from your indexed documents!

### 4. Verify Context Injection

Enable debug logging to see what's happening:

```bash
RUST_LOG=jarvis_core::rag=debug ./target/release/jarvis exec "How does RAG work?"
```

Expected debug output:
```
DEBUG jarvis_core::rag: Generating embedding for query: "How does RAG work?"
DEBUG jarvis_core::rag: Found 3 relevant chunks
DEBUG jarvis_core::rag: Injecting 156 tokens of context
```

## Advanced: Dynamic RAG Configuration

For more intelligent RAG behavior, you can adjust parameters based on query characteristics:

```rust
// Determine RAG config based on query
let config = if prompt_text.contains("code") || prompt_text.contains("implement") {
    RagContextConfig {
        max_chunks: 10,  // More context for code questions
        min_score: 0.6,  // Lower threshold
        enabled: true,
    }
} else if prompt_text.len() < 20 {
    RagContextConfig {
        max_chunks: 3,   // Less context for short queries
        min_score: 0.8,  // Higher threshold
        enabled: true,
    }
} else {
    RagContextConfig::default()
};
```

## Troubleshooting

### Context not being injected?

Add debug logging:

```rust
let enhanced_text = inject_rag_context(&prompt_text, &rag_injector, &config)
    .await
    .unwrap_or_else(|e| {
        eprintln!("⚠️  RAG injection failed: {}", e);
        prompt_text.clone()
    });

if enhanced_text.len() > prompt_text.len() {
    let context_size = enhanced_text.len() - prompt_text.len();
    eprintln!("✅ RAG injected {} chars of context", context_size);
} else {
    eprintln!("ℹ️  No RAG context found for this query");
}
```

### Performance issues?

RAG injection adds ~200-500ms latency. To optimize:

1. Use local Qdrant instead of remote VPS
2. Cache embedding generation
3. Reduce `max_chunks`
4. Increase `min_score` threshold

### Ollama not available?

RAG will gracefully disable. To verify:

```bash
curl http://100.98.213.86:11434/api/tags
```

If Ollama is down, the system continues working without RAG.

## Next Steps

After integrating in `jarvis exec`, apply the same pattern to:

1. **`jarvis` (TUI)**: Interactive chat mode
2. **`jarvis mcp-server`**: MCP server message handling
3. **`jarvis agent`**: Autonomous agent queries

The integration pattern is the same for all:
1. Initialize `rag_injector` at startup
2. Call `inject_rag_context` before sending user messages
3. Handle errors gracefully with fallback to original message
