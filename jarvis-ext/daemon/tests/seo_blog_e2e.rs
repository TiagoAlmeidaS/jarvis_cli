//! End-to-end integration test for the SEO Blog pipeline.
//!
//! Exercises the full pipeline path:
//!   RSS scraper (via wiremock) -> LLM generation (mock client) -> local output
//!
//! Validates:
//! - Pipeline reads config and sources from the database
//! - Real RSS scraper fetches + parses the mock feed
//! - LLM mock returns structured SeoArticle JSON
//! - Pipeline produces correct ContentOutput
//! - Database state is updated (source checked, logs written)

use anyhow::Result;
use async_trait::async_trait;
use pretty_assertions::assert_eq;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;
use wiremock::Mock;
use wiremock::MockServer;
use wiremock::ResponseTemplate;
use wiremock::matchers::method;
use wiremock::matchers::path;

use jarvis_daemon::pipeline::Pipeline;
use jarvis_daemon::pipeline::PipelineContext;
use jarvis_daemon::pipelines::seo_blog::SeoBlogPipeline;
use jarvis_daemon::processor::LlmClient;
use jarvis_daemon_common::ContentType;
use jarvis_daemon_common::CreatePipeline;
use jarvis_daemon_common::CreateSource;
use jarvis_daemon_common::DaemonDb;
use jarvis_daemon_common::LlmResponse;
use jarvis_daemon_common::Platform;
use jarvis_daemon_common::SourceType;
use jarvis_daemon_common::Strategy;

// ---------------------------------------------------------------------------
// Mock LLM client
// ---------------------------------------------------------------------------

/// A fake LLM client that returns a pre-built SeoArticle JSON.
struct MockLlmClient {
    response_json: String,
}

impl MockLlmClient {
    fn new(response_json: String) -> Self {
        Self { response_json }
    }
}

#[async_trait]
impl LlmClient for MockLlmClient {
    async fn generate(&self, _prompt: &str, _system: Option<&str>) -> Result<LlmResponse> {
        Ok(LlmResponse {
            text: self.response_json.clone(),
            model: "mock-model".to_string(),
            tokens_used: 150,
            cost_usd: Some(0.001),
        })
    }
}

// ---------------------------------------------------------------------------
// Test fixtures
// ---------------------------------------------------------------------------

/// Minimal valid RSS 2.0 feed with one item.
fn rss_feed_xml(base_url: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0">
  <channel>
    <title>Test Tech Blog</title>
    <link>{base_url}</link>
    <description>A test feed for E2E</description>
    <item>
      <title>Rust Async Performance Tips</title>
      <link>{base_url}/rust-async-tips</link>
      <description>Learn how to optimize async Rust code for maximum throughput and minimal latency in production systems.</description>
    </item>
  </channel>
</rss>"#
    )
}

/// Valid SeoArticle JSON that the mock LLM returns.
fn seo_article_json() -> String {
    serde_json::json!({
        "title": "Dicas de Performance Async em Rust",
        "meta_description": "Aprenda a otimizar codigo async em Rust para maximo desempenho em producao.",
        "slug": "dicas-performance-async-rust",
        "keywords": ["rust", "async", "performance", "tokio"],
        "content_markdown": "## Introducao\n\nRust e uma linguagem poderosa para programacao assincrona.\n\n## Dicas Praticas\n\n### 1. Use tokio corretamente\n\nEvite blocking calls dentro de tasks async.\n\n### 2. Buffer suas operacoes\n\nUse buffered streams para IO.\n\n## Conclusao\n\nSiga estas dicas para melhorar seu codigo async. Comece agora!",
        "category_suggestion": "Programacao",
        "tags": ["rust", "async", "performance"]
    })
    .to_string()
}

