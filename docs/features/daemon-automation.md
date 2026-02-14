# Daemon Automation - Ativos Digitais Automatizados

**Status**: 🚧 Em Progresso  
**Prioridade**: 🔴 Alta  
**Versão**: 1.0.0  
**Última atualização**: 2026-02-13

---

## 1. Visão Geral

O **Jarvis Daemon** é um sistema de automação background que transforma o Jarvis CLI em um **motor de produção de ativos digitais**. Ele executa pipelines autônomos (sem interação humana) que geram conteúdo, publicam em plataformas e monetizam via tráfego orgânico.

O daemon opera como um binário separado (`jarvis-daemon`) que:
- Executa pipelines em schedule (cron-like)
- Persiste estado em SQLite (compartilhado com o CLI)
- Integra com LLMs via API OpenAI-compatible (OpenRouter, Gemini, OpenAI, Ollama, etc.)
- Publica automaticamente em plataformas (YouTube, WordPress, Ghost, TikTok)
- Pode ser controlado/monitorado pelo CLI via `jarvis daemon <command>`

### Modelo Mental

```
┌──────────────────────────────────────────────────────────────────┐
│                       JARVIS ECOSYSTEM                           │
│                                                                  │
│  Interativo (humano)           Autônomo (daemon)                │
│  ┌──────────┐ ┌─────────┐     ┌──────────────────────────────┐  │
│  │ jarvis   │ │ jarvis  │     │       jarvis-daemon          │  │
│  │ (TUI)    │ │ daemon  │     │                              │  │
│  │          │ │ status  │     │  ┌─────────┐  ┌───────────┐  │  │
│  │ Chat     │ │ logs    │     │  │Scheduler│  │ Pipeline  │  │  │
│  │ interati-│ │ run     │     │  │ (cron)  │──│ Runner    │  │  │
│  │ vo com   │ │ metrics │     │  └─────────┘  └─────┬─────┘  │  │
│  │ LLM      │ │         │     │                     │        │  │
│  └────┬─────┘ └────┬────┘     │  ┌──────────────────▼──────┐ │  │
│       │            │          │  │     Pipeline Registry   │ │  │
│       │            │          │  │                         │ │  │
│       │            │          │  │ ┌─────┐ ┌────┐ ┌─────┐ │ │  │
│       │            │          │  │ │ SEO │ │ YT │ │SaaS │ │ │  │
│       │            │          │  │ └──┬──┘ └──┬─┘ └──┬──┘ │ │  │
│       │            │          │  └────┼───────┼──────┼────┘ │  │
│       │            │          └───────┼───────┼──────┼──────┘  │
│       │            │                  │       │      │         │
│       └────────────┴──────────────────┴───────┴──────┘         │
│                           │                                     │
│                    ┌──────▼──────┐                               │
│                    │   SQLite    │                               │
│                    │  (shared)   │                               │
│                    │             │                               │
│                    │ pipelines   │                               │
│                    │ jobs        │                               │
│                    │ content     │                               │
│                    │ sources     │                               │
│                    │ metrics     │                               │
│                    └─────────────┘                               │
└──────────────────────────────────────────────────────────────────┘
```

---

## 2. Motivação

### O Problema
Vender serviços B2B exige prospecção, negociação e interação humana constante. Para um perfil Builder/Hacker, o modelo ideal é **construir a máquina e deixar rodando**.

### A Solução
Usar o Jarvis como um **produtor de conteúdo autônomo** ou **agregador de dados** que gera receita passiva via:
- Monetização nativa de plataformas (AdSense, Programa de Criadores)
- Marketing de afiliados automatizado
- APIs self-serve com cobrança por uso

### Por que Rust?
- Daemon leve que roda 24/7 sem consumir recursos excessivos
- `tokio` fornece async runtime ideal para I/O-bound tasks (HTTP, LLM calls)
- SQLite via `sqlx` já presente no projeto
- Compilação única, deploy simples (um binário)

---

## 3. Estratégias de Monetização

### 3.1 Estratégia 1: Canais Dark Automatizados (YouTube Shorts / TikTok)

**Pipeline: `youtube-shorts`**

```
┌─────────────┐    ┌──────────────┐    ┌───────────────┐    ┌──────────────┐    ┌─────────────┐
│  1. Scrape   │───▶│ 2. Transcreve│───▶│ 3. Gemini     │───▶│ 4. FFmpeg    │───▶│ 5. Publish  │
│  Podcasts CC │    │  + Segmenta  │    │  Traduz PT-BR │    │  Burn subs   │    │  YT + TikTok│
└─────────────┘    └──────────────┘    └───────────────┘    └──────────────┘    └─────────────┘
```

