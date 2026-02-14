use crate::integrations::redis::MultiLevelCache;
use crate::integrations::sqlserver::Database;
use anyhow::Result;

use super::metrics::CacheMetrics;
use super::queries::AnalyticsQueries;
use super::suggestions::{Improvement, ImprovementCategory};

/// Self-improvement system that analyzes Jarvis performance and suggests optimizations
pub struct SelfImprovement {
    queries: AnalyticsQueries,
    cache: Option<MultiLevelCache>,
}

impl SelfImprovement {
    /// Create a new SelfImprovement instance
    pub fn new(db: Database, cache: Option<MultiLevelCache>) -> Self {
        Self {
            queries: AnalyticsQueries::new(db),
            cache,
        }
    }

    /// Analyze system performance and generate improvement suggestions
    pub async fn analyze_and_suggest(&self) -> Result<Vec<Improvement>> {
        let mut improvements = Vec::new();

        // 1. Analyze slow commands
        improvements.extend(self.analyze_slow_commands().await?);

        // 2. Analyze unreliable commands
        improvements.extend(self.analyze_unreliable_commands().await?);

        // 3. Analyze frequent errors
        improvements.extend(self.analyze_frequent_errors().await?);

        // 4. Analyze cache performance
        if self.cache.is_some() {
            improvements.extend(self.analyze_cache_performance().await?);
        }

        // 5. Analyze skill usage
        improvements.extend(self.analyze_skill_usage().await?);

        Ok(improvements)
    }

    /// Analyze slow commands (execution time > 2000ms)
    async fn analyze_slow_commands(&self) -> Result<Vec<Improvement>> {
        let slow_commands = self.queries.get_slow_commands(2000, 10).await?;
        let mut improvements = Vec::new();

        for cmd in slow_commands {
            let improvement = Improvement::high(
                ImprovementCategory::Performance,
                format!("Comando '{}' está lento", cmd.command_name),
                format!(
                    "Tempo médio de execução: {:.0}ms (min: {}ms, max: {}ms, {} execuções)",
                    cmd.avg_execution_time_ms,
                    cmd.min_execution_time_ms,
                    cmd.max_execution_time_ms,
                    cmd.execution_count
                ),
            )
            .with_action(format!(
                "Considerar otimização ou cache para '{}'",
                cmd.command_name
            ))
            .with_impact(format!(
                "Reduzir tempo de execução em ~{:.0}ms por comando",
                cmd.avg_execution_time_ms - 500.0
            ));

            improvements.push(improvement);
        }

        Ok(improvements)
    }

    /// Analyze unreliable commands (success rate < 80%)
    async fn analyze_unreliable_commands(&self) -> Result<Vec<Improvement>> {
        let unreliable_commands = self.queries.get_unreliable_commands(5, 10).await?;
        let mut improvements = Vec::new();

        for cmd in unreliable_commands {
            let success_rate_pct = cmd.success_rate() * 100.0;

            let priority = if success_rate_pct < 50.0 {
                super::suggestions::ImprovementPriority::Critical
            } else if success_rate_pct < 70.0 {
                super::suggestions::ImprovementPriority::High
            } else {
                super::suggestions::ImprovementPriority::Medium
            };

            let improvement = Improvement {
                priority,
                category: ImprovementCategory::Reliability,
                title: format!("Comando '{}' tem baixa taxa de sucesso", cmd.command_name),
                description: format!(
                    "Taxa de sucesso: {:.1}% ({} sucessos, {} falhas em {} execuções)",
                    success_rate_pct, cmd.success_count, cmd.failure_count, cmd.execution_count
                ),
                action: Some(format!(
                    "Investigar e corrigir erros em '{}'",
                    cmd.command_name
                )),
                impact: Some(format!("Aumentar taxa de sucesso para >90% (meta: 95%)")),
            };

            improvements.push(improvement);
        }

        Ok(improvements)
    }

    /// Analyze frequent errors
    async fn analyze_frequent_errors(&self) -> Result<Vec<Improvement>> {
        let frequent_errors = self.queries.get_frequent_errors(5, 10).await?;
        let mut improvements = Vec::new();

        for error in frequent_errors {
            let priority = if error.occurrence_count > 20 {
                super::suggestions::ImprovementPriority::Critical
            } else if error.occurrence_count > 10 {
                super::suggestions::ImprovementPriority::High
            } else {
                super::suggestions::ImprovementPriority::Medium
            };

            let improvement = Improvement {
                priority,
                category: ImprovementCategory::Reliability,
                title: "Erro frequente detectado".to_string(),
                description: format!(
                    "Erro: '{}' (ocorreu {} vezes)",
                    Self::truncate_error(&error.error_message, 80),
                    error.occurrence_count
                ),
                action: Some("Investigar causa raiz e adicionar tratamento de erro".to_string()),
                impact: Some(format!(
                    "Eliminar {} ocorrências deste erro",
                    error.occurrence_count
                )),
            };

            improvements.push(improvement);
        }

        Ok(improvements)
    }

