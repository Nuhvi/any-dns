use std::{net::SocketAddr, time::Instant, collections::HashMap, sync::{Mutex, Arc}};

#[derive(Debug, Clone)]
pub struct PendingQuery {
    from: SocketAddr,
    query: Vec<u8>,
    sent: Instant,
    id: u16
}

pub trait PendingQueryStore {
    /**
     * Create a new store
     */
    fn new() -> Self;
    /**
     * Insert query.
     */
    fn insert(&mut self, query: PendingQuery);
    /**
     * Remove and return removed query.
     */
    fn remove(&mut self, id: &u16) -> Option<PendingQuery>;
}

/**
 * Simple pending query store. For multi-threading, use `ConcurrentStore``.
 */
#[derive(Debug, Clone)]
pub struct SimpleStore {
    pending_queries: HashMap<u16, PendingQuery>,
}

impl PendingQueryStore for SimpleStore {
    fn insert(&mut self, query: PendingQuery) {
        self.pending_queries.insert(query.id, query);
    }

    fn remove(&mut self, id: &u16) -> Option<PendingQuery> {
        self.pending_queries.remove(id)
    }

    fn new() -> Self {
        Self {
            pending_queries: HashMap::new()
        }
    }
}

/**
 * Multi-threading safe store.
 * Use `.clone()` to give each thread one store struct.
 * The data will stay shared.
 */
#[derive(Debug)]
pub struct ConcurrentStore {
    pending_queries: Arc<Mutex<HashMap<u16, PendingQuery>>>,
}

impl PendingQueryStore for ConcurrentStore {
    fn insert(&mut self, query: PendingQuery) {
        let mut locked = self.pending_queries.lock().expect("Lock success");
        locked.insert(query.id, query);
    }

    fn remove(&mut self, id: &u16) -> Option<PendingQuery> {
        let mut locked = self.pending_queries.lock().expect("Lock success");
        locked.remove(id)
    }

    fn new() -> Self {
        Self {
            pending_queries: Arc::new(Mutex::new(HashMap::new()))
        }
    }
}

impl Clone for ConcurrentStore {
    fn clone(&self) -> Self {
        Self { pending_queries: Arc::clone(&self.pending_queries) }
    }
}