**Detalhamento:**
1. **Scraping**: Monitora RSS/API de podcasts com licença Creative Commons ou vídeos virais longos
2. **Transcrição**: Whisper (local via Ollama) ou API para transcrever áudio
3. **Curadoria IA**: Gemini Flash identifica os 3 momentos mais retentivos + traduz legendas para PT-BR
4. **Renderização**: FFmpeg queima legendas estilo MrBeast (fonte grande, colorida, centralizada)
5. **Publicação**: YouTube Data API v3 + TikTok API para upload com título/tags gerados pela IA

**Receita**: AdSense do YouTube Shorts + Programa de Criadores TikTok + links de afiliados no comentário fixado

### 3.2 Estratégia 2: Programmatic SEO (Blogs Gerados por IA) — **PRIMEIRO A IMPLEMENTAR**

**Pipeline: `seo-blog`**

```
┌─────────────┐    ┌──────────────┐    ┌───────────────┐    ┌──────────────┐
│  1. Scrape   │───▶│ 2. Gemini    │───▶│ 3. Formata    │───▶│ 4. Publica   │
│  Fontes      │    │  Analisa +   │    │  HTML/MD +    │    │  WordPress/  │
│  Públicas    │    │  Escreve SEO │    │  Imagens      │    │  Ghost API   │
└─────────────┘    └──────────────┘    └───────────────┘    └──────────────┘
```

**Detalhamento:**
1. **Coleta**: Web scrapers para fontes de dados públicas (editais de concursos, atualizações de frameworks, CVEs, etc.)
2. **Geração**: Gemini Flash analisa o conteúdo bruto e escreve artigo otimizado para SEO com headings, keywords, meta description
3. **Formatação**: Gera HTML/Markdown com formatação rica, imagens placeholder, schema.org markup
4. **Publicação**: REST API do WordPress ou Ghost para publicar com scheduling

**Receita**: Google AdSense + infoprodutos compilados (apostilas PDF) via Gumroad/Stripe

### 3.3 Estratégia 3: APIs Self-Serve de Nicho (Micro-SaaS)

**Pipeline: `saas-api`**

```
┌─────────────┐    ┌──────────────┐    ┌───────────────┐
│  HTTP Request│───▶│ Gemini Flash │───▶│ JSON Response │
│  + API Key   │    │  Processa    │    │  Estruturado  │
└─────────────┘    └──────────────┘    └───────────────┘
```

**Detalhamento:**
1. Landing page simples (HTML estático) com integração Stripe Checkout
2. Servidor Rust (axum) que valida API key, repassa para Gemini com prompt otimizado
3. Retorna dados estruturados (limpeza de JSON, extração de dados de logs, etc.)

**Receita**: Assinatura mensal ($9/mês) ou pay-per-use

---

## 4. Arquitetura Técnica

### 4.1 Novos Crates

```
jarvis-rs/
├── daemon/                    # Binário jarvis-daemon
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs            # Entry point: args + tokio runtime
│       ├── scheduler.rs       # Cron-like scheduler (tokio::time)
│       ├── runner.rs          # Executa pipelines (orquestrador)
│       ├── publisher/         # Adaptadores de publicação
│       │   ├── mod.rs
│       │   ├── wordpress.rs   # WordPress REST API client
│       │   ├── ghost.rs       # Ghost Content API client
│       │   ├── youtube.rs     # YouTube Data API v3 client
│       │   └── tiktok.rs      # TikTok API client
│       ├── scraper/           # Adaptadores de scraping
│       │   ├── mod.rs
│       │   ├── rss.rs         # RSS/Atom feed parser
│       │   ├── web.rs         # Generic web scraper (reqwest + scraper)
│       │   └── pdf.rs         # PDF text extraction
│       ├── processor/         # Processamento via LLM
│       │   ├── mod.rs
│       │   ├── seo_writer.rs  # Prompt para artigos SEO
│       │   ├── video_curator.rs # Prompt para curadoria de vídeo
│       │   └── data_cleaner.rs  # Prompt para limpeza de dados
│       └── pipelines/         # Pipelines concretos
│           ├── mod.rs
│           ├── seo_blog.rs    # Strategy 2: SEO blog
│           ├── youtube_shorts.rs # Strategy 1: YT Shorts
│           └── saas_api.rs    # Strategy 3: Micro-SaaS
│
├── daemon-common/             # Tipos compartilhados CLI <-> Daemon
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── models.rs          # DaemonPipeline, DaemonJob, DaemonContent, etc.
│       ├── db.rs              # Queries SQLite (CRUD para todas as tabelas)
│       └── schedule.rs        # Parser de cron expressions
```

### 4.2 Schema SQLite (extensão do jarvis-state)

