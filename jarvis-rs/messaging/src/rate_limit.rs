//! Rate limiting para mensageria

use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

/// Rate limiter simples baseado em token bucket
pub struct RateLimiter {
    limits: Mutex<HashMap<String, Vec<Instant>>>,
    max_requests: usize,
    window_seconds: u64,
}

impl RateLimiter {
    pub fn new(max_requests: usize, window_seconds: u64) -> Self {
        Self {
            limits: Mutex::new(HashMap::new()),
            max_requests,
            window_seconds,
        }
    }

    /// Verifica se uma requisição pode ser processada
    pub async fn check(&self, key: &str) -> bool {
        let mut limits = self.limits.lock().await;
        let now = Instant::now();
        let window = Duration::from_secs(self.window_seconds);

        // Limpa requisições antigas
        let timestamps = limits.entry(key.to_string()).or_insert_with(Vec::new);
        timestamps.retain(|&timestamp| now.duration_since(timestamp) < window);

        // Verifica se excedeu o limite
        if timestamps.len() >= self.max_requests {
            return false;
        }

        // Adiciona nova requisição
        timestamps.push(now);
        true
    }

    /// Limpa entradas antigas periodicamente
    pub async fn cleanup(&self) {
        let mut limits = self.limits.lock().await;
        let now = Instant::now();
        let window = Duration::from_secs(self.window_seconds);

        limits.retain(|_, timestamps| {
            timestamps.retain(|&timestamp| now.duration_since(timestamp) < window);
            !timestamps.is_empty()
        });
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        // Padrão: 10 requisições por minuto por chat
        Self::new(10, 60)
    }
}
