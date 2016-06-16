use mio;
use handler::ConnectionEndedReason;
use mio::tcp::TcpStream;
use std::net::SocketAddr;

pub enum EventLoopMessage{
	NewConnection(TcpStream, SocketAddr),
	Write(mio::Token),
	ContinueWrite(mio::Token, usize, Vec<u8>)
	// Register(mio::Token),
	// ReRegister(mio::Token),
	// CancelDecode(mio::Token, Vec<u8>),
	// //StartShutdown(mio::Token, ConnectionEndedReason)
	// //Write(mio::Token, )
	//NewConnection()
}