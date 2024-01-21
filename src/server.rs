//! Main server implementation

use simple_dns::Packet;
use std::{
    collections::HashMap,
    net::{SocketAddr, UdpSocket}, str::FromStr, thread::sleep, time::{Duration, Instant},
};

use crate::Result;

#[derive(Debug)]
pub struct Builder {
    icann_resolver: SocketAddr,
}

impl Builder {
    pub fn new() -> Self {
        Self {
            icann_resolver: SocketAddr::from(([192, 168, 1, 1], 53)),
        }
    }

    /// Set the DNS resolver for normal ICANN domains. Defaults to 192.168.1.1:53
    pub fn icann_resolver(mut self, icann_resolver: SocketAddr) -> Self {
        self.icann_resolver = icann_resolver;
        self
    }

    pub fn build(self) -> AnyDNS {
        AnyDNS {
            next_id: 0,
            icann_resolver: self.icann_resolver,
            pending_queries: HashMap::new(),
        }
    }
}

#[derive(Debug)]
struct PendingQuery {
    from: SocketAddr,
    query: Vec<u8>,
    sent: Instant
}

#[derive(Debug)]
pub struct AnyDNS {
    next_id: u16,
    icann_resolver: SocketAddr,
    pending_queries: HashMap<u16, PendingQuery>,
}

impl AnyDNS {
    pub fn run(&mut self) -> Result<()> {
        // Bind the server socket to localhost:53
        let listening = SocketAddr::from_str("0.0.0.0:53").expect("Valid socket address");
        let socket = UdpSocket::bind(listening)?;
        socket.set_nonblocking(true);
        // Buffer to store incoming data
        let mut buffer = [0; 1024];

        println!("Listening on {}", listening);
        loop {
            // Receive data from a client
            let mut size: usize;
            let mut from: SocketAddr;
            loop { // Loop as long as there is no significant error
                match socket.recv_from(&mut buffer) {
                    Ok((size_, from_)) => {
                        size = size_;
                        from = from_;
                        break;
                    },
                    Err(err) => {
                        let err: std::io::Error = err;
                        if err.kind() == std::io::ErrorKind::WouldBlock {
                            sleep(Duration::from_micros(1000));
                            continue;
                        } else {
                            // Other error
                            return Err(crate::error::Error::IO(err));
                        }
                    }
                };
            };
            // println!("Message from {}", from);
            let query = &mut buffer[..size];
            let instant = Instant::now();
            if from == self.icann_resolver {
                let packet = Packet::parse(query).unwrap();

                if let Some(PendingQuery { query, from , sent}) =
                    self.pending_queries.remove(&packet.id())
                {
                    let original_query = Packet::parse(&query).unwrap();

                    let mut reply = Packet::new_reply(original_query.id());

                    let qname = original_query.questions.get(0).unwrap().qname.to_string();

                    for answer in packet.answers {
                        reply.answers.push(answer)
                    }

                    for question in original_query.questions {
                        reply.questions.push(question)
                    }

                    socket
                        .send_to(&reply.build_bytes_vec().unwrap(), from)
                        .unwrap();
                    let elapsed = sent.elapsed();
                    println!("Reply to {} within {}ms {}", from, elapsed.as_millis(), qname);
                };
            } else {
                let id = self.next_id();

                self.pending_queries.insert(
                    id,
                    PendingQuery {
                        query: query.to_vec(),
                        from,
                        sent: Instant::now()
                    },
                );

                let id_bytes = id.to_be_bytes();
                query[0] = id_bytes[0];
                query[1] = id_bytes[1];

                socket.send_to(&query, self.icann_resolver).unwrap();
            }
        }
    }

    fn next_id(&mut self) -> u16 {
        let id = self.next_id;
        let _ = self.next_id.wrapping_add(1);
        id
    }
}

impl Default for AnyDNS {
    fn default() -> Self {
        let builder = Builder::new();
        builder.build()
    }
}
