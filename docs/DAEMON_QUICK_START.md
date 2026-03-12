# Jarvis Daemon - Quick Start

Deploy and run the autonomous content generation engine.

## What the Daemon Does

The daemon runs in background, executing 5 pipelines on cron schedules:

| Pipeline          | Strategy            | What it does                                                            |
| ----------------- | ------------------- | ----------------------------------------------------------------------- |
| SEO Blog          | `seo_blog`          | Scrapes sources, generates SEO articles via LLM, publishes to WordPress |
| Metrics Collector | `metrics_collector` | Collects revenue/traffic metrics from WordPress, GSC, AdSense, GA4      |
| Strategy Analyzer | `strategy_analyzer` | Analyzes metrics, generates optimization proposals                      |
| A/B Tester        | `ab_tester`         | Tests alternative SEO titles, measures CTR improvement                  |
| Prompt Optimizer  | `prompt_optimizer`  | Optimizes LLM prompt parameters based on content performance            |

## Prerequisites

**Minimum (daemon starts and generates content):**

- 1 LLM API key: `OPENROUTER_API_KEY` (cheapest) or `GOOGLE_API_KEY` (free tier)

If you hit **OpenRouter credit limits** and cannot run the daemon continuously, see [Alternativas para rodar o daemon](architecture/daemon-deploy-alternatives.md) (local models, free tiers, other providers).

**For publishing (the daemon actually earns revenue):**

- WordPress site with Application Password enabled
- `DAEMON_WP_BASE_URL`, `DAEMON_WP_USERNAME`, `DAEMON_WP_APP_PASSWORD`

**For notifications (optional, silently skipped if not set):**

- Telegram bot: `JARVIS_TELEGRAM_BOT_TOKEN`, `JARVIS_TELEGRAM_CHAT_ID`

**For analytics (optional, enriches metrics):**

- Google OAuth2: `GOOGLE_CLIENT_ID`, `GOOGLE_CLIENT_SECRET`

## Option A: Docker Compose (Recommended)

### 1. Configure environment

```bash
cp .env.example .env
# Edit .env with your credentials (minimum: OPENROUTER_API_KEY)
```

### 2. Start the daemon

```bash
# Standalone (daemon only)
docker compose -f docker-compose.daemon.yml up -d

# Home server: daemon only, OpenRouter low-cost LLM
docker compose -f docker-compose.homeserver.yml up -d

# With full infra (Qdrant, Redis, Ollama, Postgres)
docker compose -f docker-compose.vps.yml up -d
```

For **home server** deploy with OpenRouter (low-cost LLM), see [Deploy no servidor em casa](deploy-servidor-casa.md).  
For **Google only (free tier)**, see the one-page checklist: [Runbook: daemon somente com Google](RUNBOOK-DAEMON-GOOGLE.md).

**Pipeline de exemplo só Google:** use [docs/examples/daemon-pipeline-google.json](examples/daemon-pipeline-google.json) ou `jarvis-rs/daemon/examples/pipeline-google-free-tier.json`. Adicione com: `jarvis daemon pipeline add docs/examples/daemon-pipeline-google.json` e depois `jarvis daemon pipeline enable seo-google-free`.

### 3. Check status

```bash
docker logs -f jarvis-daemon
```

### 4. Run integration tests (daemon + Google)

From the repo root (or `jarvis-rs`):

```bash
cd jarvis-rs
cargo test -p jarvis-daemon --test integration_google
```

This runs: (1) daemon startup with only `GOOGLE_API_KEY`; (2) pipeline execution with provider Google against a mocked Gemini endpoint. No real API key needed for the pipeline test. See [RUNBOOK-DAEMON-GOOGLE.md](RUNBOOK-DAEMON-GOOGLE.md) § 9 and [issues/daemon-integration-tests-google.md](issues/daemon-integration-tests-google.md).

To validate that approved proposals are executed (Strategy Analyzer → Executor → action): run `cargo test -p jarvis-daemon --test proposal_executor_e2e` or follow [RUNBOOK-PROPOSAL-EXECUTOR.md](RUNBOOK-PROPOSAL-EXECUTOR.md).

## Option B: Run Locally (No Docker)

### 1. Build

```bash
cd jarvis-rs
cargo build --release --bin jarvis-daemon
```

### 2. Configure

```bash
cp .env.example .env
# Edit .env with your credentials
```

### 3. Run

```bash
# Load env vars and run
source .env  # Linux/Mac
./target/release/jarvis-daemon run

# Or on Windows:
# Set env vars manually, then:
.\target\release\jarvis-daemon.exe run
```

### CLI flags

