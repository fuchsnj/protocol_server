use {Server, Handler, EndSessionReason, Connection};
use std::io::{BufRead, Write};
use std::net::{SocketAddr, TcpStream};
use std::io;
use std::io::Read;
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
    
    fn new_connection(&self, conn: Connection<Self::Frame>) -> Result<(), ()>{
		self.sender.send(1);
        Ok(())
    }
    fn connection_ended(&self, id: usize, reason: EndSessionReason){}
    fn decode_frame<R: BufRead>(&self, reader: &mut R) -> io::Result<Result<Self::Frame, ()>>{
		self.sender.send(2);
		let mut output = Vec::new();
		try!(reader.read_until('\n' as u8, &mut output));
		Ok(Ok(String::from_utf8_lossy(&output).into_owned()))
    }
    fn on_frame(&self, frame: Self::Frame, mut conn: Connection<Self::Frame>) -> Result<(), ()>{
        self.sender.send(3);
		conn.write_all(frame.as_bytes()).unwrap();
		Ok(())
    }
}

#[test]
fn test() {
	println!("test 5002");
	let (send, recv) = channel();
	let echo_handler = TestHandler{
		sender: send
	};
	let server = Server::new(echo_handler);
    server.start("0.0.0.0:5002").unwrap();
	let mut stream = TcpStream::connect("127.0.0.1:5002").unwrap();
	stream.write_all("test".as_bytes()).unwrap();
	
	assert_eq!(recv.recv().unwrap(), 1);
	assert_eq!(recv.recv().unwrap(), 2);
	
	stream.write_all("\nabcd".as_bytes()).unwrap();
    println!("3");
	assert_eq!(recv.recv().unwrap(), 2);/////
    println!("4");
	assert_eq!(recv.recv().unwrap(), 3);
	println!("5");
	let mut data = String::new();
	println!("reading data to string");
	stream.read_to_string(&mut data).unwrap();
	
	assert_eq!(&data, "test\n");
}