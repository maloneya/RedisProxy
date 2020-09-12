/*
 * This trait defines the interface throuhgh which our consumer can
 * get data from the Cache
 */

pub trait Cache {
    fn get(&self);
}

pub struct LRUCache;

impl Cache for LRUCache {
    fn get(&self) {
        println!("cache in progress");
    }
}

impl LRUCache {
    pub fn new() -> LRUCache {
        LRUCache
    }
}