```
jarvis-daemon run [OPTIONS]
  --max-concurrent <N>       Max concurrent pipeline jobs (default: 3)
  --tick-interval-sec <N>    Scheduler tick interval in seconds (default: 60)
  --db-path <PATH>           SQLite database path (default: ~/.jarvis/daemon.db)
```

## Creating Pipelines

The daemon starts but does nothing until you create pipelines. Use the Jarvis CLI:

### Quick: Create an SEO Blog pipeline

**Google free tier:** use the ready-made pipeline [docs/examples/daemon-pipeline-google.json](examples/daemon-pipeline-google.json) (provider: `google`, model: `gemini-2.0-flash`). Run: `jarvis daemon pipeline add docs/examples/daemon-pipeline-google.json` then `jarvis daemon pipeline enable seo-google-free`.

**OpenRouter (or other provider):** create a file `seo-pipeline.json`:

```json
{
  "id": "seo-blog-tech",
  "name": "SEO Blog - Technology",
  "strategy": "seo_blog",
  "schedule_cron": "0 */4 * * *",
  "max_retries": 3,
  "retry_delay_sec": 300,
  "llm": {
    "provider": "openrouter",
    "model": "mistralai/mistral-nemo",
    "temperature": 0.8,
    "max_tokens": 4000
  },
  "content": {
    "language": "pt-BR",
    "niche": "Tecnologia",
    "audience": "Desenvolvedores e profissionais de TI",
    "tone": "informativo",
    "min_words": 1200,
    "max_words": 2500
  },
  "wordpress": {
    "base_url": "https://your-blog.com",
    "username": "admin",
    "status": "draft"
  }
}
```

Then add it:

```bash
# Using the jarvis CLI (not jarvis-daemon)
jarvis daemon pipeline add seo-pipeline.json

# List pipelines
jarvis daemon pipeline list

# Enable it
jarvis daemon pipeline enable seo-blog-tech
```

### Create a Metrics Collector pipeline

```json
{
  "id": "metrics-daily",
  "name": "Daily Metrics Collection",
  "strategy": "metrics_collector",
  "schedule_cron": "0 6 * * *",
  "search_console": {
    "site_url": "https://your-blog.com"
  }
}
```

### Create a Strategy Analyzer pipeline

```json
{
  "id": "strategy-weekly",
  "name": "Weekly Strategy Analysis",
  "strategy": "strategy_analyzer",
  "schedule_cron": "0 10 * * 1",
  "llm": {
    "provider": "openrouter",
    "model": "mistralai/mistral-nemo"
  }
}
```

## Managing the Daemon via CLI

All management commands work through `jarvis daemon <subcommand>`:

```bash
# Overall status
jarvis daemon status

# Pipeline management
jarvis daemon pipeline list
jarvis daemon pipeline enable <id>
jarvis daemon pipeline disable <id>
jarvis daemon pipeline config <id>

# View jobs
jarvis daemon jobs
jarvis daemon jobs --pipeline seo-blog-tech --status completed

# View generated content
jarvis daemon content --limit 10
jarvis daemon content --status published

# View proposals (from strategy analyzer)
jarvis daemon proposals
jarvis daemon proposals approve <id>
jarvis daemon proposals reject <id>

# View goals and progress
jarvis daemon goals

# View experiments (A/B tests)
jarvis daemon experiments

# View metrics
jarvis daemon metrics

# Rich dashboard
jarvis daemon dashboard

# System health
jarvis daemon health
```

## Google OAuth2 Setup (for GSC/AdSense/GA4)

```bash
jarvis-daemon auth google \
  --client-id $GOOGLE_CLIENT_ID \
  --client-secret $GOOGLE_CLIENT_SECRET
```

This opens a browser flow. After authorization, tokens are saved to
`~/.jarvis/credentials/google.json`.

## Logs

```bash
# Docker
docker logs -f jarvis-daemon

# Local
RUST_LOG=jarvis_daemon=debug ./target/release/jarvis-daemon run

# Via CLI (from database)
jarvis daemon logs --limit 50
```

## Architecture

```
jarvis-daemon (background process, runs 24/7)
  |
  +-- Scheduler (tick every 60s)
  |     |-- checks cron schedules
  |     |-- creates jobs for due pipelines
  |     |-- executes approved proposals
  |
  +-- PipelineRunner
  |     |-- max 3 concurrent jobs
  |     |-- LLM Router with fallback + circuit breakers
  |     |-- retry logic
  |
  +-- Notifier (Telegram)
  |     |-- daily summary at JARVIS_NOTIFY_HOUR
  |     |-- alerts on failures
  |
  +-- SQLite (daemon.db)
        |-- pipelines, jobs, content, sources
        |-- metrics, goals, proposals, experiments
        |-- shared with jarvis CLI for inspection
```
