use alloc::vec;
use alloc::vec::Vec;
use core::arch::asm;

use aarch64::memory::paging::{AccessPermissions, Flags, load_ttbr0};
use aarch64::memory::{PAGE_SIZE, VA};
use elf::ElfFile;

use crate::memory::phys;
use crate::memory::virt::{PageMap, PageNr};
use crate::userimg;

struct Process {
    pc: u64,
    page_map: PageMap,
}

impl Process {
    fn new() -> Self {
        Self {
            pc: 0,
            page_map: PageMap::new(),
        }
    }
}

pub fn run() -> ! {
    let proc = load_usermode();

    unsafe {
        load_ttbr0(proc.page_map.base(), 0);

        asm!(
            r#"
            msr spsr_el1, {spsr:x}
            msr elr_el1, {pc}
            eret
            "#,
            spsr = in(reg) 0,
            pc = in(reg) proc.pc,
        );
    }

    unreachable!();
}

fn load_usermode() -> Process {
    let mut proc = Process::new();

    let reader = userimg::Reader::new();
    let mut elf = ElfFile::open(reader);

    proc.pc = elf.entry();

    let phdrs: Vec<_> = elf.program_headers().collect();
    for phdr in phdrs {
        if !phdr.is_load() {
            continue;
        }

        let mut data = vec![0; phdr.memory_size() as usize];
        elf.read_segment(&phdr, &mut data);

        let ap = if phdr.is_writable() {
            AccessPermissions::UnprivRW
        } else {
            AccessPermissions::UnprivRO
        };
        let xn = !phdr.is_executable();
        let flags = Flags::default()
            .access_permissions(ap)
            .privileged_execute_never(true)
            .unprivileged_execute_never(xn);

        let va = VA::new(phdr.virtual_address());
        let mut vpn = PageNr::from_va(va);

        let mut pages = data.chunks_exact(PAGE_SIZE);
        for page in &mut pages {
            let mut frame = phys::alloc();
            frame.with_contents(|buf| buf.copy_from_slice(page));
            proc.page_map.map_ram_page(vpn, frame, flags);
            vpn += 1;
        }

        let rest = pages.remainder();
        let len = rest.len();
        let mut frame = phys::alloc_zero();
        frame.with_contents(|buf| buf[..len].copy_from_slice(rest));
        proc.page_map.map_ram_page(vpn, frame, flags);
    }

    proc
}
