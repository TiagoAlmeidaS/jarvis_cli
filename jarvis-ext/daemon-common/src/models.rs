//! Data models for the daemon automation system.

use serde::Deserialize;
use serde::Serialize;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

/// The type of automation strategy a pipeline implements.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "TEXT")]
#[sqlx(rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum Strategy {
    SeoBlog,
    YoutubeShorts,
    SaasApi,
    MetricsCollector,
    StrategyAnalyzer,
    AbTester,
    PromptOptimizer,
}

impl std::fmt::Display for Strategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SeoBlog => write!(f, "seo_blog"),
            Self::YoutubeShorts => write!(f, "youtube_shorts"),
            Self::SaasApi => write!(f, "saas_api"),
            Self::MetricsCollector => write!(f, "metrics_collector"),
            Self::StrategyAnalyzer => write!(f, "strategy_analyzer"),
            Self::AbTester => write!(f, "ab_tester"),
            Self::PromptOptimizer => write!(f, "prompt_optimizer"),
        }
    }
}

impl std::str::FromStr for Strategy {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "seo_blog" => Ok(Self::SeoBlog),
            "youtube_shorts" => Ok(Self::YoutubeShorts),
            "saas_api" => Ok(Self::SaasApi),
            "metrics_collector" => Ok(Self::MetricsCollector),
            "strategy_analyzer" => Ok(Self::StrategyAnalyzer),
            "ab_tester" => Ok(Self::AbTester),
            "prompt_optimizer" => Ok(Self::PromptOptimizer),
            other => Err(anyhow::anyhow!("unknown strategy: {other}")),
        }
    }
}

/// Status of a daemon job.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "TEXT")]
#[sqlx(rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

impl std::fmt::Display for JobStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::Running => write!(f, "running"),
            Self::Completed => write!(f, "completed"),
            Self::Failed => write!(f, "failed"),
            Self::Cancelled => write!(f, "cancelled"),
        }
    }
}

/// Type of content produced by a pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "TEXT")]
#[sqlx(rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum ContentType {
    Article,
    VideoShort,
    ApiResponse,
    Pdf,
    Image,
}

impl std::fmt::Display for ContentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Article => write!(f, "article"),
            Self::VideoShort => write!(f, "video_short"),
            Self::ApiResponse => write!(f, "api_response"),
            Self::Pdf => write!(f, "pdf"),
            Self::Image => write!(f, "image"),
        }
    }
}

/// Target platform for published content.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "TEXT")]
#[sqlx(rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum Platform {
    Wordpress,
    Ghost,
    Youtube,
    Tiktok,
    Gumroad,
    Stripe,
    Local,
}

impl std::fmt::Display for Platform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Wordpress => write!(f, "wordpress"),
            Self::Ghost => write!(f, "ghost"),
            Self::Youtube => write!(f, "youtube"),
            Self::Tiktok => write!(f, "tiktok"),
            Self::Gumroad => write!(f, "gumroad"),
            Self::Stripe => write!(f, "stripe"),
            Self::Local => write!(f, "local"),
        }
    }
}

/// Publication status of generated content.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "TEXT")]
#[sqlx(rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum ContentStatus {
    Draft,
    Rendering,
    Uploading,
    Published,
    Failed,
    Archived,
}

impl std::fmt::Display for ContentStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Draft => write!(f, "draft"),
            Self::Rendering => write!(f, "rendering"),
            Self::Uploading => write!(f, "uploading"),
            Self::Published => write!(f, "published"),
            Self::Failed => write!(f, "failed"),
            Self::Archived => write!(f, "archived"),
        }
    }
}

/// Type of data source to monitor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "TEXT")]
#[sqlx(rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum SourceType {
    Rss,
    Webpage,
    Api,
    PdfUrl,
    YoutubeChannel,
}

impl std::fmt::Display for SourceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Rss => write!(f, "rss"),
            Self::Webpage => write!(f, "webpage"),
            Self::Api => write!(f, "api"),
            Self::PdfUrl => write!(f, "pdf_url"),
            Self::YoutubeChannel => write!(f, "youtube_channel"),
        }
    }
}

