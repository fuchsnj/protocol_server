use {Server, Handler};
use std::io::{BufRead, Write};
use std::net::SocketAddr;
use std::io;

struct EchoHandler;
impl Handler for EchoHandler{
    type Frame = String;
    type Session = u64;
    type InvalidFrame = ();
    
    fn start_session(&self, addr: SocketAddr) -> u64{
        println!("connected");
        0
    }
    fn end_session(&self, session: u64){
        println!("disconnected: {}", session);
    }
    fn decode_frame<R: BufRead>(&self, reader: &mut R) -> io::Result<Result<String, ()>>{
        let mut output = Vec::new();
        try!(reader.read_until('\n' as u8, &mut output));
        Ok(Ok(String::from_utf8_lossy(&output).into_owned()))
    }
    fn on_frame(&self, frame: String, session: &mut u64){
        *session += 1;
        println!("processing frame: {}:{}", *session, frame);
    }
}

#[test]
fn it_works() {
	//let address = .parse().unwrap();
    let echo_handler = EchoHandler;
	let server = Server::new(echo_handler);
    
    server.start("0.0.0.0:5555").unwrap();
	// 
	// 


	
	
	// panic!("asdf");
}