use core::mem;

use aarch64::memory::paging::{MairIndexes, Shareability, load_ttbr1};
use aarch64::memory::{PA, PAGE_MAP_LEVELS, PAGE_SIZE, VA};
use aarch64::register::TCR_EL1;
use boot_info::MemoryType;

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

    pub fn map_region(&mut self, start_va: VA, start_pa: PA, pages: usize, type_: MemoryType) {
        let (attr_idx, share) = if type_ == MemoryType::Mmio {
            (self.mair_idx.device, Shareability::Outer)
        } else {
            (self.mair_idx.normal, Shareability::Inner)
        };

        let mut va = start_va;
        let mut pa = start_pa;
        for _ in 0..pages {
            let mut desc = Descriptor::new_page(pa);
            desc.set_attr_idx(attr_idx);
            desc.set_shareability(share);
            self.insert(va, desc);

            va += PAGE_SIZE;
            pa += PAGE_SIZE;
        }
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

    fn new_page(base: PA) -> Self {
        let mut desc = Self(base.into_u64() | 0b11);

        // Prevent the generation of Access flag faults.
        desc.set_access_flag();

        desc
    }

    fn valid(&self) -> bool {
        self.0 & 0b11 == 0b11
    }

    fn set_access_flag(&mut self) {
        self.0 |= 1 << 10;
    }

    fn set_attr_idx(&mut self, attr_idx: u8) {
        const MASK: u64 = 0b111;
        const SHIFT: u64 = 2;

        self.0 &= !(MASK << SHIFT);
        self.0 |= u64::from(attr_idx) << SHIFT;
    }

    fn set_shareability(&mut self, share: Shareability) {
        const MASK: u64 = 0b11;
        const SHIFT: u64 = 8;

        self.0 &= !(MASK << SHIFT);
        self.0 |= (share as u64) << SHIFT;
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
