use mio::tcp::TcpListener;
use mio;
use std::io;
use std::net::ToSocketAddrs;
use event_loop_message::EventLoopMessage;
use threadpool::ThreadPool;
use handler::Handler;

const SERVER: mio::Token = mio::Token(0);

pub struct ServerSocketHandler<H: Handler>{
	socket: TcpListener,
    sender: mio::Sender<EventLoopMessage>,
    thread_pool: ThreadPool,
    handler: H
}
impl<H: Handler> ServerSocketHandler<H>{
	pub fn new<A>(addr: A, sender: mio::Sender<EventLoopMessage>, thread_pool: ThreadPool, handler: H)
        -> io::Result<ServerSocketHandler<H>>
        where A: ToSocketAddrs
    {
        let addr = match try!(addr.to_socket_addrs()).next(){
            Some(addr) => addr,
            None => {
				return Err(
					io::Error::new(io::ErrorKind::InvalidInput, "Invalid address").into()
				)
			}
        };
        
        let socket = try!(TcpListener::bind(&addr));
        
		Ok(ServerSocketHandler{
			socket: socket,
            sender: sender,
            thread_pool: thread_pool,
            handler: handler
        })
	}
    pub fn run(&mut self) -> io::Result<()>{
        let mut event_loop = try!(mio::EventLoop::new());
         try!(event_loop.register(
            &self.socket,
            SERVER,
            mio::EventSet::readable(),
            mio::PollOpt::level()
        ));
        event_loop.run(self)
    }
}

impl<H: Handler> mio::Handler for ServerSocketHandler<H>{
	type Timeout = ();
	type Message = ();
	
	fn ready(&mut self, event_loop: &mut mio::EventLoop<Self>, token: mio::Token, _events: mio::EventSet){
            match self.socket.accept(){
                Ok(Some((socket, addr))) => {
                    self.sender.send(EventLoopMessage::NewConnection(socket, addr)).unwrap();

                    
                //     let token = self.sockets.insert_with(|token|{
                //         Socket::new(socket, token, event_loop).unwrap()
                //     }).unwrap();//fails if no tokens available
                    
                //     {
                //         let sender = event_loop.channel();
                //         let handler = self.handler.clone();
                //         self.thread_pool.execute(move ||{
                //             let (send, _receive) = channel();
                //             let conn = Connection{
                //                 sender: send,
                //                 token: token
                //             };
                //             handler.new_connection(conn).unwrap();
                //             sender.send(EventLoopMessage::Register(token)).unwrap();
                //         });
                //     }
                },
                Ok(None) => {
                    println!("server.accept() would block");
                    //break;
                },
                Err(e) => {
                    panic!("listener.accept() errored: {}", e);
                }
            }
        event_loop.reregister(
            &self.socket,
            SERVER,
            mio::EventSet::readable(),
            mio::PollOpt::level()
        ).unwrap();//server socket IO error... something is really broken
    }
}