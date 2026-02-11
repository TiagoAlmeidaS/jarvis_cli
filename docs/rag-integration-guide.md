# RAG Integration Guide

## Overview

This guide explains how the RAG (Retrieval Augmented Generation) system integrates with the Jarvis CLI chat functionality.

## Architecture

```
┌─────────────┐
│   User      │
│   Message   │
└──────┬──────┘
       │
       v
┌─────────────────────────────┐
│  RagContextInjector         │
│  1. Generate query embedding│
│  2. Search vector store     │
│  3. Retrieve relevant chunks│
│  4. Format context          │
└──────┬──────────────────────┘
       │
       v
┌─────────────────────────────┐
│  Enhanced Message           │
│  [Context] + User Message   │
└──────┬──────────────────────┘
       │
       v
┌─────────────────────────────┐
│  LLM (via JarvisThread)     │
│  Processes with context     │
└─────────────────────────────┘
```

## Components

### 1. RAG Context Injector

The `RagContextInjector` is responsible for:
- Managing connections to vector store (Qdrant or in-memory)
- Managing connections to document store (PostgreSQL, JSON, or in-memory)
- Generating embeddings via Ollama
- Retrieving relevant context for user queries
- Formatting context for LLM consumption

### 2. Chat Helper Module

The `chat_helper` module provides utility functions:

```rust
use jarvis_core::rag::{create_rag_injector, inject_rag_context, RagContextConfig};

// Create RAG injector (with automatic fallback)
let injector = create_rag_injector().await;

// Configure RAG behavior
let config = RagContextConfig {
    max_chunks: 5,      // Maximum number of context chunks
    min_score: 0.7,     // Minimum similarity score (0.0-1.0)
    enabled: true,      // Enable/disable RAG
};

// Inject context into user message
let enhanced_message = inject_rag_context(
    &user_message,
    &injector,
    &config
).await?;
```

## Integration Steps

### Step 1: Initialize RAG Injector

In your application startup (e.g., `jarvis-rs/exec/src/lib.rs` or `jarvis-rs/tui/src/app.rs`):

```rust
use jarvis_core::rag::create_rag_injector;
use std::sync::Arc;

// Early in your main function or app initialization:
let rag_injector = create_rag_injector().await;
```

The injector will:
- Attempt to connect to Qdrant (VPS: `100.98.213.86:6333`)
- Attempt to connect to PostgreSQL (VPS: `100.98.213.86:5432`)
- Attempt to connect to Ollama (VPS: `100.98.213.86:11434`)
- Fall back to in-memory/JSON storage if connections fail
- Gracefully disable if Ollama is unavailable

### Step 2: Inject Context Before Sending to LLM

**For `jarvis exec` (non-interactive):**

In `jarvis-rs/exec/src/lib.rs`, around line 400-430 where `UserInput::Text` is created:

```rust
// BEFORE:
items.push(UserInput::Text {
    text: prompt_text.clone(),
    text_elements: Vec::new(),
});

// AFTER:
use jarvis_core::rag::{inject_rag_context, RagContextConfig};

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

**For `jarvis` interactive (TUI):**

In `jarvis-rs/tui/src/app.rs` or wherever user messages are submitted:

```rust
// When user submits a message:
let user_message = self.get_user_input(); // hypothetical method

// Inject RAG context
let config = RagContextConfig::default();
let enhanced_message = inject_rag_context(&user_message, &self.rag_injector, &config)
    .await
    .unwrap_or(user_message);

// Submit to thread
self.thread.submit(Op::UserTurn {
    items: vec![UserInput::Text {
        text: enhanced_message,
        text_elements: vec![],
    }],
    // ... other params
}).await?;
```

### Step 3: (Optional) Add RAG Status Indicator

Show users when RAG is active:

```rust
use jarvis_core::rag::is_rag_ready;

if is_rag_ready(&rag_injector).await {
    eprintln!("🔮 RAG Context: Enabled");
    let stats = rag_injector.get_context_stats().await?;
    eprintln!("   Documents: {}", stats.total_documents);
    eprintln!("   Chunks: {}", stats.total_chunks);
} else {
    eprintln!("RAG Context: Disabled");
}
```

## Configuration

Users can configure RAG via command-line flags or config file:

### Command-Line (Future)

```bash
# Disable RAG for a single command
jarvis exec "question" --no-rag

# Adjust RAG parameters
jarvis exec "question" --rag-max-chunks 10 --rag-min-score 0.6
```

### Config File (`~/.jarvis/config.toml`)

```toml
[rag]
enabled = true
max_chunks = 5
min_score = 0.7

# Override service URLs
ollama_url = "http://100.98.213.86:11434"
qdrant_url = "http://100.98.213.86:6333"
postgres_url = "postgresql://jarvis:password@100.98.213.86:5432/jarvis"

# Fallback behavior
fallback_to_memory = true
```

## Testing RAG Integration

### 1. Add Documents to Context

```bash
# Add project files
jarvis context add README.md
jarvis context add jarvis-rs/core/src/rag/mod.rs --tags rust,rag

# Check stats
jarvis context stats
```

### 2. Test Search

```bash
# Verify embeddings are working
jarvis context search "How does RAG work?" -n 5

# Expected output:
# 🔍 Searching context...
# Results: (3 results)
# 1. Result [85.2% similarity]
#    Source: jarvis-rs/core/src/rag/mod.rs
```

### 3. Test Chat Integration

```bash
# Start interactive chat (RAG should inject context automatically)
jarvis

# In chat, ask questions about your project:
User: "How is RAG implemented in this project?"

# Behind the scenes:
# 1. RAG searches for relevant chunks
# 2. Injects context: "Relevant Context from Project Files: [chunk1, chunk2, ...]"
# 3. LLM receives: "[Context]... How is RAG implemented in this project?"
# 4. LLM answers using injected context
```

## Fallback Behavior

The RAG system is designed to be resilient:

| Component | Primary | Fallback 1 | Fallback 2 | Fallback 3 |
|-----------|---------|------------|------------|------------|
| **Vector Store** | Qdrant (VPS) | In-Memory | - | - |
| **Document Store** | PostgreSQL | JSON File | In-Memory | - |
| **Embeddings** | Ollama (VPS) | Ollama (local) | ❌ Disabled | - |

If Ollama is unavailable, RAG is **disabled** but the application continues to work normally.

## Troubleshooting

### RAG not working?

```bash
# Check if Ollama is running
curl http://100.98.213.86:11434/api/tags

# Check if Qdrant is running
curl http://100.98.213.86:6333/collections

# Check Jarvis logs
RUST_LOG=jarvis_core::rag=debug jarvis exec "test"
```

### No context being injected?

1. Make sure you've added documents: `jarvis context list`
2. Check minimum similarity score isn't too high: `jarvis context search "test" --min-score 0.1`
3. Verify embeddings are being generated: `jarvis context stats`

### Performance Issues?

- Reduce `max_chunks`: fewer chunks = faster retrieval
- Increase `min_score`: higher threshold = fewer results
- Use local Qdrant instead of remote VPS

## Next Steps

1. **Enable RAG by default**: After testing, enable RAG for all chats
2. **Add RAG toggle command**: `/rag on` and `/rag off` during chat
3. **Add RAG debug command**: `/rag debug` to show what context was retrieved
4. **Smart context selection**: Automatically adjust based on conversation context

## References

- RAG Core Implementation: `jarvis-rs/core/src/rag/`
- Context Commands: `jarvis-rs/cli/src/context_cmd.rs`
- Chat Integration Helper: `jarvis-rs/core/src/rag/chat_helper.rs`
- Documentation: `docs/features/rag-context-management.md`
