extern crate mio;
extern crate slab;
extern crate threadpool;

#[cfg(test)]
mod test;

mod error;
mod handler;
mod server;

pub type ProtocolResult<T> = Result<T, error::Error>;

pub use server::{Server, Connection};
pub use handler::{EndSessionReason, Handler};