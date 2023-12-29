#![no_std]

use core::ffi::CStr;
use core::ops::Range;

use crate::read::Read;

mod error;
mod read;

pub use crate::error::{Error, Result};

pub struct Parser<'d> {
    header: Header,
    data: &'d [u8],
}

impl<'d> Parser<'d> {
    pub fn new(data: &'d [u8]) -> Result<Self> {
        let header = Header::parse(data)?;

        if header.totalsize as usize > data.len() {
            return Err(Error::NotEnoughData);
        }

        Ok(Self { header, data })
    }

    pub fn memory_reservations(&self) -> Result<ReserveIter> {
        let offset = self.header.off_mem_rsvmap as usize;
        match self.data.get(offset..) {
            Some(data) => Ok(ReserveIter { data }),
            None => Err(Error::NotEnoughData),
        }
    }

    pub fn tokens(&self) -> Result<TokenIter<'d>> {
        let offset = self.header.off_dt_struct as usize;
        match self.data.get(offset..) {
            Some(data) => Ok(TokenIter::new(data)),
            None => Err(Error::NotEnoughData),
        }
    }
}

struct Header {
    magic: u32,
    totalsize: u32,
    off_dt_struct: u32,
    off_dt_strings: u32,
    off_mem_rsvmap: u32,
    version: u32,
    last_comp_version: u32,
    boot_cpuid_phys: u32,
    size_dt_strings: u32,
    size_dt_struct: u32,
}

impl Header {
    fn parse(mut data: &[u8]) -> Result<Self> {
        let header = Self {
            magic: data.read_u32()?,
            totalsize: data.read_u32()?,
            off_dt_struct: data.read_u32()?,
            off_dt_strings: data.read_u32()?,
            off_mem_rsvmap: data.read_u32()?,
            version: data.read_u32()?,
            last_comp_version: data.read_u32()?,
            boot_cpuid_phys: data.read_u32()?,
            size_dt_strings: data.read_u32()?,
            size_dt_struct: data.read_u32()?,
        };

        header.validate()
    }

    fn validate(self) -> Result<Self> {
        if self.magic != 0xd00dfeed {
            Err(Error::InvalidMagic(self.magic))
        } else if self.version != 17 {
            Err(Error::UnsupportedVersion(self.version))
        } else {
            Ok(self)
        }
    }
}

pub struct ReserveIter<'d> {
    data: &'d [u8],
}

impl ReserveIter<'_> {
    fn read_entry(&mut self) -> Result<Range<usize>> {
        let address = self.data.read_u64()?;
        let size = self.data.read_u64()?;

        let start = address as usize;
        let end = start + size as usize;
        Ok(Range { start, end })
    }
}

impl Iterator for ReserveIter<'_> {
    type Item = Result<Range<usize>>;

    fn next(&mut self) -> Option<Self::Item> {
        let entry = match self.read_entry() {
            Ok(entry) => entry,
            Err(error) => return Some(Err(error)),
        };

        if entry.start == 0 && entry.is_empty() {
            None
        } else {
            Some(Ok(entry))
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum Token<'d> {
    BeginNode { name: &'d CStr },
    EndNode,
    Prop { nameoff: u32, value: &'d [u8] },
    Nop,
    End,
}

pub struct TokenIter<'d> {
    data: &'d [u8],
    done: bool,
}

impl<'d> TokenIter<'d> {
    fn new(data: &'d [u8]) -> Self {
        Self { data, done: false }
    }

    fn read_token(&mut self) -> Result<Token<'d>> {
        let typ = self.data.read_u32()?;
        match typ {
            1 => self.read_begin_node(),
            2 => Ok(Token::EndNode),
            3 => self.read_prop(),
            4 => Ok(Token::Nop),
            9 => Ok(Token::End),
            t => Err(Error::InvalidToken(t))
        }
    }

    fn read_begin_node(&mut self) -> Result<Token<'d>> {
        let name = self.data.read_cstr()?;
        self.data.align_for::<u32>();

        Ok(Token::BeginNode { name })
    }

    fn read_prop(&mut self) -> Result<Token<'d>> {
        let len = self.data.read_u32()?;
        let nameoff = self.data.read_u32()?;
        let value = self.data.read_n(len as usize)?;
        self.data.align_for::<u32>();

        Ok(Token::Prop { nameoff, value })
    }
}

impl<'d> Iterator for TokenIter<'d> {
    type Item = Result<Token<'d>>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }

        let token = match self.read_token() {
            Ok(token) => token,
            Err(error) => return Some(Err(error)),
        };

        if token == Token::End {
            self.done = true;
        }

        Some(Ok(token))
    }
}
