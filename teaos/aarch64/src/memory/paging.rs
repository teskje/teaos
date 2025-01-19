use core::mem;

use crate::instruction::isb;
use crate::memory::{PA, VA};
use crate::register::{TCR_EL1, TTBR1_EL1};

pub const PAGE_SIZE: usize = 4 * (1 << 10);

const TABLE_LEN: usize = PAGE_SIZE / mem::size_of::<Descriptor>();

pub struct TranslationTable {
    base: PA,
    phys_offset: VA,
}

impl TranslationTable {
    pub fn new(phys_offset: VA, alloc_frame: impl Fn() -> PA) -> Self {
        let ttb = alloc_frame();
        Self::with_base(ttb, phys_offset)
    }

    pub fn with_base(base: PA, phys_offset: VA) -> Self {
        Self { base, phys_offset }
    }

    pub fn map_page(&mut self, va: VA, pa: PA, alloc_frame: impl Fn() -> PA) {
        assert_eq!(u64::from(va) >> 48, 0xffff, "only high memory supported");
        assert!(
            va.is_aligned_to(PAGE_SIZE),
            "va {va:#} not aligned to page size"
        );
        assert!(
            pa.is_aligned_to(PAGE_SIZE),
            "pa {pa:#} not aligned to page size"
        );

        let desc = self.get_descriptor(va, 3, alloc_frame);
        *desc = Descriptor::new_page(pa);
    }

    pub fn map_region(
        &mut self,
        mut va: VA,
        mut pa: PA,
        mut size: usize,
        alloc_frame: impl Fn() -> PA,
    ) {
        while size > 0 {
            self.map_page(va, pa, &alloc_frame);
            va += PAGE_SIZE;
            pa += PAGE_SIZE;
            size = size.saturating_sub(PAGE_SIZE);
        }
    }

    unsafe fn table_at(&self, pa: PA) -> &Table {
        let va = self.phys_offset + u64::from(pa);
        unsafe { &*va.as_ptr() }
    }

    unsafe fn table_at_mut(&mut self, pa: PA) -> &mut Table {
        let va = self.phys_offset + u64::from(pa);
        unsafe { &mut *va.as_mut_ptr() }
    }

    fn get_descriptor(
        &mut self,
        va: VA,
        level: usize,
        alloc_frame: impl Fn() -> PA,
    ) -> &mut Descriptor {
        use DescriptorType::*;

        let table_index = |va: VA, lvl: usize| {
            let shift = 39 - 9 * lvl;
            (usize::from(va) >> shift) & 0x1ff
        };

        let mut table_pa = self.base;
        for lvl in 0..level {
            let table = unsafe { self.table_at_mut(table_pa) };
            let idx = table_index(va, lvl);
            let desc = &mut table[idx];

            match desc.type_(lvl) {
                Invalid => {
                    let pa = alloc_frame();
                    *desc = Descriptor::new_table(pa);
                }
                Table => {}
                typ => panic!("table walk interrupted by {typ:?} (va={va:?}, lvl={lvl})"),
            }

            table_pa = desc.address();
        }

        let table = unsafe { self.table_at_mut(table_pa) };
        let idx = table_index(va, level);
        &mut table[idx]
    }

    fn walk(&self, mut cb: impl FnMut(VA, Descriptor, usize)) {
        let start_va = VA::new(0xffff000000000000);
        self.walk_inner(self.base, start_va, 0, &mut cb);
    }

    fn walk_inner(
        &self,
        table_pa: PA,
        start_va: VA,
        level: usize,
        cb: &mut impl FnMut(VA, Descriptor, usize),
    ) {
        use DescriptorType::*;

        let va_step = 1 << (39 - 9 * level);

        let table = unsafe { self.table_at(table_pa) };
        for (idx, desc) in table.iter().enumerate() {
            let va = start_va + idx * va_step;

            match desc.type_(level) {
                Table => {
                    cb(va, *desc, level);

                    let table_pa = desc.address();
                    self.walk_inner(table_pa, va, level + 1, cb);
                }
                Page | Block => cb(va, *desc, level),
                Invalid => {}
            }
        }
    }

    pub fn clone_from(&mut self, other: &TranslationTable, alloc_frame: impl Fn() -> PA) {
        other.walk(|va, desc, level| {
            use DescriptorType::*;
            match desc.type_(level) {
                Page => {
                    let pa = desc.address();
                    self.map_page(va, pa, &alloc_frame);
                }
                Block => unimplemented!(),
                Invalid | Table => {}
            }
        });
    }

    /// Load this translation table.
    pub fn load(&self) {
        let mut tcr = TCR_EL1::read();
        tcr.set_T1SZ(16);
        tcr.set_EPD1(0);
        tcr.set_IRGN1(0b00); // (normal memory, inner non-cacheable)
        tcr.set_ORGN1(0b00); // (normal memory, inner non-cacheable)
        tcr.set_SH1(0b00); // (non-shareable)
        tcr.set_TG1(0b10); // (4 KiB)

        unsafe {
            TTBR1_EL1::write(self.base);
            TCR_EL1::write(tcr);
        }
        isb();
    }
}

type Table = [Descriptor; TABLE_LEN];

#[derive(Clone, Copy, Debug)]
#[repr(transparent)]
struct Descriptor(u64);

impl Descriptor {
    fn new_table(pa: PA) -> Self {
        assert!(pa.is_aligned_to(PAGE_SIZE));
        Self(u64::from(pa) | 0b11)
    }

    fn new_page(pa: PA) -> Self {
        assert!(pa.is_aligned_to(PAGE_SIZE));
        let mut desc = Self(u64::from(pa) | 0b11);
        // Prevent the generation of Access flag faults.
        desc.set_access_flag();
        desc
    }

    fn address(&self) -> PA {
        PA::new(self.0 & 0xfffffffff000)
    }

    fn set_access_flag(&mut self) {
        self.0 |= 1 << 10;
    }

    fn type_(&self, level: usize) -> DescriptorType {
        match (self.0 & 0b11, level == 3) {
            (0b10, false) => DescriptorType::Block,
            (0b11, false) => DescriptorType::Table,
            (0b11, true) => DescriptorType::Page,
            _ => DescriptorType::Invalid,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DescriptorType {
    Invalid,
    Table,
    Page,
    Block,
}
