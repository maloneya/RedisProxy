use {
    crate::lru_cache::Cache,
    crate::redis_request::RedisRequest,
    redis::Commands,
    std::{sync::mpsc::Receiver, thread},
};

/*
 * This trait defines the interface throuhgh which our consumer can
 * get data from the backing redis
 */
pub trait RedisProvider {
    fn fetch(&self);
}

/*
 * The reddis client wrapper abstracts the implementation details
 * of getting results from the reddis client. This implemetns the
 * RedisProvider trait so that we can pass this into our redis
 * consumer, allowing it to get results from the backing redis
 */

pub struct RedisClientWraper;

impl RedisProvider for RedisClientWraper {
    fn fetch(&self) {
        //todo
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
    work_queue_rx: Receiver<RedisRequest>,
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
        work_queue_rx: Receiver<RedisRequest>,
        cache: TCache,
        redis_provider: TProvider,
    ) -> RedisConsumer<TCache, TProvider> {
        RedisConsumer {
            work_queue_rx,
            redis_provider,
            cache,
        }
    }

    /*
     * Takes ownership of the RedisConsumer and moves it
     * into a woker thread to processing incomming requests
     * Retruns a handle to that Worker to be joined at shutdown
     */
    pub fn start(self) -> thread::JoinHandle<()> {
        thread::spawn(move || {
            //todo cache integration. do we want to catch cache panics?
            //let client = redis::Client::open("todo reddis url").unwrap();
            for mut request in self.work_queue_rx {
                //let mut con = client.get_connection().unwrap();
                let key = request.key.clone();
                println!("{}", key);
                //let val: String = con.get(key).unwrap();
                let val = String::from("General Kenobi");
                request.set_result(val);
            }
        })
    }
}
