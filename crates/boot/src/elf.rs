//! Simple ELF file parser.

use alloc::vec;
use core::mem;

use crate::{uefi, validate_ptr};

pub struct File {
    inner: uefi::protocol::File,
    header: Ehdr,
}

impl File {
    pub fn open(inner: uefi::protocol::File) -> Self {
        let mut buffer = vec![0; mem::size_of::<Ehdr>()];
        inner.set_position(0);
        inner.read(&mut buffer);
        let header = Ehdr::parse(&buffer);

        Self { inner, header }
    }

    pub fn iter_program_headers(&self) -> impl Iterator<Item = Phdr> + '_ {
        const PHDR_SIZE: usize = mem::size_of::<Phdr>();

        let mut buffer = vec![0; PHDR_SIZE];
        (0..self.header.phnum).into_iter().map(move |i| {
            let offset = self.header.phoff + PHDR_SIZE * usize::from(i);
            self.inner.set_position(offset as u64);
            self.inner.read(&mut buffer);
            Phdr::parse(&buffer)
        })
    }

    pub fn read_segment(&self, phdr: &Phdr, buffer: &mut [u8]) {
        assert!(buffer.len() >= phdr.filesz as usize);

        self.inner.set_position(0);
        self.inner.read(buffer);
    }
}

#[derive(Clone, Debug)]
#[repr(C)]
pub struct Ehdr {
    ident: [u8; 16],
    type_: u16,
    machine: u16,
    version: u32,
    entry: usize,
    phoff: usize,
    shoff: usize,
    flags: u32,
    ehsize: u16,
    phentsize: u16,
    phnum: u16,
    shentsize: u16,
    shnum: u16,
    shstrndx: u16,
}

impl Ehdr {
    fn parse(data: &[u8]) -> Self {
        assert_eq!(data.len(), mem::size_of::<Ehdr>());

        let ptr: *const Ehdr = data.as_ptr().cast();
        validate_ptr(ptr);

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
    offset: usize,
    vaddr: usize,
    paddr: usize,
    filesz: u64,
    memsz: u64,
    align: u64,
}

impl Phdr {
    fn parse(data: &[u8]) -> Self {
        assert_eq!(data.len(), mem::size_of::<Phdr>());

        let ptr: *const Phdr = data.as_ptr().cast();
        validate_ptr(ptr);

        unsafe { (*ptr).clone() }
    }

    pub fn is_load(&self) -> bool {
        self.type_ == PT_LOAD
    }

    pub fn virtual_address(&self) -> usize {
        self.vaddr
    }

    pub fn memory_size(&self) -> usize {
        self.memsz as usize
    }
}

const PT_LOAD: u32 = 1;
