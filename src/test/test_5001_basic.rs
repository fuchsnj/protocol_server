use {Server, Handler, EndSessionReason, Connection};
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
    type Frame = ();
    type InvalidFrame = ();
	type Error = ();
    
    fn new_connection(&self, conn: &Connection<()>) -> Result<(), ()>{
        thread::sleep(Duration::from_millis(1000));
		self.sender.send(1);
        Ok(())
    }
    fn connection_ended(&self, id: usize, reason: EndSessionReason){}
    fn decode_frame<R: BufRead>(&self, reader: &mut R) -> io::Result<Result<(), ()>>{
        // let mut output = Vec::new();
        // try!(reader.read_until('\n' as u8, &mut output));
        // Ok(Ok(String::from_utf8_lossy(&output).into_owned()))
		self.sender.send(2);
		Ok(Ok(()))
    }
    fn on_frame(&self, frame: (), conn: &Connection<()>) -> Result<(), ()>{
		Ok(())
    }
}

#[test]
fn block_new_connection() {
	let (send, recv) = channel();
	let echo_handler = TestHandler{
		sender: send
	};
	let server = Server::new(echo_handler);
    server.start("0.0.0.0:5001").unwrap();
	let mut stream = TcpStream::connect("127.0.0.1:5001").unwrap();
	stream.write_all("test".as_bytes()).unwrap();
	assert_eq!(recv.recv().unwrap(), 1);
	assert_eq!(recv.recv().unwrap(), 2);
}