/// Type of metric tracked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "TEXT")]
#[sqlx(rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum MetricType {
    Views,
    Clicks,
    Revenue,
    Impressions,
    Subscribers,
    Ctr,
}

/// Log severity level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "TEXT")]
#[sqlx(rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

/// Type of action proposed by the strategy analyzer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "TEXT")]
#[sqlx(rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum ActionType {
    CreatePipeline,
    ModifyPipeline,
    DisablePipeline,
    ChangeNiche,
    ChangeFrequency,
    AddSource,
    RemoveSource,
    ScaleUp,
    ScaleDown,
    ChangeModel,
    Custom,
}

impl std::fmt::Display for ActionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CreatePipeline => write!(f, "create_pipeline"),
            Self::ModifyPipeline => write!(f, "modify_pipeline"),
            Self::DisablePipeline => write!(f, "disable_pipeline"),
            Self::ChangeNiche => write!(f, "change_niche"),
            Self::ChangeFrequency => write!(f, "change_frequency"),
            Self::AddSource => write!(f, "add_source"),
            Self::RemoveSource => write!(f, "remove_source"),
            Self::ScaleUp => write!(f, "scale_up"),
            Self::ScaleDown => write!(f, "scale_down"),
            Self::ChangeModel => write!(f, "change_model"),
            Self::Custom => write!(f, "custom"),
        }
    }
}

impl std::str::FromStr for ActionType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "create_pipeline" => Ok(Self::CreatePipeline),
            "modify_pipeline" => Ok(Self::ModifyPipeline),
            "disable_pipeline" => Ok(Self::DisablePipeline),
            "change_niche" => Ok(Self::ChangeNiche),
            "change_frequency" => Ok(Self::ChangeFrequency),
            "add_source" => Ok(Self::AddSource),
            "remove_source" => Ok(Self::RemoveSource),
            "scale_up" => Ok(Self::ScaleUp),
            "scale_down" => Ok(Self::ScaleDown),
            "change_model" => Ok(Self::ChangeModel),
            "custom" => Ok(Self::Custom),
            other => Err(anyhow::anyhow!("unknown action type: {other}")),
        }
    }
}

/// Risk level of a proposed action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "TEXT")]
#[sqlx(rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum RiskLevel {
    Low,
    Medium,
    High,
}

impl std::fmt::Display for RiskLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Low => write!(f, "low"),
            Self::Medium => write!(f, "medium"),
            Self::High => write!(f, "high"),
        }
    }
}

impl std::str::FromStr for RiskLevel {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "low" => Ok(Self::Low),
            "medium" => Ok(Self::Medium),
            "high" => Ok(Self::High),
            other => Err(anyhow::anyhow!("unknown risk level: {other}")),
        }
    }
}

/// Status of a proposed action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "TEXT")]
#[sqlx(rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum ProposalStatus {
    Pending,
    Approved,
    Rejected,
    Expired,
    Executed,
    Failed,
}

impl std::fmt::Display for ProposalStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::Approved => write!(f, "approved"),
            Self::Rejected => write!(f, "rejected"),
            Self::Expired => write!(f, "expired"),
            Self::Executed => write!(f, "executed"),
            Self::Failed => write!(f, "failed"),
        }
    }
}

impl std::str::FromStr for ProposalStatus {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "pending" => Ok(Self::Pending),
            "approved" => Ok(Self::Approved),
            "rejected" => Ok(Self::Rejected),
            "expired" => Ok(Self::Expired),
            "executed" => Ok(Self::Executed),
            "failed" => Ok(Self::Failed),
            other => Err(anyhow::anyhow!("unknown proposal status: {other}")),
        }
    }
}

/// Source of revenue tracking.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "TEXT")]
#[sqlx(rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum RevenueSource {
    Adsense,
    Affiliate,
    Gumroad,
    Stripe,
    Manual,
    Estimated,
}

impl std::fmt::Display for RevenueSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Adsense => write!(f, "adsense"),
            Self::Affiliate => write!(f, "affiliate"),
            Self::Gumroad => write!(f, "gumroad"),
            Self::Stripe => write!(f, "stripe"),
            Self::Manual => write!(f, "manual"),
            Self::Estimated => write!(f, "estimated"),
        }
    }
}

