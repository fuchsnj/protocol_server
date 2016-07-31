use mio;
use mio::tcp::TcpListener;
use mio::util::Slab;
use std::marker::PhantomData;
use std::net::ToSocketAddrs;
use std::io;
use mio::{TryRead, TryWrite};
use std::io::{Cursor, Read, Write, BufRead};
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{channel, Sender, Receiver};

use server_socket_handler::ServerSocketHandler;

use {Handler, ProtocolResult};
use handler::ConnectionEndedReason;
use threadpool::ThreadPool;
use std::thread;
use socket::Socket;
use socket;
use event_loop_message::EventLoopMessage;
use connection::Connection;

pub struct ClientHandler<H>
where H: Handler{
	sockets: Slab<Socket<H::Frame>>,
	handler: H,
	thread_pool: ThreadPool
}
impl<H> ClientHandler<H>
where H: Handler{
	pub fn new(handler: H, thread_pool: ThreadPool) -> ClientHandler<H>{
		ClientHandler{
			sockets: Slab::new(1000000),
			handler: handler,
			thread_pool: thread_pool
		}
	}
	pub fn run(&mut self, event_loop: &mut mio::EventLoop<ClientHandler<H>>) -> io::Result<()>{
		event_loop.run(self)
	}
	// fn connection_ended(&mut self, token: mio::Token, reason: EndSessionReason){
	// 	if let Some(sock) = self.sockets.remove(token){
	// 		//TODO: run on threadpool
	// 		self.handler.connection_ended(sock.get_id(), reason);
	// 	}
	// }
	pub fn get_connection(&self, token: mio::Token, event_loop: &mut mio::EventLoop<Self>) -> Connection<H>{
		let write_buf = self.sockets[token].get_write_buf();
		Connection::new(event_loop.channel(), token, write_buf)
	}
}

impl<H> mio::Handler for ClientHandler<H>
where H: Handler{
	type Timeout = ();
	type Message = EventLoopMessage;
	
	fn notify(&mut self, event_loop: &mut mio::EventLoop<Self>, msg: EventLoopMessage){
		match msg{
			EventLoopMessage::NewConnection(socket, addr) => {
				let token = self.sockets.insert_with(|token|{
					Socket::new(socket, token, event_loop).unwrap()
				}).unwrap();//fails if no tokens available
				let handler = self.handler.clone();
				let conn = self.get_connection(token, event_loop);
				{
					let sender = event_loop.channel();
					self.thread_pool.execute(move||{
						handler.new_connection(conn).unwrap();
						sender.send(EventLoopMessage::Read(token)).unwrap();
					});
				}
			},
			EventLoopMessage::Write(token) => {
				//TODO: use .is_empty when implemented to prevent waiting for Write when not needed
				self.sockets[token].set_write_readiness(true, event_loop).unwrap();
			},
			EventLoopMessage::ContinueWrite(token, index, buf) => {
				self.sockets[token].continue_writing(index, buf, event_loop);
			},
			EventLoopMessage::EndShutdown(token, reason) => {
				self.handler.connection_ended(token.0, reason);
				self.sockets.remove(token);
			}
			EventLoopMessage::Read(token) => {
				if let Some(socket) = self.sockets.get_mut(token){
					socket.set_read_readiness(true, event_loop).unwrap();
				}
			}
		}
	}
	
	fn ready(&mut self, event_loop: &mut mio::EventLoop<Self>, token: mio::Token, events: mio::EventSet){
		println!("ready: {:?}", events);
		if events.is_hup(){
			self.sockets[token].start_shutdown(ConnectionEndedReason::Hangup, event_loop);
			return;
		}
		if events.is_writable(){
			if let Some(socket) = self.sockets.get_mut(token){
				socket.set_write_readiness(false, event_loop).unwrap();

				let mut handler = self.handler.clone();
				let mut socket_writer = socket.get_socket_writer().unwrap();
				let sender = event_loop.channel();
				self.thread_pool.execute(move||{
					if let Err(err) = socket_writer.write(&mut handler){
						match err{
							socket::WriteError::WouldBlock(index, buf) => {
								sender.send(EventLoopMessage::ContinueWrite(token, index, buf)).unwrap();
							},
							socket::WriteError::IoError(err) => {
								println!("REAL IO ERR");
								panic!("real io err");
							}
						}
					}
					println!("DONE WRITING - no error");
				});	
			}
			
		}
		if events.is_readable(){
			if let Some(socket) = self.sockets.get_mut(token){
				socket.set_read_readiness(false, event_loop).unwrap();
				
				let mut handler = self.handler.clone();
				let mut socket_reader = socket.get_socket_reader().unwrap();
				let sender = event_loop.channel();
				let conn = Connection::new(event_loop.channel(), token, socket.get_write_buf());
				self.thread_pool.execute(move ||{
					println!("reading from socket reader");
					if let Err(err) = socket_reader.read(&mut handler, conn){
						println!("GOT READ ERR: {:?}", err);
					}
					sender.send(EventLoopMessage::Read(token)).unwrap();
				});
			}
		}
		if events.is_error(){
			panic!("err handling not implemented");
		}
		//TODO: run this in another function and catch all IO errors at once (close conn)
		// if let Some(read_buf) = self.sockets[token].read(event_loop){
		// 	let handler = self.handler.clone();
		// 	let sender = event_loop.channel();
		// 	self.thread_pool.execute(move ||{
		// 		let mut async_reader = AsyncReader::new(Cursor::new(read_buf));
		// 		match handler.decode_frame(&mut async_reader){
		// 			Ok(Ok(frame)) => {
		// 				let (send, _receive) = channel();
		// 				let conn = Connection{
		// 					sender: send,
		// 					token: token
		// 				};
		// 				handler.on_frame(frame, conn).unwrap();
		// 				// let cursor = async_reader.into_inner();
		// 				// let pos = cursor.position();
		// 				// let read_buf = cursor.into_inner();
		// 				// if read_buf.len() as u64 != pos+1{
		// 				// 	self.sockets[token].read_buf = Some((&read_buf[pos as usize..]).to_vec());
		// 				// }
		// 			},
		// 			Ok(Err(_bad_frame)) => {
		// 				println!("protocol error");
		// 			}
		// 			Err(io_err) => {
		// 				if io_err.kind() == io::ErrorKind::WouldBlock{
		// 					let read_buf = async_reader.into_inner().into_inner();
		// 					sender.send(EventLoopMessage::CancelDecode(token, read_buf)).unwrap();
		// 				}else{
		// 					panic!("real IO error");
		// 				}
		// 			}
		// 		}
		// 	});	
		// };
	}
}
