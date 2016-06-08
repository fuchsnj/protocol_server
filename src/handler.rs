use std::io::{BufRead, Write};
use std::net::SocketAddr;
use std::io;

pub enum EndSessionReason{
	IoError(io::Error)
}

pub trait Handler{
	type Frame;
	type Session;
	type InvalidFrame;
	
	fn start_session(&self, addr: SocketAddr) -> Self::Session;
	fn end_session(&self, session: Self::Session);
	fn decode_frame<R: BufRead>(&self, reader: &mut R) -> io::Result<Result<Self::Frame, Self::InvalidFrame>>;
	fn on_frame(&self, frame: Self::Frame, session: &mut Self::Session);
}