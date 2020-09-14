use {
    crate::lru_cache::Cache, crate::redis_request::Message, redis::Commands,
    std::sync::mpsc::Receiver,
};

/*
 * This trait defines the interface through which our consumer can
 * get data from the backing redis
 */
pub trait RedisProvider {
    fn fetch(&self, key: &String) -> Result<Option<String>, redis::RedisError>;
}

/*
 * The redis client wrapper abstracts the implementation details
 * of getting results from the redis client. This implements the
 * RedisProvider trait so that we can pass this into our redis
 * consumer, allowing it to get results from the backing redis
 */

pub struct RedisClientWrapper {
    redis_url: String,
}

impl RedisProvider for RedisClientWrapper {
    fn fetch(&self, key: &String) -> Result<Option<String>, redis::RedisError> {
        let client = redis::Client::open(self.redis_url.clone())?;
        let mut con = client.get_connection()?;
        con.get(key)
    }
}

impl RedisClientWrapper {
    pub fn new(redis_url: String) -> RedisClientWrapper {
        //TODO sleep and Retry once on failure, its possible the
        //proxy started up before redis
        println!("Initializing redis client at addr {:?}", redis_url);
        let client = redis::Client::open(redis_url.clone()).unwrap();
        let mut con = client
            .get_connection()
            .expect("RedisClientWrapper failed to connect to redis ");
        let pong: Option<String> = redis::cmd("PING").query(&mut con).unwrap();
        assert_eq!(pong, Some("PONG".to_string()));

        RedisClientWrapper { redis_url }
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
 * To achieve our generic consumer we need to promise a few things to
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
                        Some(val) => request.set_result(Ok(Some(val))),
                        None => {
                            let redis_get = self.redis_provider.fetch(&key);
                            //Only fill cache on successful redis response
                            if let Ok(Some(ref val)) = redis_get {
                                self.cache.put(&key, val.clone());
                            }
                            request.set_result(redis_get);
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
        crate::redis_request::RedisRequest,
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
        fn fetch(&self, key: &String) -> Result<Option<String>, redis::RedisError> {
            if key == "redis_hit" {
                return Ok(Some(String::from("hit_redis")));
            } else if key == "redis_err" {
                return Err(redis::RedisError::from((
                    redis::ErrorKind::ResponseError,
                    "err",
                )));
            }
            Ok(None)
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
        assert_eq!(val, Some("hit_cache".to_string()));
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
        assert_eq!(val, Some("hit_redis".to_string()));
    }

    #[test]
    fn test_redis_err() {
        let (tx, rx): (SyncSender<Message>, Receiver<Message>) = sync_channel(20);
        let consumer = RedisConsumer::new(rx, MockCache, MockRedis);

        let request = RedisRequest::new(String::from("redis_err"));

        tx.send(Message::Request(request.clone()));
        tx.send(Message::Shutdown);
        consumer.consume_requests();
        let val = request.get_result();
        assert_eq!(val, Some("err".to_string()));
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
        assert_eq!(val, None);
    }
}
