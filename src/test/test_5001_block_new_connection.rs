use {Server, Handler, ConnectionEndedReason, Connection};
use std::io::{BufRead, Write};
use std::net::{SocketAddr, TcpStream};
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
		thread::sleep(Duration::from_millis(100));
		self.sender.send(1);
        Ok(())
    }
    fn connection_ended(&self, id: usize, reason: ConnectionEndedReason){}
    fn decode_frame<R: BufRead>(&self, reader: &mut R) -> io::Result<Result<Self::Frame, ()>>{
		self.sender.send(2);
		let mut output = Vec::new();
		try!(reader.read_until('\n' as u8, &mut output));
		Ok(Ok(String::from_utf8_lossy(&output).into_owned()))
    }
	fn encode_frame<W: Write> (&self, frame: Self::Frame, writer: &mut W) -> io::Result<()>{
		writer.write_all(frame.as_bytes())
	}
    fn on_frame(&self, frame: Self::Frame, conn: Connection<Self>) -> Result<(), ()>{
		Ok(())
    }
}

#[test]
fn test() {
	println!("test 5001");
	let (send, recv) = channel();
	let echo_handler = TestHandler{
		sender: send
	};
	let thread_handler = thread::spawn(move ||{
		let server = Server::new(echo_handler);
    	server.run("0.0.0.0:5001").unwrap();
	});
	thread::sleep(Duration::from_millis(1000));
	println!("connecting to server");
	let mut stream = TcpStream::connect("127.0.0.1:5001").unwrap();
	println!("connected to server");
	stream.write_all("test".as_bytes()).unwrap();
	assert_eq!(recv.recv().unwrap(), 1);
	assert_eq!(recv.recv().unwrap(), 2);
}