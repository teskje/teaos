use alloc::vec;
use alloc::vec::Vec;
use core::arch::asm;
use kstd::io;

use aarch64::memory::paging::{AccessPermissions, Flags, load_ttbr0};
use aarch64::memory::{PAGE_SIZE, VA};
use elf::ElfFile;

use crate::memory::phys;
use crate::memory::virt::{PageMap, PageNr};
use crate::userimg;

const STACK_TOP: VA = VA::new(0x0001_0000_0000_0000);
const STACK_SIZE: usize = 16 << 10;

const HEAP_START: VA = VA::new(0x0000_1000_0000_0000);
const HEAP_SIZE: usize = 10 << 20;

struct Process {
    page_map: PageMap,
}

impl Process {
    fn new() -> Self {
        Self {
            page_map: PageMap::new(),
        }
    }
}

pub fn run() -> ! {
    let mut proc = Process::new();

    let userimg = userimg::Reader::new();
    let mut elf = ElfFile::open(userimg);

    load_address_space(&mut proc.page_map, &mut elf);
    alloc_stack(&mut proc.page_map);
    alloc_heap(&mut proc.page_map);

    unsafe {
        load_ttbr0(proc.page_map.base(), 1);

        asm!(
            r#"
            msr spsr_el1, {spsr:x}
            msr elr_el1, {entry}
            msr sp_el0, {sp}
            eret
            "#,
            spsr = in(reg) 0,
            entry = in(reg) elf.entry(),
            sp = in(reg) STACK_TOP.into_u64(),
            in("x0") HEAP_START.into_u64(),
            in("x1") HEAP_SIZE,
        );
    }

    unreachable!();
}

fn load_address_space<R>(page_map: &mut PageMap, elf: &mut ElfFile<R>)
where
    R: io::Read + io::Seek,
{
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
            page_map.map_ram_page(vpn, frame, flags);
            vpn += 1;
        }

        let rest = pages.remainder();
        let len = rest.len();
        let mut frame = phys::alloc_zero();
        frame.with_contents(|buf| buf[..len].copy_from_slice(rest));
        page_map.map_ram_page(vpn, frame, flags);
    }
}

fn alloc_stack(page_map: &mut PageMap) {
    let pages = STACK_SIZE / PAGE_SIZE;

    let flags = Flags::default()
        .access_permissions(AccessPermissions::UnprivRW)
        .privileged_execute_never(true)
        .unprivileged_execute_never(true);

    let mut vpn = PageNr::from_va(STACK_TOP);
    for _ in 0..pages {
        vpn -= 1;
        let frame = phys::alloc_zero();
        page_map.map_ram_page(vpn, frame, flags);
    }
}

fn alloc_heap(page_map: &mut PageMap) {
    let pages = HEAP_SIZE / PAGE_SIZE;

    let flags = Flags::default()
        .access_permissions(AccessPermissions::UnprivRW)
        .privileged_execute_never(true)
        .unprivileged_execute_never(true);

    let mut vpn = PageNr::from_va(HEAP_START);
    for _ in 0..pages {
        let frame = phys::alloc_zero();
        page_map.map_ram_page(vpn, frame, flags);
        vpn += 1;
    }
}
