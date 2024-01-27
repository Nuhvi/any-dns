use std::{
    collections::HashMap,
    net::{SocketAddr, UdpSocket},
    ops::Range,
    str::FromStr,
    sync::{atomic::AtomicBool, mpsc::Sender, Arc, Mutex},
    thread::{sleep, JoinHandle},
    time::{Duration, Instant},
};

use simple_dns::Packet;

use crate::{
    custom_handler::HandlerHolder,
    error::{Error, Result},
    pending_queries::{PendingQuery, ThreadSafeStore},
};



#[derive(thiserror::Error, Debug)]
pub enum ProcessingError {
    #[error("User stopped dns manually.")]
    Stopped(),
    #[error(transparent)]
    /// Transparent [std::io::Error]
    IO(#[from] std::io::Error),
}

/**
 * Single DNS packet processor.
 */
#[derive(Debug)]
pub struct DnsProcessor {
    pending_queries: ThreadSafeStore,
    socket: UdpSocket,
    icann_resolver: SocketAddr,
    next_id: u16,
    id_range: Range<u16>,
    stop_signal: Arc<AtomicBool>,
    handler: HandlerHolder,
    verbose: bool
}

impl DnsProcessor {
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
        pending_queries: ThreadSafeStore,
        id_range: Range<u16>,
        stop_signal: Arc<AtomicBool>,
        handler: HandlerHolder,
        verbose: bool
    ) -> Self {
        DnsProcessor {
            socket,
            pending_queries,
            icann_resolver,
            id_range: id_range.clone(),
            next_id: id_range.start,
            stop_signal,
            handler,
            verbose
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
    fn recv_from(&self, buffer: &mut [u8; 1024]) -> Result<(usize, SocketAddr), ProcessingError> {
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
                            return Err(ProcessingError::Stopped());
                        } else {
                            // Ok, let's continue
                            continue;
                        }
                    }
                    return Err(ProcessingError::IO(err));
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
     * Forward query to icann
     */
    fn forward_to_icann(&mut self, mut query: Vec<u8>, from: SocketAddr) -> Result<(), ProcessingError> {
        let received = Instant::now();
        let packet = Packet::parse(&query).unwrap();
        let id = self.next_id();
        self.pending_queries.insert(PendingQuery {
            icann_id: id,
            query: query.to_vec(),
            from,
            received_at: received,
        });

        let id_bytes = id.to_be_bytes();
        query[0] = id_bytes[0];
        query[1] = id_bytes[1];

        self.socket.send_to(&query, self.icann_resolver)?;
        Ok(())
    }

    /**
     * Send answers to client.
     */
    fn respond_to_client(&mut self, mut reply: Vec<u8>) -> Result<(), ProcessingError> {
        let reply_packet = Packet::parse(&reply).unwrap();
        let mut removed_opt: Option<PendingQuery> = self.pending_queries.remove(&reply_packet.id());
        if removed_opt.is_none() {
            if self.verbose {
                eprintln!("No pending query to respond to.");
            }
            return Ok(());
        }

        let pending = removed_opt.unwrap();
        let pending_packet = Packet::parse(&pending.query).unwrap();
        let id_bytes = pending_packet.id().to_be_bytes();
        reply[0] = pending.query[0];
        reply[1] = pending.query[1];

        self.socket
            .send_to(&reply, pending.from)?;

        if self.verbose {
            let elapsed = pending.received_at.elapsed();
            let question = pending_packet.questions.get(0).unwrap().clone();
            println!(
                "Reply {:?} within {}ms",
                question,
                elapsed.as_millis()
            );
        }

        Ok(())
    }


    /** Receive and process one udp packet.  */
    fn process_packet(&mut self) -> Result<(), ProcessingError> {
        let mut buffer = [0; 1024];
        let (size, from) = self.recv_from(&mut buffer)?;
        let query = buffer[..size].to_vec();
        if from == self.icann_resolver {
            self.respond_to_client(query)?;
        } else {
            let result = self.handler.call(&query);
            if result.is_ok() {
                self.respond_to_client(result.unwrap())?;
            } else {
                self.forward_to_icann(query, from)?;
            }

        }
        Ok(())
    }

    /**
     * Run actual dns query logic.
     */
    pub fn run(&mut self) -> Result<()> {
        loop {
            let result = self.process_packet();
            if result.is_ok() {
                continue;
            };
            match result.unwrap_err() {
                ProcessingError::Stopped() => {
                    return Ok(());
                },
                ProcessingError::IO(err) => {
                    if self.verbose {
                        eprintln!("IO error {}", err);
                    }
                }
            }
        }
        Ok(())
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
        pending_queries: &ThreadSafeStore,
        id_range: Range<u16>,
        handler: &HandlerHolder,
        verbose: bool
    ) -> Self {
        let socket = socket.try_clone().expect("Should clone");
        let icann_resolver = icann_resolver.clone();
        let stop_signal = Arc::new(AtomicBool::new(false));
        let mut processor = DnsProcessor::new_threadsafe(
            socket,
            icann_resolver,
            pending_queries.clone(),
            id_range,
            stop_signal.clone(),
            handler.clone(),
            verbose
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