    /// Analyze cache performance
    async fn analyze_cache_performance(&self) -> Result<Vec<Improvement>> {
        let mut improvements = Vec::new();

        // Mock cache metrics for now
        // In real implementation, get from cache.get_metrics()
        let cache_metrics = CacheMetrics {
            l1_hits: 450,
            l1_misses: 550,
            l2_hits: 200,
            l2_misses: 350,
            avg_response_time_ms: 25.0,
            total_requests: 1550,
        };

        let l1_hit_rate = cache_metrics.l1_hit_rate();
        let overall_hit_rate = cache_metrics.overall_hit_rate();

        // L1 cache hit rate too low
        if l1_hit_rate < 0.7 {
            let improvement = Improvement::medium(
                ImprovementCategory::Cache,
                "L1 cache hit rate baixo".to_string(),
                format!("L1 hit rate: {:.1}% (ideal: >70%)", l1_hit_rate * 100.0),
            )
            .with_action("Aumentar L1 TTL ou tamanho do cache".to_string())
            .with_impact(format!(
                "Melhorar hit rate em ~{:.0} pontos percentuais",
                (0.75 - l1_hit_rate) * 100.0
            ));

            improvements.push(improvement);
        }

        // Overall cache hit rate
        if overall_hit_rate < 0.6 {
            let improvement = Improvement::high(
                ImprovementCategory::Cache,
                "Taxa geral de cache hit baixa".to_string(),
                format!(
                    "Hit rate geral: {:.1}% (ideal: >80%)",
                    overall_hit_rate * 100.0
                ),
            )
            .with_action("Revisar estratégia de caching e TTLs".to_string())
            .with_impact("Reduzir latência média em 50-70%".to_string());

            improvements.push(improvement);
        }

        Ok(improvements)
    }

    /// Analyze skill usage patterns
    async fn analyze_skill_usage(&self) -> Result<Vec<Improvement>> {
        let skills_needing_optimization = self.queries.get_skills_needing_optimization(10).await?;
        let mut improvements = Vec::new();

        for skill in skills_needing_optimization {
            let improvement = Improvement::medium(
                ImprovementCategory::Skills,
                format!("Skill '{}' precisa de otimização", skill.skill_name),
                format!(
                    "Taxa de sucesso: {:.1}%, Tempo médio: {}ms ({} execuções)",
                    skill.success_rate * 100.0,
                    skill.avg_execution_time_ms,
                    skill.execution_count
                ),
            )
            .with_action(format!(
                "Otimizar performance e confiabilidade de '{}'",
                skill.skill_name
            ))
            .with_impact("Melhorar experiência do usuário com esta skill".to_string());

            improvements.push(improvement);
        }

        // Suggest caching for popular skills
        let top_skills = self.queries.get_top_skills(5).await?;
        for skill in top_skills {
            if skill.is_popular() && skill.avg_execution_time_ms > 1000 {
                let improvement = Improvement::low(
                    ImprovementCategory::Skills,
                    format!("Skill popular '{}' poderia ter cache", skill.skill_name),
                    format!(
                        "Executada {} vezes com tempo médio de {}ms",
                        skill.execution_count, skill.avg_execution_time_ms
                    ),
                )
                .with_action("Implementar cache específico para resultados desta skill".to_string())
                .with_impact(format!(
                    "Economizar ~{:.0}s de tempo total de execução",
                    (skill.execution_count as f64 * skill.avg_execution_time_ms as f64) / 1000.0
                ));

                improvements.push(improvement);
            }
        }

        Ok(improvements)
    }

    /// Truncate long error messages
    fn truncate_error(error: &str, max_len: usize) -> String {
        if error.len() <= max_len {
            error.to_string()
        } else {
            format!("{}...", &error[..max_len])
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_error() {
        let long_error = "This is a very long error message that should be truncated to fit within the specified length limit";
        let truncated = SelfImprovement::truncate_error(long_error, 50);
        assert!(truncated.len() <= 53); // 50 + "..."
        assert!(truncated.ends_with("..."));
    }

    #[test]
    fn test_truncate_error_short_message() {
        let short_error = "Short error";
        let truncated = SelfImprovement::truncate_error(short_error, 50);
        assert_eq!(truncated, "Short error");
        assert!(!truncated.ends_with("..."));
    }

    #[test]
    fn test_truncate_error_exact_length() {
        let exact_error = "x".repeat(50);
        let truncated = SelfImprovement::truncate_error(&exact_error, 50);
        assert_eq!(truncated.len(), 50);
        assert!(!truncated.ends_with("..."));
    }

    #[test]
    fn test_truncate_error_zero_length() {
        let error = "Some error";
        let truncated = SelfImprovement::truncate_error(error, 0);
        assert_eq!(truncated, "...");
    }

    // Note: The following tests would require mocking Database and MultiLevelCache
    // These should be implemented as integration tests instead:
    // - test_analyze_slow_commands()
    // - test_analyze_unreliable_commands()
    // - test_analyze_frequent_errors()
    // - test_analyze_cache_performance()
    // - test_analyze_skill_usage()
    // - test_analyze_and_suggest()
    //
    // Integration tests will verify:
    // - Query execution and result parsing
    // - Priority assignment logic (Critical for >20 errors, High for >10, etc.)
    // - Improvement generation with correct categories
    // - Cache metrics analysis thresholds
    // - Skill optimization detection criteria
}
