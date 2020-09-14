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
                    "LRU Cache received unexpected error from system
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
 * This trait defines the interface through which our consumer can
 * get and set data from the Cache
 */
pub trait Cache {
    fn get(&mut self, key: &String) -> Option<String>;
    fn put(&mut self, key: &String, val: String);
}

pub struct LRUCache {
    //TODO evaluate what synchronization primitives are necessary
    //to ensure there aren't weird races between the map and the vec
    cache_elements: HashMap<String, CacheEntry>,
    keys_ordered_by_use: Vec<String>,
    max_cache_entry_lifetime: Duration,
    capacity: usize,
}

impl Cache for LRUCache {
    fn get(&mut self, key: &String) -> Option<String> {
        let entry = self.cache_elements.get(key);

        match entry {
            Some(e) => {
                if e.expired(self.max_cache_entry_lifetime) {
                    self.remove_expired_element(key);
                    return None;
                }
                //we need to clone val and release 'entry'
                //first because entry holds an immutable borrow to
                //self. and mark_key_used needs a mutable borrow
                let val = e.val.clone();
                self.mark_key_used(key);
                Some(val)
            }
            None => None,
        }
    }

    fn put(&mut self, key: &String, val: String) {
        if self.cache_elements.contains_key(key) {
            eprintln!("unexpected double write of {} - ignoring", key);
            return;
        }
        if self.cache_elements.len() == self.capacity {
            self.remove_oldest_element();
        }

        let put_time = SystemTime::now();
        self.cache_elements
            .insert(key.clone(), CacheEntry { val, put_time });
        //insert into the front of the vec
        self.keys_ordered_by_use.insert(0, key.clone());
    }
}

impl LRUCache {
    pub fn new(capacity: usize, max_cache_entry_lifetime: Duration) -> LRUCache {
        LRUCache {
            cache_elements: HashMap::with_capacity(capacity),
            keys_ordered_by_use: Vec::with_capacity(capacity),
            max_cache_entry_lifetime,
            capacity,
        }
    }

    fn remove_oldest_element(&mut self) {
        let oldest_elm_key = self
            .keys_ordered_by_use
            .pop()
            .expect("LRU Remove called on empty cache");
        self.cache_elements.remove(&oldest_elm_key);
    }

    /*
     * When a cache entry is expired we must remove it to preserve the
     * ordering of the keys_ordered_by_use
     */
    fn remove_expired_element(&mut self, key: &String) {
        let index_of_expired_key = self
            .keys_ordered_by_use
            .iter()
            .position(|k| k == key) /* todo - whats the complexity of this */
            .expect("Expired key missing from keys_ordered_by_use");
        self.keys_ordered_by_use.remove(index_of_expired_key);
        self.cache_elements.remove(key);
    }

    fn mark_key_used(&mut self, key: &String) {
        let index_of_used_key = self
            .keys_ordered_by_use
            .iter()
            .position(|k| k == key) /* todo - whats the complexity of this */
            .expect("Used key missing from keys_ordered_by_use");

        let elm = self.keys_ordered_by_use.remove(index_of_used_key);
        self.keys_ordered_by_use.insert(0, elm);
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
        let mut cache = LRUCache::new(10, Duration::from_secs(1));
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
        let mut cache = LRUCache::new(1, Duration::from_secs(1));
        let key = String::from("foo");
        let expected_value = String::from("bar");
        cache.put(&key, expected_value.clone());
        assert_eq!(cache.get(&key), Some(expected_value));

        let key2 = String::from("baz");
        let expected_value2 = String::from("bazoink!");
        cache.put(&key2, expected_value2.clone());
        assert_eq!(cache.get(&key2), Some(expected_value2));
        assert_eq!(cache.get(&key), None);
    }

    #[test]
    fn test_lru_ordering() {
        let mut cache = LRUCache::new(10, Duration::from_secs(1));
        let key = String::from("foo");
        let expected_value = String::from("bar");
        cache.put(&key, expected_value.clone());

        let key2 = String::from("baz");
        let expected_value2 = String::from("bazoink!");
        cache.put(&key2, expected_value2.clone());
        // Expected order of keys : new [key2, key] old
        assert_eq!(cache.keys_ordered_by_use[0], key2);
        assert_eq!(cache.keys_ordered_by_use[1], key);

        cache.get(&key);
        // Expected order of keys : new [key, key2] old
        assert_eq!(cache.keys_ordered_by_use[0], key);
        assert_eq!(cache.keys_ordered_by_use[1], key2);
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
