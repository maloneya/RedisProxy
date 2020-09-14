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
    redis_consumer::{RedisClientWrapper, RedisConsumer},
    redis_request::{Message, RedisRequest},
    rocket::State,
    std::sync::mpsc::{sync_channel, Receiver, SyncSender},
};

/*
 * This defines the producer of RedisRequests and is responsible
 * for passing incoming web requests to the RedisConsumer
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

#[get("/<key>")]
fn get(key: String, request_producer: State<RedisProducer>) -> String {
    let request = request_producer.produce_requests(key);
    match request.get_result() {
        Some(val) => val,
        None => "".to_string(),
    }
}

/*
 * The Redis Worker takes ownership of a redisConsumer and begins a new thread
 * to run consume requests on.
 *
 * Implements the Drop trait which will trigger the worker thread to shutdown
 * when the RedisWorker goes out of scope on shutdown
 */

pub struct RedisWorker {
    worker_handle: Option<std::thread::JoinHandle<()>>,
    msg_queue_for_shutdown: SyncSender<Message>,
}

impl Drop for RedisWorker {
    fn drop(&mut self) {
        self.msg_queue_for_shutdown
            .send(Message::Shutdown)
            .expect("shutdown failed");
        self.worker_handle
            .take()
            .unwrap()
            .join()
            .expect("worker thread join failed");
    }
}

impl RedisWorker {
    pub fn new(
        consumer: RedisConsumer<LRUCache, RedisClientWrapper>,
        msg_queue_for_shutdown: SyncSender<Message>,
    ) -> RedisWorker {
        RedisWorker {
            worker_handle: Some(std::thread::spawn(move || consumer.consume_requests())),
            msg_queue_for_shutdown,
        }
    }
}

fn help() {
    println!(
        "Usage:
    redis_proxy [options]
    
    Options:
    --cache_expr_sec sets the time in seconds that values will remain in the cache
    --cache_size     sets the Size of the internal LRU cache
    --redis_addr     the address of the backing redis"
    )
}

fn parse_args() -> Option<(std::time::Duration, usize, String)> {
    let args: Vec<String> = std::env::args().collect();
    match args.len() {
        1 | 3 | 5 | 7 => println!("parsing args"),
        _ => {
            println!("Unexpected args {:?}", args);
            help();
            return None;
        }
    }

    if let Some(_) = args.iter().find(|arg| (*arg).eq("--help")) {
        help();
        return None;
    }

    let expr = match args.iter().position(|arg| (*arg).eq("--cache_expr_sec")) {
        Some(arg_pos) => std::time::Duration::from_secs(args[arg_pos + 1].parse::<u64>().unwrap()),
        None => {
            println!("using default duration 10 sec");
            std::time::Duration::from_secs(10)
        }
    };

    let size = match args.iter().position(|arg| (*arg).eq("--cache_size")) {
        Some(arg_pos) => args[arg_pos + 1].parse::<usize>().unwrap(),
        None => {
            println!("using default cache size 100");
            100
        }
    };

    let addr = match args.iter().position(|arg| (*arg).eq("--redis_addr")) {
        Some(arg_pos) => args[arg_pos + 1].clone(),
        None => {
            let default_addr = "redis://127.0.0.1/".to_string();
            println!("using default redis addr {}", default_addr);
            default_addr
        }
    };

    Some((expr, size, addr))
}

fn main() {
    let (expr, size, addr) = match parse_args() {
        Some(args) => args,
        None => return,
    };

    let (tx, rx): (SyncSender<Message>, Receiver<Message>) = sync_channel(100);
    let producer = RedisProducer::new(tx.clone());

    let lru = LRUCache::new(size, expr);
    let redis_provider = RedisClientWrapper::new(addr);
    let consumer = RedisConsumer::new(rx, lru, redis_provider);
    let worker = RedisWorker::new(consumer, tx.clone());

    //Using
    rocket::ignite()
        .manage(producer)
        .manage(worker) /* passing ownership to rocket triggers cleanup of worker on shutdown */
        .mount("/", routes![get])
        .launch();
    println!("end");
}
