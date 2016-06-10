use std::io::{BufRead, Write};
use std::net::SocketAddr;
use std::io;
use std::fmt::Debug;
use {Connection};

#[derive(Debug)]
pub enum EndSessionReason{
	IoError(io::Error),
	ClientShutdown
}

pub trait Handler: Sized + Clone + Send + 'static{
	type Frame;
	type InvalidFrame;
	type Error: Debug;
	
	/// Called when a new connection has been established.
	/// You can `clone` the connection and save it for later
	/// if out of band messaging is needed.
	///
	/// You can retrieve a unique connection id from `conn.get_id()` if you
	/// want to associate data with the connection
	///
	/// This function is called on it's own dedicated thread. Blocking actions are
	/// intended to be run here if needed and will only block incoming IO on the current connection.
	/// No other connections are affected.
	///
	/// Any error returned here will close the connection, calling `connection_ended`
	
	fn new_connection(&self, conn: &Connection<Self::Frame>) -> Result<(), Self::Error>;
	
	/// Called when a connection has closed.
	///
	/// The unique id of the connection is returned to perform any needed cleanup.
	///
	/// If you stored your own copy of the `Connection`, read/writes to that connection
	/// will fail.
	fn connection_ended(&self, id: usize, reason: EndSessionReason);
	
	/// Called when there is data on the connection ready to be read.
	///
	/// This function is called on it's own dedicated thread. Blocking actions ran
	/// here will 
	/// No other connections are affected.
	///
	/// If any read operation reaches EOF, it is assumed there is not enough data
	/// for a complete frame, causing a `WouldBlock` IoError. You should return any IoError you receive.
	///
	/// If a `WouldBlock` IoError is returned, all currently read data is restored and
	/// this function will be called later when there is more data available.
	///
	/// If a protocol error occurs, you should save any state you need to handle it later as an InvalidFrame.
	///
	/// Actions that would return an error other than IOError should not be performed in this function.
	fn decode_frame<R: BufRead>(&self, reader: &mut R) -> io::Result<Result<Self::Frame, Self::InvalidFrame>>;
	
	
	/// Called when a frame is ready to be processed.
	///
	/// This function is called on it's own dedicated thread. Blocking actions are
	/// intended to be run here if needed and will not slow down processing of
	/// this, or any other connection. 
	///
	/// It is possible that multiple frames could be processed by this function for the
	/// same connection in parallel. While it is guaranteed that this function will be called
	/// in the order requests are received, since they are being processed in parallel
	/// responses could be sent in any order.
	///
	/// Any error returned will end the current connection, calling `connection_ended`.
	fn on_frame(&self, frame: Self::Frame, conn: &Connection<Self::Frame>) -> Result<(), Self::Error>;
}