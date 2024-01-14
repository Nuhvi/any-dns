use simple_dns::Packet;
use std::{
    collections::HashMap,
    net::{SocketAddr, UdpSocket},
};

#[derive(Debug)]
struct PendingQuery {
    id: u16,
    from: SocketAddr,
}

struct AnyDNS {
    next_id: u16,
    icann_resolver: SocketAddr,
    pending_queries: HashMap<u16, PendingQuery>,
}

impl AnyDNS {
    pub fn run(&mut self) -> std::io::Result<()> {
        // Bind the server socket to localhost:53
        let socket = UdpSocket::bind(("0.0.0.0", 53))?;

        // Buffer to store incoming data
        let mut buffer = [0; 1024];

        loop {
            // Receive data from a client
            let (size, from) = socket.recv_from(&mut buffer)?;

            let query = &mut buffer[..size];
            let packet = Packet::parse(query).unwrap();

            if from == self.icann_resolver {
                if let Some(PendingQuery { id, from }) = self.pending_queries.remove(&packet.id()) {
                    let mut reply = Packet::new_reply(id);

                    for answer in packet.answers {
                        reply.answers.push(answer)
                    }

                    socket
                        .send_to(&reply.build_bytes_vec().unwrap(), from)
                        .unwrap();
                };
            } else {
                let id = self.next_id();

                self.pending_queries.insert(
                    id,
                    PendingQuery {
                        id: packet.id(),
                        from,
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
        // let icann_resolver: SocketAddr = SocketAddr::from(([8, 8, 8, 8], 53));
        let icann_resolver: SocketAddr = SocketAddr::from(([1, 1, 1, 1], 53));
        // let icann_resolver: SocketAddr = SocketAddr::from(([192, 168, 1, 1], 53));

        Self {
            next_id: 0,
            icann_resolver,
            pending_queries: HashMap::new(),
        }
    }
}

fn main() -> std::io::Result<()> {
    let mut anydns = AnyDNS::default();
    anydns.run()?;

    Ok(())
}
