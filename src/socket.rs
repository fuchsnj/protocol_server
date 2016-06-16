use mio;
use std::sync::{Arc, Mutex};
use mio::tcp::TcpStream;
use Handler;
use client_handler::ClientHandler;
use std::io;
use mio::TryRead;
use event_loop_message::EventLoopMessage;
use handler::ConnectionEndedReason;
use crossbeam::sync::SegQueue;
use std::io::Write;

pub enum Error{
	IoError(io::Error),
	ReadShutdown,
	WouldBlock,
	ShuttingDown
}

#[derive(Debug)]
pub enum WriteError{
	WouldBlock(usize, Vec<u8>),
	IoError(io::Error)
}
impl From<io::Error> for WriteError{
	fn from(err: io::Error) -> WriteError{
		WriteError::IoError(err)
	}
}

pub struct SocketWriter<Frame>{
	frame_out_buf: Arc<SegQueue<Frame>>,
	write_buf: Option<(usize, Vec<u8>)>,
	socket: TcpStream
}
impl<F: Send> SocketWriter<F>{
	pub fn write_buffer<H:Handler<Frame = F>>(&mut self, index: usize, buf: Vec<u8>, handler: &mut H) -> Result<(), WriteError>{
		println!("write buffer starting from index={}", index);
		let mut total_written = index;
		
		while total_written < buf.len(){
			let num_bytes = match self.socket.write(&buf[index..]){
				Ok(bytes) => bytes,
				Err(err) => {
					if err.kind() == io::ErrorKind::WouldBlock{
						return Err(WriteError::WouldBlock(total_written, buf));
					}else{
						return Err(WriteError::IoError(err))
					}
				}
			};
			total_written += num_bytes;
			println!("wrote bytes: {} total={}", num_bytes, total_written);
		}
		Ok(())
	}
	pub fn get_buf_to_write<H:Handler<Frame = F>>(&mut self, handler: &mut H) -> Result<Option<(usize, Vec<u8>)>, WriteError>{
		if let Some(indexed_buf) = self.write_buf.take(){
			Ok(Some(indexed_buf))
		}else{
			match self.frame_out_buf.try_pop(){
				Some(frame) => {
					//TODO: get buffer from buffer pool
					let mut buf = Vec::new();
					handler.encode_frame(frame, &mut buf).unwrap();
					Ok(Some((0, buf)))
				},
				None => {
					println!("no frame to send");
					Ok(None)
				}
			}
		}
	}
	pub fn write<H:Handler<Frame = F>>(&mut self, handler: &mut H) -> Result<(), WriteError>{
		loop{//send until WouldBlock or all frames sent
			if let Some((index, buf)) = try!(self.get_buf_to_write(handler)){
				try!(self.write_buffer(index, buf, handler));
			}else{
				return Ok(());
			}
		}
	}
}

