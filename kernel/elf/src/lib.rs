//! ELF file parser.

#![no_std]

extern crate alloc;

use alloc::vec;
use alloc::vec::Vec;
use core::ffi::CStr;
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

    pub fn entry(&self) -> u64 {
        self.header.entry
    }

    pub fn program_headers(&mut self) -> impl Iterator<Item = Phdr> + '_ {
        self.reader.seek(self.header.phoff).unwrap();

        let mut buffer = vec![0; mem::size_of::<Phdr>()];
        (0..self.header.phnum).map(move |_| {
            self.reader.read_exact(&mut buffer).unwrap();
            Phdr::parse(&buffer)
        })
    }

    pub fn section_headers(&mut self) -> impl Iterator<Item = Shdr> + '_ {
        self.reader.seek(self.header.shoff).unwrap();

        let mut buffer = vec![0; mem::size_of::<Shdr>()];
        (0..self.header.shnum).map(move |_| {
            self.reader.read_exact(&mut buffer).unwrap();
            Shdr::parse(&buffer)
        })
    }

    pub fn read_segment(&mut self, phdr: &Phdr, buffer: &mut [u8]) {
        let buffer = &mut buffer[..phdr.filesz as usize];

        self.reader.seek(phdr.offset).unwrap();
        self.reader.read_exact(buffer).unwrap();
    }

    pub fn read_section(&mut self, shdr: &Shdr, buffer: &mut [u8]) {
        let buffer = &mut buffer[..shdr.size as usize];

        self.reader.seek(shdr.offset).unwrap();
        self.reader.read_exact(buffer).unwrap();
    }

    pub fn sh_symtab(&mut self) -> Option<Shdr> {
        let sh = self.section_headers().find(|sh| sh.is_symtab())?;
        assert_eq!(sh.entsize as usize, mem::size_of::<Sym>());
        Some(sh)
    }

    pub fn symbols(&mut self) -> Option<impl Iterator<Item = Sym> + '_> {
        let sh_symtab = self.sh_symtab()?;
        let num_symbols = sh_symtab.size / sh_symtab.entsize;

        self.reader.seek(sh_symtab.offset).unwrap();

        let mut buffer = vec![0; mem::size_of::<Sym>()];
        let iter = (0..num_symbols).map(move |_| {
            self.reader.read_exact(&mut buffer).unwrap();
            Sym::parse(&buffer)
        });

        Some(iter)
    }

    pub fn symbol_strtab(&mut self) -> Option<Vec<u8>> {
        let sh_symtab = self.sh_symtab()?;
        let strtab_idx = sh_symtab.link as usize;
        let sh_strtab = self.section_headers().nth(strtab_idx)?;
        assert_eq!(sh_strtab.type_, SHT_STRTAB);

        let mut strtab = vec![0; sh_strtab.size as usize];
        self.read_section(&sh_strtab, &mut strtab);

        Some(strtab)
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
    /// Parse the given raw data as a [`Ehdr`].
    ///
    /// # Panics
    ///
    /// Panics if `data` has the wrong size or alignment.
    /// Panics if any of the header fields have unexpected values.
    fn parse(data: &[u8]) -> Self {
        assert_eq!(data.len(), mem::size_of::<Self>());

        let ptr: *const Self = data.as_ptr().cast();
        assert!(ptr.is_aligned());

        let header = unsafe { (*ptr).clone() };
        assert_eq!(&header.ident[..4], b"\x7fELF");
        assert_eq!(header.ident[4], ELFCLASS64);
        assert_eq!(header.type_, ET_EXEC);
        assert_eq!(header.machine, EM_AARCH64);
        assert_eq!(usize::from(header.ehsize), mem::size_of::<Ehdr>());
        assert_eq!(usize::from(header.phentsize), mem::size_of::<Phdr>());
        assert_eq!(usize::from(header.shentsize), mem::size_of::<Shdr>());

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
    /// Panics if `data` has the wrong size or alignment.
    fn parse(data: &[u8]) -> Self {
        assert_eq!(data.len(), mem::size_of::<Self>());

        let ptr: *const Self = data.as_ptr().cast();
        assert!(ptr.is_aligned());

        unsafe { (*ptr).clone() }
    }

    pub fn is_load(&self) -> bool {
        self.type_ == PT_LOAD
    }

    pub fn is_executable(&self) -> bool {
        self.flags & PF_X != 0
    }

    pub fn is_writable(&self) -> bool {
        self.flags & PF_W != 0
    }

    pub fn virtual_address(&self) -> u64 {
        self.vaddr
    }

    pub fn memory_size(&self) -> u64 {
        self.memsz
    }
}

const PT_LOAD: u32 = 1;

const PF_X: u32 = 0b01;
const PF_W: u32 = 0b10;

#[derive(Clone, Debug)]
#[repr(C)]
pub struct Shdr {
    name: u32,
    type_: u32,
    flags: u64,
    addr: u64,
    offset: u64,
    size: u64,
    link: u32,
    info: u32,
    addralign: u64,
    entsize: u64,
}

impl Shdr {
    /// Parse the given raw data as a [`Shdr`].
    ///
    /// # Panics
    ///
    /// Panics if `data` has the wrong size or alignment.
    fn parse(data: &[u8]) -> Self {
        assert_eq!(data.len(), mem::size_of::<Self>());

        let ptr: *const Self = data.as_ptr().cast();
        assert!(ptr.is_aligned());

        unsafe { (*ptr).clone() }
    }

    pub fn is_symtab(&self) -> bool {
        self.type_ == SHT_SYMTAB
    }

    pub fn is_strtab(&self) -> bool {
        self.type_ == SHT_STRTAB
    }
}

const SHT_SYMTAB: u32 = 2;
const SHT_STRTAB: u32 = 3;

#[derive(Clone, Debug)]
#[repr(C)]
pub struct Sym {
    name: u32,
    info: u8,
    other: u8,
    shndx: u16,
    value: u64,
    size: u64,
}

impl Sym {
    /// Parse the given raw data as a [`Sym`].
    ///
    /// # Panics
    ///
    /// Panics if `data` has the wrong size or alignment.
    fn parse(data: &[u8]) -> Self {
        assert_eq!(data.len(), mem::size_of::<Self>());

        let ptr: *const Self = data.as_ptr().cast();
        assert!(ptr.is_aligned());

        unsafe { (*ptr).clone() }
    }

    /// Extract the symbol's name from the given `strtab`.
    ///
    /// # Panics
    ///
    /// Panics if the symbol's name is not contained in the given `strtab`.
    pub fn name<'a>(&self, strtab: &'a [u8]) -> &'a CStr {
        let idx = self.name as usize;
        CStr::from_bytes_until_nul(&strtab[idx..]).unwrap()
    }

    pub fn value(&self) -> u64 {
        self.value
    }
}
