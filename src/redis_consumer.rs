use {
    crate::lru_cache::Cache,
    crate::redis_request::{Message, RedisRequest},
    redis::Commands,
    std::{sync::mpsc::Receiver, thread},
};

/*
 * This trait defines the interface throuhgh which our consumer can
 * get data from the backing redis
 */
pub trait RedisProvider {
    fn fetch(&self, key: &String) -> Option<Result<String, i32>>;
}

/*
 * The reddis client wrapper abstracts the implementation details
 * of getting results from the reddis client. This implemetns the
 * RedisProvider trait so that we can pass this into our redis
 * consumer, allowing it to get results from the backing redis
 */

pub struct RedisClientWraper;

impl RedisProvider for RedisClientWraper {
    fn fetch(&self, key: &String) -> Option<Result<String, i32>> {
        Some(Ok("hello world".to_string()))
    }
}

impl RedisClientWraper {
    pub fn new() -> RedisClientWraper {
        RedisClientWraper
    }
}

/*
 * Generic Consumer of RedisRequests intended to provide loose coupling
 * between our Request consumer, cache implementation, and backing redis
 * client integration. Dependency injection helps us more easily mock
 * dependencies and test this code
 */

pub struct RedisConsumer<TCache: Cache, TProvider: RedisProvider> {
    work_queue_rx: Receiver<Message>,
    redis_provider: TProvider,
    cache: TCache,
}

/*
 * To achive our generic consumer we need to promise a few things to
 * the compiler about our Generic types.
 *
 * TCache    - Requires that this type implement the cache trait
 *           - Requires Send so we can pass object to worker thread
 *           - 'static lifetime promises that the TCache object holds
 *             no references that could be dropped.
 * TProvider - Requires that this type implement the RedisProvider trait
 *           - Requires Send so we can pass object to worker thread
 *           - 'static lifetime promises that the TProvider object holds
 *             no references that could be dropped.
 */
impl<TCache, TProvider> RedisConsumer<TCache, TProvider>
where
    TCache: Cache + Send + 'static,
    TProvider: RedisProvider + Send + 'static,
{
    pub fn new(
        work_queue_rx: Receiver<Message>,
        cache: TCache,
        redis_provider: TProvider,
    ) -> RedisConsumer<TCache, TProvider> {
        RedisConsumer {
            work_queue_rx,
            redis_provider,
            cache,
        }
    }

    pub fn consume_requests(mut self) {
        for msg in self.work_queue_rx {
            match msg {
                Message::Shutdown => return,
                Message::Request(mut request) => {
                    let key = request.key.clone();
                    let cached_get = self.cache.get(&key);
                    match cached_get {
                        Some(val) => request.set_result(Ok(val)),
                        None => {
                            let redis_get = self.redis_provider.fetch(&key);
                            match redis_get {
                                Some(res) => {
                                    let val = res.unwrap(); //todo
                                    request.set_result(Ok(val.clone()));
                                    self.cache.put(&key, val);
                                }
                                //todo better error handling here
                                None => request.set_result(Err(-1)),
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use {
        crate::redis_consumer::*,
        std::sync::mpsc::{sync_channel, Receiver, SyncSender},
    };

    struct MockCache;
    impl Cache for MockCache {
        fn get(&mut self, key: &String) -> Option<String> {
            if key == "cache_hit" {
                return Some(String::from("hit_cache"));
            }
            None
        }
        fn put(&mut self, _: &String, _: String) {}
    }

    struct MockRedis;
    impl RedisProvider for MockRedis {
        fn fetch(&self, key: &String) -> Option<Result<String, i32>> {
            if key == "redis_hit" {
                return Some(Ok(String::from("hit_redis")));
            }
            if key == "redis_err" {
                return Some(Err(-2));
            }

            None
        }
    }

    #[test]
    fn test_shutdown() {
        let (tx, rx): (SyncSender<Message>, Receiver<Message>) = sync_channel(20);
        let consumer = RedisConsumer::new(rx, MockCache, MockRedis);

        tx.send(Message::Shutdown);
        //expect to exit immediately.
        consumer.consume_requests()
    }

    #[test]
    fn test_cache_get() {
        let (tx, rx): (SyncSender<Message>, Receiver<Message>) = sync_channel(20);
        let consumer = RedisConsumer::new(rx, MockCache, MockRedis);

        let request = RedisRequest::new(String::from("cache_hit"));

        tx.send(Message::Request(request.clone()));
        tx.send(Message::Shutdown);
        consumer.consume_requests();
        let val = request.get_result();
        assert_eq!(val, Ok("hit_cache".to_string()));
    }
    #[test]
    fn test_redis_get() {
        let (tx, rx): (SyncSender<Message>, Receiver<Message>) = sync_channel(20);
        let consumer = RedisConsumer::new(rx, MockCache, MockRedis);

        let request = RedisRequest::new(String::from("redis_hit"));

        tx.send(Message::Request(request.clone()));
        tx.send(Message::Shutdown);
        consumer.consume_requests();
        let val = request.get_result();
        assert_eq!(val, Ok("hit_redis".to_string()));
    }
    #[test]
    fn test_redis_miss() {
        let (tx, rx): (SyncSender<Message>, Receiver<Message>) = sync_channel(20);
        let consumer = RedisConsumer::new(rx, MockCache, MockRedis);

        let request = RedisRequest::new(String::from("redis_miss"));

        tx.send(Message::Request(request.clone()));
        tx.send(Message::Shutdown);
        consumer.consume_requests();
        let val = request.get_result();
        assert_eq!(val, Err(-1));
    }
}
