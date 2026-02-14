# Real Data Integration — Status da Implementacao

**Data**: 2026-02-13  
**Ultima atualizacao**: 2026-02-13  
**Status**: Implementado (WordPress + Google Search Console + Google AdSense)

## Componentes Implementados

### Infraestrutura

| Componente | Status | Arquivo |
|------------|--------|---------|
| Revenue manual via CLI (`jarvis daemon revenue add`) | Implementado | `cli/src/daemon_cmd.rs` |
| Trait `DataSource` + `DataSourceRegistry` | Implementado | `daemon/src/data_sources/mod.rs` |
| `find_content_by_url()` / `find_content_by_slug()` no DB | Implementado | `daemon-common/src/db.rs` |
| `sum_metrics()` no DB | Implementado | `daemon-common/src/db.rs` |
| Goal Pageviews/Clicks atualizados com dados reais | Implementado | `daemon/src/pipelines/metrics_collector.rs` |

### WordPress Stats

| Componente | Status | Arquivo |
|------------|--------|---------|
| WordPress Stats client (WP Statistics plugin) | Implementado | `daemon/src/data_sources/wordpress_stats.rs` |
| Integracao no Metrics Collector | Implementado | `daemon/src/pipelines/metrics_collector.rs` |

### Google APIs

| Componente | Status | Arquivo |
|------------|--------|---------|
| Google OAuth2 shared auth (installed app flow) | Implementado | `daemon/src/data_sources/google_auth.rs` |
| Google Search Console client (clicks, impressions, CTR, position) | Implementado | `daemon/src/data_sources/google_search_console.rs` |
| Google AdSense client (earnings per page, page views, RPM) | Implementado | `daemon/src/data_sources/google_adsense.rs` |
| Integracao Search Console + AdSense no Metrics Collector | Implementado | `daemon/src/pipelines/metrics_collector.rs` |
| CLI `jarvis-daemon auth google` para OAuth flow | Implementado | `daemon/src/main.rs` |
| Token persistence (`~/.jarvis/credentials/google.json`) | Implementado | `daemon/src/data_sources/google_auth.rs` |
| Auto-refresh de access tokens expirados | Implementado | `daemon/src/data_sources/google_auth.rs` |

## Testes (75 total no daemon, incluindo novos)

### WordPress Stats tests
- `data_sources::tests::registry_starts_empty`
- `data_sources::tests::registry_register_and_iterate`
- `data_sources::wordpress_stats::tests::parse_config_defaults`
- `data_sources::wordpress_stats::tests::parse_config_with_auth`
- `data_sources::wordpress_stats::tests::parse_config_jetpack`
- `data_sources::wordpress_stats::tests::parse_wp_statistics_post_view`
- `data_sources::wordpress_stats::tests::parse_wp_statistics_post_view_alternate_fields`
- `data_sources::wordpress_stats::tests::parse_wp_post`
- `data_sources::wordpress_stats::tests::sync_result_default`
- `data_sources::wordpress_stats::tests::client_construction_succeeds`
- `data_sources::wordpress_stats::tests::jetpack_sync_returns_early`

### Google Auth tests
- `data_sources::google_auth::tests::default_scopes_include_search_console_and_adsense`
- `data_sources::google_auth::tests::tokens_not_expired_when_fresh`
- `data_sources::google_auth::tests::tokens_expired_when_past`
- `data_sources::google_auth::tests::tokens_expired_within_safety_margin`
- `data_sources::google_auth::tests::authorization_url_contains_required_params`
- `data_sources::google_auth::tests::save_and_load_tokens_roundtrip`
- `data_sources::google_auth::tests::load_tokens_returns_none_when_missing`
- `data_sources::google_auth::tests::parse_oauth_config`

### Google Search Console tests
- `data_sources::google_search_console::tests::parse_config_minimal`
- `data_sources::google_search_console::tests::parse_config_full`
- `data_sources::google_search_console::tests::parse_search_analytics_row`
- `data_sources::google_search_console::tests::parse_search_analytics_response_empty`
- `data_sources::google_search_console::tests::parse_search_analytics_response_with_rows`
- `data_sources::google_search_console::tests::urlencoding_works`
- `data_sources::google_search_console::tests::client_construction_succeeds`

