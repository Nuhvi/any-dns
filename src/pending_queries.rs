use std::{net::SocketAddr, time::Instant, collections::HashMap, sync::{Mutex, Arc}};

#[derive(Debug, Clone)]
pub struct PendingQuery {
    pub from: SocketAddr,
    pub query: Vec<u8>,
    pub received_at: Instant,
    pub icann_id: u16,
}

/**
 * Simple pending query store. For multi-threading, use `ThreadSafeStore``.
 */
#[derive(Debug, Clone)]
pub struct SimpleStore {
    pending_queries: HashMap<u16, PendingQuery>,
}

impl SimpleStore {
    pub fn insert(&mut self, query: PendingQuery) {
        self.pending_queries.insert(query.icann_id, query);
    }

    pub fn remove(&mut self, id: &u16) -> Option<PendingQuery> {
        self.pending_queries.remove(id)
    }

    pub fn new() -> Self {
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
pub struct ThreadSafeStore {
    pending_queries: Arc<Mutex<HashMap<u16, PendingQuery>>>,
}

impl ThreadSafeStore {
    fn insert(&mut self, query: PendingQuery) {
        let mut locked = self.pending_queries.lock().expect("Lock success");
        locked.insert(query.icann_id, query);
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

impl Clone for ThreadSafeStore {
    fn clone(&self) -> Self {
        Self { pending_queries: Arc::clone(&self.pending_queries) }
    }
}


#[derive(Debug, Clone)]
pub enum PendingStore {
    Simple(SimpleStore),
    ThreadSafe(ThreadSafeStore)
}

impl PendingStore {
    pub fn new_simple() -> Self {
        Self::Simple(SimpleStore::new())
    }

    pub fn new_thread_safe() -> Self {
        Self::ThreadSafe(ThreadSafeStore::new())
    }

    pub fn insert(&mut self, query: PendingQuery) {
        match self {
            Self::Simple(store) => {
                store.insert(query)
            },
            Self::ThreadSafe(store) => {
                store.insert(query)
            }
        }
    }

    pub fn remove(&mut self, id: &u16) -> Option<PendingQuery> {
        match self {
            Self::Simple(store) => {
                store.remove(id)
            },
            Self::ThreadSafe(store) => {
                store.remove(id)
            }
        }
    }
}