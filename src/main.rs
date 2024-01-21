#![allow(unused)]

mod error;
mod server;

use error::Result;
use server::AnyDNS;

fn main() -> Result<()> {
    let mut anydns = AnyDNS::default();
    anydns.run()?;
    
    Ok(())
}