```sql
-- ============================================================================
-- DAEMON AUTOMATION TABLES
-- ============================================================================

-- Pipelines registrados (templates de automação)
CREATE TABLE IF NOT EXISTS daemon_pipelines (
    id              TEXT PRIMARY KEY,                -- ex: "seo-concursos-ti"
    name            TEXT NOT NULL,                   -- ex: "SEO Concursos de TI"
    strategy        TEXT NOT NULL CHECK(strategy IN ('seo_blog', 'youtube_shorts', 'saas_api')),
    config_json     TEXT NOT NULL DEFAULT '{}',      -- config específica do pipeline
    schedule_cron   TEXT NOT NULL DEFAULT '0 3 * * *', -- cron expression
    enabled         INTEGER NOT NULL DEFAULT 1,
    max_retries     INTEGER NOT NULL DEFAULT 3,
    retry_delay_sec INTEGER NOT NULL DEFAULT 300,    -- 5 minutos entre retries
    created_at      INTEGER NOT NULL,                -- unix timestamp
    updated_at      INTEGER NOT NULL
);

-- Jobs (cada execução individual de um pipeline)
CREATE TABLE IF NOT EXISTS daemon_jobs (
    id              TEXT PRIMARY KEY,                -- UUID
    pipeline_id     TEXT NOT NULL REFERENCES daemon_pipelines(id),
    status          TEXT NOT NULL DEFAULT 'pending'
                    CHECK(status IN ('pending', 'running', 'completed', 'failed', 'cancelled')),
    attempt         INTEGER NOT NULL DEFAULT 1,
    started_at      INTEGER,                         -- unix timestamp
    completed_at    INTEGER,
    input_json      TEXT,                            -- dados de entrada
    output_json     TEXT,                            -- resultado
    error_message   TEXT,
    error_stack     TEXT,
    duration_ms     INTEGER,                         -- tempo de execução
    created_at      INTEGER NOT NULL
);

-- Conteúdo gerado (rastreabilidade + métricas)
CREATE TABLE IF NOT EXISTS daemon_content (
    id              TEXT PRIMARY KEY,                -- UUID
    job_id          TEXT NOT NULL REFERENCES daemon_jobs(id),
    pipeline_id     TEXT NOT NULL REFERENCES daemon_pipelines(id),
    content_type    TEXT NOT NULL CHECK(content_type IN ('article', 'video_short', 'api_response', 'pdf', 'image')),
    platform        TEXT NOT NULL CHECK(platform IN ('wordpress', 'ghost', 'youtube', 'tiktok', 'gumroad', 'stripe', 'local')),
    title           TEXT,
    slug            TEXT,                            -- URL-friendly identifier
    url             TEXT,                            -- URL final publicado
    status          TEXT NOT NULL DEFAULT 'draft'
                    CHECK(status IN ('draft', 'rendering', 'uploading', 'published', 'failed', 'archived')),
    word_count      INTEGER,
    llm_model       TEXT,                            -- modelo usado (ex: "gemini-2.0-flash")
    llm_tokens_used INTEGER,                         -- tokens consumidos
    llm_cost_usd    REAL,                            -- custo estimado em USD
    content_hash    TEXT,                            -- SHA-256 do conteúdo para dedup
    created_at      INTEGER NOT NULL,
    published_at    INTEGER
);

-- Fontes monitoradas (RSS feeds, URLs, PDFs)
CREATE TABLE IF NOT EXISTS daemon_sources (
    id              TEXT PRIMARY KEY,                -- UUID
    pipeline_id     TEXT NOT NULL REFERENCES daemon_pipelines(id),
    source_type     TEXT NOT NULL CHECK(source_type IN ('rss', 'webpage', 'api', 'pdf_url', 'youtube_channel')),
    name            TEXT NOT NULL,                   -- ex: "PCI Concursos - TI"
    url             TEXT NOT NULL,
    scrape_selector TEXT,                            -- CSS selector para web scraping
    last_checked_at INTEGER,
    last_content_hash TEXT,                          -- detectar mudanças (SHA-256)
    check_interval_sec INTEGER NOT NULL DEFAULT 86400, -- 24h default
    enabled         INTEGER NOT NULL DEFAULT 1,
    created_at      INTEGER NOT NULL,
    updated_at      INTEGER NOT NULL
);

-- Métricas de performance (revenue tracking)
CREATE TABLE IF NOT EXISTS daemon_metrics (
    id              TEXT PRIMARY KEY,                -- UUID
    content_id      TEXT REFERENCES daemon_content(id),
    pipeline_id     TEXT NOT NULL REFERENCES daemon_pipelines(id),
    metric_type     TEXT NOT NULL CHECK(metric_type IN ('views', 'clicks', 'revenue', 'impressions', 'subscribers', 'ctr')),
    value           REAL NOT NULL,
    currency        TEXT DEFAULT 'USD',
    period_start    INTEGER NOT NULL,                -- unix timestamp
    period_end      INTEGER NOT NULL,
    source          TEXT NOT NULL,                   -- 'adsense', 'youtube_analytics', 'stripe', 'gumroad'
    created_at      INTEGER NOT NULL
);

-- Log de execução detalhado (para debug via CLI)
CREATE TABLE IF NOT EXISTS daemon_logs (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    job_id          TEXT REFERENCES daemon_jobs(id),
    pipeline_id     TEXT NOT NULL,
    level           TEXT NOT NULL CHECK(level IN ('trace', 'debug', 'info', 'warn', 'error')),
    message         TEXT NOT NULL,
    context_json    TEXT,                            -- metadata adicional
    created_at      INTEGER NOT NULL
);

-- Índices para queries frequentes
CREATE INDEX IF NOT EXISTS idx_jobs_pipeline_status ON daemon_jobs(pipeline_id, status);
CREATE INDEX IF NOT EXISTS idx_jobs_created ON daemon_jobs(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_content_pipeline ON daemon_content(pipeline_id, status);
CREATE INDEX IF NOT EXISTS idx_content_published ON daemon_content(published_at DESC);
CREATE INDEX IF NOT EXISTS idx_sources_pipeline ON daemon_sources(pipeline_id, enabled);
CREATE INDEX IF NOT EXISTS idx_sources_next_check ON daemon_sources(last_checked_at);
CREATE INDEX IF NOT EXISTS idx_metrics_pipeline_type ON daemon_metrics(pipeline_id, metric_type, period_start);
CREATE INDEX IF NOT EXISTS idx_logs_job ON daemon_logs(job_id, created_at);
CREATE INDEX IF NOT EXISTS idx_logs_pipeline ON daemon_logs(pipeline_id, created_at DESC);
```

