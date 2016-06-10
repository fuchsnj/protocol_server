use mio;
use mio::tcp::*;
use mio::util::Slab;
use std::marker::PhantomData;
use std::net::ToSocketAddrs;
use std::io;
use mio::{TryRead, TryWrite};
use std::io::{Cursor, Read, Write, BufRead};
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{channel, Sender, Receiver};

use {Handler, ProtocolResult};
use handler::EndSessionReason;
use threadpool::ThreadPool;
use std::thread;

const SERVER: mio::Token = mio::Token(0);

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

struct Socket{
	token: mio::Token,
	read_buf: Option<Vec<u8>>,
	write_buf: Option<Vec<u8>>,
	socket: TcpStream
}
impl Socket{
	fn new<H: Handler>(socket: TcpStream, token: mio::Token,
		event_loop: &mut mio::EventLoop<RunningServer<H>>) -> io::Result<Socket>{

		let conn = Socket{
			token: token,
			socket: socket,
			read_buf: None,
			write_buf: None
		};
		Ok(conn)
	}
	fn reregister<H: Handler>(&self, event_loop: &mut mio::EventLoop<RunningServer<H>>) -> io::Result<()>{
		event_loop.reregister(
			&self.socket,
			self.token,
			self.get_event_set(),
			mio::PollOpt::edge() | mio::PollOpt::oneshot()
		)
	}
	fn register<H: Handler>(&self, event_loop: &mut mio::EventLoop<RunningServer<H>>) -> io::Result<()>{
		event_loop.register(
			&self.socket,
			self.token,
			self.get_event_set(),
			mio::PollOpt::edge() | mio::PollOpt::oneshot()
		)
	}
	fn get_event_set(&self) -> mio::EventSet{
		let write = match self.write_buf{
			Some(_) => mio::EventSet::writable(),
			None => mio::EventSet::none()
		};
		mio::EventSet::readable() | write
	}
	fn get_id(&self) -> usize{
		self.token.0
	}
}

pub struct Connection<F>{
	sender: Sender<F>,
	token: mio::Token
}

enum Message{
	Register(mio::Token),
	ReRegister(mio::Token)
}

struct RunningServer<H>
where H: Handler{
	server: TcpListener,
	sockets: Slab<Socket>,
	handler: H,
	thread_pool: ThreadPool
}
impl<H> RunningServer<H>
where H: Handler{
	fn new(server: TcpListener, handler: H) -> RunningServer<H>{
		RunningServer{
			server: server,
			sockets: Slab::new_starting_at(mio::Token(1), 1000000),
			handler: handler,
			thread_pool: ThreadPool::new(100)
		}
	}
	fn shutdown_connection(&mut self, token: mio::Token, reason: EndSessionReason){
		if let Some(sock) = self.sockets.remove(token){
			self.handler.connection_ended(sock.get_id(), reason);
		}
	}
}

impl<H> mio::Handler for RunningServer<H>
where H: Handler{
	type Timeout = ();
	type Message = Message;
	
	fn notify(&mut self, event_loop: &mut mio::EventLoop<Self>, msg: Message){
		match msg{
			Message::Register(token) => {
				self.sockets[token].register(event_loop).unwrap();
			},
			Message::ReRegister(token) => {
				self.sockets[token].reregister(event_loop).unwrap();
			}
		}
	}
	
	fn ready(&mut self, event_loop: &mut mio::EventLoop<Self>, token: mio::Token, _events: mio::EventSet){
		if token == SERVER{
			loop{
				match self.server.accept(){
					Ok(Some((socket, _addr))) => {
						println!("connection accepted");
						// let (send, _receive) = channel();
						// let conn = Connection{
						// 	sender: send
						// };
						
									
						let token = self.sockets.insert_with(|token|{
							Socket::new(socket, token, event_loop).unwrap()
						}).unwrap();//fails if no tokens available
						
						{
							let sender = event_loop.channel();
							let handler = self.handler.clone();
							self.thread_pool.execute(move ||{
								println!("NEW CONNECTION!");
								
								let (send, _receive) = channel();
								let conn = Connection{
									sender: send,
									token: token
								};
								handler.new_connection(&conn).unwrap();
								sender.send(Message::Register(token)).unwrap();
							});
						}
					},
					Ok(None) => {
						break;
					},
					Err(e) => {
						panic!("listener.accept() errored: {}", e);
					}
				}
			}
			event_loop.reregister(
				&self.server,
				SERVER,
				mio::EventSet::readable(),
				mio::PollOpt::edge() | mio::PollOpt::oneshot()
			).unwrap();//server socket IO error... something is really broken
			
		}else{
			let mut read_buf = match self.sockets[token].read_buf.take(){
				Some(buf) => buf,
				None => Vec::new()
			};
			match self.sockets[token].socket.try_read_buf(&mut read_buf){
				Ok(Some(0)) => {
					//TODO:
					//reading half of socket has closed. Flush remaining write buffer
					//then close the socket
					self.shutdown_connection(token, EndSessionReason::ClientShutdown);
					return;
				},
				Err(e) => {
					self.shutdown_connection(token, EndSessionReason::IoError(e));
					return;
				},
				Ok(_) => {/* do nothing */}
			}
			
			let mut async_reader = AsyncReader::new(Cursor::new(read_buf));
			match self.handler.decode_frame(&mut async_reader){
				Ok(Ok(frame)) => {
					let (send, _receive) = channel();
					let conn = Connection{
						sender: send,
						token: token
					};
					self.handler.on_frame(frame, &conn).unwrap();
					let cursor = async_reader.into_inner();
					let pos = cursor.position();
					let read_buf = cursor.into_inner();
					if read_buf.len() as u64 != pos+1{
						self.sockets[token].read_buf = Some((&read_buf[pos as usize..]).to_vec());
					}
				},
				Ok(Err(_bad_frame)) => {
					println!("protocol error");
				}
				Err(io_err) => {
					if io_err.kind() == io::ErrorKind::WouldBlock{
						self.sockets[token].read_buf = Some(async_reader.into_inner().into_inner());
					}else{
						panic!("real IO error");
					}
				}
			};
			self.sockets[token].reregister(event_loop).unwrap();
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
		thread::spawn(move ||{
			event_loop.run(&mut running_server).unwrap();
		});
		Ok(())
	}
}

