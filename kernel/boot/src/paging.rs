use core::mem;

use aarch64::memory::paging::{Flags, MairIndexes, Shareability, load_ttbr1};
use aarch64::memory::{PA, PAGE_MAP_LEVELS, PAGE_SIZE, VA};
use aarch64::register::TCR_EL1;

use crate::uefi;

/// Simple page mapper, for creating the kernel page tables.
pub struct KernelPager {
    root: *mut Table,
    mair_idx: MairIndexes,
}

impl KernelPager {
    pub fn new() -> Self {
        Self {
            root: alloc_page_table(),
            mair_idx: MairIndexes::read(),
        }
    }

    fn map_region(&mut self, start_va: VA, start_pa: PA, pages: usize, flags: Flags) {
        let flags = flags.unprivileged_execute_never(true).access_flag(true);

        let mut va = start_va;
        let mut pa = start_pa;
        for _ in 0..pages {
            let desc = Descriptor::new_page(pa, flags);
            self.insert(va, desc);

            va += PAGE_SIZE;
            pa += PAGE_SIZE;
        }
    }

    pub fn map_ram_region(&mut self, start_va: VA, start_pa: PA, pages: usize, flags: Flags) {
        let flags = flags
            .attr_idx(self.mair_idx.normal)
            .shareability(Shareability::Inner);
        self.map_region(start_va, start_pa, pages, flags);
    }

    pub fn map_mmio_region(&mut self, start_va: VA, start_pa: PA, pages: usize, flags: Flags) {
        let flags = flags
            .attr_idx(self.mair_idx.device)
            .shareability(Shareability::Outer);
        self.map_region(start_va, start_pa, pages, flags);
    }

    fn insert(&mut self, va: VA, desc: Descriptor) {
        // Traverse through intermediary levels, creating page tables as needed.
        let mut table = unsafe { &mut *self.root };
        for level in 0..PAGE_MAP_LEVELS {
            let idx = va.page_table_idx(level);
            if !table[idx].valid() {
                let table_ptr = alloc_page_table();
                let table_pa = PA::new(table_ptr as u64);
                table[idx] = Descriptor::new_table(table_pa);
            }

            table = unsafe { &mut *table[idx].next_table() };
        }

        let idx = va.page_table_idx(PAGE_MAP_LEVELS);
        table[idx] = desc;
    }

    pub fn apply(self) {
        let tcr = TCR_EL1::read();
        assert_eq!(tcr.EPD1(), 1);

        let ttb = PA::new(self.root as u64);
        unsafe { load_ttbr1(ttb) };
    }
}

const TABLE_LEN: usize = PAGE_SIZE / mem::size_of::<Descriptor>();
type Table = [Descriptor; TABLE_LEN];

#[derive(Clone, Copy, Debug)]
#[repr(transparent)]
struct Descriptor(u64);

impl Descriptor {
    fn new_table(base: PA) -> Self {
        Self(base.into_u64() | 0b11)
    }

    fn new_page(base: PA, flags: Flags) -> Self {
        Self(u64::from(base) | u64::from(flags) | 0b11)
    }

    fn valid(&self) -> bool {
        self.0 & 0b11 == 0b11
    }

    fn next_table(&self) -> *mut Table {
        let addr = self.0 & 0xfffffffff000;
        addr as *mut _
    }
}

fn alloc_page_table() -> *mut Table {
    // `allocate_page` already zeroes the returned memory.
    let buf = uefi::allocate_page(uefi::sys::LoaderCode);
    buf.as_mut_ptr().cast()
}