/// Build the pipeline config JSON for seo_blog with an LLM pointing to a
/// custom base URL (the wiremock server is unused by the mock client, but
/// the config must be valid for deserialization).
fn pipeline_config_json() -> serde_json::Value {
    serde_json::json!({
        "llm": {
            "provider": "custom",
            "api_key": "test-key",
            "model": "mock-model"
        },
        "seo": {
            "niche": "Tecnologia",
            "language": "pt-BR",
            "min_word_count": 50,
            "max_word_count": 5000
        },
        "publisher": {
            "platform": "local"
        }
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// Full E2E: RSS source -> LLM generation -> local output.
///
/// Verifies:
/// 1. One ContentOutput is produced with correct title, slug, body (HTML)
/// 2. Platform is Local, content_type is Article
/// 3. Word count is populated
/// 4. Source's `last_checked_at` is updated in the DB
/// 5. Logs are written to the DB
#[tokio::test]
async fn seo_blog_e2e_rss_to_local_output() -> Result<()> {
    // -- 1. Start wiremock to serve the RSS feed --
    let rss_server = MockServer::start().await;
    let feed_xml = rss_feed_xml(&rss_server.uri());

    Mock::given(method("GET"))
        .and(path("/feed.xml"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(&feed_xml)
                .insert_header("content-type", "application/rss+xml"),
        )
        .expect(1..)
        .mount(&rss_server)
        .await;

    // -- 2. Set up in-memory database --
    let db = Arc::new(DaemonDb::open_memory().await?);

    // Create pipeline
    let pipeline = db
        .create_pipeline(&CreatePipeline {
            id: "test-seo-pipeline".to_string(),
            name: "Test SEO Blog".to_string(),
            strategy: Strategy::SeoBlog,
            config_json: pipeline_config_json(),
            schedule_cron: "0 */6 * * *".to_string(),
            max_retries: None,
            retry_delay_sec: None,
        })
        .await?;

    // Create an RSS source pointing at the wiremock server
    let _source = db
        .create_source(&CreateSource {
            pipeline_id: pipeline.id.clone(),
            source_type: SourceType::Rss,
            name: "Test RSS Feed".to_string(),
            url: format!("{}/feed.xml", rss_server.uri()),
            scrape_selector: None,
            check_interval_sec: Some(0), // always due
        })
        .await?;

    // Create a job (the pipeline needs a valid job in its context)
    let job = db.create_job(&pipeline.id).await?;

    // Reload sources for the context
    let sources = db.get_sources_due_for_check(&pipeline.id).await?;
    assert!(!sources.is_empty(), "source should be due for checking");

    // -- 3. Build PipelineContext with mock LLM --
    let mock_llm = Arc::new(MockLlmClient::new(seo_article_json()));

    let ctx = PipelineContext {
        job,
        pipeline: pipeline.clone(),
        sources,
        llm_client: mock_llm,
        db: db.clone(),
        cancellation_token: CancellationToken::new(),
    };

    // -- 4. Execute the pipeline --
    let outputs = SeoBlogPipeline.execute(&ctx).await?;

    // -- 5. Assertions on output --
    assert_eq!(outputs.len(), 1, "should produce exactly one article");

    let article = &outputs[0];
    assert_eq!(article.title, "Dicas de Performance Async em Rust");
    assert_eq!(article.slug, "dicas-performance-async-rust");
    assert_eq!(article.content_type, ContentType::Article);
    assert_eq!(article.platform, Platform::Local);
    assert!(article.url.is_none(), "local platform should not have URL");
    assert!(
        article.word_count.unwrap_or(0) > 0,
        "word count should be populated"
    );

    // Body should be HTML (converted from markdown)
    assert!(
        article.body.contains("<h2>"),
        "body should contain HTML headings"
    );
    assert!(
        article.body.contains("<p>"),
        "body should contain HTML paragraphs"
    );

    // LLM model should match config
    assert_eq!(article.llm_model, "mock-model");

    // -- 6. Verify DB state --
    // Source should have been marked as checked (last_checked_at != NULL)
    let updated_sources = db.list_sources(&pipeline.id).await?;
    assert_eq!(updated_sources.len(), 1);
    assert!(
        updated_sources[0].last_checked_at.is_some(),
        "source last_checked_at should be set after checking"
    );
    assert!(
        updated_sources[0].last_content_hash.is_some(),
        "source last_content_hash should be set after checking"
    );

    // Logs should have been written
    let logs = db
        .list_logs(&jarvis_daemon_common::LogFilter {
            pipeline_id: Some(pipeline.id.clone()),
            ..Default::default()
        })
        .await?;
    assert!(
        !logs.is_empty(),
        "pipeline should write info logs during execution"
    );

    // At least one log should mention "completed"
    let has_completion_log = logs
        .iter()
        .any(|log| log.message.to_lowercase().contains("completed"));
    assert!(
        has_completion_log,
        "should have a completion log entry, got: {:?}",
        logs.iter().map(|l| &l.message).collect::<Vec<_>>()
    );

    Ok(())
}

/// E2E with empty RSS feed — pipeline should produce zero outputs.
#[tokio::test]
async fn seo_blog_e2e_empty_feed_no_output() -> Result<()> {
    let rss_server = MockServer::start().await;

    // Empty feed (no <item> elements)
    let empty_feed = r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0">
  <channel>
    <title>Empty Blog</title>
    <link>https://example.com</link>
    <description>Nothing here</description>
  </channel>
</rss>"#;

    Mock::given(method("GET"))
        .and(path("/feed.xml"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(empty_feed)
                .insert_header("content-type", "application/rss+xml"),
        )
        .expect(1..)
        .mount(&rss_server)
        .await;

    let db = Arc::new(DaemonDb::open_memory().await?);

    let pipeline = db
        .create_pipeline(&CreatePipeline {
            id: "test-empty-feed".to_string(),
            name: "Empty Feed Test".to_string(),
            strategy: Strategy::SeoBlog,
            config_json: pipeline_config_json(),
            schedule_cron: "0 * * * *".to_string(),
            max_retries: None,
            retry_delay_sec: None,
        })
        .await?;

    let _source = db
        .create_source(&CreateSource {
            pipeline_id: pipeline.id.clone(),
            source_type: SourceType::Rss,
            name: "Empty Feed".to_string(),
            url: format!("{}/feed.xml", rss_server.uri()),
            scrape_selector: None,
            check_interval_sec: Some(0),
        })
        .await?;

    let job = db.create_job(&pipeline.id).await?;
    let sources = db.get_sources_due_for_check(&pipeline.id).await?;

    let mock_llm = Arc::new(MockLlmClient::new(seo_article_json()));

    let ctx = PipelineContext {
        job,
        pipeline,
        sources,
        llm_client: mock_llm,
        db: db.clone(),
        cancellation_token: CancellationToken::new(),
    };

    let outputs = SeoBlogPipeline.execute(&ctx).await?;
    assert_eq!(outputs.len(), 0, "empty feed should produce no articles");

    Ok(())
}

/// E2E deduplication: running the pipeline twice on the same unchanged feed
/// should produce zero outputs on the second run because `check_for_updates`
/// detects the feed hash hasn't changed.
#[tokio::test]
async fn seo_blog_e2e_deduplication() -> Result<()> {
    let rss_server = MockServer::start().await;
    let feed_xml = rss_feed_xml(&rss_server.uri());

    Mock::given(method("GET"))
        .and(path("/feed.xml"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(&feed_xml)
                .insert_header("content-type", "application/rss+xml"),
        )
        .mount(&rss_server)
        .await;

    let db = Arc::new(DaemonDb::open_memory().await?);

    let pipeline = db
        .create_pipeline(&CreatePipeline {
            id: "test-dedup".to_string(),
            name: "Dedup Test".to_string(),
            strategy: Strategy::SeoBlog,
            config_json: pipeline_config_json(),
            schedule_cron: "0 * * * *".to_string(),
            max_retries: None,
            retry_delay_sec: None,
        })
        .await?;

    let _source = db
        .create_source(&CreateSource {
            pipeline_id: pipeline.id.clone(),
            source_type: SourceType::Rss,
            name: "Dedup Feed".to_string(),
            url: format!("{}/feed.xml", rss_server.uri()),
            scrape_selector: None,
            check_interval_sec: Some(0),
        })
        .await?;

    let mock_llm = Arc::new(MockLlmClient::new(seo_article_json()));

    // --- First run: should produce 1 article ---
    let job1 = db.create_job(&pipeline.id).await?;
    let sources1 = db.get_sources_due_for_check(&pipeline.id).await?;
    let ctx1 = PipelineContext {
        job: job1,
        pipeline: pipeline.clone(),
        sources: sources1,
        llm_client: mock_llm.clone(),
        db: db.clone(),
        cancellation_token: CancellationToken::new(),
    };

    let outputs1 = SeoBlogPipeline.execute(&ctx1).await?;
    assert_eq!(outputs1.len(), 1, "first run should produce 1 article");

    // After the first run, the source's last_content_hash is updated with the
    // feed's hash. A second run with the same feed content will see the hash
    // hasn't changed and skip the source entirely.

    // --- Second run: same unchanged feed -> 0 outputs ---
    // The source has check_interval_sec=0, so it's always "due" time-wise,
    // but check_for_updates will return None because the feed hash matches.
    let job2 = db.create_job(&pipeline.id).await?;
    let sources2 = db.get_sources_due_for_check(&pipeline.id).await?;

    let ctx2 = PipelineContext {
        job: job2,
        pipeline: pipeline.clone(),
        sources: sources2,
        llm_client: mock_llm,
        db: db.clone(),
        cancellation_token: CancellationToken::new(),
    };

    let outputs2 = SeoBlogPipeline.execute(&ctx2).await?;
    assert_eq!(
        outputs2.len(),
        0,
        "second run with unchanged feed should produce 0 articles"
    );

    Ok(())
}

/// E2E cancellation: pipeline should stop early when the cancellation
/// token is triggered.
#[tokio::test]
async fn seo_blog_e2e_cancellation() -> Result<()> {
    let rss_server = MockServer::start().await;
    let feed_xml = rss_feed_xml(&rss_server.uri());

    Mock::given(method("GET"))
        .and(path("/feed.xml"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(&feed_xml)
                .insert_header("content-type", "application/rss+xml"),
        )
        .mount(&rss_server)
        .await;

    let db = Arc::new(DaemonDb::open_memory().await?);

    let pipeline = db
        .create_pipeline(&CreatePipeline {
            id: "test-cancel".to_string(),
            name: "Cancel Test".to_string(),
            strategy: Strategy::SeoBlog,
            config_json: pipeline_config_json(),
            schedule_cron: "0 * * * *".to_string(),
            max_retries: None,
            retry_delay_sec: None,
        })
        .await?;

    let _source = db
        .create_source(&CreateSource {
            pipeline_id: pipeline.id.clone(),
            source_type: SourceType::Rss,
            name: "Cancel Feed".to_string(),
            url: format!("{}/feed.xml", rss_server.uri()),
            scrape_selector: None,
            check_interval_sec: Some(0),
        })
        .await?;

    let job = db.create_job(&pipeline.id).await?;
    let sources = db.get_sources_due_for_check(&pipeline.id).await?;
    let mock_llm = Arc::new(MockLlmClient::new(seo_article_json()));

    let cancel_token = CancellationToken::new();
    // Cancel immediately before execution
    cancel_token.cancel();

    let ctx = PipelineContext {
        job,
        pipeline,
        sources,
        llm_client: mock_llm,
        db: db.clone(),
        cancellation_token: cancel_token,
    };

    let outputs = SeoBlogPipeline.execute(&ctx).await?;
    assert_eq!(
        outputs.len(),
        0,
        "cancelled pipeline should produce no articles"
    );

    Ok(())
}
