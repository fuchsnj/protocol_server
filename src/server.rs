use mio;
use mio::tcp::*;
use mio::util::Slab;
use std::marker::PhantomData;
use std::net::ToSocketAddrs;
use std::io;
use mio::{TryRead, TryWrite};
use std::io::{Cursor, Read, Write, BufRead};

use {Handler, ProtocolResult};

const SERVER: mio::Token = mio::Token(0);

struct Connection<H>
where H: Handler{
	socket: TcpStream,
	token: mio::Token,
	session: H::Session,
	read_buf: Option<Vec<u8>>
}
// impl<H> Connection<H>
// where H: Handler{
// 	// fn new(socket: TcpStream, token: mio::Token) -> Connection<H>{
// 	// 	Connection{
// 	// 		socket: socket,
// 	// 		token: token
// 	// 	}
// 	// }
// 	// fn ready<H>(&mut self, event_loop: &mut mio::EventLoop<RunningServer<H>>, events: mio::EventSet){
// 	// 	println!("ready to read: {}", events.is_readable());
// 	// 	println!("ready to write: {}", events.is_writable());
// 	// 	let mut data = Vec::new();
// 	// 	self.socket.try_read_buf(&mut data).unwrap();
// 	// 	println!("read string: {:?}", data);
// 	// 	//let connection = &mut self.connections[token];
// 	// 	//let connection = self.connections[token];
// 	// }
// }

struct AsyncReader<R: BufRead>{
	inner: R
}
impl<R: BufRead> AsyncReader<R>{
	fn new(inner: R) -> AsyncReader<R>{
		AsyncReader{
			inner: inner
		}
	}
	fn into_inner(self) -> R{
		self.inner
	}
}
impl<R: BufRead> BufRead for AsyncReader<R>{
	fn fill_buf(&mut self) -> io::Result<&[u8]>{
		match self.inner.fill_buf(){
			Ok(data) => {
				if data.len() == 0{
					Err(io::Error::new(io::ErrorKind::WouldBlock, "Reading more data would block"))
				}else{
					Ok(data)
				}
			},
			Err(err) => Err(err)
		}
	}
    fn consume(&mut self, amt: usize){
		self.inner.consume(amt)
	}
}
impl<R: BufRead> Read for AsyncReader<R>{
	fn read(&mut self, buf: &mut [u8]) -> io::Result<usize>{
		match self.inner.read(buf){
			Ok(size) => {
				println!("size: {}", size);
				Ok(size)
			},
			Err(err) => Err(err)
		}
	}
}

struct RunningServer<H>
where H: Handler{
	server: TcpListener,
	connections: Slab<Connection<H>>,
	handler: H
}
impl<H> RunningServer<H>
where H: Handler{
	fn new(server: TcpListener, handler: H) -> RunningServer<H>{
		RunningServer{
			server: server,
			connections: Slab::new_starting_at(mio::Token(1), 1000000),
			handler: handler
		}
	}
}

impl<H> mio::Handler for RunningServer<H>
where H: Handler{
	type Timeout = ();
	type Message = ();
	
	fn ready(&mut self, event_loop: &mut mio::EventLoop<Self>, token: mio::Token, events: mio::EventSet){
		if token == SERVER{
			println!("SERVER ready");
			match self.server.accept(){
				Ok(Some((socket, addr))) => {
					println!("accepted connection from addr: {:?}", addr);
					let session = self.handler.start_session(addr);
					let token = self.connections
						.insert_with(|token|{
							Connection{
								socket: socket,
								token: token,
								session: session,
								read_buf: None
							}
						}).unwrap();
					
					event_loop.register(
						&self.connections[token].socket,
						token,
						mio::EventSet::readable(),
						mio::PollOpt::edge() | mio::PollOpt::oneshot()
					).unwrap();
					
					event_loop.reregister(
						&self.server,
						SERVER,
						mio::EventSet::readable(),
						mio::PollOpt::edge() | mio::PollOpt::oneshot()
					).unwrap();
					// println!("accepted a socket!");
				}
				Ok(None) => {
					println!("the server socket wasn't actually ready");
				}
				Err(e) => {
					println!("listener.accept() errored: {}", e);
					event_loop.shutdown();
				}
			}
		}else{
			let mut read_buf = match self.connections[token].read_buf.take(){
				Some(buf) => buf,
				None => Vec::new()
			};
			self.connections[token].socket.try_read_buf(&mut read_buf).unwrap();
			
			let mut async_reader = AsyncReader::new(Cursor::new(read_buf));
			match self.handler.decode_frame(&mut async_reader){
				Ok(Ok(frame)) => {
					self.handler.on_frame(frame, &mut self.connections[token].session);
					let cursor = async_reader.into_inner();
					let pos = cursor.position();
					let read_buf = cursor.into_inner();
					if read_buf.len() as u64 != pos+1{
						self.connections[token].read_buf = Some((&read_buf[pos as usize..]).to_vec());
					}
				},
				Ok(Err(bad_frame)) => {
					println!("protocol error");
				}
				Err(io_err) => {
					if io_err.kind() == io::ErrorKind::WouldBlock{
						println!("would block: {:?}", io_err);
						self.connections[token].read_buf = Some(async_reader.into_inner().into_inner());
					}else{
						println!("real IO error");
					}
				}
			};
			
			event_loop.reregister(
				&self.connections[token].socket,
				token,
				mio::EventSet::readable(),
				mio::PollOpt::edge() | mio::PollOpt::oneshot()
			).unwrap();
			//let connection = &mut self.connections[token];
			//let connection = self.connections[token];
		}
	}
}

pub struct Server<H>{
	handler: H
}

impl<H> Server<H>
where H: Handler{
	pub fn new(handler: H) -> Server<H>{
		Server{
			handler: handler
        }
	}
	pub fn start<A: ToSocketAddrs>(self, addr: A) -> ProtocolResult<()>{
		let addr = match try!(addr.to_socket_addrs()).next(){
            Some(addr) => addr,
            None => {
				return Err(
					io::Error::new(io::ErrorKind::InvalidInput, "asdf").into()
				)
			}
        };

        let tcp_listener = try!(TcpListener::bind(&addr));
		
		let mut event_loop = mio::EventLoop::new().unwrap();
		try!(event_loop.register(
			&tcp_listener,
			SERVER,
			mio::EventSet::readable(),
			mio::PollOpt::edge() | mio::PollOpt::oneshot()
		));
		
		let mut running_server = RunningServer::new(tcp_listener, self.handler);
		
		println!("running event loop");
		try!(event_loop.run(&mut running_server));
		Ok(())
	}
}

