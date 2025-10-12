use core::ptr;

use kstd::io;

use crate::memory::virt::{USERIMG_SIZE, USERIMG_START};

pub struct Reader {
    pos: usize,
}

impl Reader {
    pub fn new() -> Self {
        Self { pos: 0 }
    }
}

impl io::Read for Reader {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, io::Error> {
        let remaining = USERIMG_SIZE - self.pos;
        let len = buf.len().min(remaining);

        let src = (USERIMG_START + self.pos).as_ptr::<u8>();
        let dst = buf.as_mut_ptr().cast();
        unsafe { ptr::copy_nonoverlapping(src, dst, len) };

        self.pos += len;
        Ok(len)
    }
}

impl io::Seek for Reader {
    fn seek(&mut self, pos: u64) -> Result<(), io::Error> {
        let pos = pos as usize;
        if pos < USERIMG_SIZE {
            self.pos = pos;
            Ok(())
        } else {
            Err(io::Error::SeekOutOfBounds)
        }
    }
}
