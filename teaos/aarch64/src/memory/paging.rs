use core::marker::PhantomData;
use core::mem;

use crate::instruction::{dsb_ish, dsb_ishst, isb, tlbi_vae1is, tlbi_vmalle1is};
use crate::memory::{PA, VA};
use crate::register::{MAIR_EL1, TCR_EL1, TTBR1_EL1};

pub const PAGE_SIZE: usize = 4 << 10;

/// Data structure for manipulating page tables.
pub struct PageMap<Alloc, Mapper> {
    root: PA,
    mair_idx: MairIndexes,
    _frame_alloc: PhantomData<Alloc>,
    _addr_mapper: PhantomData<Mapper>,
}

impl<Alloc, Mapper> PageMap<Alloc, Mapper>
where
    Alloc: FrameAlloc,
    Mapper: AddrMapper,
{
    pub fn new() -> Self {
        let root = Alloc::alloc_frame();
        Self::with_root(root)
    }

    pub fn with_root(root: PA) -> Self {
        assert!(root.is_aligned_to(PAGE_SIZE), "unaligned: {root:#}");

        Self {
            root,
            mair_idx: MairIndexes::read(),
            _frame_alloc: PhantomData,
            _addr_mapper: PhantomData,
        }
    }

    unsafe fn table_at(&self, pa: PA) -> &Table {
        assert!(pa.is_aligned_to(PAGE_SIZE), "unaligned: {pa:#}");

        let va = Mapper::pa_to_va(pa);
        unsafe { &*va.as_ptr() }
    }

    unsafe fn table_at_mut(&mut self, pa: PA) -> &mut Table {
        assert!(pa.is_aligned_to(PAGE_SIZE), "unaligned: {pa:#}");

        let va = Mapper::pa_to_va(pa);
        unsafe { &mut *va.as_mut_ptr() }
    }

    pub fn map_page(&mut self, va: VA, pa: PA, class: MemoryClass) {
        assert!(va.is_aligned_to(PAGE_SIZE), "unaligned: {va:#}");
        assert!(pa.is_aligned_to(PAGE_SIZE), "unaligned: {pa:#}");

        let (attr_idx, share) = match class {
            MemoryClass::Normal => (self.mair_idx.normal, Shareability::Inner),
            MemoryClass::Device => (self.mair_idx.device, Shareability::Outer),
        };

        let mut desc = Descriptor::new_page(pa);
        desc.set_attr_idx(attr_idx);
        desc.set_shareability(share);

        self.insert(va, desc);
    }

    pub fn map_region(&mut self, mut va: VA, mut pa: PA, size: usize, class: MemoryClass) {
        let end = va + size;
        while va < end {
            self.map_page(va, pa, class);

            va += PAGE_SIZE;
            pa += PAGE_SIZE;
        }
    }

    fn insert(&mut self, va: VA, desc: Descriptor) {
        assert!(va.is_aligned_to(PAGE_SIZE), "unaligned: {va:#}");

        let slot = self.lookup(va);

        // We require the existing descriptor to be invalid. Updating a valid entry requires a
        // "break-before-make" sequence, so the caller should unmap first.
        assert_eq!(slot.type_(3), DescriptorType::Invalid);

        *slot = desc;
    }

    fn lookup(&mut self, va: VA) -> &mut Descriptor {
        assert!(va.is_aligned_to(PAGE_SIZE), "unaligned: {va:#}");

        use DescriptorType::*;

        let table_index = |va: VA, lvl: usize| {
            let shift = 39 - 9 * lvl;
            (usize::from(va) >> shift) & 0x1ff
        };

        let mut table_base = self.root;
        for level in 0..=2 {
            let table = unsafe { self.table_at(table_base) };
            let idx = table_index(va, level);
            let mut table_desc = table[idx];

            match table_desc.type_(level) {
                Invalid => {
                    let pa = Alloc::alloc_frame();
                    table_desc = Descriptor::new_table(pa);
                    let table = unsafe { self.table_at_mut(table_base) };
                    table[idx] = table_desc;
                }
                Table => {}
                typ => panic!("unexpected {typ:?} descriptor on level {level} (va={va:?})"),
            }

            table_base = table_desc.address();
        }

        let table = unsafe { self.table_at_mut(table_base) };
        let idx = table_index(va, 3);
        &mut table[idx]
    }

    fn walk(&self, mut f: impl FnMut(VA, Descriptor, usize)) {
        self.walk_inner(self.root, VA::new(0), 0, &mut f);
    }

    fn walk_inner(
        &self,
        table_base: PA,
        mut va: VA,
        level: usize,
        f: &mut impl FnMut(VA, Descriptor, usize),
    ) {
        use DescriptorType::*;

        let va_step: usize = 1 << (39 - 9 * level);

        let table = unsafe { self.table_at(table_base) };
        for desc in table {
            match desc.type_(level) {
                Table => {
                    f(va, *desc, level);

                    let table_pa = desc.address();
                    self.walk_inner(table_pa, va, level + 1, f);
                }
                Page | Block => f(va, *desc, level),
                Invalid => {}
            }

            va += va_step;
        }
    }

    pub fn clone_from(&mut self, other: &Self) {
        other.walk(|va, desc, level| {
            use DescriptorType::*;
            match desc.type_(level) {
                Page => self.insert(va, desc),
                Block => unimplemented!(),
                Invalid | Table => {}
            }
        });
    }
}