impl std::str::FromStr for RevenueSource {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "adsense" => Ok(Self::Adsense),
            "affiliate" => Ok(Self::Affiliate),
            "gumroad" => Ok(Self::Gumroad),
            "stripe" => Ok(Self::Stripe),
            "manual" => Ok(Self::Manual),
            "estimated" => Ok(Self::Estimated),
            other => Err(anyhow::anyhow!("unknown revenue source: {other}")),
        }
    }
}

// ---------------------------------------------------------------------------
// Row models (map 1:1 to SQLite tables)
// ---------------------------------------------------------------------------

/// A registered automation pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonPipeline {
    pub id: String,
    pub name: String,
    pub strategy: String,
    pub config_json: String,
    pub schedule_cron: String,
    pub enabled: bool,
    pub max_retries: i32,
    pub retry_delay_sec: i32,
    pub created_at: i64,
    pub updated_at: i64,
}

impl DaemonPipeline {
    /// Parse the stored `config_json` into a typed value.
    pub fn config<T: serde::de::DeserializeOwned>(&self) -> anyhow::Result<T> {
        Ok(serde_json::from_str(&self.config_json)?)
    }

    /// Parse the `strategy` field into the [`Strategy`] enum.
    pub fn strategy_enum(&self) -> anyhow::Result<Strategy> {
        self.strategy.parse()
    }
}

/// A single execution of a pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonJob {
    pub id: String,
    pub pipeline_id: String,
    pub status: String,
    pub attempt: i32,
    pub started_at: Option<i64>,
    pub completed_at: Option<i64>,
    pub input_json: Option<String>,
    pub output_json: Option<String>,
    pub error_message: Option<String>,
    pub error_stack: Option<String>,
    pub duration_ms: Option<i64>,
    pub created_at: i64,
}

impl DaemonJob {
    pub fn new(pipeline_id: &str) -> Self {
        let now = chrono::Utc::now().timestamp();
        Self {
            id: Uuid::new_v4().to_string(),
            pipeline_id: pipeline_id.to_string(),
            status: JobStatus::Pending.to_string(),
            attempt: 1,
            started_at: None,
            completed_at: None,
            input_json: None,
            output_json: None,
            error_message: None,
            error_stack: None,
            duration_ms: None,
            created_at: now,
        }
    }
}

/// Content produced by a pipeline execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonContent {
    pub id: String,
    pub job_id: String,
    pub pipeline_id: String,
    pub content_type: String,
    pub platform: String,
    pub title: Option<String>,
    pub slug: Option<String>,
    pub url: Option<String>,
    pub status: String,
    pub word_count: Option<i32>,
    pub llm_model: Option<String>,
    pub llm_tokens_used: Option<i64>,
    pub llm_cost_usd: Option<f64>,
    pub content_hash: Option<String>,
    pub created_at: i64,
    pub published_at: Option<i64>,
}

/// A data source monitored by a pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonSource {
    pub id: String,
    pub pipeline_id: String,
    pub source_type: String,
    pub name: String,
    pub url: String,
    pub scrape_selector: Option<String>,
    pub last_checked_at: Option<i64>,
    pub last_content_hash: Option<String>,
    pub check_interval_sec: i32,
    pub enabled: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

/// A tracked performance metric.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonMetric {
    pub id: String,
    pub content_id: Option<String>,
    pub pipeline_id: String,
    pub metric_type: String,
    pub value: f64,
    pub currency: Option<String>,
    pub period_start: i64,
    pub period_end: i64,
    pub source: String,
    pub created_at: i64,
}

/// A structured log entry from a daemon execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonLog {
    pub id: i64,
    pub job_id: Option<String>,
    pub pipeline_id: String,
    pub level: String,
    pub message: String,
    pub context_json: Option<String>,
    pub created_at: i64,
}

