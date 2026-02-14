# Jetpack Stats Integration

**Data**: 2026-02-13
**Status**: Implementado
**Modulo**: `jarvis-rs/daemon/src/data_sources/wordpress_stats.rs`

## Overview

Integracao com o Jetpack / WordPress.com Stats API para coleta de pageviews reais por post. Esta implementacao completa o stub que existia anteriormente no modulo `wordpress_stats.rs`.

## Modos Suportados

O data source `wordpress_stats` agora suporta dois modos:

### 1. WP Statistics Plugin (`wp_statistics`)
- REST endpoint: `/wp-json/wp-statistics/v2/posts`
- Requer o plugin WP Statistics instalado no WordPress
- Autenticacao via Application Password

### 2. Jetpack / WordPress.com Stats (`jetpack`)
- Endpoint principal: `/wp-json/wpcom/v2/stats/top-posts`
- Fallback: `https://public-api.wordpress.com/rest/v1.1/sites/{site}/stats/top-posts`
- Autenticacao via Jetpack API Key
- Agrega views por post ao longo de multiplos dias

## API Endpoints

### Endpoint Principal (Self-hosted com Jetpack)
```
GET {base_url}/wp-json/wpcom/v2/stats/top-posts?num={days}&period=day
```

### Endpoint Fallback (WordPress.com)
```
GET https://public-api.wordpress.com/rest/v1.1/sites/{site}/stats/top-posts?num={days}&period=day
```

## Configuracao

```json
{
  "wordpress_stats": {
    "base_url": "https://meu-blog.com",
    "auth": {
      "type": "jetpack_api_key",
      "api_key": "minha-chave-jetpack"
    },
    "stats_plugin": "jetpack",
    "sync_days": 30
  }
}
```

### Opcoes de Autenticacao

| Tipo | Uso |
|------|-----|
| `application_password` | WP Statistics plugin (self-hosted) |
| `jetpack_api_key` | Jetpack / WordPress.com |
| `none` | Endpoints publicos |

## Fluxo de Dados

1. O `MetricsCollectorPipeline` invoca `WordPressStatsClient::sync()`
2. Se `stats_plugin == jetpack`, chama `fetch_jetpack_views()`
3. Tenta o endpoint REST local primeiro
4. Se falhar, faz fallback para a API publica do WordPress.com
5. Agrega views por post ID ao longo dos dias
6. Faz match com `daemon_content` via URL ou slug
7. Registra metricas com `source = "jetpack_stats"` na tabela `daemon_metrics`

## Tipos de Resposta

| Tipo | Descricao |
|------|-----------|
| `JetpackTopPostsResponse` | Resposta principal com summary e dados por dia |
| `JetpackDayData` | Posts e views totais de um dia |
| `JetpackPostView` | View individual de um post |
| `JetpackStatsResponse` | Formato alternativo com lista de posts |
| `JetpackSummary` | Totais de views e visitors |

## Testes

- `parse_jetpack_top_posts_response` — Parse de resposta multi-dia com summary
- `parse_jetpack_post_view` — Parse de post view individual
- `parse_jetpack_response_empty_days` — Resposta sem dados
- `parse_jetpack_stats_response` — Parse de formato alternativo
- `parse_config_jetpack` — Parse de config com Jetpack
- `client_construction_succeeds` — Construcao do client HTTP

## Arquivos Afetados

- `daemon/src/data_sources/wordpress_stats.rs` — Implementacao completa do Jetpack
- `daemon/src/pipelines/metrics_collector.rs` — Ja integrado (sem alteracoes necessarias)