/// Trait for page frame allocators.
pub trait FrameAlloc {
    /// Allocate a new page frame.
    ///
    /// The allocated frame must be zeroed and its address must be page-aligned.
    fn alloc_frame() -> PA;
}

/// Trait for PA-to-VA address mappers.
pub trait AddrMapper {
    /// Map the given PA to a VA that can be used to access that physical memory location.
    fn pa_to_va(pa: PA) -> VA;
}

#[derive(Clone, Copy, Debug)]
pub enum MemoryClass {
    Normal,
    Device,
}

const TABLE_LEN: usize = PAGE_SIZE / mem::size_of::<Descriptor>();
type Table = [Descriptor; TABLE_LEN];

#[derive(Clone, Copy, Debug)]
#[repr(transparent)]
struct Descriptor(u64);

impl Descriptor {
    fn new_table(pa: PA) -> Self {
        assert!(pa.is_aligned_to(PAGE_SIZE));
        assert!(pa.into_u64() < (1 << 48));

        Self(pa.into_u64() | 0b11)
    }

    fn new_page(pa: PA) -> Self {
        assert!(pa.is_aligned_to(PAGE_SIZE));
        assert!(pa.into_u64() < (1 << 48));

        let mut desc = Self(pa.into_u64() | 0b11);

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

struct MairIndexes {
    device: u8,
    normal: u8,
}

impl MairIndexes {
    fn read() -> Self {
        let mut device = None;
        let mut normal = None;

        let mut check = |idx, attr| {
            if attr == 0x00 {
                device = Some(idx);
            } else if attr == 0xff {
                normal = Some(idx);
            }
        };

        let mair = MAIR_EL1::read();
        check(0, mair.ATTR0());
        check(1, mair.ATTR1());
        check(2, mair.ATTR2());
        check(3, mair.ATTR3());
        check(4, mair.ATTR4());
        check(5, mair.ATTR5());
        check(6, mair.ATTR6());
        check(7, mair.ATTR7());

        Self {
            device: device.expect("missing device attr"),
            normal: normal.expect("missing normal attr"),
        }
    }
}

enum Shareability {
    Inner = 0b11,
    Outer = 0b10,
}

/// Load a page map into TTBR1.
///
/// # Safety
///
/// The caller must ensure no concurrent writers to the relevant system registers exist, and all
/// existing mappings still required by existing threads are also present in the new mappings.
pub unsafe fn load_ttbr1<A, M>(map: &PageMap<A, M>) {
    let mut tcr = TCR_EL1::read();
    tcr.set_T1SZ(16);
    tcr.set_EPD1(0);
    tcr.set_IRGN1(0b01); // (normal memory, WBWA cacheable)
    tcr.set_ORGN1(0b01); // (normal memory, WBWA cacheable)
    tcr.set_SH1(0b11); // (inner shareable)
    tcr.set_TG1(0b10); // (4 KiB)

    // Make previous translation table writes visible.
    dsb_ishst();

    unsafe {
        TTBR1_EL1::write(map.root);
        TCR_EL1::write(tcr);
    }

    // Make sure all subsequent instructions use the new translation tables.
    isb();

    // Invalidate all EL1 TLB entries.
    tlbi_vmalle1is();

    // Wait for TLBI to complete and refetch.
    dsb_ish();
    isb();
}

/// Disable address translation using TTBR0.
///
/// # Safety
///
/// The caller must ensure no concurrent writers to the relevant system registers exist, and no
/// TTBR0 mappings are still required by existing threads.
pub unsafe fn disable_ttbr0() {
    let mut tcr = TCR_EL1::read();
    tcr.set_EPD0(1);

    unsafe { TCR_EL1::write(tcr) };

    // Make sure all subsequent instructions observe the change.
    isb();

    // Invalidate all EL1 TLB entries.
    tlbi_vmalle1is();

    // Wait for TLBI to complete and refetch.
    dsb_ish();
    isb();
}

pub fn tlb_invalidate(mut va: VA, size: usize) {
    assert!(va.is_aligned_to(PAGE_SIZE), "unaligned: {va:#}");

    let end = va + size;

    // Make previous translation table writes visible.
    dsb_ishst();

    // Invalidate all pages in range.
    while va < end {
        tlbi_vae1is(va);
        va += PAGE_SIZE;
    }

    // Wait for TLBIs to complete and refetch.
    dsb_ish();
    isb();
}
