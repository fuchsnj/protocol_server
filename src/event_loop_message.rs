use mio;
use handler::ConnectionEndedReason;
use mio::tcp::TcpStream;
use std::net::SocketAddr;

pub enum EventLoopMessage{
	NewConnection(TcpStream, SocketAddr),
	Write(mio::Token),
	ContinueWrite(mio::Token, usize, Vec<u8>),
	EndShutdown(mio::Token, ConnectionEndedReason),
	Read(mio::Token)
	// Register(mio::Token),
	// ReRegister(mio::Token),
	// CancelDecode(mio::Token, Vec<u8>),
	// //
	// //Write(mio::Token, )
	//NewConnection()
}