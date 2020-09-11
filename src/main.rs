#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use]
extern crate rocket;
extern crate futures;
extern crate redis;

use {
    redis::Commands,
    rocket::State,
    std::{
        sync::{
            mpsc::{sync_channel, Receiver, SyncSender},
            Arc, Condvar, Mutex,
        },
        thread,
    },
};

#[derive(Clone)]
struct RedisRequest {
    key: String,
    result: Arc<(Mutex<Option<Result<String, i32>>>, Condvar)>,
}

impl RedisRequest {
    pub fn new(key: String) -> RedisRequest {
        RedisRequest {
            key: key,
            result: Arc::new((Mutex::new(None), Condvar::new())),
        }
    }

    pub fn set_result(&mut self, res: String) {
        let (locked_result, cvar) = &*self.result;
        let mut result_guard = locked_result.lock().unwrap();
        *result_guard = Some(Ok(res));
        cvar.notify_one();
    }
}

#[derive(Clone)]
struct RedisProducer {
    work_queue_tx: SyncSender<RedisRequest>,
}

impl RedisProducer {
    pub fn new(work_queue_tx: SyncSender<RedisRequest>) -> RedisProducer {
        RedisProducer { work_queue_tx }
    }

    pub fn get(&self, key: String) -> String {
        let request = RedisRequest::new(key);
        self.work_queue_tx.send(request.clone());
        RedisProducer::wait_for_result(request)
    }

    fn wait_for_result(request: RedisRequest) -> String {
        let (locked_result, cvar) = &*request.result;
        let mut result_guard = locked_result.lock().unwrap();
        while let None = *result_guard {
            result_guard = cvar.wait(result_guard).unwrap();
        }

        //TODO this is jank that we have to clone the string
        match result_guard.as_ref() {
            Some(result) => match result {
                Ok(r) => r.clone(),
                Err(_) => panic!("got error from redis result"),
            },
            None => panic!("expected value from redis result"),
        }
    }
}

struct RedisConsumer {
    worker_thread: thread::JoinHandle<()>,
}

impl RedisConsumer {
    pub fn new(work_queue_rx: Receiver<RedisRequest>) -> RedisConsumer {
        let worker_thread = thread::spawn(move || {
            RedisConsumer::consume_requests(work_queue_rx);
        });

        RedisConsumer { worker_thread }
    }

    fn consume_requests(work_queue_rx: Receiver<RedisRequest>) {
        //let client = redis::Client::open("todo reddis url").unwrap();
        for mut request in work_queue_rx {
            //let mut con = client.get_connection().unwrap();
            let key = request.key.clone();
            println!("{}", key);
            //let val: String = con.get(key).unwrap();
            let val = String::from("General Kenobi");
            request.set_result(val);
        }
    }
}

#[get("/get?<key>")]
fn get(key: String, request_producer: State<RedisProducer>) -> &'static str {
    let res = request_producer.get(key);
    println!("{}", res);
    "fix return types"
}

fn main() {
    let (tx, rx): (SyncSender<RedisRequest>, Receiver<RedisRequest>) = sync_channel(20);
    let producer = RedisProducer::new(tx);

    let consumer = RedisConsumer::new(rx);

    rocket::ignite()
        .manage(producer)
        .mount("/", routes![get])
        .launch();
}