### Google AdSense tests
- `data_sources::google_adsense::tests::parse_config_minimal`
- `data_sources::google_adsense::tests::parse_config_full`
- `data_sources::google_adsense::tests::parse_adsense_report_response_empty`
- `data_sources::google_adsense::tests::parse_adsense_report_response_with_rows`
- `data_sources::google_adsense::tests::parse_page_report_from_rows`
- `data_sources::google_adsense::tests::client_construction_succeeds`

## Como usar

### 1. Autenticar com Google (uma vez)

```bash
jarvis-daemon auth google \
  --client-id YOUR_CLIENT_ID.apps.googleusercontent.com \
  --client-secret YOUR_CLIENT_SECRET
```

Isso abre o flow OAuth2, salva tokens em `~/.jarvis/credentials/google.json`.
O daemon renova automaticamente os tokens expirados.

Alternativamente, use variaveis de ambiente:

```bash
export GOOGLE_CLIENT_ID=YOUR_CLIENT_ID.apps.googleusercontent.com
export GOOGLE_CLIENT_SECRET=YOUR_CLIENT_SECRET
jarvis-daemon auth google
```

### 2. Configurar WordPress Stats no pipeline config

Adicione a secao `wordpress_stats` no `config_json` do pipeline `metrics_collector`:

```json
{
  "lookback_days": 30,
  "wordpress_stats": {
    "base_url": "https://seu-blog.com",
    "auth": {
      "type": "application_password",
      "username": "admin"
    },
    "stats_plugin": "wp_statistics",
    "sync_days": 30
  }
}
```

A senha de aplicacao WordPress deve ser definida via variavel de ambiente:

```
WORDPRESS_APP_PASSWORD=xxxx-xxxx-xxxx-xxxx
```

### 3. Configurar Google Search Console

Adicione `search_console` ao config do pipeline `metrics_collector`:

```json
{
  "search_console": {
    "site_url": "https://seu-blog.com/",
    "oauth": {
      "client_id": "YOUR_CLIENT_ID.apps.googleusercontent.com",
      "client_secret": "YOUR_SECRET"
    },
    "days": 30
  }
}
```

O `site_url` deve corresponder ao registrado no Search Console.
Para propriedades de dominio, use o formato `sc-domain:example.com`.

**Metricas coletadas**: clicks, impressions, CTR (%), position media.

### 4. Configurar Google AdSense

Adicione `adsense` ao config do pipeline `metrics_collector`:

```json
{
  "adsense": {
    "account_id": "accounts/pub-1234567890",
    "oauth": {
      "client_id": "YOUR_CLIENT_ID.apps.googleusercontent.com",
      "client_secret": "YOUR_SECRET"
    },
    "days": 30
  }
}
```

**Metricas coletadas**: estimated earnings ($), page views, page views RPM.
Revenue e persistida em `daemon_revenue` com source `adsense`.

### Fluxo de dados completo

```
WordPress (WP Statistics)     Google Search Console     Google AdSense
         |                            |                        |
         v                            v                        v
  WordPressStatsClient       SearchConsoleClient         AdSenseClient
         |                            |                        |
         v                            v                        v
  daemon_metrics             daemon_metrics              daemon_revenue
  (views, source:            (clicks, impressions,       (source: adsense,
   wordpress_stats)           ctr, source:                amount per page)
                              google_search_console)
         \                           |                        /
          \                          v                       /
           +-------> MetricsCollector <--------------------+
                            |
                            v
                   Goals atualizados (Pageviews, Clicks, Revenue)
                            |
                            v
               StrategyAnalyzer usa dados reais para propostas
```

## Proximos passos

1. **Jetpack Stats** — Alternativa ao WP Statistics (stub ja existe)
2. **Google Analytics 4** — Metricas de engajamento (bounce rate, session duration)
3. **Dashboard TUI** — Visualizacao de metricas em tempo real no terminal
