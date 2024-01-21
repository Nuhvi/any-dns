use std::{
    collections::HashMap,
    net::{SocketAddr, UdpSocket},
    str::FromStr,
    sync::{atomic::AtomicBool, mpsc::Sender, Arc, Mutex},
    thread::JoinHandle,
    time::{Instant, Duration}, ops::Range,
};

use simple_dns::Packet;


use crate::error::{Result, Error};

#[derive(Debug)]
pub struct PendingQuery {
    from: SocketAddr,
    query: Vec<u8>,
    sent: Instant,
}

#[derive(Debug)]
pub struct DnsProcessor {
    pending_queries: Arc<Mutex<HashMap<u16, PendingQuery>>>,
    socket: UdpSocket,
    icann_resolver: SocketAddr,
    next_id: u16,
    id_range: Range<u16>,
    should_stop: Arc<AtomicBool>,
}

impl DnsProcessor {
    /**
     * Creates a new dns processor.
     * `socket` is a socket handler.
     * `id_range` is a range of dns packet ids this thread can use to send to `icann_resolver`.
     */
    pub fn new(
        socket: UdpSocket,
        icann_resolver: SocketAddr,
        pending_queries: Arc<Mutex<HashMap<u16, PendingQuery>>>,
        id_range: Range<u16>,
        should_stop: Arc<AtomicBool>,
    ) -> Self {
        DnsProcessor {
            socket,
            pending_queries,
            icann_resolver,
            id_range: id_range.clone(),
            next_id: id_range.start,
            should_stop,
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
     * Receives data from the socket. Honors the timeout so the server can be stopped by join().
     */
    fn recv_from(&self, buffer: &mut [u8; 1024]) -> Result<(usize, SocketAddr)> {
        loop {
            match self.socket.recv_from(buffer) {
                Ok((size, from)) => {
                    break Ok((size, from));
                },
                Err(err) => {
                    if err.kind() == std::io::ErrorKind::WouldBlock || err.kind() == std::io::ErrorKind::TimedOut { 
                        // Run into timeout.
                        // https://doc.rust-lang.org/std/net/struct.UdpSocket.html#method.set_read_timeout
                        if self.should_stop_thread() {
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

    fn should_stop_thread(&self) -> bool {
        return self.should_stop.load(std::sync::atomic::Ordering::Relaxed);
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

                let mut removed_opt: Option<PendingQuery> = None;
                {
                    // Open new block so lock gets released quickly again.
                    let mut locked_pending_queries =
                        self.pending_queries.lock().expect("Lock success");
                    removed_opt = locked_pending_queries.remove(&packet.id());
                }

                if let Some(PendingQuery { query, from, sent }) = removed_opt {
                    let original_query = Packet::parse(&query).unwrap();

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
                    println!(
                        "Reply {:?} within {}ms",
                        qname,
                        elapsed.as_millis()
                    );
                };
            } else {
                let id = self.next_id();
                {
                    let mut locked_pending_queries =
                        self.pending_queries.lock().expect("Lock success");
                    locked_pending_queries.insert(
                        id,
                        PendingQuery {
                            query: query.to_vec(),
                            from,
                            sent: Instant::now(),
                        },
                    );
                }

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
    should_stop: Arc<AtomicBool>,
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
        id_range: Range<u16>
    ) -> Self {
        let socket = socket.try_clone().expect("Should clone");
        let icann_resolver = icann_resolver.clone();
        let pending_queries = Arc::clone(pending_queries);
        let should_stop = Arc::new(AtomicBool::new(false));
        let mut processor = DnsProcessor::new(
            socket,
            icann_resolver,
            pending_queries,
            id_range,
            should_stop.clone(),
        );
        let thread_work = std::thread::spawn(move || processor.run());
        DnsThread {
            handler: thread_work,
            should_stop,
        }
    }

    /** Sends the stop signal to the thread. */
    pub fn stop(&mut self) {
        self.should_stop.store(true, std::sync::atomic::Ordering::Relaxed)
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
        str::FromStr,
        sync::{Arc, Mutex, atomic::AtomicBool},
        thread::sleep,
        time::Duration, ops::Range,
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