/// An action proposed by the strategy analyzer, awaiting user approval.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonProposal {
    pub id: String,
    pub pipeline_id: Option<String>,
    pub action_type: String,
    pub title: String,
    pub description: String,
    pub reasoning: String,
    pub confidence: f64,
    pub risk_level: String,
    pub status: String,
    pub proposed_config: Option<String>,
    pub metrics_snapshot: Option<String>,
    pub auto_approvable: bool,
    pub created_at: i64,
    pub reviewed_at: Option<i64>,
    pub executed_at: Option<i64>,
    pub expires_at: Option<i64>,
}

/// Revenue tracked from external sources or estimates.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonRevenue {
    pub id: String,
    pub content_id: Option<String>,
    pub pipeline_id: String,
    pub source: String,
    pub amount: f64,
    pub currency: String,
    pub period_start: i64,
    pub period_end: i64,
    pub external_id: Option<String>,
    pub metadata_json: Option<String>,
    pub created_at: i64,
}

// ---------------------------------------------------------------------------
// Goal enums
// ---------------------------------------------------------------------------

/// Type of metric a goal tracks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "TEXT")]
#[sqlx(rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum GoalMetricType {
    Revenue,
    ContentCount,
    Pageviews,
    Clicks,
    Ctr,
    Subscribers,
    CostLimit,
    Custom,
}

impl std::fmt::Display for GoalMetricType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Revenue => write!(f, "revenue"),
            Self::ContentCount => write!(f, "content_count"),
            Self::Pageviews => write!(f, "pageviews"),
            Self::Clicks => write!(f, "clicks"),
            Self::Ctr => write!(f, "ctr"),
            Self::Subscribers => write!(f, "subscribers"),
            Self::CostLimit => write!(f, "cost_limit"),
            Self::Custom => write!(f, "custom"),
        }
    }
}

impl std::str::FromStr for GoalMetricType {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "revenue" => Ok(Self::Revenue),
            "content_count" => Ok(Self::ContentCount),
            "pageviews" => Ok(Self::Pageviews),
            "clicks" => Ok(Self::Clicks),
            "ctr" => Ok(Self::Ctr),
            "subscribers" => Ok(Self::Subscribers),
            "cost_limit" => Ok(Self::CostLimit),
            "custom" => Ok(Self::Custom),
            other => Err(anyhow::anyhow!("unknown goal metric type: {other}")),
        }
    }
}

/// Period over which a goal is measured.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "TEXT")]
#[sqlx(rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum GoalPeriod {
    Daily,
    Weekly,
    Monthly,
    Quarterly,
    Yearly,
}

impl std::fmt::Display for GoalPeriod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Daily => write!(f, "daily"),
            Self::Weekly => write!(f, "weekly"),
            Self::Monthly => write!(f, "monthly"),
            Self::Quarterly => write!(f, "quarterly"),
            Self::Yearly => write!(f, "yearly"),
        }
    }
}

impl std::str::FromStr for GoalPeriod {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "daily" => Ok(Self::Daily),
            "weekly" => Ok(Self::Weekly),
            "monthly" => Ok(Self::Monthly),
            "quarterly" => Ok(Self::Quarterly),
            "yearly" => Ok(Self::Yearly),
            other => Err(anyhow::anyhow!("unknown goal period: {other}")),
        }
    }
}

/// Status of a goal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "TEXT")]
#[sqlx(rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum GoalStatus {
    Active,
    Achieved,
    Paused,
    Failed,
    Archived,
}

impl std::fmt::Display for GoalStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Active => write!(f, "active"),
            Self::Achieved => write!(f, "achieved"),
            Self::Paused => write!(f, "paused"),
            Self::Failed => write!(f, "failed"),
            Self::Archived => write!(f, "archived"),
        }
    }
}

impl std::str::FromStr for GoalStatus {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "active" => Ok(Self::Active),
            "achieved" => Ok(Self::Achieved),
            "paused" => Ok(Self::Paused),
            "failed" => Ok(Self::Failed),
            "archived" => Ok(Self::Archived),
            other => Err(anyhow::anyhow!("unknown goal status: {other}")),
        }
    }
}

