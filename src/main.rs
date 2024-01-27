#![allow(unused)]

mod error;
mod server;
mod dns_thread;
mod pending_queries;
mod custom_handler;

use std::{cmp::Ordering, error::Error, net::Ipv4Addr, sync::{atomic::AtomicBool, Arc}, thread::sleep, time::Duration};

use any_dns::{CustomHandler, Builder};
use error::Result;
use server::AnyDNS;
use simple_dns::{Packet, PacketFlag, ResourceRecord, QTYPE};

#[derive(Clone, Debug)]
struct MyHandler {}

impl CustomHandler for MyHandler {
    /**
     * Only resolve 1 custom domain 7fmjpcuuzf54hw18bsgi3zihzyh4awseeuq5tmojefaezjbd64cy.
     */
    fn lookup(&mut self, query: &Vec<u8>) -> std::prelude::v1::Result<Vec<u8>, Box<dyn Error>> {
        let packet = Packet::parse(query).unwrap();
        let question = packet.questions.get(0).expect("Valid query");
        if question.qname.to_string() != "7fmjpcuuzf54hw18bsgi3zihzyh4awseeuq5tmojefaezjbd64cy" || question.qtype != QTYPE::TYPE(simple_dns::TYPE::A) {
            return Err("Not Implemented".into());
        };

        let mut reply = Packet::new_reply(packet.id());
        reply.questions.push(question.clone());
        let ip: Ipv4Addr = "37.27.13.182".parse().unwrap();
        let record = ResourceRecord::new(question.qname.clone(), simple_dns::CLASS::IN, 120, simple_dns::rdata::RData::A(ip.try_into().unwrap()));
        reply.answers.push(record);
        Ok(reply.build_bytes_vec().unwrap())
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
    let anydns = Builder::new().handler(handler).verbose(true).build();

    while running.load(std::sync::atomic::Ordering::Relaxed) {
        sleep(Duration::from_millis(100));
    };
    println!("Got it! Exiting...");
    anydns.join();

    Ok(())
}
