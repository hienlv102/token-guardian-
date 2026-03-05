use anyhow::Result;
use sha2::{Digest, Sha256};
use std::time::{SystemTime, UNIX_EPOCH};

pub struct ReasoningCache {
    db: sled::Db,
    ttl_seconds: u64,
}

impl ReasoningCache {
    /// Open or create a cache store at the given path.
    pub fn new(path: &str, ttl_seconds: u64) -> Result<Self> {
        let db = sled::open(path)?;
        Ok(Self { db, ttl_seconds })
    }

    /// Generate a cache key from task description and file paths.
    pub fn make_key(task: &str, files: &[&str]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(task.as_bytes());
        for f in files {
            hasher.update(b"|");
            hasher.update(f.as_bytes());
        }
        let hash = hasher.finalize();
        hex::encode(&hash[..16]) // 128-bit key is sufficient
    }

    /// Get a cached value if it exists and hasn't expired.
    pub fn get(&self, key: &str) -> Option<String> {
        let bytes = self.db.get(key).ok()??;
        let (timestamp, data): (u64, String) = bincode::deserialize(&bytes).ok()?;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .ok()?
            .as_secs();
        if now - timestamp < self.ttl_seconds {
            Some(data)
        } else {
            // Expired — remove it
            let _ = self.db.remove(key);
            None
        }
    }

    /// Store a value in the cache.
    pub fn set(&self, key: &str, value: &str) -> Result<()> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_secs();
        let data = bincode::serialize(&(timestamp, value.to_string()))?;
        self.db.insert(key, data)?;
        Ok(())
    }

    /// Remove a specific key.
    #[allow(dead_code)]
    pub fn invalidate(&self, key: &str) -> Result<()> {
        self.db.remove(key)?;
        Ok(())
    }

    /// Clear all cached data.
    pub fn clear(&self) -> Result<()> {
        self.db.clear()?;
        Ok(())
    }

    /// Get cache statistics.
    #[allow(dead_code)]
    pub fn stats(&self) -> (usize, u64) {
        let count = self.db.len();
        let size = self.db.size_on_disk().unwrap_or(0);
        (count, size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_cache(ttl: u64) -> ReasoningCache {
        let dir = tempfile::tempdir().unwrap();
        ReasoningCache::new(dir.path().to_str().unwrap(), ttl).unwrap()
    }

    #[test]
    fn test_set_and_get() {
        let cache = temp_cache(3600);
        cache.set("key1", "value1").unwrap();
        assert_eq!(cache.get("key1"), Some("value1".to_string()));
    }

    #[test]
    fn test_get_missing() {
        let cache = temp_cache(3600);
        assert_eq!(cache.get("nonexistent"), None);
    }

    #[test]
    fn test_expired() {
        let cache = temp_cache(0); // 0 second TTL = immediate expiry
        cache.set("key1", "value1").unwrap();
        // Should be expired immediately
        assert_eq!(cache.get("key1"), None);
    }

    #[test]
    fn test_invalidate() {
        let cache = temp_cache(3600);
        cache.set("key1", "value1").unwrap();
        cache.invalidate("key1").unwrap();
        assert_eq!(cache.get("key1"), None);
    }

    #[test]
    fn test_clear() {
        let cache = temp_cache(3600);
        cache.set("key1", "value1").unwrap();
        cache.set("key2", "value2").unwrap();
        cache.clear().unwrap();
        assert_eq!(cache.get("key1"), None);
        assert_eq!(cache.get("key2"), None);
    }

    #[test]
    fn test_make_key() {
        let k1 = ReasoningCache::make_key("fix bug", &["main.rs"]);
        let k2 = ReasoningCache::make_key("fix bug", &["main.rs"]);
        let k3 = ReasoningCache::make_key("add feature", &["main.rs"]);
        assert_eq!(k1, k2);
        assert_ne!(k1, k3);
    }
}