pub struct Socket<Frame>{
	token: mio::Token,
	read_buf: Option<Vec<u8>>,
	frame_out_buf: Arc<SegQueue<Frame>>,
	write_buf: Option<(usize, Vec<u8>)>,
	socket: TcpStream,
	reading_blocked: bool,
	shutting_down: bool,
	registered: bool,
	write_ready: bool,
	read_ready: bool,
	//event_loop_sender: mio::Sender<EventLoopMessage>
}
impl<F> Socket<F>{
	pub fn new<H: Handler>(socket: TcpStream, token: mio::Token,
		event_loop: &mut mio::EventLoop<ClientHandler<H>>) -> io::Result<Socket<F>>{

		let conn = Socket{
			token: token,
			socket: socket,
			read_buf: None,
			frame_out_buf: Arc::new(SegQueue::new()),
			write_buf: None,
			reading_blocked: false,
			shutting_down: false,
			registered: false,
			write_ready: false,
			read_ready: false
		};
		Ok(conn)
	}
	pub fn continue_writing<H: Handler>(&mut self, index: usize, buf: Vec<u8>, event_loop: &mut mio::EventLoop<ClientHandler<H>>){
		self.write_buf = Some((index, buf));
		self.set_write_readiness(true, event_loop);
	}
	pub fn get_write_buf(&self) -> Arc<SegQueue<F>>{
		self.frame_out_buf.clone()
	}
	pub fn get_socket_writer(&self) -> io::Result<SocketWriter<F>>{
		Ok(SocketWriter{
			frame_out_buf: self.frame_out_buf.clone(),
			write_buf: self.write_buf.clone(),
			socket: try!(self.socket.try_clone())
		})
	}
	//pub fn append_write_buffer(&mut self)
	// pub fn read<H: Handler>(&mut self, event_loop: &mut mio::EventLoop<ClientHandler<H>>) -> Option<Vec<u8>>{
	// 	if self.shutting_down{
	// 		return None
	// 	}
	// 	let mut read_buf = match self.read_buf.take(){
	// 		Some(buf) => buf,
	// 		None => Vec::new()
	// 	};
	// 	match self.socket.try_read_buf(&mut read_buf){
	// 		Ok(Some(0)) => {
	// 			//READ SHUTDOWN, need to attempt to write remaining write
	// 			// buffer if any, then shutdown
	// 			None
	// 		},
	// 		Ok(Some(_)) => {
	// 			self.reading_blocked = true;
	// 			Some(read_buf)
	// 		},
	// 		Err(err) => {
	// 			//let msg = EventLoopMessage::StartShutdown(self.token, ConnectionEndedReason::IoError(err));
	// 			//self.event_loop_sender.send(msg).unwrap();
	// 			None
	// 		},
	// 		Ok(None) => {//WouldBlock
   	// 			self.set_read_buffer(read_buf);
	// 			self.reregister(event_loop);
	// 			None
	// 		}
	// 	}
	// }
	// pub fn set_read_buffer(&mut self, buf: Vec<u8>){
	// 	self.read_buf = Some(buf);
	// }
	// pub fn take_read_buffer(&mut self) -> Option<Vec<u8>>{
	//     self.read_buf.take()
	// }
	fn get_event_set(&self) -> mio::EventSet{
		let mut output = mio::EventSet::none();
		if self.read_ready{
			output = output | mio::EventSet::readable();
		}
		if self.write_ready{
			output = output | mio::EventSet::writable();
		}
		output
	}
	pub fn set_readiness<H: Handler>(&mut self, read: Option<bool>, write: Option<bool>, event_loop: &mut mio::EventLoop<ClientHandler<H>>) -> io::Result<()>{
		let mut registration_needed = false;
		
		if let Some(read) = read{
			if read != self.read_ready{
				self.read_ready = read;
				registration_needed = true;
			}
		}
		if let Some(write) = write{
			if write != self.write_ready{
				self.write_ready = write;
				registration_needed = true;
			}
		}
		if registration_needed{
			try!(self.register(event_loop));
		}
		Ok(())
	}
	
	pub fn set_write_readiness<H: Handler>(&mut self, ready: bool, event_loop: &mut mio::EventLoop<ClientHandler<H>>) -> io::Result<()>{
		self.set_readiness(None, Some(ready), event_loop)
	}
	pub fn set_read_readiness<H: Handler>(&mut self, ready: bool, event_loop: &mut mio::EventLoop<ClientHandler<H>>) -> io::Result<()>{
		self.set_readiness(Some(ready), None, event_loop)
	}
	fn register<H: Handler>(&mut self, event_loop: &mut mio::EventLoop<ClientHandler<H>>) -> io::Result<()>{
		let event_set = self.get_event_set();
		println!("registering for [{:?}]", event_set);
		if !self.registered{
			try!(event_loop.register(
				&self.socket,
				self.token,
				event_set,
				mio::PollOpt::level()
			));
		}else{
			try!(event_loop.reregister(
				&self.socket,
				self.token,
				event_set,
				mio::PollOpt::level()
			));
		}
		self.registered = true;
		Ok(())
	}
	// fn get_event_set(&self) -> mio::EventSet{
	// 	let write = match self.write_buf{
	// 		Some(_) => mio::EventSet::writable(),
	// 		None => mio::EventSet::none()
	// 	};
	// 	mio::EventSet::readable() | write
	// }
	pub fn get_id(&self) -> usize{
		self.token.0
	}
}