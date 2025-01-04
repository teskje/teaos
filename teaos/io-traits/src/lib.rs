//! Traits for common I/O operations.

#![no_std]

pub trait Read {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, IoError>;

    fn read_exact(&mut self, buf: &mut [u8]) -> Result<(), IoError> {
        if self.read(buf)? == buf.len() {
            Ok(())
        } else {
            Err(IoError::UnexpectedEof)
        }
    }
}

pub trait Write {
    fn write(&mut self, buf: &[u8]) -> Result<usize, IoError>;
    fn flush(&mut self) -> Result<(), IoError>;
}

pub trait Seek {
    fn seek(&mut self, pos: u64) -> Result<(), IoError>;
}

#[derive(Debug)]
pub enum IoError {
    UnexpectedEof,
}
