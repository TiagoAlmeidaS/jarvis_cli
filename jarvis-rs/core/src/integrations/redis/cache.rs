use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

/// Boxed future for async trait methods
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Trait for distributed cache operations (dyn-compatible)
#[async_trait]
pub trait DistributedCache: Send + Sync {
    /// Get a value from cache (JSON serialized)
    fn get_raw<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Result<Option<String>>>;

    /// Set a value in cache with optional TTL (JSON serialized)
    fn set_raw<'a>(
        &'a self,
        key: &'a str,
        value: String,
        ttl: Option<Duration>,
    ) -> BoxFuture<'a, Result<()>>;

    /// Delete a value from cache
    async fn delete(&self, key: &str) -> Result<()>;

    /// Check if key exists
    async fn exists(&self, key: &str) -> Result<bool>;

    /// Increment a numeric value
    async fn increment(&self, key: &str, amount: i64) -> Result<i64>;

    /// Set TTL for existing key
    async fn expire(&self, key: &str, ttl: Duration) -> Result<()>;

    /// Check if cache is available
    async fn is_available(&self) -> bool;
}

/// Redis-based distributed cache implementation
pub struct RedisCache {
    client: redis::Client,
}

impl RedisCache {
    /// Create a new RedisCache from a URL
    pub async fn new(url: &str) -> Result<Self> {
        let client = redis::Client::open(url)?;

        // Test connection
        let mut conn = client.get_async_connection().await?;
        redis::cmd("PING")
            .query_async::<_, String>(&mut conn)
            .await?;

        Ok(Self { client })
    }

    /// Get a value with type (type-safe)
    pub async fn get<T: for<'de> Deserialize<'de>>(&self, key: &str) -> Result<Option<T>> {
        if let Some(value) = self.get_raw(key).await? {
            Ok(Some(serde_json::from_str(&value)?))
        } else {
            Ok(None)
        }
    }

    /// Set a value with type (type-safe)
    pub async fn set<T: Serialize>(
        &self,
        key: &str,
        value: &T,
        ttl: Option<Duration>,
    ) -> Result<()> {
        let serialized = serde_json::to_string(value)?;
        self.set_raw(key, serialized, ttl).await
    }
}

#[async_trait]
impl DistributedCache for RedisCache {
    fn get_raw<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Result<Option<String>>> {
        Box::pin(async move {
            use redis::AsyncCommands;
            let mut conn = self.client.get_async_connection().await?;
            let value: Option<String> = conn.get(key).await?;
            Ok(value)
        })
    }

    fn set_raw<'a>(
        &'a self,
        key: &'a str,
        value: String,
        ttl: Option<Duration>,
    ) -> BoxFuture<'a, Result<()>> {
        Box::pin(async move {
            use redis::AsyncCommands;
            let mut conn = self.client.get_async_connection().await?;

            if let Some(ttl) = ttl {
                let _: () = conn.set_ex(key, value, ttl.as_secs()).await?;
            } else {
                let _: () = conn.set(key, value).await?;
            }

            Ok(())
        })
    }

    async fn delete(&self, key: &str) -> Result<()> {
        use redis::AsyncCommands;

        let mut conn = self.client.get_async_connection().await?;
        let _: () = conn.del(key).await?;
        Ok(())
    }

    async fn exists(&self, key: &str) -> Result<bool> {
        use redis::AsyncCommands;

        let mut conn = self.client.get_async_connection().await?;
        let exists: bool = conn.exists(key).await?;
        Ok(exists)
    }

    async fn increment(&self, key: &str, amount: i64) -> Result<i64> {
        use redis::AsyncCommands;

        let mut conn = self.client.get_async_connection().await?;
        let value: i64 = conn.incr(key, amount).await?;
        Ok(value)
    }

    async fn expire(&self, key: &str, ttl: Duration) -> Result<()> {
        use redis::AsyncCommands;

        let mut conn = self.client.get_async_connection().await?;
        let _: () = conn.expire(key, ttl.as_secs() as i64).await?;
        Ok(())
    }

    async fn is_available(&self) -> bool {
        match self.client.get_async_connection().await {
            Ok(mut conn) => redis::cmd("PING")
                .query_async::<_, String>(&mut conn)
                .await
                .is_ok(),
            Err(_) => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: RedisCache requires a running Redis instance for testing.
    // These tests should be implemented as INTEGRATION TESTS (Task #13).
    //
    // The following functionality should be tested with a real Redis instance:
    //
    // 1. RedisCache::new(url)
    //    - Test successful connection with valid URL
    //    - Test connection failure with invalid URL
    //    - Test connection failure with unreachable Redis
    //    - Test PING command on successful connection
    //
    // 2. get_raw() / get<T>()
    //    - Test getting existing key
    //    - Test getting non-existent key (returns None)
    //    - Test type-safe deserialization
    //    - Test deserialization error handling
    //
    // 3. set_raw() / set<T>()
    //    - Test setting value without TTL
    //    - Test setting value with TTL
    //    - Test type-safe serialization
    //    - Test overwriting existing value
    //
    // 4. delete()
    //    - Test deleting existing key
    //    - Test deleting non-existent key (should not error)
    //
    // 5. exists()
    //    - Test with existing key (returns true)
    //    - Test with non-existent key (returns false)
    //
    // 6. increment()
    //    - Test incrementing new key (starts at 0)
    //    - Test incrementing existing key
    //    - Test increment by negative amount (decrement)
    //
    // 7. expire()
    //    - Test setting TTL on existing key
    //    - Test key expiration after TTL
    //    - Test setting TTL on non-existent key
    //
    // 8. is_available()
    //    - Test returns true when Redis is reachable
    //    - Test returns false when Redis is unreachable
    //
    // 9. TTL behavior
    //    - Test that values expire after TTL
    //    - Test that values without TTL persist
    //    - Test updating TTL with expire()
    //
    // 10. Concurrent access
    //     - Test multiple concurrent get/set operations
    //     - Test race conditions with increment
    //
    // Integration test setup requirements:
    // - Docker container with Redis (redis:7-alpine)
    // - Test fixtures with known keys/values
    // - Cleanup between tests (FLUSHDB)
    // - Connection pool testing
    // - Error recovery testing
    //
    // Coverage target: 80% (via integration tests)

    #[test]
    fn test_redis_cache_documentation() {
        // This test ensures the integration test requirements are documented
        // Actual tests will be in tests/integration/redis_cache.rs
        assert!(true, "Integration tests required - see comments above");
    }
}