### 4.3 Traits e Abstrações Principais

```rust
/// Representa um pipeline de automação executável pelo daemon.
#[async_trait]
pub trait Pipeline: Send + Sync {
    /// Identificador único do tipo de pipeline (ex: "seo_blog")
    fn strategy(&self) -> &str;

    /// Nome legível para logs
    fn display_name(&self) -> &str;

    /// Valida a configuração do pipeline antes de executar
    async fn validate_config(&self, config: &serde_json::Value) -> Result<()>;

    /// Executa o pipeline completo para um job.
    /// Retorna uma lista de conteúdos gerados.
    async fn execute(&self, ctx: &PipelineContext) -> Result<Vec<ContentOutput>>;

    /// Cancela a execução em andamento (graceful shutdown)
    async fn cancel(&self) -> Result<()>;
}

/// Contexto passado para cada execução de pipeline.
pub struct PipelineContext {
    pub job: DaemonJob,
    pub pipeline: DaemonPipeline,
    pub sources: Vec<DaemonSource>,
    pub llm_client: Arc<dyn LlmClient>,
    pub db: Arc<DaemonDb>,
    pub logger: PipelineLogger,
    pub cancellation_token: CancellationToken,
}

/// Output padronizado de cada pipeline step.
pub struct ContentOutput {
    pub content_type: ContentType,
    pub platform: Platform,
    pub title: String,
    pub slug: String,
    pub body: String,            // HTML, Markdown, ou path do arquivo
    pub url: Option<String>,     // preenchido após publicação
    pub word_count: Option<i32>,
    pub llm_model: String,
    pub llm_tokens_used: i64,
}

/// Adaptador de publicação (WordPress, Ghost, YouTube, etc.)
#[async_trait]
pub trait Publisher: Send + Sync {
    fn platform(&self) -> Platform;
    async fn publish(&self, content: &ContentOutput) -> Result<PublishResult>;
    async fn check_status(&self, external_id: &str) -> Result<PublishStatus>;
}

/// Adaptador de scraping (RSS, Web, PDF, etc.)
#[async_trait]
pub trait Scraper: Send + Sync {
    fn source_type(&self) -> SourceType;
    async fn check_for_updates(&self, source: &DaemonSource) -> Result<Option<ScrapedContent>>;
    async fn scrape(&self, source: &DaemonSource) -> Result<Vec<ScrapedContent>>;
}

/// Client LLM genérico (reutiliza jarvis-api internamente)
#[async_trait]
pub trait LlmClient: Send + Sync {
    async fn generate(&self, prompt: &str, system: Option<&str>) -> Result<LlmResponse>;
    async fn generate_structured<T: DeserializeOwned>(&self, prompt: &str, system: Option<&str>) -> Result<T>;
}
```

