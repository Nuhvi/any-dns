use std::{
    collections::HashMap,
    net::{SocketAddr, UdpSocket},
    ops::Range,
    str::FromStr,
    sync::{atomic::AtomicBool, mpsc::Sender, Arc, Mutex},
    thread::JoinHandle,
    time::{Duration, Instant},
};

use simple_dns::Packet;

use crate::{
    error::{Error, Result},
    pending_queries::{PendingQuery, PendingStore, ThreadSafeStore},
};

#[derive(Debug)]
pub struct DnsProcessor {
    pending_queries: PendingStore,
    socket: UdpSocket,
    icann_resolver: SocketAddr,
    next_id: u16,
    id_range: Range<u16>,
    stop_signal: Arc<AtomicBool>,
    handler: for<'a> fn(&'a Packet<'a>) -> Result<Packet<'a>, String>,
}

impl DnsProcessor {
    /**
     * Creates a new non-threadsafe dns processor.
     * `socket` is a socket handler.
     * `handler` custom packet handler.
     */
    pub fn new(
        socket: UdpSocket,
        icann_resolver: SocketAddr,
        handler: for<'a> fn(&'a Packet<'a>) -> Result<Packet<'a>, String>,
    ) -> Self {
        DnsProcessor {
            socket,
            pending_queries: PendingStore::new_simple(),
            icann_resolver,
            id_range: 0..u16::MAX,
            next_id: 0,
            stop_signal: Arc::new(AtomicBool::new(false)),
            handler,
        }
    }

    /**
     * Creates a new thread safe dns processor.
     * `socket` is a socket handler.
     * `pending_queries` must be a `PendingStore::ThreadSafe` store, otherwise udp packets will be missed.
     * `id_range` is a range of dns packet ids this thread can use to send to `icann_resolver`.
     * `handler` custom packet handler.
     */
    pub fn new_threadsafe(
        socket: UdpSocket,
        icann_resolver: SocketAddr,
        pending_queries: PendingStore,
        id_range: Range<u16>,
        stop_signal: Arc<AtomicBool>,
        handler: for<'a> fn(&'a Packet<'a>) -> Result<Packet<'a>, String>,
    ) -> Self {
        match &pending_queries {
            PendingStore::ThreadSafe(store) => {},
            _ => panic!("PendingStore::ThreadSafe required.")
        };
        DnsProcessor {
            socket,
            pending_queries,
            icann_resolver,
            id_range: id_range.clone(),
            next_id: id_range.start,
            stop_signal,
            handler,
        }
    }

    fn next_id(&mut self) -> u16 {
        let mut id = self.next_id + 1;
        if id > self.id_range.end {
            id = self.id_range.start;
        }
        self.next_id = id;
        id
    }

    /**
     * Receives data from the socket. Honors the timeout so the server can be stopped by the stop signal.
     */
    fn recv_from(&self, buffer: &mut [u8; 1024]) -> Result<(usize, SocketAddr)> {
        loop {
            match self.socket.recv_from(buffer) {
                Ok((size, from)) => {
                    break Ok((size, from));
                }
                Err(err) => {
                    if err.kind() == std::io::ErrorKind::WouldBlock
                        || err.kind() == std::io::ErrorKind::TimedOut
                    {
                        // Run into timeout.
                        // https://doc.rust-lang.org/std/net/struct.UdpSocket.html#method.set_read_timeout
                        if self.should_stop() {
                            return Err(Error::Static("Stopped"));
                        } else {
                            // Ok, let's continue
                            continue;
                        }
                    }
                    return Err(Error::IO(err));
                }
            }
        }
    }

    /**
     * If stop signal has been given
     */
    fn should_stop(&self) -> bool {
        return self.stop_signal.load(std::sync::atomic::Ordering::Relaxed);
    }

