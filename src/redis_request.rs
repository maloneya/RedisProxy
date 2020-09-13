use std::{
    error::Error,
    sync::{Arc, Condvar, Mutex},
};

/*
 * The RedisRequest is responsible for passing the reqeusted key
 * from Producer to consumer, providing a method for the consumer
 * to safely pass a result back to the producer, and allowing the
 * consumer to wait for the reuslt to be completed wihtout consuming
 * CPU time
 *
 * To achvie this the result member utilizes:
 *   - ARC (atomic reference count): a smart pointer that can be shared
 *     across threads (allows the producer and the consuemr to read the value)
 *   - Mutex: guards the result value, preventing simultainous acccess and
 *     any data race
 *   - Optional and Result: this allows us to check if a result has been set,
 *     and check if the request was complete succefully.
 *   - Condvar: allows us to syncronize the completion of the request. The
 *     consumer can notify the producer when the optional result has been set
 */

#[derive(Clone)]
pub struct RedisRequest {
    pub key: String,
    pub result: Arc<(Mutex<Option<Result<String, redis::RedisError>>>, Condvar)>,
}

impl RedisRequest {
    pub fn new(key: String) -> RedisRequest {
        RedisRequest {
            key: key,
            result: Arc::new((Mutex::new(None), Condvar::new())),
        }
    }

    pub fn set_result(&mut self, res: Result<String, redis::RedisError>) {
        let (locked_result, cvar) = &*self.result;
        let mut result_guard = locked_result.lock().unwrap();
        *result_guard = Some(res);
        cvar.notify_one();
    }

    /*
     * Consumes the redis request. Blocks until a result is
     * ready. Returns the result.
     */
    pub fn get_result(self) -> String {
        let (locked_result, cvar) = &*self.result;
        let mut result_guard = locked_result.lock().unwrap();
        while let None = *result_guard {
            result_guard = cvar.wait(result_guard).unwrap();
        }

        //Deref mutex guard, then get ref to mutex data
        match &(*result_guard) {
            Some(result) => match result {
                Ok(r) => r.clone(),
                Err(e) => e.description().to_string(),
            },
            None => panic!("Woke up from condvar.wait with no resut"),
        }
    }
}

pub enum Message {
    Request(RedisRequest),
    Shutdown,
}
