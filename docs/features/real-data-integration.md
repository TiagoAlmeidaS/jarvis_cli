# Real Data Integration — Dados Reais de Analytics e Revenue

**Status**: Planejado
**Prioridade**: CRITICA
**Gap**: G3
**Roadmap**: Fase 1, Steps 1.3 e 1.4
**Ultima atualizacao**: 2026-02-13

---

## 1. Visao Geral

O daemon atualmente **estima** metricas com CPC ficticio. Para que o
`strategy_analyzer` tome decisoes uteis, precisa de **dados reais**:
pageviews por artigo, clicks de busca organica, revenue real do AdSense.

### Fontes de dados por prioridade

| # | Fonte | Dados | Complexidade | Prioridade |
|---|-------|-------|-------------|------------|
| 1 | **Input manual via CLI** | Revenue, pageviews | Trivial | IMEDIATA |
| 2 | **WordPress REST API** | Pageviews por post, comentarios | Baixa | ALTA |
| 3 | **Google Search Console API** | Clicks, impressoes, CTR, posicao | Media | ALTA |
| 4 | **Google AdSense API** | Revenue por pagina, RPM | Media | MEDIA |
| 5 | **Stripe API** | Revenue de Micro-SaaS | Baixa | Quando Strategy 3 |
| 6 | **YouTube Analytics API** | Views, watch time, revenue | Media | Quando Strategy 1 |

**Estrategia**: Comecar por 1 (manual) e 2 (WordPress), que dao resultado
imediato com zero complexidade de OAuth.

---

## 2. Fonte 1: Input Manual via CLI (Step 1.4)

### Motivacao

Mesmo sem APIs integradas, o usuario pode registrar revenue manualmente.
Isso permite que o Goal System e o Strategy Analyzer funcionem com dados
reais desde o primeiro dia.

### Comandos

```
jarvis daemon revenue add 15.50 --source adsense --period 2026-02
jarvis daemon revenue add 45.00 --source affiliate --pipeline seo-concursos --period 2026-02
jarvis daemon revenue add 9.99 --source stripe --pipeline saas-api --period 2026-02

jarvis daemon metrics add --pipeline seo-concursos \
    --type pageviews --value 3200 --period 2026-02
jarvis daemon metrics add --pipeline seo-concursos \
    --type clicks --value 850 --period 2026-02
```

### Implementacao

Usar os metodos `create_revenue()` e `insert_metric()` que **ja existem** no `DaemonDb`.
Apenas adicionar novos subcomandos no `cli/src/daemon_cmd.rs`.

```rust
// Em daemon_cmd.rs:
#[derive(Subcommand)]
enum RevenueCommand {
    Summary { ... },  // ja existe
    List { ... },     // ja existe
    Add(RevenueAddArgs),  // NOVO
}

#[derive(Args)]
struct RevenueAddArgs {
    /// Amount in USD
    amount: f64,
    /// Source: adsense, affiliate, stripe, manual
    #[arg(long)]
    source: String,
    /// Pipeline ID (optional)
    #[arg(long)]
    pipeline: Option<String>,
    /// Period: YYYY-MM format
    #[arg(long, default_value = "current")]
    period: String,
}
```

**Estimativa**: 0.5 dia. Zero dependencias novas.

---

## 3. Fonte 2: WordPress REST API (Step 1.3)

### API Endpoints

O WordPress REST API com plugin **WP Statistics** ou **Jetpack Stats** expoe:

```
GET /wp-json/wp/v2/posts                   # Lista posts
GET /wp-json/wpstatistics/v2/hits          # Pageviews (WP Statistics plugin)
GET /wp-json/jetpack/v4/module/stats       # Stats (Jetpack plugin)
```

Sem plugin, usar a **WordPress.com Stats API** (requer Jetpack):

```
GET https://stats.wordpress.com/csv.php?api_key=KEY&blog_uri=BLOG&table=postviews&days=30
```

### Integracao no Metrics Collector

```rust
// Novo modulo: daemon/src/data_sources/wordpress_stats.rs

pub struct WordPressStatsClient {
    base_url: String,
    auth: WordPressAuth, // Application Password ou API key
    http: reqwest::Client,
}

impl WordPressStatsClient {
    /// Fetch pageviews for all posts in the given period.
    pub async fn fetch_post_views(&self, days: u32) -> Result<Vec<PostViewStats>> {
        // ...
    }

    /// Match WordPress posts to daemon_content by URL/slug.
    pub async fn sync_views_to_content(&self, db: &DaemonDb) -> Result<SyncSummary> {
        let views = self.fetch_post_views(30).await?;
        for pv in &views {
            // Find content by URL match.
            if let Some(content) = db.find_content_by_url(&pv.url).await? {
                db.insert_metric(
                    Some(&content.id),
                    &content.pipeline_id,
                    "views",
                    pv.views as f64,
                    None, // currency
                    period_start,
                    period_end,
                    "wordpress",
                ).await?;
            }
        }
        Ok(summary)
    }
}
```

### Configuracao

```toml
# Em config.toml ou no pipeline config_json:
[daemon.data_sources.wordpress]
enabled = true
base_url = "https://seu-blog.com"
auth_type = "application_password"  # ou "jetpack_api_key"
username = "admin"
# password via env: WORDPRESS_APP_PASSWORD (ja configurado para publisher)
stats_plugin = "wp_statistics"      # ou "jetpack"
sync_interval_hours = 6
```

### Integracao no Metrics Collector

```rust
// Em metrics_collector.rs execute():
if let Some(wp_config) = config.wordpress_stats.as_ref() {
    let wp_client = WordPressStatsClient::new(wp_config)?;
    let sync = wp_client.sync_views_to_content(&db).await?;
    ctx.log_info(&format!(
        "WordPress stats synced: {} posts updated, {} total views",
        sync.posts_updated, sync.total_views
    )).await;
}
```

