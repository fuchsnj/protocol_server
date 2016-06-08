extern crate mio;

#[cfg(test)]
mod test;

mod error;
mod handler;
mod server;

pub type ProtocolResult<T> = Result<T, error::Error>;

pub use server::Server;
pub use handler::Handler;