// ---------------------------------------------------------------------------
// Input types (for creating new records)
// ---------------------------------------------------------------------------

/// Parameters for creating a new pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePipeline {
    pub id: String,
    pub name: String,
    pub strategy: Strategy,
    pub config_json: serde_json::Value,
    pub schedule_cron: String,
    pub max_retries: Option<i32>,
    pub retry_delay_sec: Option<i32>,
}

/// Parameters for adding a source to a pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSource {
    pub pipeline_id: String,
    pub source_type: SourceType,
    pub name: String,
    pub url: String,
    pub scrape_selector: Option<String>,
    pub check_interval_sec: Option<i32>,
}

/// Parameters for creating a new proposal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateProposal {
    pub pipeline_id: Option<String>,
    pub action_type: ActionType,
    pub title: String,
    pub description: String,
    pub reasoning: String,
    pub confidence: f64,
    pub risk_level: RiskLevel,
    pub proposed_config: Option<serde_json::Value>,
    pub metrics_snapshot: Option<serde_json::Value>,
    pub auto_approvable: bool,
    pub expires_in_hours: Option<i64>,
}

/// Parameters for recording revenue.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateRevenue {
    pub content_id: Option<String>,
    pub pipeline_id: String,
    pub source: RevenueSource,
    pub amount: f64,
    pub currency: Option<String>,
    pub period_start: i64,
    pub period_end: i64,
    pub external_id: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

/// A measurable goal that drives daemon strategy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonGoal {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub metric_type: String,
    pub target_value: f64,
    pub target_unit: String,
    pub period: String,
    pub pipeline_id: Option<String>,
    pub current_value: f64,
    pub last_measured: Option<i64>,
    pub status: String,
    pub priority: i32,
    pub deadline: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
}

/// Parameters for creating a new goal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateGoal {
    pub name: String,
    pub description: Option<String>,
    pub metric_type: GoalMetricType,
    pub target_value: f64,
    pub target_unit: Option<String>,
    pub period: GoalPeriod,
    pub pipeline_id: Option<String>,
    pub priority: Option<i32>,
    pub deadline: Option<i64>,
}

/// Filters for listing goals.
#[derive(Debug, Clone, Default)]
pub struct GoalFilter {
    pub status: Option<GoalStatus>,
    pub pipeline_id: Option<String>,
    pub limit: Option<i64>,
}

/// Computed progress for a goal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoalProgress {
    pub goal: DaemonGoal,
    pub gap: f64,
    pub progress_pct: f64,
    pub on_track: bool,
    pub days_remaining: Option<i64>,
}

// ---------------------------------------------------------------------------
// Pipeline output (returned from pipeline execution)
// ---------------------------------------------------------------------------

/// Standard output from a pipeline step, later persisted as [`DaemonContent`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentOutput {
    pub content_type: ContentType,
    pub platform: Platform,
    pub title: String,
    pub slug: String,
    pub body: String,
    pub url: Option<String>,
    pub word_count: Option<i32>,
    pub llm_model: String,
    pub llm_tokens_used: i64,
    pub llm_cost_usd: Option<f64>,
}

/// Result of publishing content to an external platform.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishResult {
    pub external_url: String,
    pub external_id: Option<String>,
    pub platform: Platform,
}

/// Data extracted by a scraper.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScrapedContent {
    pub title: String,
    pub body: String,
    pub url: String,
    pub content_hash: String,
    pub metadata: serde_json::Value,
}

/// Response from an LLM call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmResponse {
    pub text: String,
    pub model: String,
    pub tokens_used: i64,
    pub cost_usd: Option<f64>,
}

// ---------------------------------------------------------------------------
// Query helpers
// ---------------------------------------------------------------------------

/// Filters for listing jobs.
#[derive(Debug, Clone, Default)]
pub struct JobFilter {
    pub pipeline_id: Option<String>,
    pub status: Option<JobStatus>,
    pub limit: Option<i64>,
}

/// Filters for listing content.
#[derive(Debug, Clone, Default)]
pub struct ContentFilter {
    pub pipeline_id: Option<String>,
    pub platform: Option<Platform>,
    pub status: Option<ContentStatus>,
    pub since_days: Option<i64>,
    pub limit: Option<i64>,
}