**Estimativa**: 1-2 dias. Dependencia: `reqwest` (ja no projeto).

---

## 4. Fonte 3: Google Search Console API (Fase 3)

### Visao Geral

Search Console fornece os dados mais valiosos para SEO:
- **Clicks**: quantas vezes clicaram no seu resultado
- **Impressions**: quantas vezes apareceu na busca
- **CTR**: click-through rate
- **Position**: posicao media nos resultados

### API

```
POST https://www.googleapis.com/webmasters/v3/sites/{site}/searchAnalytics/query

{
    "startDate": "2026-01-01",
    "endDate": "2026-01-31",
    "dimensions": ["page", "query"],
    "rowLimit": 1000
}
```

### Auth

Requer OAuth 2.0 com scope `https://www.googleapis.com/auth/webmasters.readonly`.

O flow seria:
1. Usuario roda `jarvis daemon auth google` uma vez
2. Abre browser, autoriza, recebe refresh_token
3. Token salvo em `~/.jarvis/credentials/google.json`
4. Daemon usa refresh_token para obter access_token automaticamente

### Dados mapeados

| Search Console | daemon_metrics.metric_type | Uso |
|----------------|---------------------------|-----|
| clicks | clicks | Medir trafego real |
| impressions | impressions | Medir visibilidade |
| ctr | ctr | Medir qualidade de titulos |
| position | custom (position) | Medir ranking |

**Estimativa**: 2-3 dias (inclui OAuth flow).

---

## 5. Fonte 4: Google AdSense API (Fase 3)

### API

```
GET https://adsense.googleapis.com/v2/accounts/{account}/reports:generate
    ?dateRange.startDate.year=2026&dateRange.startDate.month=1&dateRange.startDate.day=1
    &dateRange.endDate.year=2026&dateRange.endDate.month=1&dateRange.endDate.day=31
    &metrics=ESTIMATED_EARNINGS,PAGE_VIEWS,PAGE_VIEWS_RPM
    &dimensions=PAGE
```

### Auth

Mesmo OAuth flow do Search Console (pode compartilhar o mesmo token/project).
Scope: `https://www.googleapis.com/auth/adsense.readonly`

### Dados mapeados

| AdSense | daemon_revenue | Uso |
|---------|----------------|-----|
| ESTIMATED_EARNINGS | amount (source=adsense) | Revenue real |
| PAGE_VIEWS_RPM | metadata_json | Revenue per mille |

**Estimativa**: 2-3 dias (compartilha infra OAuth com Search Console).

---

## 6. Novo modulo: `daemon/src/data_sources/`

```
daemon/src/data_sources/
├── mod.rs                    # Trait DataSource + registry
├── wordpress_stats.rs        # WordPress REST API stats
├── google_search_console.rs  # Search Console API
├── google_adsense.rs         # AdSense API
├── google_auth.rs            # OAuth2 flow compartilhado
└── stripe.rs                 # Stripe revenue (futuro)
```

### Trait

```rust
#[async_trait]
pub trait DataSource: Send + Sync {
    /// Nome da fonte de dados.
    fn name(&self) -> &str;

    /// Sincronizar dados da fonte para o banco.
    async fn sync(&self, db: &DaemonDb) -> Result<SyncResult>;
}

pub struct SyncResult {
    pub records_synced: u64,
    pub errors: Vec<String>,
}
```

---

## 7. Prioridade de implementacao

```
Semana 1:  [1] Revenue manual CLI (0.5 dia)
           [2] WordPress Stats API (1-2 dias)
           → Daemon ja tem dados reais para analytics

Semana 2:  [3] Google Search Console (2-3 dias)
           → Dados de SEO real (clicks, CTR, posicao)

Semana 3:  [4] Google AdSense (2-3 dias)
           → Revenue real automatico

Futuro:    [5] Stripe (quando Strategy 3 for implementada)
           [6] YouTube Analytics (quando Strategy 1 for implementada)
```

---

## 8. Testes

| Teste | Tipo | Descricao |
|-------|------|-----------|
| `test_revenue_add_cli` | Unitario | CLI registra revenue manual |
| `test_metrics_add_cli` | Unitario | CLI registra metricas manuais |
| `test_wp_stats_parse` | Unitario | Parse de resposta WordPress API |
| `test_wp_stats_content_match` | Unitario | Match por URL entre WP post e daemon_content |
| `test_search_console_parse` | Unitario | Parse de resposta Search Console |
| `test_adsense_parse` | Unitario | Parse de resposta AdSense |
| `test_metrics_collector_with_wp` | Integracao | Collector sincroniza dados WP |

---

## 9. Arquivos a criar/modificar

| Arquivo | Acao | Descricao |
|---------|------|-----------|
| `cli/src/daemon_cmd.rs` | Modificar | Subcomandos `revenue add` e `metrics add` |
| `daemon/src/data_sources/mod.rs` | **Criar** | Trait DataSource + registry |
| `daemon/src/data_sources/wordpress_stats.rs` | **Criar** | WordPress stats client |
| `daemon/src/pipelines/metrics_collector.rs` | Modificar | Integrar data sources |
| `daemon-common/src/db.rs` | Modificar | `find_content_by_url()`, `find_content_by_slug()` |

---

## 10. Estimativa total

- **Step 1.3 (WordPress)**: 1-2 dias
- **Step 1.4 (CLI manual)**: 0.5 dia
- **Search Console**: 2-3 dias (Fase 3)
- **AdSense**: 2-3 dias (Fase 3)
- **Total Fase 1**: 2-3 dias
- **Total com Fase 3**: 7-10 dias