### 4.4 Scheduler (Cron-like)

O scheduler é um loop `tokio::time` que:
1. A cada minuto, lê `daemon_pipelines` onde `enabled = 1`
2. Para cada pipeline, calcula se está na hora de executar (baseado em `schedule_cron`)
3. Se sim, cria um `DaemonJob` com status `pending` e enfileira
4. O runner pega jobs pendentes e executa em paralelo (com limite de concorrência configurável)

```rust
pub struct Scheduler {
    db: Arc<DaemonDb>,
    runner: Arc<PipelineRunner>,
    tick_interval: Duration,     // 60 segundos
    max_concurrent_jobs: usize,  // 3 por padrão
}

impl Scheduler {
    pub async fn run(&self, shutdown: CancellationToken) -> Result<()> {
        let mut interval = tokio::time::interval(self.tick_interval);
        loop {
            tokio::select! {
                _ = interval.tick() => {
                    self.tick().await?;
                }
                _ = shutdown.cancelled() => {
                    tracing::info!("Scheduler shutting down gracefully");
                    break;
                }
            }
        }
        Ok(())
    }
}
```

### 4.5 Integração CLI

Novo subcomando `jarvis daemon` no CLI existente:

```
jarvis daemon                          # Mostra status geral
jarvis daemon start                    # Inicia o daemon (foreground ou background)
jarvis daemon stop                     # Para o daemon (via PID file)
jarvis daemon status                   # Status dos pipelines e jobs ativos

jarvis daemon pipeline list            # Lista pipelines registrados
jarvis daemon pipeline add <config>    # Adiciona novo pipeline via JSON/TOML
jarvis daemon pipeline enable <id>     # Ativa pipeline
jarvis daemon pipeline disable <id>    # Desativa pipeline
jarvis daemon pipeline run <id>        # Executa imediatamente (bypass schedule)
jarvis daemon pipeline config <id>     # Mostra config do pipeline

jarvis daemon jobs [--pipeline <id>]   # Lista jobs recentes
jarvis daemon jobs <job-id>            # Detalhes de um job
jarvis daemon logs [--pipeline <id>]   # Logs de execução
jarvis daemon logs --tail              # Logs em tempo real

jarvis daemon content [--last 7d]      # Conteúdo publicado
jarvis daemon metrics [--pipeline <id>] # Revenue, views, etc.

jarvis daemon source list [--pipeline <id>]   # Lista fontes monitoradas
jarvis daemon source add <pipeline-id> <url>  # Adiciona nova fonte
jarvis daemon source check <source-id>        # Força verificação
```

---

## 5. Pipeline SEO Blog — Implementação Detalhada (Strategy 2)

Este é o primeiro pipeline a ser implementado por ter a menor complexidade técnica.

### 5.1 Fluxo Completo

```
                          ┌─────────────────┐
                          │  daemon_sources  │
                          │                  │
                          │ RSS: pci.com.br  │
                          │ Web: gov.br/...  │
                          │ RSS: dotnet blog │
                          └────────┬────────┘
                                   │
                          ┌────────▼────────┐
                          │  Step 1: Scrape  │
                          │                  │
                          │ - Fetch RSS/HTML │
                          │ - Compare hash   │
                          │ - Extract text   │
                          │ - Detect new     │
                          └────────┬────────┘
                                   │ (novo conteúdo detectado)
                          ┌────────▼────────┐
                          │  Step 2: LLM     │
                          │                  │
                          │ System: "Você é  │
                          │  um redator SEO" │
                          │                  │
                          │ Prompt:          │
                          │ - Dados brutos   │
                          │ - Nicho alvo     │
                          │ - Keywords       │
                          │ - Tom de voz     │
                          │                  │
                          │ Output:          │
                          │ - Título SEO     │
                          │ - Meta desc      │
                          │ - H1, H2, H3     │
                          │ - Corpo artigo   │
                          │ - Tags           │
                          └────────┬────────┘
                                   │
                          ┌────────▼────────┐
                          │  Step 3: Format  │
                          │                  │
                          │ - MD -> HTML     │
                          │ - Schema.org     │
                          │ - OG tags        │
                          │ - Image placehld │
                          │ - Internal links │
                          └────────┬────────┘
                                   │
                          ┌────────▼────────┐
                          │  Step 4: Publish │
                          │                  │
                          │ WordPress REST:  │
                          │ POST /wp-json/   │
                          │  wp/v2/posts     │
                          │                  │
                          │ - title          │
                          │ - content (HTML) │
                          │ - status: publish│
                          │ - categories     │
                          │ - tags           │
                          │ - meta (SEO)     │
                          └────────┬────────┘
                                   │
                          ┌────────▼────────┐
                          │  Step 5: Record  │
                          │                  │
                          │ daemon_content:  │
                          │ - url do post    │
                          │ - word_count     │
                          │ - tokens usados  │
                          │ - custo LLM      │
                          └─────────────────┘
```

