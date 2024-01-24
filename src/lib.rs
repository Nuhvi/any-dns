#![allow(unused)]

pub mod error;
pub mod server;
pub mod dns_thread;
pub mod closuri;
pub mod pending_queries;

pub use crate::error::{Error, Result};
pub use crate::server::{AnyDNS, Builder};
