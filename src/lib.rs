#![allow(unused)]

pub mod error;
pub mod server;
mod dns_thread;
mod custom_handler;
mod pending_queries;

pub use crate::error::{Error, Result};
pub use crate::server::{AnyDNS, Builder};
pub use crate::custom_handler::{CustomHandler};