    /**
     * Run actual dns query logic.
     */
    pub fn run(&mut self) -> Result<()> {
        let mut buffer = [0; 1024];
        loop {
            let (size, from) = self.recv_from(&mut buffer)?;
            let query = &mut buffer[..size];
            if from == self.icann_resolver {
                let packet = Packet::parse(query).unwrap();

                let mut removed_opt: Option<PendingQuery> =
                    self.pending_queries.remove(&packet.id());

                if let Some(PendingQuery {
                    id,
                    query,
                    from,
                    sent,
                }) = removed_opt
                {
                    let original_query = Packet::parse(&query).unwrap();
                    (self.handler)(&original_query);

                    let mut reply = Packet::new_reply(original_query.id());

                    let qname = original_query.questions.get(0).unwrap().clone();

                    for answer in packet.answers {
                        reply.answers.push(answer)
                    }

                    for question in original_query.questions {
                        reply.questions.push(question)
                    }

                    self.socket
                        .send_to(&reply.build_bytes_vec().unwrap(), from)
                        .unwrap();
                    let elapsed = sent.elapsed();
                    println!("Reply {:?} within {}ms", qname, elapsed.as_millis());
                };
            } else {
                let id = self.next_id();
                self.pending_queries.insert(PendingQuery {
                    id: id,
                    query: query.to_vec(),
                    from,
                    sent: Instant::now(),
                });

                let id_bytes = id.to_be_bytes();
                query[0] = id_bytes[0];
                query[1] = id_bytes[1];

                self.socket.send_to(&query, self.icann_resolver).unwrap();
            }
        }
    }
}

/**
 * Threaded DnsProcessor.
 */
#[derive(Debug)]
pub struct DnsThread {
    stop_signal: Arc<AtomicBool>,
    handler: JoinHandle<Result<(), Error>>,
}

impl DnsThread {
    /**
     * Creates a new thread that processes DNS queries async.
     */
    pub fn new(
        socket: &UdpSocket,
        icann_resolver: &SocketAddr,
        pending_queries: &Arc<Mutex<HashMap<u16, PendingQuery>>>,
        id_range: Range<u16>,
        handler: for<'a> fn(&'a Packet<'a>) -> Result<Packet<'a>, String>,
    ) -> Self {
        let socket = socket.try_clone().expect("Should clone");
        let icann_resolver = icann_resolver.clone();
        let pending_queries = PendingStore::new_concurrent();
        let stop_signal = Arc::new(AtomicBool::new(false));
        let mut processor = DnsProcessor::new_threadsafe(
            socket,
            icann_resolver,
            pending_queries,
            id_range,
            stop_signal.clone(),
            handler,
        );
        let thread_work = std::thread::spawn(move || processor.run());
        DnsThread {
            handler: thread_work,
            stop_signal,
        }
    }

    /** Sends the stop signal to the thread. */
    pub fn stop(&mut self) {
        self.stop_signal
            .store(true, std::sync::atomic::Ordering::Relaxed)
    }

    /**
     * Stops the thread and waits until it properly terminated. Consumes this instance.
     */
    pub fn join(mut self) {
        self.stop();
        self.handler.join();
    }
}

#[cfg(test)]
mod tests {
    use std::{
        collections::HashMap,
        net::{SocketAddr, UdpSocket},
        ops::Range,
        str::FromStr,
        sync::{atomic::AtomicBool, Arc, Mutex},
        thread::sleep,
        time::Duration,
    };

    use super::{DnsProcessor, PendingQuery};

    // #[test]
    // fn run_processor() {
    //     let listening = SocketAddr::from_str("0.0.0.0:53").expect("Valid socket address");
    //     let icann_resolver = SocketAddr::from_str("192.168.1.1:53").expect("Valid socket address");
    //     let socket = UdpSocket::bind(listening).expect("Address available");
    //     socket.set_read_timeout(Some(Duration::from_millis(500)));
    //     println!("Listening on {}...", listening);
    //     let pending_queries: Arc<Mutex<HashMap<u16, PendingQuery>>> =
    //         Arc::new(Mutex::new(HashMap::new()));
    //     let mut processor = DnsProcessor::new(
    //         socket,
    //         icann_resolver,
    //         pending_queries,
    //         Range{start: 0, end: 1000},
    //         Arc::new(AtomicBool::new(false))
    //     );
    //     processor.run();
    // }
}
