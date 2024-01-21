#![allow(unused)]

pub mod error;
pub mod server;
pub mod dns_thread;

pub use crate::error::{Error, Result};
pub use crate::server::{AnyDNS, Builder};
