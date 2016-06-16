use mio;
use mio::Sender;
use std::io::Write;
use std::io;
use event_loop_message::EventLoopMessage;
use handler::Handler;
use crossbeam::sync::SegQueue;
use std::sync::Arc;

pub struct Connection<H: Handler>{
	sender: Sender<EventLoopMessage>,
	token: mio::Token,
	handler: H,
	write_queue: Arc<SegQueue<H::Frame>>
}
impl<H: Handler> Connection<H>{
	pub fn new(sender: Sender<EventLoopMessage>, token: mio::Token,
		handler: H, write_queue: Arc<SegQueue<H::Frame>>) -> Connection<H>{
		Connection{
			sender: sender,
			token: token,
			handler: handler,
			write_queue: write_queue
		}
	}
	
	/// Queues a frame to be sent
	pub fn send(&self, data: H::Frame){
		self.write_queue.push(data);
		self.sender.send(EventLoopMessage::Write(self.token));
		//let mut buf = Vec::new();
		//self.handler.encode_frame(data, &mut buf).unwrap();
		//println!("encoded frame: {:?}", buf);
		// self.sender.send(
		// 	EventLoopMessage::Write(self.token, data)
		// ).unwrap();
	}
}