/// Filters for listing logs.
#[derive(Debug, Clone, Default)]
pub struct LogFilter {
    pub pipeline_id: Option<String>,
    pub job_id: Option<String>,
    pub level: Option<LogLevel>,
    pub limit: Option<i64>,
}

/// Filters for listing proposals.
#[derive(Debug, Clone, Default)]
pub struct ProposalFilter {
    pub pipeline_id: Option<String>,
    pub status: Option<ProposalStatus>,
    pub risk_level: Option<RiskLevel>,
    pub limit: Option<i64>,
}

// ---------------------------------------------------------------------------
// Experiment (A/B testing) models
// ---------------------------------------------------------------------------

/// Status of an A/B experiment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "TEXT")]
#[sqlx(rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum ExperimentStatus {
    Running,
    Completed,
    Cancelled,
}

impl std::fmt::Display for ExperimentStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Running => write!(f, "running"),
            Self::Completed => write!(f, "completed"),
            Self::Cancelled => write!(f, "cancelled"),
        }
    }
}

impl std::str::FromStr for ExperimentStatus {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "running" => Ok(Self::Running),
            "completed" => Ok(Self::Completed),
            "cancelled" => Ok(Self::Cancelled),
            other => Err(anyhow::anyhow!("unknown experiment status: {other}")),
        }
    }
}

/// What kind of element is being A/B tested.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "TEXT")]
#[sqlx(rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum ExperimentType {
    Title,
    MetaDescription,
    Headline,
    Custom,
}

impl std::fmt::Display for ExperimentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Title => write!(f, "title"),
            Self::MetaDescription => write!(f, "meta_description"),
            Self::Headline => write!(f, "headline"),
            Self::Custom => write!(f, "custom"),
        }
    }
}

impl std::str::FromStr for ExperimentType {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "title" => Ok(Self::Title),
            "meta_description" => Ok(Self::MetaDescription),
            "headline" => Ok(Self::Headline),
            "custom" => Ok(Self::Custom),
            other => Err(anyhow::anyhow!("unknown experiment type: {other}")),
        }
    }
}

/// An A/B experiment on a piece of content.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonExperiment {
    pub id: String,
    pub content_id: String,
    pub pipeline_id: String,
    pub experiment_type: String,
    pub status: String,
    /// The original value (e.g. original title).
    pub variant_a: String,
    /// The challenger value (e.g. new title).
    pub variant_b: String,
    /// Which variant is currently live ("a" or "b").
    pub active_variant: String,
    /// Metric being measured to pick a winner.
    pub metric: String,
    /// Aggregated metric for variant A.
    pub metric_a: f64,
    /// Aggregated metric for variant B.
    pub metric_b: f64,
    /// Winning variant ("a", "b", or null if still running).
    pub winner: Option<String>,
    /// Minimum days to run before declaring a winner.
    pub min_duration_days: i32,
    pub created_at: i64,
    pub updated_at: i64,
    pub completed_at: Option<i64>,
}

/// Parameters for creating a new experiment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateExperiment {
    pub content_id: String,
    pub pipeline_id: String,
    pub experiment_type: ExperimentType,
    pub variant_a: String,
    pub variant_b: String,
    pub metric: String,
    pub min_duration_days: Option<i32>,
}

/// Filters for listing experiments.
#[derive(Debug, Clone, Default)]
pub struct ExperimentFilter {
    pub pipeline_id: Option<String>,
    pub content_id: Option<String>,
    pub status: Option<ExperimentStatus>,
    pub limit: Option<i64>,
}

/// Filters for listing revenue records.
#[derive(Debug, Clone, Default)]
pub struct RevenueFilter {
    pub pipeline_id: Option<String>,
    pub source: Option<RevenueSource>,
    pub since_days: Option<i64>,
    pub limit: Option<i64>,
}

/// Aggregated revenue summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevenueSummary {
    pub total_usd: f64,
    pub period_days: i64,
    pub by_pipeline: Vec<PipelineRevenue>,
    pub by_source: Vec<SourceRevenue>,
}

