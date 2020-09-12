use std::{
    collections::HashMap,
    time::{Duration, SystemTime},
    vec::Vec,
};

struct CacheEntry {
    val: String,
    put_time: SystemTime,
}

impl CacheEntry {
    pub fn expired(&self, max_cache_entry_lifetime: Duration) -> bool {
        let cache_entry_lifetime = match self.put_time.elapsed() {
            Ok(lifetime) => lifetime,
            Err(err) => {
                eprintln!(
                    "LRU Cache recieved unexpected error from system
                     clock when evaluating cache timeout {} 
                     - Defaulting to expired",
                    err
                );
                return false;
            }
        };
        cache_entry_lifetime > max_cache_entry_lifetime
    }
}

/*
 * This trait defines the interface throuhgh which our consumer can
 * get and set data from the Cache
 */
pub trait Cache {
    fn get(&self, key: &String) -> Option<String>;
    fn put(&mut self, key: &String, val: String);
}

pub struct LRUCache {
    //TODO evaluate what syncronization primitives are necessary
    //to ensure there arent weird races between the map and the vec
    cache_elements: HashMap<String, CacheEntry>,
    time_ordered_cache_keys: Vec<String>,
    max_cache_entry_lifetime: Duration,
}

impl Cache for LRUCache {
    fn get(&self, key: &String) -> Option<String> {
        match self.cache_elements.get(key) {
            Some(entry) => {
                if entry.expired(self.max_cache_entry_lifetime) {
                    return None;
                }
                Some(entry.val.clone())
            }
            None => None,
        }
    }

    fn put(&mut self, key: &String, val: String) {
        //TODO handle LRU eviction
        let put_time = SystemTime::now();
        self.cache_elements
            .insert(key.clone(), CacheEntry { val, put_time });
    }
}

impl LRUCache {
    pub fn new(capacity: usize, max_cache_entry_lifetime: Duration) -> LRUCache {
        LRUCache {
            cache_elements: HashMap::with_capacity(capacity),
            time_ordered_cache_keys: Vec::with_capacity(capacity),
            max_cache_entry_lifetime,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::lru_cache::*;

    #[test]
    fn test_allocation() {
        LRUCache::new(10, Duration::from_secs(1));
    }
    #[test]
    fn test_empty_get() {
        let cache = LRUCache::new(10, Duration::from_secs(1));
        let key = String::from("foo");
        assert_eq!(cache.get(&key), None);
    }
    #[test]
    fn test_put_and_get() {
        let mut cache = LRUCache::new(10, Duration::from_secs(1));
        let key = String::from("foo");
        let expected_value = String::from("bar");
        cache.put(&key, expected_value.clone());
        assert_eq!(cache.get(&key), Some(expected_value));
    }
    #[test]
    fn test_lru_eviction() {
        assert_eq!(1, 1)
    }
    #[test]
    fn test_timeout() {
        let timeout_duration = Duration::from_secs(1);
        let mut cache = LRUCache::new(10, timeout_duration);
        let key = String::from("foo");
        let expected_value = String::from("bar");
        cache.put(&key, expected_value.clone());
        assert_eq!(cache.get(&key), Some(expected_value));
        std::thread::sleep(timeout_duration);
        assert_eq!(cache.get(&key), None);
    }
}
