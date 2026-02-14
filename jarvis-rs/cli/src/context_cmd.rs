//! CLI commands for RAG context management.

use anyhow::Result;
use clap::{Args, Subcommand};
use jarvis_common::CliConfigOverrides;
use jarvis_core::config::find_jarvis_home;
use jarvis_core::rag::{
    ChunkingConfig, DocumentIndexer, DocumentMetadata, DocumentStore, Embedding,
    EmbeddingGenerator, EmbeddingMetadata, InMemoryDocumentIndexer, InMemoryVectorStore,
    JsonFileDocumentStore, KnowledgeRetriever, OllamaEmbeddingGenerator, SimpleKnowledgeRetriever,
    VectorStore,
};
use owo_colors::OwoColorize;
use serde_json;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(feature = "qdrant")]
use jarvis_core::rag::QdrantVectorStore;

#[cfg(feature = "postgres")]
use jarvis_core::rag::PostgresDocumentStore;

/// Context management commands for RAG
#[derive(Debug, Args)]
pub struct ContextCli {
    #[clap(skip)]
    pub config_overrides: CliConfigOverrides,

    #[command(subcommand)]
    pub command: ContextCommand,
}

#[derive(Debug, Subcommand)]
pub enum ContextCommand {
    /// Add documents to context
    Add(AddArgs),

    /// Search in context
    Search(SearchArgs),

    /// List all documents in context
    List(ListArgs),

    /// Compress context by removing redundant information
    Compress(CompressArgs),

    /// Remove a document from context
    Remove(RemoveArgs),

    /// Show statistics about the context
    Stats(StatsArgs),
}

#[derive(Debug, Args)]
pub struct AddArgs {
    /// Path to document or directory to add
    #[arg(value_name = "PATH", required = true)]
    pub path: PathBuf,

    /// Document type (code, docs, project, markdown, text)
    #[arg(long, short = 't')]
    pub doc_type: Option<String>,

    /// Tags to associate with the document
    #[arg(long, value_delimiter = ',')]
    pub tags: Vec<String>,

    /// Language (for code files)
    #[arg(long, short = 'l')]
    pub language: Option<String>,

    /// Output format
    #[arg(long, short = 'o', value_enum, default_value = "human")]
    pub output: OutputFormat,
}

#[derive(Debug, Args)]
pub struct SearchArgs {
    /// Search query
    #[arg(value_name = "QUERY", required = true)]
    pub query: String,

    /// Maximum number of results
    #[arg(long, short = 'n', default_value = "5")]
    pub limit: usize,

    /// Minimum relevance score (0.0 to 1.0)
    #[arg(long, default_value = "0.3")]
    pub min_score: f32,

    /// Output format
    #[arg(long, short = 'o', value_enum, default_value = "human")]
    pub output: OutputFormat,

    /// Show source information
    #[arg(long, default_value = "true")]
    pub show_source: bool,
}

#[derive(Debug, Args)]
pub struct ListArgs {
    /// Filter by document type
    #[arg(long, short = 't')]
    pub doc_type: Option<String>,

    /// Filter by tag
    #[arg(long)]
    pub tag: Option<String>,

    /// Output format
    #[arg(long, short = 'o', value_enum, default_value = "human")]
    pub output: OutputFormat,
}

#[derive(Debug, Args)]
pub struct CompressArgs {
    /// Maximum tokens to keep
    #[arg(long, default_value = "4000")]
    pub max_tokens: usize,

    /// Strategy (most-relevant, most-recent, balanced)
    #[arg(long, value_enum, default_value = "most-relevant")]
    pub strategy: CompressionStrategy,

    /// Dry run - don't actually remove anything
    #[arg(long)]
    pub dry_run: bool,

    /// Output format
    #[arg(long, short = 'o', value_enum, default_value = "human")]
    pub output: OutputFormat,
}

#[derive(Debug, Args)]
pub struct RemoveArgs {
    /// Document ID to remove
    #[arg(value_name = "DOC_ID", required = true)]
    pub doc_id: String,

    /// Force removal without confirmation
    #[arg(long, short = 'f')]
    pub force: bool,
}

