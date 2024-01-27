use std::{net::SocketAddr, time::Instant, collections::HashMap, sync::{Mutex, Arc}};

#[derive(Debug, Clone)]
pub struct PendingQuery {
    pub from: SocketAddr,
    pub query: Vec<u8>,
    pub received_at: Instant,
    pub icann_id: u16,
}


/**
 * Multi-threading safe store.
 * Use `.clone()` to give each thread one store struct.
 * The data will stay shared.
 */
#[derive(Debug, Clone)]
pub struct ThreadSafeStore {
    pending_queries: Arc<Mutex<HashMap<u16, PendingQuery>>>,
}

impl ThreadSafeStore {
    pub fn insert(&mut self, query: PendingQuery) {
        let mut locked = self.pending_queries.lock().expect("Lock success");
        locked.insert(query.icann_id, query);
    }

    pub fn remove(&mut self, id: &u16) -> Option<PendingQuery> {
        let mut locked = self.pending_queries.lock().expect("Lock success");
        locked.remove(id)
    }

    pub fn new() -> Self {
        Self {
            pending_queries: Arc::new(Mutex::new(HashMap::new()))
        }
    }
}

// impl Clone for ThreadSafeStore {
//     fn clone(&self) -> Self {
//         Self { pending_queries: Arc::clone(&self.pending_queries) }
//     }
// }
