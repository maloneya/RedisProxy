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
    redis_request::{Message, RedisRequest},
    rocket::State,
    std::sync::mpsc::{sync_channel, Receiver, SyncSender},
};

/*
 * This defines the producer of RedisRequests and is responsible
 * for passing incomming web requests to the RedisConsumer
 */

#[derive(Clone)]
struct RedisProducer {
    work_queue_tx: SyncSender<Message>,
}

impl RedisProducer {
    pub fn new(work_queue_tx: SyncSender<Message>) -> RedisProducer {
        RedisProducer { work_queue_tx }
    }

    pub fn produce_requests(&self, key: String) -> RedisRequest {
        let request = RedisRequest::new(key);
        self.work_queue_tx
            .send(Message::Request(request.clone()))
            .expect("producer channel failed");
        request
    }
}

#[get("/get?<key>")]
fn get(key: String, request_producer: State<RedisProducer>) -> String {
    let request = request_producer.produce_requests(key);
    match request.get_result() {
        Some(val) => val,
        None => "".to_string(),
    }
}

/*
 * The Redis Worker takes ownsership of a redisConsumer and begins a new thread
 * to run consume requests on.
 *
 * Implements the Drop trait which will trigger the worker thread to shutdown
 * when the RedisWorker goes out of scope on shutdown
 */

pub struct RedisWorker {
    worker_handle: std::thread::JoinHandle<()>,
    msg_queue_for_shutdown: SyncSender<Message>,
}

impl Drop for RedisWorker {
    fn drop(&mut self) {
        self.msg_queue_for_shutdown.send(Message::Shutdown);
    }
}

impl RedisWorker {
    pub fn new(
        consumer: RedisConsumer<LRUCache, RedisClientWraper>,
        msg_queue_for_shutdown: SyncSender<Message>,
    ) -> RedisWorker {
        RedisWorker {
            worker_handle: std::thread::spawn(move || consumer.consume_requests()),
            msg_queue_for_shutdown,
        }
    }
}

fn main() {
    //todo channel size
    let (tx, rx): (SyncSender<Message>, Receiver<Message>) = sync_channel(20);
    let producer = RedisProducer::new(tx.clone());

    let lru = LRUCache::new(1000, std::time::Duration::from_secs(10));
    let redis_provider = RedisClientWraper::new("redis://127.0.0.1/".to_string());

    let consumer = RedisConsumer::new(rx, lru, redis_provider);
    let worker = RedisWorker::new(consumer, tx.clone());
    //todo where do i set numberf of web threads?
    rocket::ignite()
        .manage(producer)
        .manage(worker) /* passing ownership to rocket triggers cleanup of worker on shutdown */
        .mount("/", routes![get])
        .launch();
    println!("end");
}