### 5.2 Configuração do Pipeline SEO

```toml
# Exemplo de config para pipeline SEO de concursos de TI
[pipeline]
id = "seo-concursos-ti"
name = "SEO - Concursos de TI"
strategy = "seo_blog"
schedule_cron = "0 3 * * *"  # 3h da manhã, todo dia

# Opção A: OpenRouter com modelo gratuito (recomendado para começar)
[pipeline.llm]
provider = "openrouter"         # Env var: OPENROUTER_API_KEY
model = "openrouter/free"       # Router gratuito automático
max_tokens = 4096
temperature = 0.7

# Opção B: OpenRouter com modelo barato e previsível
# [pipeline.llm]
# provider = "openrouter"
# model = "qwen/qwen3-coder-next"  # ~$0.07/M input, $0.30/M output
# max_tokens = 4096

# Opção C: Google AI Studio (free tier, precisa de GOOGLE_API_KEY)
# [pipeline.llm]
# provider = "google"
# model = "gemini-2.0-flash"

# Opção D: Ollama local (zero custo, precisa de GPU ou VPS)
# [pipeline.llm]
# provider = "ollama"
# model = "llama3.2"
# base_url = "http://localhost:11434/v1"  # ou IP da VPS

[pipeline.seo]
niche = "Concursos Públicos de TI"
target_audience = "Desenvolvedores e profissionais de TI"
language = "pt-BR"
tone = "informativo, profissional, acessível"
min_word_count = 800
max_word_count = 2000
keywords_per_article = 5

[pipeline.publisher]
platform = "wordpress"
base_url = "https://seu-blog.com"
username = "admin"
# password via env var: WORDPRESS_APP_PASSWORD
category_ids = [1, 5]         # Categorias no WordPress
default_status = "publish"    # "draft" para revisão manual

[pipeline.affiliate]
enabled = true
amazon_tag = "seu-tag-20"
hotmart_links = [
    { keyword = "apostila", url = "https://hotmart.com/..." },
]

[[pipeline.sources]]
type = "rss"
name = "PCI Concursos - TI"
url = "https://www.pciconcursos.com.br/rss/concursos-ti.xml"
check_interval_sec = 43200    # 12 horas

[[pipeline.sources]]
type = "webpage"
name = "CESPE/Cebraspe - Editais"
url = "https://www.cebraspe.org.br/concursos"
scrape_selector = ".concurso-item"
check_interval_sec = 86400    # 24 horas

[[pipeline.sources]]
type = "rss"
name = ".NET Blog"
url = "https://devblogs.microsoft.com/dotnet/feed/"
check_interval_sec = 21600    # 6 horas
```

### 5.3 Prompts LLM (SEO Writer)

**System Prompt:**
```
Você é um redator SEO especializado em {niche}. Seu público-alvo é {target_audience}.

Regras:
1. Escreva em {language} com tom {tone}
2. Use headings (H2, H3) para estruturar o conteúdo
3. Inclua keyword principal no título, primeiro parágrafo e pelo menos 2 headings
4. Meta description com até 160 caracteres
5. Parágrafos curtos (máximo 3 linhas)
6. Inclua listas quando apropriado
7. Termine com CTA (Call to Action)
8. Mínimo {min_word_count} palavras, máximo {max_word_count} palavras

Formato de saída (JSON):
{
  "title": "Título SEO otimizado",
  "meta_description": "Descrição até 160 chars",
  "slug": "url-friendly-slug",
  "keywords": ["keyword1", "keyword2"],
  "content_markdown": "# Heading\n\nConteúdo...",
  "category_suggestion": "categoria",
  "tags": ["tag1", "tag2"]
}
```

---

## 6. Dependências Externas

### 6.1 Crates Rust necessários (novos)

| Crate | Versão | Uso |
|-------|--------|-----|
| `cron` | latest | Parsing de cron expressions |
| `scraper` | latest | HTML parsing + CSS selectors |
| `feed-rs` | latest | RSS/Atom feed parsing |
| `pdf-extract` | latest | Extração de texto de PDFs |
| `tokio-cron-scheduler` | latest | Scheduling alternativo (avaliação) |
| `sha2` | workspace | Content hashing (já no workspace) |
| `axum` | workspace | HTTP server para Strategy 3 (já no workspace) |

### 6.2 APIs Externas

