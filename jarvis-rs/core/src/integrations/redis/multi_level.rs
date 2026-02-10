use super::DistributedCache;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Entry in L1 cache
struct CacheEntry {
    value: String,
    expires_at: Option<Instant>,
}

/// Multi-level cache (L1: Memory, L2: Redis)
pub struct MultiLevelCache {
    l1: Arc<RwLock<HashMap<String, CacheEntry>>>,
    l2: Option<Arc<dyn DistributedCache>>,
    l1_ttl: Duration,
    l2_ttl: Duration,
}

impl MultiLevelCache {
    /// Create a new multi-level cache
    pub fn new(
        l2: Option<Arc<dyn DistributedCache>>,
        l1_ttl: Duration,
        l2_ttl: Duration,
    ) -> Self {
        Self {
            l1: Arc::new(RwLock::new(HashMap::new())),
            l2,
            l1_ttl,
            l2_ttl,
        }
    }

    /// Get value from cache (tries L1, then L2) - type-safe version
    pub async fn get<T: for<'de> Deserialize<'de>>(&self, key: &str) -> Result<Option<T>> {
        // Try L1 first
        {
            let l1 = self.l1.read().await;
            if let Some(entry) = l1.get(key) {
                if let Some(expires_at) = entry.expires_at {
                    if expires_at > Instant::now() {
                        return Ok(Some(serde_json::from_str(&entry.value)?));
                    }
                } else {
                    return Ok(Some(serde_json::from_str(&entry.value)?));
                }
            }
        }

        // Try L2 if available
        if let Some(l2) = &self.l2 {
            if let Some(value) = l2.get_raw(key).await? {
                // Update L1 cache
                {
                    let mut l1 = self.l1.write().await;
                    l1.insert(
                        key.to_string(),
                        CacheEntry {
                            value: value.clone(),
                            expires_at: Some(Instant::now() + self.l1_ttl),
                        },
                    );
                }
                return Ok(Some(serde_json::from_str(&value)?));
            }
        }

        Ok(None)
    }

    /// Set value in both L1 and L2 caches - type-safe version
    pub async fn set<T: Serialize>(&self, key: &str, value: &T) -> Result<()> {
        let serialized = serde_json::to_string(value)?;

        // Store in L1
        {
            let mut l1 = self.l1.write().await;
            l1.insert(
                key.to_string(),
                CacheEntry {
                    value: serialized.clone(),
                    expires_at: Some(Instant::now() + self.l1_ttl),
                },
            );
        }

        // Store in L2 if available
        if let Some(l2) = &self.l2 {
            l2.set_raw(key, serialized, Some(self.l2_ttl)).await?;
        }

        Ok(())
    }

    /// Delete from both caches
    pub async fn delete(&self, key: &str) -> Result<()> {
        // Remove from L1
        {
            let mut l1 = self.l1.write().await;
            l1.remove(key);
        }

        // Remove from L2 if available
        if let Some(l2) = &self.l2 {
            l2.delete(key).await?;
        }

        Ok(())
    }

    /// Check if either cache has the key
    pub async fn exists(&self, key: &str) -> Result<bool> {
        // Check L1 first
        {
            let l1 = self.l1.read().await;
            if l1.contains_key(key) {
                return Ok(true);
            }
        }

        // Check L2 if available
        if let Some(l2) = &self.l2 {
            return l2.exists(key).await;
        }

        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    /// Mock DistributedCache for testing
    struct MockDistributedCache {
        data: Arc<RwLock<HashMap<String, String>>>,
    }

    impl MockDistributedCache {
        fn new() -> Self {
            Self {
                data: Arc::new(RwLock::new(HashMap::new())),
            }
        }
    }

    #[async_trait::async_trait]
    impl DistributedCache for MockDistributedCache {
        fn get_raw<'a>(&'a self, key: &'a str) -> super::super::cache::BoxFuture<'a, Result<Option<String>>> {
            Box::pin(async move {
                let data = self.data.read().await;
                Ok(data.get(key).cloned())
            })
        }

        fn set_raw<'a>(
            &'a self,
            key: &'a str,
            value: String,
            _ttl: Option<Duration>,
        ) -> super::super::cache::BoxFuture<'a, Result<()>> {
            Box::pin(async move {
                let mut data = self.data.write().await;
                data.insert(key.to_string(), value);
                Ok(())
            })
        }

        async fn delete(&self, key: &str) -> Result<()> {
            let mut data = self.data.write().await;
            data.remove(key);
            Ok(())
        }

        async fn exists(&self, key: &str) -> Result<bool> {
            let data = self.data.read().await;
            Ok(data.contains_key(key))
        }

        async fn increment(&self, _key: &str, _amount: i64) -> Result<i64> {
            Ok(0)
        }

        async fn expire(&self, _key: &str, _ttl: Duration) -> Result<()> {
            Ok(())
        }