/// Revenue breakdown per pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineRevenue {
    pub pipeline_id: String,
    pub pipeline_name: Option<String>,
    pub total_usd: f64,
    pub content_count: i64,
}

/// Revenue breakdown per source.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceRevenue {
    pub source: String,
    pub total_usd: f64,
    pub record_count: i64,
}

// ---------------------------------------------------------------------------
// Prompt Optimization
// ---------------------------------------------------------------------------

/// A scored record correlating prompt parameters with content performance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptScore {
    pub id: String,
    pub pipeline_id: String,
    pub content_id: String,
    /// Hash of the full prompt template (for grouping identical prompts).
    pub prompt_hash: String,
    /// Prompt parameters captured at generation time.
    pub params_json: serde_json::Value,
    /// Average CTR for content produced with these parameters.
    pub avg_ctr: f64,
    /// Total clicks.
    pub total_clicks: i64,
    /// Total impressions.
    pub total_impressions: i64,
    /// Revenue attributed to this content.
    pub revenue_usd: f64,
    /// Composite score (weighted combination of CTR, clicks, revenue).
    pub composite_score: f64,
    pub created_at: i64,
    pub updated_at: i64,
}

/// Input for recording a prompt score.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePromptScore {
    pub pipeline_id: String,
    pub content_id: String,
    pub prompt_hash: String,
    pub params_json: serde_json::Value,
}

/// Aggregated performance per prompt parameter set.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptPerformanceSummary {
    pub prompt_hash: String,
    pub params_json: serde_json::Value,
    pub content_count: i64,
    pub avg_ctr: f64,
    pub avg_clicks: f64,
    pub total_revenue: f64,
    pub composite_score: f64,
}

/// A suggestion generated by the prompt optimizer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptOptimizationSuggestion {
    pub parameter: String,
    pub current_value: String,
    pub suggested_value: String,
    pub reason: String,
    pub expected_improvement_pct: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn strategy_round_trip() {
        let s = Strategy::SeoBlog;
        let text = s.to_string();
        let parsed: Strategy = text.parse().expect("parse strategy");
        assert_eq!(s, parsed);
    }

    #[test]
    fn daemon_job_new_defaults() {
        let job = DaemonJob::new("test-pipeline");
        assert_eq!(job.pipeline_id, "test-pipeline");
        assert_eq!(job.status, "pending");
        assert_eq!(job.attempt, 1);
        assert!(job.started_at.is_none());
    }

    #[test]
    fn action_type_round_trip() {
        let a = ActionType::ScaleUp;
        let text = a.to_string();
        let parsed: ActionType = text.parse().expect("parse action type");
        assert_eq!(a, parsed);
    }

    #[test]
    fn risk_level_round_trip() {
        let r = RiskLevel::Medium;
        let text = r.to_string();
        let parsed: RiskLevel = text.parse().expect("parse risk level");
        assert_eq!(r, parsed);
    }

    #[test]
    fn proposal_status_round_trip() {
        let s = ProposalStatus::Approved;
        let text = s.to_string();
        let parsed: ProposalStatus = text.parse().expect("parse proposal status");
        assert_eq!(s, parsed);
    }

    #[test]
    fn revenue_source_round_trip() {
        let s = RevenueSource::Adsense;
        let text = s.to_string();
        let parsed: RevenueSource = text.parse().expect("parse revenue source");
        assert_eq!(s, parsed);
    }

    #[test]
    fn pipeline_config_parse() {
        let pipeline = DaemonPipeline {
            id: "test".to_string(),
            name: "Test".to_string(),
            strategy: "seo_blog".to_string(),
            config_json: r#"{"niche":"tech"}"#.to_string(),
            schedule_cron: "0 3 * * *".to_string(),
            enabled: true,
            max_retries: 3,
            retry_delay_sec: 300,
            created_at: 0,
            updated_at: 0,
        };

        let config: serde_json::Value = pipeline.config().expect("parse config");
        assert_eq!(config["niche"], "tech");
    }
}
