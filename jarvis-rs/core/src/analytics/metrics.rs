use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

/// Cache performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheMetrics {
    pub l1_hits: u64,
    pub l1_misses: u64,
    pub l2_hits: u64,
    pub l2_misses: u64,
    pub avg_response_time_ms: f64,
    pub total_requests: u64,
}

impl CacheMetrics {
    /// Calculate L1 cache hit rate (0.0 to 1.0)
    pub fn l1_hit_rate(&self) -> f64 {
        if self.l1_hits + self.l1_misses == 0 {
            return 0.0;
        }
        self.l1_hits as f64 / (self.l1_hits + self.l1_misses) as f64
    }

    /// Calculate L2 cache hit rate (0.0 to 1.0)
    pub fn l2_hit_rate(&self) -> f64 {
        if self.l2_hits + self.l2_misses == 0 {
            return 0.0;
        }
        self.l2_hits as f64 / (self.l2_hits + self.l2_misses) as f64
    }

    /// Calculate overall cache hit rate (0.0 to 1.0)
    pub fn overall_hit_rate(&self) -> f64 {
        let total_hits = self.l1_hits + self.l2_hits;
        let total_requests = total_hits + self.l1_misses + self.l2_misses;

        if total_requests == 0 {
            return 0.0;
        }

        total_hits as f64 / total_requests as f64
    }
}

/// Command execution metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandMetrics {
    pub command_name: String,
    pub execution_count: u64,
    pub success_count: u64,
    pub failure_count: u64,
    pub avg_execution_time_ms: f64,
    pub min_execution_time_ms: i32,
    pub max_execution_time_ms: i32,
    #[serde(with = "time::serde::rfc3339")]
    pub last_executed_at: OffsetDateTime,
}

impl CommandMetrics {
    /// Calculate success rate (0.0 to 1.0)
    pub fn success_rate(&self) -> f64 {
        if self.execution_count == 0 {
            return 0.0;
        }
        self.success_count as f64 / self.execution_count as f64
    }

    /// Check if command is slow (avg > 2000ms)
    pub fn is_slow(&self) -> bool {
        self.avg_execution_time_ms > 2000.0
    }

    /// Check if command has low success rate (< 80%)
    pub fn has_low_success_rate(&self) -> bool {
        self.success_rate() < 0.8
    }
}

/// Skill usage metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillMetrics {
    pub skill_name: String,
    pub execution_count: u64,
    pub avg_execution_time_ms: i32,
    pub success_rate: f64,
    #[serde(with = "time::serde::rfc3339")]
    pub last_used_at: OffsetDateTime,
}

impl SkillMetrics {
    /// Check if skill is popular (> 10 executions)
    pub fn is_popular(&self) -> bool {
        self.execution_count > 10
    }

    /// Check if skill needs optimization
    pub fn needs_optimization(&self) -> bool {
        self.success_rate < 0.9 || self.avg_execution_time_ms > 5000
    }
}

/// Overall system performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub total_commands: u64,
    pub total_errors: u64,
    pub avg_response_time_ms: f64,
    pub uptime_hours: f64,
    pub cache_metrics: CacheMetrics,
}

impl PerformanceMetrics {
    /// Calculate overall error rate (0.0 to 1.0)
    pub fn error_rate(&self) -> f64 {
        if self.total_commands == 0 {
            return 0.0;
        }
        self.total_errors as f64 / self.total_commands as f64
    }

    /// Check if system is healthy
    pub fn is_healthy(&self) -> bool {
        self.error_rate() < 0.05 && self.avg_response_time_ms < 2000.0
    }
}

/// System-wide metrics aggregation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetrics {
    pub performance: PerformanceMetrics,
    pub top_commands: Vec<CommandMetrics>,
    pub top_skills: Vec<SkillMetrics>,
    pub frequent_errors: Vec<ErrorMetrics>,
}

