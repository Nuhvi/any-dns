#![allow(unused)]

mod error;
mod server;
mod dns_thread;

use std::{sync::{atomic::AtomicBool, Arc}, cmp::Ordering, thread::sleep, time::Duration};

use error::Result;
use server::AnyDNS;

fn main() -> Result<()> {
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    ctrlc::set_handler(move || {
        r.store(false, std::sync::atomic::Ordering::Relaxed);
    }).expect("Error setting Ctrl-C handler");

    println!("Listening on 0.0.0.0:53. Waiting for Ctrl-C...");
    let mut anydns = AnyDNS::default();

    while running.load(std::sync::atomic::Ordering::Relaxed) {
        sleep(Duration::from_millis(100));
    };
    println!("Got it! Exiting...");
    anydns.join();

    Ok(())
}
