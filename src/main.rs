#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use] extern crate rocket;
extern crate redis; 
extern crate futures; 

use rocket::request::Form;
use rocket::State;
use std::sync::mpsc::{SyncSender, Receiver};
use std::sync::mpsc::sync_channel;
use std::thread;
use redis::Commands;
use futures::Future;
use std::pin::Pin;
use std::task::{Context, Poll};


struct RedisQuery;
impl Future for RedisQuery {
    type Output = i32; //fixme 
    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        Poll::Pending
    }
}



#[derive(FromForm, Debug)]
struct User {
    name: String, 
    account: usize,
}

#[get("/?<id>&<user..>")]
fn get(id: usize, user: Form<User>, getQueue: State<SyncSender<String>>) -> &'static str {
    println!("id: {}, {:?}", id, user);
    getQueue.send(String::from("hello"));
    "item hit"
}

//figure out how to handle get and set. 
//how are we passing values back to the user. (promise future?)
fn redisWorker(getCmds: Receiver<String>) {
    let client = redis::Client::open("todo reddis url").unwrap();
    for key in getCmds {
        let mut con = client.get_connection().unwrap();
        let val : i32 = con.get(key).unwrap();
    }
}

fn main() {
    let (tx, rx): (SyncSender<String>, Receiver<String>) = sync_channel(20);
    rocket::ignite().manage(tx);

    let handle = thread::spawn(move || {
        redisWorker(rx);
    });

    rocket::ignite().mount("/", routes![get]).launch();
}