/// Error occurrence metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorMetrics {
    pub error_message: String,
    pub occurrence_count: u64,
    pub last_occurred_at: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_hit_rate() {
        let metrics = CacheMetrics {
            l1_hits: 70,
            l1_misses: 30,
            l2_hits: 20,
            l2_misses: 10,
            avg_response_time_ms: 50.0,
            total_requests: 130,
        };

        assert_eq!(metrics.l1_hit_rate(), 0.7);
        assert_eq!(metrics.l2_hit_rate(), 0.6666666666666666);
    }

    #[test]
    fn test_command_success_rate() {
        let metrics = CommandMetrics {
            command_name: "test".to_string(),
            execution_count: 100,
            success_count: 95,
            failure_count: 5,
            avg_execution_time_ms: 1500.0,
            min_execution_time_ms: 100,
            max_execution_time_ms: 5000,
            last_executed_at: OffsetDateTime::now_utc(),
        };

        assert_eq!(metrics.success_rate(), 0.95);
        assert!(!metrics.has_low_success_rate());
        assert!(!metrics.is_slow());
    }

    #[test]
    fn test_cache_overall_hit_rate() {
        let metrics = CacheMetrics {
            l1_hits: 100,
            l1_misses: 50,
            l2_hits: 30,
            l2_misses: 20,
            avg_response_time_ms: 25.0,
            total_requests: 200,
        };

        // Overall: (L1 hits + L2 hits) / total
        // (100 + 30) / (100 + 50 + 30 + 20) = 130 / 200 = 0.65
        assert_eq!(metrics.overall_hit_rate(), 0.65);
    }

    #[test]
    fn test_cache_zero_requests() {
        let metrics = CacheMetrics {
            l1_hits: 0,
            l1_misses: 0,
            l2_hits: 0,
            l2_misses: 0,
            avg_response_time_ms: 0.0,
            total_requests: 0,
        };

        assert_eq!(metrics.l1_hit_rate(), 0.0);
        assert_eq!(metrics.l2_hit_rate(), 0.0);
        assert_eq!(metrics.overall_hit_rate(), 0.0);
    }

    #[test]
    fn test_command_is_slow() {
        let slow = CommandMetrics {
            command_name: "slow".to_string(),
            execution_count: 10,
            success_count: 10,
            failure_count: 0,
            avg_execution_time_ms: 3000.0, // > 2000ms
            min_execution_time_ms: 2500,
            max_execution_time_ms: 3500,
            last_executed_at: OffsetDateTime::now_utc(),
        };

        assert!(slow.is_slow());

        let fast = CommandMetrics {
            command_name: "fast".to_string(),
            execution_count: 10,
            success_count: 10,
            failure_count: 0,
            avg_execution_time_ms: 500.0, // < 2000ms
            min_execution_time_ms: 100,
            max_execution_time_ms: 1000,
            last_executed_at: OffsetDateTime::now_utc(),
        };

        assert!(!fast.is_slow());
    }

    #[test]
    fn test_command_has_low_success_rate() {
        let unreliable = CommandMetrics {
            command_name: "unreliable".to_string(),
            execution_count: 100,
            success_count: 70,
            failure_count: 30,
            avg_execution_time_ms: 1000.0,
            min_execution_time_ms: 100,
            max_execution_time_ms: 2000,
            last_executed_at: OffsetDateTime::now_utc(),
        };

        assert!(unreliable.has_low_success_rate()); // 70% < 80%

        let reliable = CommandMetrics {
            command_name: "reliable".to_string(),
            execution_count: 100,
            success_count: 95,
            failure_count: 5,
            avg_execution_time_ms: 1000.0,
            min_execution_time_ms: 100,
            max_execution_time_ms: 2000,
            last_executed_at: OffsetDateTime::now_utc(),
        };

        assert!(!reliable.has_low_success_rate()); // 95% > 80%
    }

    #[test]
    fn test_skill_is_popular() {
        let popular = SkillMetrics {
            skill_name: "popular".to_string(),
            execution_count: 50, // > 10
            avg_execution_time_ms: 1000,
            success_rate: 0.95,
            last_used_at: OffsetDateTime::now_utc(),
        };

        assert!(popular.is_popular());

        let unpopular = SkillMetrics {
            skill_name: "unpopular".to_string(),
            execution_count: 5, // < 10
            avg_execution_time_ms: 1000,
            success_rate: 0.95,
            last_used_at: OffsetDateTime::now_utc(),
        };

        assert!(!unpopular.is_popular());
    }

    #[test]
    fn test_skill_needs_optimization() {
        let needs_opt_low_success = SkillMetrics {
            skill_name: "low_success".to_string(),
            execution_count: 20,
            avg_execution_time_ms: 1000,
            success_rate: 0.85, // < 0.9
            last_used_at: OffsetDateTime::now_utc(),
        };

        assert!(needs_opt_low_success.needs_optimization());

        let needs_opt_slow = SkillMetrics {
            skill_name: "slow".to_string(),
            execution_count: 20,
            avg_execution_time_ms: 6000, // > 5000
            success_rate: 0.95,
            last_used_at: OffsetDateTime::now_utc(),
        };

        assert!(needs_opt_slow.needs_optimization());

        let good = SkillMetrics {
            skill_name: "good".to_string(),
            execution_count: 20,
            avg_execution_time_ms: 1000, // < 5000
            success_rate: 0.95,          // > 0.9
            last_used_at: OffsetDateTime::now_utc(),
        };

        assert!(!good.needs_optimization());
    }

    #[test]
    fn test_performance_metrics_error_rate() {
        let metrics = PerformanceMetrics {
            total_commands: 1000,
            total_errors: 50,
            avg_response_time_ms: 500.0,
            uptime_hours: 24.0,
            cache_metrics: CacheMetrics {
                l1_hits: 100,
                l1_misses: 20,
                l2_hits: 10,
                l2_misses: 10,
                avg_response_time_ms: 25.0,
                total_requests: 140,
            },
        };

        assert_eq!(metrics.error_rate(), 0.05); // 50/1000 = 5%
    }

    #[test]
    fn test_performance_metrics_is_healthy() {
        let healthy = PerformanceMetrics {
            total_commands: 1000,
            total_errors: 30,             // 3% error rate < 5%
            avg_response_time_ms: 1500.0, // < 2000ms
            uptime_hours: 24.0,
            cache_metrics: CacheMetrics {
                l1_hits: 100,
                l1_misses: 20,
                l2_hits: 10,
                l2_misses: 10,
                avg_response_time_ms: 25.0,
                total_requests: 140,
            },
        };

        assert!(healthy.is_healthy());

        let unhealthy_errors = PerformanceMetrics {
            total_commands: 1000,
            total_errors: 60, // 6% error rate > 5%
            avg_response_time_ms: 1500.0,
            uptime_hours: 24.0,
            cache_metrics: CacheMetrics {
                l1_hits: 100,
                l1_misses: 20,
                l2_hits: 10,
                l2_misses: 10,
                avg_response_time_ms: 25.0,
                total_requests: 140,
            },
        };

        assert!(!unhealthy_errors.is_healthy());

        let unhealthy_slow = PerformanceMetrics {
            total_commands: 1000,
            total_errors: 30,
            avg_response_time_ms: 2500.0, // > 2000ms
            uptime_hours: 24.0,
            cache_metrics: CacheMetrics {
                l1_hits: 100,
                l1_misses: 20,
                l2_hits: 10,
                l2_misses: 10,
                avg_response_time_ms: 25.0,
                total_requests: 140,
            },
        };

        assert!(!unhealthy_slow.is_healthy());
    }
}
