//! SEO Blog pipeline — Strategy 2: Programmatic SEO.
//!
//! Flow:
//! 1. Check sources for new content (RSS feeds, web pages)
//! 2. For each new item, generate an SEO-optimized article via LLM
//! 3. Format the article (Markdown -> HTML)
//! 4. Publish to WordPress/Ghost
//! 5. Record in database

use anyhow::Result;
use async_trait::async_trait;
use jarvis_daemon_common::{ContentOutput, ContentType, LogLevel, Platform};
use serde::{Deserialize, Serialize};

use crate::pipeline::{Pipeline, PipelineContext};
use crate::processor;
use crate::publisher;
use crate::scraper;

/// SEO Blog pipeline implementation.
pub struct SeoBlogPipeline;

/// Configuration specific to the SEO blog pipeline.
#[derive(Debug, Clone, Deserialize)]
pub struct SeoBlogConfig {
    /// LLM configuration (provider, model, etc.) — uses the generic LlmConfig.
    #[serde(default)]
    pub llm: crate::processor::LlmConfig,
    /// SEO-specific parameters.
    #[serde(default)]
    pub seo: SeoSection,
    /// Publisher configuration.
    #[serde(default)]
    pub publisher: PublisherSection,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SeoSection {
    #[serde(default = "default_niche")]
    pub niche: String,
    #[serde(default = "default_audience")]
    pub target_audience: String,
    #[serde(default = "default_language")]
    pub language: String,
    #[serde(default = "default_tone")]
    pub tone: String,
    #[serde(default = "default_min_words")]
    pub min_word_count: u32,
    #[serde(default = "default_max_words")]
    pub max_word_count: u32,
}

impl Default for SeoSection {
    fn default() -> Self {
        Self {
            niche: default_niche(),
            target_audience: default_audience(),
            language: default_language(),
            tone: default_tone(),
            min_word_count: default_min_words(),
            max_word_count: default_max_words(),
        }
    }
}

fn default_niche() -> String {
    "Tecnologia".to_string()
}
fn default_audience() -> String {
    "Desenvolvedores e profissionais de TI".to_string()
}
fn default_language() -> String {
    "pt-BR".to_string()
}
fn default_tone() -> String {
    "informativo, profissional, acessível".to_string()
}
fn default_min_words() -> u32 {
    800
}
fn default_max_words() -> u32 {
    2000
}

#[derive(Debug, Clone, Deserialize)]
pub struct PublisherSection {
    #[serde(default = "default_platform")]
    pub platform: String,
    #[serde(default)]
    pub base_url: String,
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub password: Option<String>,
    #[serde(default)]
    pub category_ids: Vec<i64>,
    #[serde(default = "default_publish_status")]
    pub default_status: String,
}

impl Default for PublisherSection {
    fn default() -> Self {
        Self {
            platform: default_platform(),
            base_url: String::new(),
            username: String::new(),
            password: None,
            category_ids: Vec::new(),
            default_status: default_publish_status(),
        }
    }
}

fn default_platform() -> String {
    "local".to_string()
}
fn default_publish_status() -> String {
    "draft".to_string()
}

/// Structured response expected from the LLM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeoArticle {
    pub title: String,
    pub meta_description: String,
    pub slug: String,
    pub keywords: Vec<String>,
    pub content_markdown: String,
    pub category_suggestion: String,
    pub tags: Vec<String>,
}

#[async_trait]
impl Pipeline for SeoBlogPipeline {
    fn strategy(&self) -> &str {
        "seo_blog"
    }

    fn display_name(&self) -> &str {
        "SEO Blog Generator"
    }

    async fn validate_config(&self, config: &serde_json::Value) -> Result<()> {
        let _: SeoBlogConfig = serde_json::from_value(config.clone())?;
        Ok(())
    }