| API | Uso | Auth |
|-----|-----|------|
| WordPress REST API | Publicação de artigos | Application Passwords |
| Ghost Content API | Alternativa ao WordPress | Admin API Key |
| OpenRouter | Agregador de modelos (free/paid) | API Key |
| Google AI Studio | Gemini Flash (free tier) | API Key |
| YouTube Data API v3 | Upload de vídeos | OAuth 2.0 |
| TikTok API | Upload de vídeos | OAuth 2.0 |
| Stripe API | Cobrança Micro-SaaS | API Key |
| Gumroad API | Venda de infoprodutos | API Key |

### 6.3 Modelos LLM Recomendados (OpenRouter)

O daemon usa qualquer API OpenAI-compatible. Para o pipeline SEO, recomendamos o **OpenRouter** por ter os menores preços e não precisar de múltiplas contas.

**Default do projeto**: `mistralai/mistral-nemo` (12B, multilingual, $0.02-0.04/M tokens)

#### Tier 1 — Ultra-barato (< $0.001 por artigo)

| Modelo | Params | Input/1M | Output/1M | ~Custo/artigo | ~900 artigos/mês | JSON |
|--------|--------|----------|-----------|---------------|------------------|------|
| `meta-llama/llama-3.2-3b-instruct` | 3B | $0.02 | $0.02 | $0.00008 | **$0.07** | Sim |
| `mistralai/mistral-nemo` | 12B | $0.02 | $0.04 | $0.00013 | **$0.12** | Sim |
| `meta-llama/llama-3.1-8b-instruct` | 8B | $0.02 | $0.05 | $0.000155 | **$0.14** | Sim |
| `nousresearch/deephermes-3-mistral-24b-preview` | 24B | $0.02 | $0.10 | $0.00028 | **$0.25** | Sim |

#### Tier 2 — Muito barato, mais inteligente (< $0.005 por artigo)

| Modelo | Params | Input/1M | Output/1M | ~Custo/artigo | ~900 artigos/mês | JSON |
|--------|--------|----------|-----------|---------------|------------------|------|
| `z-ai/glm-4.7-flash` | 30B | $0.06 | $0.40 | $0.0011 | **$0.98** | Sim |
| `qwen/qwen3-coder-next` | 80B MoE | $0.07 | $0.30 | $0.00086 | **$0.77** | Sim |
| `bytedance-seed/seed-1.6-flash` | — | $0.075 | $0.30 | $0.00086 | **$0.78** | Sim |
| `stepfun/step-3.5-flash` | 196B MoE | $0.10 | $0.30 | $0.0009 | **$0.81** | Sim |

> **Nota**: Estimativa de custo/artigo baseada em ~1500 tokens input + ~2500 tokens output.
> Todos os modelos listados suportam `response_format` (structured JSON output).

#### Recomendação por caso de uso

- **Começar com custo zero**: `openrouter/free` (router gratuito, qualidade variável)
- **Melhor custo-benefício**: `mistralai/mistral-nemo` (12B, multilingual, $0.12/mês)
- **Melhor qualidade barata**: `nousresearch/deephermes-3-mistral-24b-preview` (24B, reasoning, $0.25/mês)
- **Máxima qualidade acessível**: `qwen/qwen3-coder-next` (80B MoE, $0.77/mês)

### 6.4 Ferramentas do Sistema

| Ferramenta | Uso | Obrigatório |
|------------|-----|-------------|
| FFmpeg | Renderização de vídeo (Strategy 1) | Apenas Strategy 1 |
| yt-dlp | Download de vídeos (Strategy 1) | Apenas Strategy 1 |

---

## 7. Configuração (extensão do config.toml)

```toml
# Seção daemon no config.toml existente
[daemon]
enabled = true
data_dir = "~/.jarvis/daemon"         # onde ficam os arquivos de trabalho
log_level = "info"
max_concurrent_jobs = 3
pid_file = "~/.jarvis/daemon.pid"

[daemon.defaults]
llm_provider = "google"               # provider padrão para daemons
llm_model = "gemini-2.0-flash"        # modelo padrão (free tier)
retry_max = 3
retry_delay_sec = 300

[daemon.notifications]
# Notificar via Telegram quando job completa/falha
telegram_enabled = true
telegram_chat_id = "your_chat_id"
```

---

## 8. Roadmap de Implementação

### Fase 1: Fundação (Semana 1-2) — **ATUAL**

