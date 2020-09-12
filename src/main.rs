// Needed to enable Rocket Webserver CodeGen
#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use]
extern crate rocket;
extern crate redis;

mod lru_cache;
mod redis_consumer;
mod redis_request;

use {
    lru_cache::LRUCache,
    redis_consumer::{RedisClientWraper, RedisConsumer},
    redis_request::RedisRequest,
    rocket::State,
    std::sync::mpsc::{sync_channel, Receiver, SyncSender},
};

/*
 * This defines the producer of RedisRequests and is responsible
 * for passing incomming web requests to the RedisConsumer
 */

#[derive(Clone)]
struct RedisProducer {
    work_queue_tx: SyncSender<RedisRequest>,
}

impl RedisProducer {
    pub fn new(work_queue_tx: SyncSender<RedisRequest>) -> RedisProducer {
        RedisProducer { work_queue_tx }
    }

    pub fn produce_requests(&self, key: String) -> RedisRequest {
        let request = RedisRequest::new(key);
        self.work_queue_tx
            .send(request.clone())
            .expect("producer channel failed");
        request
    }
}

#[get("/get?<key>")]
fn get(key: String, request_producer: State<RedisProducer>) -> &'static str {
    let request = request_producer.produce_requests(key);
    let value = request.get_result();
    println!("{}", value);
    "fix return types"
}

fn main() {
    let (tx, rx): (SyncSender<RedisRequest>, Receiver<RedisRequest>) = sync_channel(20);
    let producer = RedisProducer::new(tx);

    let lru = LRUCache::new(10, std::time::Duration::from_secs(1));
    let redis_provider = RedisClientWraper::new();

    let consumer = RedisConsumer::new(rx, lru, redis_provider);
    let worker_handler = consumer.start();

    rocket::ignite()
        .manage(producer)
        .mount("/", routes![get])
        .launch();
}
