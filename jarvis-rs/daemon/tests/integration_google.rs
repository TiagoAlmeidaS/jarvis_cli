//! Integration tests for the daemon with Google (Gemini) as the only LLM provider.
//!
//! Validates:
//! - Daemon starts with only GOOGLE_API_KEY (no OpenRouter).
//! - Pipeline with provider "google" and model "gemini-2.0-flash" runs successfully
//!   when the LLM endpoint is mocked (WireMock).

use anyhow::Result;
use pretty_assertions::assert_eq;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::AsyncBufReadExt;
use tokio::process::Command as TokioCommand;
use tokio::time::timeout;
use tokio_util::sync::CancellationToken;
use wiremock::Mock;
use wiremock::MockServer;
use wiremock::ResponseTemplate;
use wiremock::matchers::method;
use wiremock::matchers::path;

use jarvis_daemon::pipeline::Pipeline;
use jarvis_daemon::pipeline::PipelineContext;
use jarvis_daemon::pipelines::seo_blog::SeoBlogPipeline;
use jarvis_daemon::processor::router::LlmRouter;
use jarvis_daemon_common::ContentType;
use jarvis_daemon_common::CreatePipeline;
use jarvis_daemon_common::CreateSource;
use jarvis_daemon_common::DaemonDb;
use jarvis_daemon_common::Platform;
use jarvis_daemon_common::SourceType;
use jarvis_daemon_common::Strategy;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn rss_feed_xml(base_url: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0">
  <channel>
    <title>Test Tech Blog</title>
    <link>{base_url}</link>
    <description>E2E Google flow</description>
    <item>
      <title>Rust Async Tips</title>
      <link>{base_url}/rust-async</link>
      <description>Optimize async Rust code for production.</description>
    </item>
  </channel>
</rss>"#
    )
}

fn seo_article_json() -> String {
    serde_json::json!({
        "title": "Dicas Async em Rust",
        "meta_description": "Otimize codigo async em Rust.",
        "slug": "dicas-async-rust",
        "keywords": ["rust", "async"],
        "content_markdown": "## Intro\n\nRust async.\n\n## Dicas\n\nUse tokio.\n\n## Fim",
        "category_suggestion": "Programacao",
        "tags": ["rust", "async"]
    })
    .to_string()
}

/// OpenAI-compatible response body for WireMock (Gemini-style endpoint).
fn openai_compatible_response(content: &str) -> String {
    serde_json::json!({
        "choices": [
            {
                "message": {
                    "role": "assistant",
                    "content": content
                },
                "finish_reason": "stop"
            }
        ],
        "usage": {
            "prompt_tokens": 10,
            "completion_tokens": 20,
            "total_tokens": 30
        }
    })
    .to_string()
}

// ---------------------------------------------------------------------------
// Test: Daemon starts with only GOOGLE_API_KEY
// ---------------------------------------------------------------------------

/// Daemon binary starts and logs "Jarvis Daemon started" when only GOOGLE_API_KEY is set.
/// OPENROUTER_API_KEY is unset so the flow does not depend on OpenRouter.
#[tokio::test]
async fn daemon_starts_with_google_api_key_only() -> Result<()> {
    let bin = assert_cmd::cargo::cargo_bin!("jarvis-daemon");

    let temp = tempfile::tempdir()?;
    let db_path = temp.path().join("daemon.db");

    let mut child = TokioCommand::new(&bin)
        .arg("run")
        .env("GOOGLE_API_KEY", "test-key-for-integration-test")
        .env_remove("OPENROUTER_API_KEY")
        .env("JARVIS_DAEMON_DB", db_path.as_os_str())
        .env("RUST_LOG", "jarvis_daemon=info")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()?;

    let stderr = child.stderr.take().expect("stderr piped");
    let mut reader = tokio::io::BufReader::new(stderr);

    let found = timeout(Duration::from_secs(8), async {
        let mut line = String::new();
        loop {
            line.clear();
            let n = reader.read_line(&mut line).await.ok().unwrap_or(0);
            if n == 0 {
                tokio::time::sleep(Duration::from_millis(50)).await;
                continue;
            }
            if line.contains("Jarvis Daemon started") {
                return true;
            }
        }
    })
    .await
    .unwrap_or(false);

    let _ = child.start_kill();
    let _ = timeout(Duration::from_secs(2), child.wait()).await;

    assert!(
        found,
        "Expected daemon to log 'Jarvis Daemon started' within 8s when GOOGLE_API_KEY is set"
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// Test: Pipeline with provider Google runs against WireMock Gemini endpoint
// ---------------------------------------------------------------------------

/// Registers a pipeline with provider "google" and model "gemini-2.0-flash",
/// points the client at WireMock, and runs the SEO blog pipeline. Asserts
/// that the LLM was called and the pipeline produced content.
#[tokio::test]
async fn pipeline_google_executes_against_mock_gemini() -> Result<()> {
    let mock = MockServer::start().await;
    let gemini_path = "/v1beta/openai/chat/completions";
    let response_body = openai_compatible_response(&seo_article_json());

    Mock::given(method("POST"))
        .and(path(gemini_path))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(&response_body)
                .insert_header("content-type", "application/json"),
        )
        .expect(1..)
        .mount(&mock)
        .await;

    let base_url = format!("{}/v1beta/openai", mock.uri());

    let db = Arc::new(DaemonDb::open_memory().await?);

    let pipeline_config = serde_json::json!({
        "llm": {
            "provider": "google",
            "model": "gemini-2.0-flash",
            "base_url": base_url,
            "api_key": "test-key"
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
    });

    let pipeline = db
        .create_pipeline(&CreatePipeline {
            id: "google-mock-pipeline".to_string(),
            name: "Google Mock Pipeline".to_string(),
            strategy: Strategy::SeoBlog,
            config_json: pipeline_config,
            schedule_cron: "0 */6 * * *".to_string(),
            max_retries: None,
            retry_delay_sec: None,
        })
        .await?;

    let rss_feed = rss_feed_xml(&mock.uri());
    Mock::given(method("GET"))
        .and(path("/feed.xml"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(&rss_feed)
                .insert_header("content-type", "application/rss+xml"),
        )
        .expect(1..)
        .mount(&mock)
        .await;

    let _source = db
        .create_source(&CreateSource {
            pipeline_id: pipeline.id.clone(),
            source_type: SourceType::Rss,
            name: "Test RSS".to_string(),
            url: format!("{}/feed.xml", mock.uri()),
            scrape_selector: None,
            check_interval_sec: Some(0),
        })
        .await?;

    let job = db.create_job(&pipeline.id).await?;
    let sources = db.get_sources_due_for_check(&pipeline.id).await?;
    assert!(!sources.is_empty(), "sources should be due for check");

    let llm_client = LlmRouter::from_pipeline_config(&pipeline).await?;

    let ctx = PipelineContext {
        job,
        pipeline: pipeline.clone(),
        sources,
        llm_client,
        db: db.clone(),
        cancellation_token: CancellationToken::new(),
    };

    let outputs = SeoBlogPipeline.execute(&ctx).await?;

    assert_eq!(outputs.len(), 1, "pipeline should produce one article");
    let article = &outputs[0];
    assert_eq!(article.title, "Dicas Async em Rust");
    assert_eq!(article.slug, "dicas-async-rust");
    assert_eq!(article.content_type, ContentType::Article);
    assert_eq!(article.platform, Platform::Local);
    assert!(
        article.word_count.unwrap_or(0) > 0,
        "word count should be set"
    );
    assert!(
        article.llm_model == "gemini-2.0-flash",
        "LLM model should be gemini-2.0-flash"
    );

    Ok(())
}
