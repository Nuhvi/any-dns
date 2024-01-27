#![allow(unused)]

mod error;
mod server;
mod dns_thread;
mod pending_queries;
mod custom_handler;

use std::{sync::{atomic::AtomicBool, Arc}, cmp::Ordering, thread::sleep, time::Duration, error::Error};

use any_dns::{CustomHandler, Builder};
use error::Result;
use server::AnyDNS;
use simple_dns::Packet;

#[derive(Clone, Debug)]
struct MyHandler {}

impl CustomHandler for MyHandler {
    fn lookup(&self, query: &Vec<u8>) -> std::prelude::v1::Result<Vec<u8>, Box<dyn Error>> {
        let packet = Packet::parse(query).unwrap();
        let question = packet.questions.get(0).expect("Valid query");
        Err("Not Implemented".into())
    }
}


fn main() -> Result<()> {
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    ctrlc::set_handler(move || {
        r.store(false, std::sync::atomic::Ordering::Relaxed);
    }).expect("Error setting Ctrl-C handler");

    println!("Listening on 0.0.0.0:53. Waiting for Ctrl-C...");
    let handler = MyHandler{};
    let anydns = Builder::new().threads(1).handler(handler).build();

    while running.load(std::sync::atomic::Ordering::Relaxed) {
        sleep(Duration::from_millis(100));
    };
    println!("Got it! Exiting...");
    anydns.join();

    Ok(())
}
