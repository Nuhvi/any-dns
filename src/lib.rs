#![allow(unused)]

pub mod error;
pub mod server;
mod task_queue;
mod dns_thread;

pub use crate::error::{Error, Result};
pub use crate::server::{AnyDNS, Builder};
