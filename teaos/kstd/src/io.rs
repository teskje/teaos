//! Traits for common I/O operations.

pub trait Read {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Error>;

    fn read_exact(&mut self, buf: &mut [u8]) -> Result<(), Error> {
        if self.read(buf)? == buf.len() {
            Ok(())
        } else {
            Err(Error::UnexpectedEof)
        }
    }
}

pub trait Write {
    fn write(&mut self, buf: &[u8]) -> Result<usize, Error>;
    fn flush(&mut self) -> Result<(), Error>;
}

pub trait Seek {
    fn seek(&mut self, pos: u64) -> Result<(), Error>;
}

#[derive(Debug)]
pub enum Error {
    UnexpectedEof,
    SeekOutOfBounds,
}
