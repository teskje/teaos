//! ELF file parser.

#![no_std]

extern crate alloc;

use alloc::vec;
use alloc::vec::Vec;
use core::mem;

use kstd::io::{Read, Seek};

pub struct ElfFile<R> {
    reader: R,
    header: Ehdr,
}

impl<R: Read + Seek> ElfFile<R> {
    pub fn open(mut reader: R) -> Self {
        let mut buffer = vec![0; mem::size_of::<Ehdr>()];
        reader.seek(0).unwrap();
        reader.read_exact(&mut buffer).unwrap();
        let header = Ehdr::parse(&buffer);

        Self { reader, header }
    }

    pub fn entry(&self) -> usize {
        self.header.entry as usize
    }

    pub fn program_headers(&mut self) -> Vec<Phdr> {
        self.reader.seek(self.header.phoff).unwrap();

        let mut buffer = vec![0; mem::size_of::<Phdr>()];
        (0..self.header.phnum)
            .map(move |_| {
                self.reader.read_exact(&mut buffer).unwrap();
                Phdr::parse(&buffer)
            })
            .collect()
    }

    pub fn read_segment(&mut self, phdr: &Phdr, buffer: &mut [u8]) {
        let buffer = &mut buffer[..phdr.filesz as usize];

        self.reader.seek(phdr.offset).unwrap();
        self.reader.read_exact(buffer).unwrap();
    }
}

#[derive(Clone, Debug)]
#[repr(C)]
pub struct Ehdr {
    ident: [u8; 16],
    type_: u16,
    machine: u16,
    version: u32,
    entry: u64,
    phoff: u64,
    shoff: u64,
    flags: u32,
    ehsize: u16,
    phentsize: u16,
    phnum: u16,
    shentsize: u16,
    shnum: u16,
    shstrndx: u16,
}

impl Ehdr {
    /// Parse the given raw data as a [`Phdr`].
    ///
    /// # Panics
    ///
    /// Panics if `data` has the wrong size of alignment.
    /// Panics if any of the header fields have unexpected values.
    fn parse(data: &[u8]) -> Self {
        assert_eq!(data.len(), mem::size_of::<Ehdr>());

        let ptr: *const Ehdr = data.as_ptr().cast();
        assert!(ptr.is_aligned());

        let header = unsafe { (*ptr).clone() };
        assert_eq!(&header.ident[..4], b"\x7fELF");
        assert_eq!(header.ident[4], ELFCLASS64);
        assert_eq!(header.type_, ET_EXEC);
        assert_eq!(header.machine, EM_AARCH64);
        assert_eq!(usize::from(header.ehsize), mem::size_of::<Ehdr>());
        assert_eq!(usize::from(header.phentsize), mem::size_of::<Phdr>());

        header
    }
}

const ELFCLASS64: u8 = 2;
const ET_EXEC: u16 = 2;
const EM_AARCH64: u16 = 183;

#[derive(Clone, Debug)]
#[repr(C)]
pub struct Phdr {
    type_: u32,
    flags: u32,
    offset: u64,
    vaddr: u64,
    paddr: u64,
    filesz: u64,
    memsz: u64,
    align: u64,
}

impl Phdr {
    /// Parse the given raw data as a [`Phdr`].
    ///
    /// # Panics
    ///
    /// Panics if `data` has the wrong size of alignment.
    fn parse(data: &[u8]) -> Self {
        assert_eq!(data.len(), mem::size_of::<Phdr>());

        let ptr: *const Phdr = data.as_ptr().cast();
        assert!(ptr.is_aligned());

        unsafe { (*ptr).clone() }
    }

    pub fn is_load(&self) -> bool {
        self.type_ == PT_LOAD
    }

    pub fn virtual_address(&self) -> u64 {
        self.vaddr
    }

    pub fn memory_size(&self) -> u64 {
        self.memsz
    }
}

const PT_LOAD: u32 = 1;
