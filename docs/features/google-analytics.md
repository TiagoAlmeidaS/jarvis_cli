# Google Analytics 4 Integration

**Status**: Implementado  
**Data**: 2026-02-13  
**Roadmap**: Fase 4, item 4.1

## Visao Geral

Integracao com a Google Analytics 4 Data API (v1beta) para coletar metricas de
engajamento do usuario no site, complementando os dados de SEO (Search Console)
e monetizacao (AdSense).

## Metricas Coletadas

| Metrica | Campo GA4 | Uso |
|---------|-----------|-----|
| Sessions | `sessions` | Volume de trafego por pagina |
| Engaged Sessions | `engagedSessions` | Sessoes com >10s, conversao, ou 2+ page views |
| Page Views | `screenPageViews` | Total de visualizacoes |
| Avg Session Duration | `averageSessionDuration` | Tempo medio na pagina (segundos) |
| Bounce Rate | `bounceRate` | % de sessoes single-page |

## Arquivos

- `daemon/src/data_sources/google_analytics.rs` — Client GA4 + DataSource impl
- `daemon/src/data_sources/mod.rs` — Registro do modulo
- `daemon/src/pipelines/metrics_collector.rs` — Integracao no sync
- `daemon/src/data_sources/google_auth.rs` — Scope `analytics.readonly` adicionado

## Autenticacao

Usa o mesmo fluxo Google OAuth2 compartilhado (google_auth.rs):
1. `jarvis daemon auth google` — autentica uma vez
2. Tokens persistidos em `~/.jarvis/credentials/google.json`
3. Auto-refresh silencioso via refresh_token

Scope requerido: `https://www.googleapis.com/auth/analytics.readonly`

## Configuracao

```json
{
  "google_analytics": {
    "property_id": "123456789",
    "oauth": {
      "client_id": "xxx.apps.googleusercontent.com",
      "client_secret": "xxx"
    },
    "days": 30
  }
}
```

## API Usada

- Endpoint: `POST /v1beta/properties/{propertyId}:runReport`
- Dimensao: `pagePath`
- Metricas: sessions, engagedSessions, screenPageViews, averageSessionDuration, bounceRate

## Testes (6 testes)

- Config parsing (minimal + full)
- Client construction
- Response parsing (empty + with rows)
- Missing metrics handling