| # | Tarefa | Crate | Status |
|---|--------|-------|--------|
| 1 | Criar `jarvis-daemon-common` com models + DB | daemon-common | 🚧 |
| 2 | Migrations SQLite no `jarvis-state` | state | 📝 |
| 3 | Criar `jarvis-daemon` com scheduler + runner | daemon | 📝 |
| 4 | Trait `Pipeline` + trait `Publisher` + trait `Scraper` | daemon | 📝 |
| 5 | Implementar RSS scraper | daemon | 📝 |
| 6 | Implementar Web scraper básico | daemon | 📝 |
| 7 | Implementar LLM client (wrapper sobre jarvis-api) | daemon | 📝 |
| 8 | Subcomando `jarvis daemon` no CLI | cli | 📝 |

### Fase 2: Pipeline SEO Blog (Semana 3-4)

| # | Tarefa | Crate |
|---|--------|-------|
| 1 | Pipeline `seo_blog` completo | daemon |
| 2 | WordPress publisher | daemon |
| 3 | Ghost publisher (alternativo) | daemon |
| 4 | Prompts SEO otimizados + testes | daemon |
| 5 | Deduplicação de conteúdo (content_hash) | daemon-common |
| 6 | Sistema de logging estruturado (daemon_logs) | daemon-common |
| 7 | `jarvis daemon content` + `jarvis daemon logs` | cli |

### Fase 3: Pipeline YouTube Shorts (Semana 5-7)

| # | Tarefa | Crate |
|---|--------|-------|
| 1 | Pipeline `youtube_shorts` | daemon |
| 2 | Integração FFmpeg via `tokio::process` | daemon |
| 3 | YouTube Data API v3 publisher | daemon |
| 4 | TikTok API publisher | daemon |
| 5 | Whisper transcrição (via Ollama) | daemon |
| 6 | Prompts de curadoria de vídeo | daemon |

### Fase 4: Micro-SaaS API (Semana 8-9)

| # | Tarefa | Crate |
|---|--------|-------|
| 1 | HTTP server (axum) para API pública | daemon |
| 2 | Stripe integration (checkout + webhooks) | daemon |
| 3 | API key management | daemon |
| 4 | Rate limiting per-user | daemon |
| 5 | Landing page estática | daemon |

### Fase 5: Métricas & Otimização (Semana 10+)

| # | Tarefa | Crate |
|---|--------|-------|
| 1 | Google Analytics / AdSense API | daemon |
| 2 | YouTube Analytics API | daemon |
| 3 | Dashboard de métricas via CLI (`jarvis daemon metrics`) | cli |
| 4 | A/B testing de títulos SEO | daemon |
| 5 | Auto-otimização de prompts baseado em performance | daemon |

---

## 9. Considerações de Segurança

1. **API Keys**: Todas via variáveis de ambiente ou `jarvis-secrets`, nunca em config.toml
2. **Rate Limiting**: Respeitar limites de API (YouTube: 10k units/dia, WordPress: sem limite hard)
3. **Conteúdo**: Não plagiar — usar LLM para reescrever, nunca copiar verbatim
4. **Licenças**: Só usar fontes Creative Commons ou dados públicos
5. **Deduplicação**: Hash de conteúdo para evitar republicação
6. **Graceful Shutdown**: Jobs em andamento completam antes do daemon parar
7. **PID File**: Garantir single-instance do daemon

## 10. Métricas de Sucesso

| Métrica | Meta Mensal (6 meses) |
|---------|----------------------|
| Artigos publicados | 90-120 (3-4/dia) |
| Tráfego orgânico | 10k+ pageviews |
| Revenue AdSense | $50-200 |
| Revenue afiliados | $100-500 |
| Custo LLM (OpenRouter free / Gemini Flash) | $0-5 |
| Uptime daemon | >99% |

---

**Status Atual**: Fases 1-3 implementadas. Fase 3 adicionou o feedback loop (observacao + analise + aprovacao).

**Implementado**:
- Fase 1: Fundacao (database, scheduler, runner, CLI)
- Fase 2: Pipeline SEO Blog (scraper, LLM processor, publisher)
- Fase 3: Feedback Loop — ver `docs/features/daemon-feedback-loop.md`
  - Tabelas `daemon_proposals` e `daemon_revenue`
  - Pipeline `metrics_collector` (coleta metricas e estima revenue)
  - Pipeline `strategy_analyzer` (LLM analisa e propoe acoes)
  - CLI: `jarvis daemon proposals` (list/show/approve/reject)
  - CLI: `jarvis daemon revenue` (summary/list)
  - Workflow de aprovacao com auto-approve para low-risk

**Proximo passo**: Integrar fontes de dados reais:
1. Google Search Console API para metricas reais (clicks, impressoes, CTR)
2. AdSense API para revenue real
3. WordPress stats para pageviews por artigo
4. Implementar execucao automatica de propostas aprovadas no scheduler

**Providers LLM suportados**: OpenRouter (recomendado), Google AI Studio, OpenAI, Ollama, Databricks, Custom.
