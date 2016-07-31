use std::io::{BufRead, Read};
use std::io;

pub struct AsyncReader<R: BufRead>{
	inner: R
}
impl<R: BufRead> AsyncReader<R>{
	pub fn new(inner: R) -> AsyncReader<R>{
		AsyncReader{
			inner: inner
		}
	}
	pub fn into_inner(self) -> R{
		self.inner
	}
}
impl<R: BufRead> BufRead for AsyncReader<R>{
	fn fill_buf(&mut self) -> io::Result<&[u8]>{
		match self.inner.fill_buf(){
			Ok(data) => {
                println!("fill buf: {:?}", data);
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
				Ok(size)
			},
			Err(err) => Err(err)
		}
	}
}