    async fn execute(&self, ctx: &PipelineContext) -> Result<Vec<ContentOutput>> {
        let config: SeoBlogConfig = ctx.pipeline.config()?;
        let mut all_outputs = Vec::new();

        ctx.log_info(&format!(
            "Starting SEO blog pipeline for niche: {}",
            config.seo.niche
        ))
        .await;

        // Step 1: Check sources for new content.
        let due_sources = ctx.db.get_sources_due_for_check(&ctx.pipeline.id).await?;

        if due_sources.is_empty() {
            ctx.log_info("No sources due for checking").await;
            return Ok(all_outputs);
        }

        ctx.log_info(&format!("{} sources due for checking", due_sources.len()))
            .await;

        for source in &due_sources {
            // Check cancellation.
            if ctx.cancellation_token.is_cancelled() {
                ctx.log_info("Pipeline cancelled").await;
                break;
            }

            ctx.log_info(&format!(
                "Checking source: {} ({})",
                source.name, source.url
            ))
            .await;

            // Get the appropriate scraper.
            let scraper_impl = match scraper::get_scraper(&source.source_type) {
                Some(s) => s,
                None => {
                    ctx.log(
                        LogLevel::Warn,
                        &format!("No scraper for source type: {}", source.source_type),
                    )
                    .await;
                    continue;
                }
            };

            // Check for updates first.
            let new_hash = match scraper_impl.check_for_updates(source).await {
                Ok(Some(hash)) => hash,
                Ok(None) => {
                    ctx.log_info(&format!("Source '{}' unchanged, skipping", source.name))
                        .await;
                    continue;
                }
                Err(e) => {
                    ctx.log_error(&format!("Failed to check source '{}': {e}", source.name))
                        .await;
                    continue;
                }
            };

            // Scrape new content.
            let scraped_items = match scraper_impl.scrape(source).await {
                Ok(items) => items,
                Err(e) => {
                    ctx.log_error(&format!("Failed to scrape '{}': {e}", source.name))
                        .await;
                    continue;
                }
            };

            ctx.log_info(&format!(
                "Found {} items from source '{}'",
                scraped_items.len(),
                source.name
            ))
            .await;

            // Step 2: Generate SEO articles for each scraped item.
            for item in &scraped_items {
                if ctx.cancellation_token.is_cancelled() {
                    break;
                }

                // Check deduplication.
                if ctx.db.content_exists_by_hash(&item.content_hash).await? {
                    ctx.log_info(&format!(
                        "Content '{}' already processed, skipping",
                        item.title
                    ))
                    .await;
                    continue;
                }

                ctx.log_info(&format!("Generating article for: {}", item.title))
                    .await;

                // Build the LLM prompt.
                let system_prompt = build_system_prompt(&config.seo);
                let user_prompt = build_user_prompt(&item.title, &item.body, &item.url);

                // Call LLM.
                let article: SeoArticle = match processor::generate_structured(
                    ctx.llm_client.as_ref(),
                    &user_prompt,
                    Some(&system_prompt),
                )
                .await
                {
                    Ok(a) => a,
                    Err(e) => {
                        ctx.log_error(&format!("LLM generation failed for '{}': {e}", item.title))
                            .await;
                        continue;
                    }
                };

                // Step 3: Convert Markdown to HTML.
                let html_body = markdown_to_html(&article.content_markdown);
                let word_count = count_words(&article.content_markdown);

                // Determine platform.
                let platform = match config.publisher.platform.as_str() {
                    "wordpress" => Platform::Wordpress,
                    "ghost" => Platform::Ghost,
                    _ => Platform::Local,
                };

                let output = ContentOutput {
                    content_type: ContentType::Article,
                    platform,
                    title: article.title,
                    slug: article.slug,
                    body: html_body,
                    url: None, // filled after publishing
                    word_count: Some(word_count as i32),
                    llm_model: config.llm.effective_model(),
                    llm_tokens_used: 0, // TODO: track from LLM response
                    llm_cost_usd: None,
                };

                // Step 4: Publish if platform is configured.
                if platform != Platform::Local && !config.publisher.base_url.is_empty() {
                    let config_value: serde_json::Value = ctx.pipeline.config()?;
                    match publisher::get_publisher(platform, &config_value) {
                        Ok(pub_impl) => match pub_impl.publish(&output).await {
                            Ok(result) => {
                                ctx.log_info(&format!(
                                    "Published to {}: {}",
                                    result.platform, result.external_url
                                ))
                                .await;
                                // Update output with published URL.
                                let mut published_output = output;
                                published_output.url = Some(result.external_url);
                                all_outputs.push(published_output);
                                continue;
                            }
                            Err(e) => {
                                ctx.log_error(&format!("Publish failed: {e}")).await;
                            }
                        },
                        Err(e) => {
                            ctx.log_error(&format!("Publisher init failed: {e}")).await;
                        }
                    }
                }

                all_outputs.push(output);
            }

            // Update source check timestamp.
            if let Err(e) = ctx.db.update_source_checked(&source.id, &new_hash).await {
                ctx.log_error(&format!(
                    "Failed to update source '{}' check time: {e}",
                    source.name
                ))
                .await;
            }
        }

        ctx.log_info(&format!(
            "Pipeline completed: {} articles generated",
            all_outputs.len()
        ))
        .await;

        Ok(all_outputs)
    }
}

