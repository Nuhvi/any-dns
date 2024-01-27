//! Main server implementation

use simple_dns::{Packet, Name, Question};
use std::{
    collections::HashMap,
    net::{SocketAddr, UdpSocket}, str::FromStr, thread::sleep, time::{Duration, Instant}, sync::{Arc, Mutex}, ops::Range,
};

use crate::{dns_thread::DnsThread, pending_queries::{self, PendingQuery, ThreadSafeStore}, custom_handler::{HandlerHolder, EmptyHandler, CustomHandler}};



pub struct Builder {
    icann_resolver: SocketAddr,
    listen: SocketAddr,
    thread_count: u8,
    handler: HandlerHolder,
    verbose: bool
}

impl Builder {
    pub fn new() -> Self {
        Self {
            icann_resolver: SocketAddr::from(([192, 168, 1, 1], 53)),
            listen: SocketAddr::from(([0, 0, 0, 0], 53)),
            thread_count: 1,
            handler: HandlerHolder::new(EmptyHandler::new()),
            verbose: false
        }
    }

    /// Set the DNS resolver for normal ICANN domains. Defaults to 192.168.1.1:53
    pub fn icann_resolver(mut self, icann_resolver: SocketAddr) -> Self {
        self.icann_resolver = icann_resolver;
        self
    }

    /// Set socket the server should listen on. Defaults to 0.0.0.0:53
    pub fn listen(mut self, listen: SocketAddr) -> Self {
        self.listen = listen;
        self
    }

    /// Set the number of threads used. Default: 1.
    pub fn threads(mut self, thread_count: u8) -> Self {
        self.thread_count = thread_count;
        self
    }

    /// Makes the server log verbosely.
    pub fn verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }

    /** Set handler to process the dns packet. `Ok()`` should include a dns packet with answers. `Err()` will fallback to ICANN. */
    pub fn handler(mut self, handler: impl CustomHandler + 'static) -> Self {
        self.handler = HandlerHolder::new(handler);
        self
    }

    pub fn build(self) -> AnyDNS {
        let socket = UdpSocket::bind(self.listen).expect("Address available");
        socket.set_read_timeout(Some(Duration::from_millis(500))); // So the DNS can be stopped.
        let pending_queries = ThreadSafeStore::new();
        let mut threads = vec![];
        for i in 0..self.thread_count {
            let id_range = Self::calculate_id_range(self.thread_count as u16, i as u16);
            let thread = DnsThread::new(&socket, &self.icann_resolver, &pending_queries, id_range, self.handler.clone(), self.verbose);
            threads.push(thread);
        }

        AnyDNS {
            threads,
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