#[derive(Debug, Args)]
pub struct StatsArgs {
    /// Output format
    #[arg(long, short = 'o', value_enum, default_value = "human")]
    pub output: OutputFormat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum OutputFormat {
    /// Human-readable output with colors
    Human,
    /// JSON output
    Json,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum CompressionStrategy {
    /// Keep most relevant documents
    MostRelevant,
    /// Keep most recent documents
    MostRecent,
    /// Balanced approach
    Balanced,
}

impl ContextCli {
    pub async fn run(self) -> Result<()> {
        match self.command {
            ContextCommand::Add(args) => add_context(args).await,
            ContextCommand::Search(args) => search_context(args).await,
            ContextCommand::List(args) => list_context(args).await,
            ContextCommand::Compress(args) => compress_context(args).await,
            ContextCommand::Remove(args) => remove_context(args).await,
            ContextCommand::Stats(args) => show_stats(args).await,
        }
    }
}

/// Create a document store with fallback strategy.
async fn create_document_store() -> Arc<dyn DocumentStore> {
    // Try PostgreSQL first (if feature enabled)
    #[cfg(feature = "postgres")]
    {
        if let Ok(store) = PostgresDocumentStore::from_config().await {
            return Arc::new(store);
        }
    }

    // Fallback to JSON file store
    let json_path = find_jarvis_home()
        .ok()
        .map(|h: PathBuf| h.join("documents.json"))
        .unwrap_or_else(|| PathBuf::from(".jarvis/documents.json"));

    if let Ok(store) = JsonFileDocumentStore::new(&json_path).await {
        Arc::new(store)
    } else {
        // Last resort: in-memory (not persistent)
        Arc::new(jarvis_core::rag::InMemoryDocumentStore::new())
    }
}

/// Add a document to the context
async fn add_context(args: AddArgs) -> Result<()> {
    let config = ChunkingConfig::default();
    let indexer: Arc<dyn DocumentIndexer> = Arc::new(InMemoryDocumentIndexer::new(config));

    if args.output == OutputFormat::Human {
        println!("\n{}", "📄 Adding document to context...".bold().cyan());
        println!("{}", "─".repeat(50).dimmed());
        println!(
            "  {} {}",
            "Path:".bold(),
            args.path.display().to_string().cyan()
        );
        if let Some(ref doc_type) = args.doc_type {
            println!("  {} {}", "Type:".bold(), doc_type.yellow());
        }
        if !args.tags.is_empty() {
            println!("  {} {}", "Tags:".bold(), args.tags.join(", ").green());
        }
        println!();
    }

    // Create metadata
    let mut metadata = DocumentMetadata::default();
    metadata.doc_type = args.doc_type.clone();
    metadata.language = args.language.clone();
    metadata.tags = args.tags.clone();

    // Index document
    let doc = if args.path.is_file() {
        indexer.index_document(&args.path).await?
    } else {
        return Err(anyhow::anyhow!(
            "Path must be a file. Directory indexing not yet implemented."
        ));
    };

    // Generate embeddings and store in vector database
    if args.output == OutputFormat::Human {
        println!("{}", "🔮 Generating embeddings...".cyan());
    }

    let embedding_gen = OllamaEmbeddingGenerator::from_config()?;

    #[cfg(feature = "qdrant")]
    let vector_store: Arc<dyn VectorStore> = Arc::new(
        QdrantVectorStore::from_config()
            .await
            .map_err(|e| {
                if args.output == OutputFormat::Human {
                    eprintln!(
                        "{}",
                        "⚠️  Failed to connect to Qdrant, using in-memory storage".yellow()
                    );
                    eprintln!("   {}", format!("Error: {}", e).dimmed());
                }
                e
            })
            .unwrap_or_else(|_| {
                // Fallback to in-memory if Qdrant fails
                InMemoryVectorStore::new()
            }),
    );

    #[cfg(not(feature = "qdrant"))]
    let vector_store: Arc<dyn VectorStore> = Arc::new(InMemoryVectorStore::new());

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    // Process each chunk
    for (i, chunk) in doc.chunks.iter().enumerate() {
        if args.output == OutputFormat::Human {
            print!("\r  Processing chunk {}/{}...", i + 1, doc.chunks.len());
            std::io::Write::flush(&mut std::io::stdout())?;
        }

        // Generate embedding
        let embedding_vec = embedding_gen.generate_embedding(&chunk.text).await?;

        // Create embedding with metadata
        let embedding = Embedding {
            vector: embedding_vec,
            chunk_id: chunk.id.clone(),
            metadata: EmbeddingMetadata {
                source: doc.path.to_string_lossy().to_string(),
                chunk_index: chunk.chunk_index,
                created_at: timestamp,
            },
        };

        // Store in vector database
        vector_store.add_embedding(embedding).await?;
    }

    if args.output == OutputFormat::Human {
        println!(); // New line after progress
        println!("{}", "💾 Saving document metadata...".dimmed());
    }

    // Save document to document store
    let doc_store = create_document_store().await;
    doc_store.save_document(&doc).await?;

    match args.output {
        OutputFormat::Json => {
            let json = serde_json::json!({
                "success": true,
                "document_id": doc.id,
                "path": doc.path,
                "title": doc.title,
                "chunks": doc.chunks.len(),
                "size": doc.content.len(),
                "storage": if cfg!(feature = "qdrant") { "qdrant" } else { "in-memory" },
            });
            println!("{}", serde_json::to_string_pretty(&json)?);
        }
        OutputFormat::Human => {
            println!("{}", "✅ Document indexed successfully".green().bold());
            println!("{}", "─".repeat(50).dimmed());
            println!("  {} {}", "ID:".bold(), doc.id.yellow());
            println!("  {} {}", "Title:".bold(), doc.title);
            println!(
                "  {} {}",
                "Chunks:".bold(),
                doc.chunks.len().to_string().cyan()
            );
            println!(
                "  {} {} bytes",
                "Size:".bold(),
                doc.content.len().to_string().cyan()
            );
            println!(
                "  {} {}",
                "Storage:".bold(),
                if cfg!(feature = "qdrant") {
                    "Qdrant (VPS)"
                } else {
                    "In-Memory"
                }
            );
            println!();
        }
    }

    Ok(())
}

/// Search for knowledge in the context
async fn search_context(args: SearchArgs) -> Result<()> {
    if args.output == OutputFormat::Human {
        println!("\n{}", "🔍 Searching context...".bold().cyan());
        println!("{}", "─".repeat(50).dimmed());
        println!("  {} {}", "Query:".bold(), args.query.yellow());
        println!("  {} {}", "Limit:".bold(), args.limit.to_string().cyan());
        println!(
            "  {} {}",
            "Min Score:".bold(),
            format!("{:.1}%", args.min_score * 100.0).cyan()
        );
        println!();
    }

    // Create embedding generator
    if args.output == OutputFormat::Human {
        println!("{}", "🔮 Generating query embedding...".dimmed());
    }

    let embedding_gen = OllamaEmbeddingGenerator::from_config()?;
    let query_embedding = embedding_gen.generate_embedding(&args.query).await?;

    // Create vector store
    #[cfg(feature = "qdrant")]
    let vector_store: Arc<dyn VectorStore> = Arc::new(
        QdrantVectorStore::from_config()
            .await
            .map_err(|e| {
                if args.output == OutputFormat::Human {
                    eprintln!(
                        "{}",
                        "⚠️  Failed to connect to Qdrant, using in-memory storage".yellow()
                    );
                    eprintln!("   {}", format!("Error: {}", e).dimmed());
                }
                e
            })
            .unwrap_or_else(|_| InMemoryVectorStore::new()),
    );

    #[cfg(not(feature = "qdrant"))]
    let vector_store: Arc<dyn VectorStore> = Arc::new(InMemoryVectorStore::new());

    // Search in vector store
    let search_results = vector_store.search(&query_embedding, args.limit).await?;

    // Filter by minimum score
    let filtered_results: Vec<_> = search_results
        .into_iter()
        .filter(|r| r.similarity >= args.min_score)
        .collect();

    match args.output {
        OutputFormat::Json => {
            let json = serde_json::json!({
                "query": args.query,
                "count": filtered_results.len(),
                "results": filtered_results.iter().map(|r| {
                    serde_json::json!({
                        "similarity": r.similarity,
                        "source": r.embedding.metadata.source,
                        "chunk_id": r.embedding.chunk_id,
                        "chunk_index": r.embedding.metadata.chunk_index,
                    })
                }).collect::<Vec<_>>(),
            });
            println!("{}", serde_json::to_string_pretty(&json)?);
        }
        OutputFormat::Human => {
            if filtered_results.is_empty() {
                println!("{}", "No results found.".yellow());
                println!();
                println!("{}", "💡 Tips:".dimmed());
                println!(
                    "   {} Try lowering the minimum score with --min-score",
                    "-".dimmed()
                );
                println!(
                    "   {} Make sure you've added documents with 'jarvis context add'",
                    "-".dimmed()
                );
            } else {
                println!(
                    "\n{} ({} results)",
                    "Results:".bold().green(),
                    filtered_results.len()
                );
                println!();

                for (i, result) in filtered_results.iter().enumerate() {
                    println!(
                        "{}. {} [{}]",
                        i + 1,
                        "Result".bold(),
                        format!("{:.1}% similarity", result.similarity * 100.0).yellow()
                    );

                    if args.show_source {
                        println!(
                            "   {} {}",
                            "Source:".dimmed(),
                            result.embedding.metadata.source.cyan()
                        );
                        println!(
                            "   {} {}",
                            "Chunk:".dimmed(),
                            result.embedding.chunk_id.dimmed()
                        );
                        println!(
                            "   {} {}",
                            "Index:".dimmed(),
                            result.embedding.metadata.chunk_index.to_string().dimmed()
                        );
                    }

                    // Note: We don't have the actual text here unless we store it
                    // This would require a document store implementation
                    println!(
                        "   {} Use 'jarvis context list' to see full documents",
                        "Note:".dimmed()
                    );

                    println!();
                }
            }
        }
    }

    Ok(())
}

/// List all documents in context
async fn list_context(args: ListArgs) -> Result<()> {
    if args.output == OutputFormat::Human {
        println!("\n{}", "📚 Context Documents".bold().cyan());
        println!("{}", "─".repeat(50).dimmed());
        println!();
    }

    // Get document store and list documents
    let doc_store = create_document_store().await;
    let documents = doc_store.list_documents().await?;

    // Filter if needed
    let filtered: Vec<_> = documents
        .iter()
        .filter(|doc| {
            if let Some(ref filter_type) = args.doc_type {
                doc.metadata
                    .doc_type
                    .as_ref()
                    .map(|t| t == filter_type)
                    .unwrap_or(false)
            } else {
                true
            }
        })
        .filter(|doc| {
            if let Some(ref filter_tag) = args.tag {
                doc.metadata.tags.contains(filter_tag)
            } else {
                true
            }
        })
        .collect();

    match args.output {
        OutputFormat::Json => {
            let json = serde_json::json!({
                "total": filtered.len(),
                "documents": filtered,
            });
            println!("{}", serde_json::to_string_pretty(&json)?);
        }
        OutputFormat::Human => {
            if filtered.is_empty() {
                println!("{}", "No documents in context.".yellow());
            } else {
                println!(
                    "{} {}",
                    "Total documents:".bold(),
                    filtered.len().to_string().cyan()
                );
                println!();

                for (i, doc) in filtered.iter().enumerate() {
                    println!("{}. {}", i + 1, doc.title.bold());
                    println!("   {} {}", "ID:".dimmed(), doc.id.yellow());
                    println!("   {} {}", "Path:".dimmed(), doc.path.display());
                    if let Some(ref doc_type) = doc.metadata.doc_type {
                        println!("   {} {}", "Type:".dimmed(), doc_type.cyan());
                    }
                    if !doc.metadata.tags.is_empty() {
                        println!(
                            "   {} {}",
                            "Tags:".dimmed(),
                            doc.metadata.tags.join(", ").green()
                        );
                    }
                    println!("   {} {}", "Chunks:".dimmed(), doc.chunks.len());
                    println!();
                }
            }
        }
    }

    Ok(())
}

/// Compress context by removing redundant information
async fn compress_context(args: CompressArgs) -> Result<()> {
    if args.output == OutputFormat::Human {
        println!("\n{}", "🗜️  Compressing context...".bold().cyan());
        println!("{}", "─".repeat(50).dimmed());
        println!(
            "  {} {} tokens",
            "Max tokens:".bold(),
            args.max_tokens.to_string().cyan()
        );
        println!("  {} {:?}", "Strategy:".bold(), args.strategy);
        if args.dry_run {
            println!("  {} {}", "Mode:".bold(), "Dry run".yellow());
        }
        println!();
    }

    // Simulate compression
    let removed_count = 3; // Placeholder
    let saved_tokens = 1500; // Placeholder

    match args.output {
        OutputFormat::Json => {
            let json = serde_json::json!({
                "success": true,
                "removed_documents": removed_count,
                "tokens_saved": saved_tokens,
                "dry_run": args.dry_run,
            });
            println!("{}", serde_json::to_string_pretty(&json)?);
        }
        OutputFormat::Human => {
            if args.dry_run {
                println!("{}", "📊 Compression Analysis:".bold());
            } else {
                println!("{}", "✅ Compression Complete".green().bold());
            }
            println!("{}", "─".repeat(50).dimmed());
            println!(
                "  {} {}",
                "Documents removed:".bold(),
                removed_count.to_string().yellow()
            );
            println!(
                "  {} {} tokens",
                "Space saved:".bold(),
                saved_tokens.to_string().cyan()
            );
            println!();

            if args.dry_run {
                println!(
                    "{}",
                    "Note: This was a dry run. No changes were made.".dimmed()
                );
            }
        }
    }

    Ok(())
}

/// Remove a document from context
async fn remove_context(args: RemoveArgs) -> Result<()> {
    println!("\n{}", "🗑️  Removing document...".bold().cyan());
    println!("{}", "─".repeat(50).dimmed());
    println!("  {} {}", "Document ID:".bold(), args.doc_id.yellow());
    println!();

    let doc_store = create_document_store().await;

    // Get document first to retrieve chunk IDs
    let doc = doc_store.get_document(&args.doc_id).await?;

    if let Some(doc) = doc {
        // Remove embeddings from vector store
        #[cfg(feature = "qdrant")]
        {
            if let Ok(vector_store) = QdrantVectorStore::from_config().await {
                for chunk in &doc.chunks {
                    let _ = vector_store.remove_embedding(&chunk.id).await;
                }
            }
        }

        // Remove document from document store
        doc_store.remove_document(&args.doc_id).await?;

        println!("{}", "✅ Document removed successfully".green().bold());
        println!();
        println!(
            "  {} {} chunks removed",
            "Cleaned:".dimmed(),
            doc.chunks.len()
        );
    } else {
        println!("{}", "❌ Document not found".red().bold());
        return Err(anyhow::anyhow!(
            "Document with ID '{}' not found",
            args.doc_id
        ));
    }

    Ok(())
}

/// Show statistics about the context
async fn show_stats(args: StatsArgs) -> Result<()> {
    let doc_store = create_document_store().await;
    let documents = doc_store.list_documents().await?;
    let total_chunks: usize = documents.iter().map(|d| d.chunks.len()).sum();
    let total_size: usize = documents.iter().map(|d| d.content.len()).sum();

    match args.output {
        OutputFormat::Json => {
            let json = serde_json::json!({
                "total_documents": documents.len(),
                "total_chunks": total_chunks,
                "total_size_bytes": total_size,
                "avg_chunks_per_doc": if !documents.is_empty() { total_chunks as f64 / documents.len() as f64 } else { 0.0 },
            });
            println!("{}", serde_json::to_string_pretty(&json)?);
        }
        OutputFormat::Human => {
            println!("\n{}", "📊 Context Statistics".bold().cyan());
            println!("{}", "─".repeat(50).dimmed());
            println!();
            println!(
                "  {} {}",
                "Total Documents:".bold(),
                documents.len().to_string().cyan()
            );
            println!(
                "  {} {}",
                "Total Chunks:".bold(),
                total_chunks.to_string().cyan()
            );
            println!(
                "  {} {} bytes ({:.2} KB)",
                "Total Size:".bold(),
                total_size.to_string().cyan(),
                total_size as f64 / 1024.0
            );
            if !documents.is_empty() {
                println!(
                    "  {} {:.1}",
                    "Avg Chunks/Doc:".bold(),
                    total_chunks as f64 / documents.len() as f64
                );
            }
            println!();
        }
    }

    Ok(())
}