/// Build the system prompt for the SEO writer LLM.
fn build_system_prompt(seo: &SeoSection) -> String {
    let json_example = r#"{
  "title": "Título SEO otimizado",
  "meta_description": "Descrição até 160 chars",
  "slug": "url-friendly-slug",
  "keywords": ["keyword1", "keyword2", "keyword3"],
  "content_markdown": "Conteúdo completo em Markdown com headings H2 e H3...",
  "category_suggestion": "categoria",
  "tags": ["tag1", "tag2", "tag3"]
}"#;

    format!(
        "Você é um redator SEO especializado em {niche}. Seu público-alvo é {audience}.\n\
         \n\
         Regras:\n\
         1. Escreva em {lang} com tom {tone}\n\
         2. Use headings H2 e H3 para estruturar o conteúdo\n\
         3. Inclua a keyword principal no título, primeiro parágrafo e pelo menos 2 headings\n\
         4. Meta description com até 160 caracteres\n\
         5. Parágrafos curtos (máximo 3 linhas)\n\
         6. Inclua listas quando apropriado\n\
         7. Termine com um CTA (Call to Action)\n\
         8. Mínimo {min_words} palavras, máximo {max_words} palavras\n\
         \n\
         IMPORTANTE: Responda APENAS com JSON válido no formato abaixo, sem nenhum texto adicional:\n\
         \n\
         {json_example}",
        niche = seo.niche,
        audience = seo.target_audience,
        lang = seo.language,
        tone = seo.tone,
        min_words = seo.min_word_count,
        max_words = seo.max_word_count,
    )
}

/// Build the user prompt with scraped data.
fn build_user_prompt(title: &str, body: &str, source_url: &str) -> String {
    // Truncate body to avoid token limits.
    let truncated_body = if body.len() > 3000 {
        format!("{}...", &body[..3000])
    } else {
        body.to_string()
    };

    format!(
        r#"Escreva um artigo SEO completo baseado na seguinte informação:

**Fonte**: {source_url}
**Título original**: {title}
**Conteúdo da fonte**:
{truncated_body}

Gere o artigo no formato JSON especificado."#
    )
}

/// Convert Markdown to basic HTML.
fn markdown_to_html(markdown: &str) -> String {
    // Basic Markdown -> HTML conversion.
    // In production, use pulldown-cmark or similar.
    let mut html = String::new();

    for line in markdown.lines() {
        let trimmed = line.trim();

        if let Some(heading) = trimmed.strip_prefix("### ") {
            html.push_str(&format!("<h3>{heading}</h3>\n"));
        } else if let Some(heading) = trimmed.strip_prefix("## ") {
            html.push_str(&format!("<h2>{heading}</h2>\n"));
        } else if let Some(heading) = trimmed.strip_prefix("# ") {
            html.push_str(&format!("<h1>{heading}</h1>\n"));
        } else if let Some(item) = trimmed
            .strip_prefix("- ")
            .or_else(|| trimmed.strip_prefix("* "))
        {
            html.push_str(&format!("<li>{item}</li>\n"));
        } else if trimmed.is_empty() {
            html.push('\n');
        } else {
            html.push_str(&format!("<p>{trimmed}</p>\n"));
        }
    }

    html
}

/// Count words in text.
fn count_words(text: &str) -> usize {
    text.split_whitespace().count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_build_system_prompt() {
        let seo = SeoSection::default();
        let prompt = build_system_prompt(&seo);
        assert!(prompt.contains("Tecnologia"));
        assert!(prompt.contains("pt-BR"));
        assert!(prompt.contains("800"));
    }

    #[test]
    fn test_build_user_prompt() {
        let prompt = build_user_prompt("Test Title", "Some body content", "https://example.com");
        assert!(prompt.contains("Test Title"));
        assert!(prompt.contains("example.com"));
    }

    #[test]
    fn test_markdown_to_html() {
        let md = "## Heading\n\nSome paragraph.\n\n- Item 1\n- Item 2";
        let html = markdown_to_html(md);
        assert!(html.contains("<h2>Heading</h2>"));
        assert!(html.contains("<p>Some paragraph.</p>"));
        assert!(html.contains("<li>Item 1</li>"));
    }

    #[test]
    fn test_count_words() {
        assert_eq!(count_words("hello world foo bar"), 4);
        assert_eq!(count_words(""), 0);
        assert_eq!(count_words("single"), 1);
    }

    #[test]
    fn test_truncate_body_in_prompt() {
        let long_body = "a".repeat(5000);
        let prompt = build_user_prompt("Title", &long_body, "https://x.com");
        // Should contain truncated version.
        assert!(prompt.contains("..."));
        assert!(prompt.len() < 5000);
    }

    #[test]
    fn test_seo_config_deserialize() {
        let json = serde_json::json!({
            "llm": {"model": "gemini-2.0-flash"},
            "seo": {"niche": "Concursos de TI", "language": "pt-BR"},
            "publisher": {"platform": "wordpress", "base_url": "https://blog.com"}
        });

        let config: SeoBlogConfig = serde_json::from_value(json).expect("deserialize");
        assert_eq!(config.seo.niche, "Concursos de TI");
        assert_eq!(config.publisher.platform, "wordpress");
        assert_eq!(config.llm.effective_model(), "gemini-2.0-flash");
    }
}
