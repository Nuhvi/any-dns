//! Main server implementation

use simple_dns::{Packet, Name, Question};
use std::{
    collections::HashMap,
    net::{SocketAddr, UdpSocket}, str::FromStr, thread::sleep, time::{Duration, Instant}, sync::{Arc, Mutex}, ops::Range,
};

use crate::dns_thread::{DnsThread, PendingQuery};





pub struct Builder {
    icann_resolver: SocketAddr,
    thread_count: u8,
    handler: for<'a> fn(&'a Packet<'a>) -> Result<Packet<'a>, String>,
}

impl Builder {
    pub fn new() -> Self {
        Self {
            icann_resolver: SocketAddr::from(([192, 168, 1, 1], 53)),
            thread_count: 8,
            handler: |p| {
                println!("Called handler");
                Err("Not processed".to_string())
            }
        }
    }

    /// Set the DNS resolver for normal ICANN domains. Defaults to 192.168.1.1:53
    pub fn icann_resolver(mut self, icann_resolver: SocketAddr) -> Self {
        self.icann_resolver = icann_resolver;
        self
    }

    /// Set the number of threads used. Default: 8.
    pub fn threads(mut self, thread_count: u8) -> Self {
        self.thread_count = thread_count;
        self
    }

    /** Set handler to process the dns packet. `Ok()`` should include a dns packet with answers. `Err()` will fallback to ICANN. */
    pub fn handler(mut self, handler: for<'a> fn(&Packet<'a>) -> Result<Packet<'a>, String>) -> Self {
        self.handler = handler;
        self
    }

    pub fn build(self) -> AnyDNS {
        let listening = SocketAddr::from_str("0.0.0.0:53").expect("Valid socket address");
        let socket = UdpSocket::bind(listening).expect("Address available");
        socket.set_read_timeout(Some(Duration::from_secs(1)));
        let pending_queries: Arc<Mutex<HashMap<u16, PendingQuery>>> =
            Arc::new(Mutex::new(HashMap::new()));

        let mut threads = vec![];
        for i in 0..self.thread_count {
            let id_range = Self::calculate_id_range(self.thread_count as u16, i as u16);
            let thread = DnsThread::new(&socket, &self.icann_resolver, &pending_queries, id_range, self.handler.clone());
            threads.push(thread);
        }

        AnyDNS {
            threads,
            pending_queries,
            icann_resolver: self.icann_resolver
        }
    }

    fn calculate_id_range(thread_count: u16, i: u16) -> Range<u16> {
        let bucket_size = u16::MAX / thread_count;
        Range{
            start: i * bucket_size,
            end: (i + 1) * bucket_size -1
        }
    }
}

#[derive(Debug)]
pub struct AnyDNS {
    icann_resolver: SocketAddr,
    threads: Vec<DnsThread>,
    pending_queries: Arc<Mutex<HashMap<u16, PendingQuery>>>,
}

impl AnyDNS {
    /**
     * Stops the server and consumes the instance.
     */
    pub fn join(mut self) {
        for thread in self.threads.iter_mut() {
            thread.stop();
        };
        for thread in self.threads {
            thread.join()
        };
    }
}

impl Default for AnyDNS {
    fn default() -> Self {
        let builder = Builder::new();
        builder.build()
    }
}