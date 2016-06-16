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
use client_handler::ClientHandler;

use {Handler, ProtocolResult};
use handler::ConnectionEndedReason;
use threadpool::ThreadPool;
use std::thread;
use socket::Socket;
use socket;
use event_loop_message::EventLoopMessage;

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

pub struct Server<H>{
	handler: H
}
//TODO: allow setting read/write timeout (per connection maybe?)
//TODO: set notify queue size very high
impl<H> Server<H>
where H: Handler{
	pub fn new(handler: H) -> Server<H>{
		Server{
			handler: handler
        }
	}
	pub fn start<A: ToSocketAddrs>(self, addr: A) -> ProtocolResult<()>{
		
		let thread_pool = ThreadPool::new(100);
		
		
		
		let mut event_loop = try!(mio::EventLoop::new());
		let client_sender = event_loop.channel();
		let mut client_handler = ClientHandler::new(self.handler.clone(), thread_pool.clone());
		let client_thread = thread::spawn(move ||{
			client_handler.run(&mut event_loop);
		});
		
		
		let mut server_socket_handler = try!(
			ServerSocketHandler::new(addr, client_sender, thread_pool, self.handler.clone())
		);
		let server_socket_thread = thread::spawn(move ||{
			server_socket_handler.run();
		});
		
		//let mut event_loop = mio::EventLoop::new().unwrap();
		// let mut event_loop_2 = mio::EventLoop::new().unwrap();
		// let sender1 = event_loop_1.channel();
		// let sender2 = event_loop_2.channel();
		// let mut handler1 = ClientHandler::new(self.handler.clone(), 0);
		// let mut handler2 = ClientHandler::new(self.handler.clone(), 1);
		



        
		
		// let stream;
		// let addr;
		// loop{
		// 	if let Some((a, b)) = tcp_listener.accept().unwrap(){
		// 		stream = a;
		// 		addr = b;
		// 		break;	
		// 	}
		// }
		// let stream2 = stream.try_clone().unwrap();
		// println!("got connection from: {:?}", addr);
		
		// try!(event_loop_1.register(
		// 	&stream,
		// 	mio::Token(0),
		// 	mio::EventSet::readable(),
		// 	mio::PollOpt::edge() | mio::PollOpt::oneshot()
		// ));
		// // event_loop_1.deregister(&stream);
		
		// try!(event_loop_2.register(
		// 	&stream2,
		// 	mio::Token(1),
		// 	mio::EventSet::readable(),
		// 	mio::PollOpt::edge() | mio::PollOpt::oneshot()
		// ));
		// event_loop_2.deregister(&stream2);
		//drop(stream);
		//drop(stream2);
		
		// let client_thread_1 = thread::spawn(move ||{
		// 	event_loop_1.run(&mut handler1).unwrap();
		// });
		// let client_thread_2 = thread::spawn(move ||{
		// 	event_loop_2.run(&mut handler2).unwrap();
		// }).join();
		
		
		// let mut server_handler = ServerSocketHandler::new(tcp_listener);
		
		
		// let server_thread = thread::spawn(move ||{
		// 	server_handler.run();
		// }).join();
		
		
		//let num_event_loops = 2;
		
		// for index in 0..num_event_loops{
		// 	let mut event_loop = mio::EventLoop::new().unwrap();
		// 	let running_server = running_server.clone();
		// 	thread::spawn(move ||{
		// 		event_loop.run(&mut running_server).unwrap();
		// 	});
		// }
		
		
		
		
		
		// try!(event_loop.register(
		// 	&tcp_listener,
		// 	SERVER,
		// 	mio::EventSet::readable(),
		// 	mio::PollOpt::edge() | mio::PollOpt::oneshot()
		// ));
		//TODO: spawn multiple event loops in multiple threads
		// each time reregistration occurs, choose random eventloop
		// using the event loop channels and register to random eventloop
		
		server_socket_thread.join();
		client_thread.join();
		Ok(())
	}
}

