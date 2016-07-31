extern crate mio;
extern crate slab;
extern crate threadpool;
extern crate crossbeam;

#[cfg(test)]
mod test;

mod error;
mod handler;
mod server;
mod socket;
mod event_loop_message;
mod server_socket_handler;
mod client_handler;
mod connection;
mod async_reader;

pub type ProtocolResult<T> = Result<T, error::Error>;

pub use server::{Server};
pub use connection::Connection;
pub use handler::{ConnectionEndedReason, Handler};