use {Server, Handler, ConnectionEndedReason, Connection};
use std::io::{BufRead, Write, Read, BufReader};
use std::net::{SocketAddr, TcpStream, Shutdown};
use std::io;
use std::sync::mpsc::channel;
use std::sync::mpsc::{Sender, Receiver};
use std::thread;
use std::sync::Arc;
use std::time::Duration;

#[derive(Clone)]
struct TestHandler{
	sender: Sender<u64>
}
impl Handler for TestHandler{
	type Frame = String;
	type InvalidFrame = ();
	type Error = ();
	
	fn new_connection(&self, conn: Connection<Self>) -> Result<(), ()>{
		conn.send("test\n".to_owned());
		Ok(())
	}
	fn connection_ended(&self, id: usize, reason: ConnectionEndedReason){
		self.sender.send(1);
	}
	fn decode_frame<R: BufRead>(&self, reader: &mut R) -> io::Result<Result<Self::Frame, ()>>{
		unreachable!()
	}
	fn encode_frame<W: Write> (&self, frame: Self::Frame, writer: &mut W) -> io::Result<()>{
		writer.write_all(frame.as_bytes())
	}
	fn on_frame(&self, frame: Self::Frame, conn: Connection<Self>) -> Result<(), ()>{
		unreachable!()
	}
}

#[test]
fn test() {
	let (send, recv) = channel();
	let echo_handler = TestHandler{
		sender: send
	};
	let thread_handle = thread::spawn(move ||{
		let server = Server::new(echo_handler);
		server.run("0.0.0.0:5003").unwrap();
	});
	thread::sleep(Duration::from_millis(1000));
	let mut stream = TcpStream::connect("127.0.0.1:5003").unwrap();
	let mut out = String::new();
	let mut buf_reader = BufReader::new(stream);
	buf_reader.read_line(&mut out).unwrap();
	assert_eq!(out, "test\n".to_owned());
	drop(buf_reader);
	assert_eq!(recv.recv().unwrap(), 1);
}