        async fn is_available(&self) -> bool {
            true
        }
    }

    #[tokio::test]
    async fn test_multi_level_cache_l1_hit() {
        let l2 = Arc::new(MockDistributedCache::new());
        let cache = MultiLevelCache::new(
            Some(l2),
            Duration::from_secs(60),
            Duration::from_secs(300),
        );

        // Store value
        cache.set("key1", &"value1").await.unwrap();

        // Should hit L1
        let result: Option<String> = cache.get("key1").await.unwrap();
        assert_eq!(result, Some("value1".to_string()));
    }

    #[tokio::test]
    async fn test_multi_level_cache_l1_miss_l2_hit() {
        let l2 = Arc::new(MockDistributedCache::new());

        // Pre-populate L2
        l2.set_raw("key2", "\"value2\"".to_string(), None).await.unwrap();

        let cache = MultiLevelCache::new(
            Some(l2.clone()),
            Duration::from_secs(60),
            Duration::from_secs(300),
        );

        // L1 miss, L2 hit
        let result: Option<String> = cache.get("key2").await.unwrap();
        assert_eq!(result, Some("value2".to_string()));

        // Verify it was promoted to L1 by getting again
        let result2: Option<String> = cache.get("key2").await.unwrap();
        assert_eq!(result2, Some("value2".to_string()));
    }

    #[tokio::test]
    async fn test_multi_level_cache_both_miss() {
        let l2 = Arc::new(MockDistributedCache::new());
        let cache = MultiLevelCache::new(
            Some(l2),
            Duration::from_secs(60),
            Duration::from_secs(300),
        );

        // Both L1 and L2 miss
        let result: Option<String> = cache.get("nonexistent").await.unwrap();
        assert_eq!(result, None);
    }

    #[tokio::test]
    async fn test_multi_level_cache_no_l2() {
        let cache = MultiLevelCache::new(
            None,
            Duration::from_secs(60),
            Duration::from_secs(300),
        );

        // Set and get with L1 only
        cache.set("key3", &"value3").await.unwrap();
        let result: Option<String> = cache.get("key3").await.unwrap();
        assert_eq!(result, Some("value3".to_string()));
    }

    #[tokio::test]
    async fn test_multi_level_cache_delete() {
        let l2 = Arc::new(MockDistributedCache::new());
        let cache = MultiLevelCache::new(
            Some(l2.clone()),
            Duration::from_secs(60),
            Duration::from_secs(300),
        );

        // Set value
        cache.set("key4", &"value4").await.unwrap();

        // Verify exists
        assert!(cache.exists("key4").await.unwrap());

        // Delete
        cache.delete("key4").await.unwrap();

        // Verify deleted
        assert!(!cache.exists("key4").await.unwrap());
        let result: Option<String> = cache.get("key4").await.unwrap();
        assert_eq!(result, None);
    }

    #[tokio::test]
    async fn test_multi_level_cache_exists_l1() {
        let cache = MultiLevelCache::new(
            None,
            Duration::from_secs(60),
            Duration::from_secs(300),
        );

        cache.set("key5", &"value5").await.unwrap();
        assert!(cache.exists("key5").await.unwrap());
        assert!(!cache.exists("nonexistent").await.unwrap());
    }

    #[tokio::test]
    async fn test_multi_level_cache_exists_l2() {
        let l2 = Arc::new(MockDistributedCache::new());

        // Pre-populate L2 only
        l2.set_raw("key6", "\"value6\"".to_string(), None).await.unwrap();

        let cache = MultiLevelCache::new(
            Some(l2),
            Duration::from_secs(60),
            Duration::from_secs(300),
        );

        // Should find in L2
        assert!(cache.exists("key6").await.unwrap());
    }

    #[tokio::test]
    async fn test_multi_level_cache_l1_expiration() {
        let l2 = Arc::new(MockDistributedCache::new());
        let cache = MultiLevelCache::new(
            Some(l2),
            Duration::from_millis(50), // Very short L1 TTL
            Duration::from_secs(300),
        );

        // Set value
        cache.set("key7", &"value7").await.unwrap();

        // Should hit L1
        let result1: Option<String> = cache.get("key7").await.unwrap();
        assert_eq!(result1, Some("value7".to_string()));

        // Wait for L1 expiration
        tokio::time::sleep(Duration::from_millis(100)).await;

        // L1 expired, should get from L2
        let result2: Option<String> = cache.get("key7").await.unwrap();
        assert_eq!(result2, Some("value7".to_string()));
    }

    #[tokio::test]
    async fn test_multi_level_cache_type_safety() {
        let cache = MultiLevelCache::new(
            None,
            Duration::from_secs(60),
            Duration::from_secs(300),
        );

        // Test with different types
        #[derive(Serialize, Deserialize, PartialEq, Debug)]
        struct TestData {
            id: u32,
            name: String,
        }

        let data = TestData {
            id: 42,
            name: "test".to_string(),
        };

        cache.set("struct_key", &data).await.unwrap();
        let result: Option<TestData> = cache.get("struct_key").await.unwrap();
        assert_eq!(result, Some(data));
    }

    #[tokio::test]
    async fn test_multi_level_cache_concurrent_access() {
        let l2 = Arc::new(MockDistributedCache::new());
        let cache = Arc::new(MultiLevelCache::new(
            Some(l2),
            Duration::from_secs(60),
            Duration::from_secs(300),
        ));

        // Spawn multiple tasks
        let mut handles = vec![];
        for i in 0..10 {
            let cache_clone = cache.clone();
            let handle = tokio::spawn(async move {
                let key = format!("key{}", i);
                let value = format!("value{}", i);
                cache_clone.set(&key, &value).await.unwrap();
                let result: Option<String> = cache_clone.get(&key).await.unwrap();
                assert_eq!(result, Some(value));
            });
            handles.push(handle);
        }

        // Wait for all tasks
        for handle in handles {
            handle.await.unwrap();
        }
    }
}
