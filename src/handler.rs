use std::io::{BufRead, Write};
use std::net::SocketAddr;
use std::io;
use std::fmt::Debug;
use {Connection};

#[derive(Debug)]
pub enum ConnectionEndedReason{
	IoError(io::Error),
	Hangup
}

pub trait Handler: Sized + Clone + Send + 'static{
	type Frame: Send;
	type InvalidFrame;
	type Error: Debug;
	
	/// Called when a new connection has been established.
	/// You can `clone` the connection and save it for later
	/// if out of band messaging is needed.
	///
	/// You can retrieve a unique connection id from `conn.get_id()` if you
	/// want to associate data with the connection
	///
	/// Blocking actions are intended to be run here if needed and will NOT affect processing
	/// of other connections. Processing for this connection will not start until after
	/// `new_connection` returns.
	///
	/// Any error returned here will close the connection, calling `connection_ended`
	
	fn new_connection(&self, conn: Connection<Self>) -> Result<(), Self::Error>;
	
	/// Called when a connection has closed.
	///
	/// The unique id of the connection is returned to perform any needed cleanup.
	/// Any blocking actions performed here will NOT affect performance of any other connections.
	///
	/// If you stored your own copy of the `Connection`, read/writes to that connection
	/// will fail.
	fn connection_ended(&self, id: usize, reason: ConnectionEndedReason);
	
	/// Called when there is data on the connection ready to be read.
	///
	/// This function is called on it's own dedicated thread. Blocking actions ran
	/// here will only slow down processing on the current connection. (frames are decoded
	/// in the same order they are received from the socket)
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
	
	/// Called when a frame is being written
	///
	/// This should be completed as quickly as possible and avoid anything that
	/// results in an error other than an io error.
	/// 
	/// Any blocking operations performed here will slow down processing for the current
	/// connection, but will not affect any other connection.
	///
	/// IO errors returned here will close the connection.
	fn encode_frame<W: Write>(&self, frame: Self::Frame, writer: &mut W) -> io::Result<()>;
	
	/// Called when a frame is ready to be processed.
	///
	/// This function is called on it's own dedicated thread. Blocking actions can be run
	/// here if needed without affecting any other connections. `on_frame` is called serially
	/// in the order that frames are received. The next frame from the same connection cannot
	/// be processed until this function returns. To allow faster processing of the next frame,
	/// you may `clone()` the connection to respond after returning from this function, but you
	/// are then in charge of responding in the correct order (if required by your protocol).
	///
	/// Any error returned will end the current connection, calling `connection_ended`.
	fn on_frame(&self, frame: Self::Frame, conn: &Connection<Self>) -> Result<(), Self::Error>;
}