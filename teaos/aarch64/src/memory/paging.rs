use core::marker::PhantomData;
use core::mem;

use crate::instruction::{dsb_ish, dsb_ishst, isb, tlbi_vae1is, tlbi_vmalle1is};
use crate::register::{MAIR_EL1, TCR_EL1, TTBR1_EL1};

use super::{AddrMapper, Frame, FrameAlloc, PA, Page, VA};

pub const PAGE_SIZE: usize = 4 << 10;

const LAST_LEVEL: u64 = 3;

/// Data structure for manipulating page tables.
pub struct PageMap<Alloc, Mapper> {
    root: Frame,
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
        let root = Self::alloc_page_table();
        Self::with_root(root)
    }

    pub fn with_root(root: Frame) -> Self {
        Self {
            root,
            mair_idx: MairIndexes::read(),
            _frame_alloc: PhantomData,
            _addr_mapper: PhantomData,
        }
    }

    fn alloc_page_table() -> Frame {
        let frame = Alloc::alloc_frame();

        let page = Mapper::frame_to_page(frame);
        unsafe { (*page.as_mut_ptr()).fill(0) };

        frame
    }

    unsafe fn view_table(&self, frame: Frame) -> &Table {
        let page = Mapper::frame_to_page(frame);
        unsafe { &*page.as_ptr().cast() }
    }

    unsafe fn view_table_mut(&mut self, frame: Frame) -> &mut Table {
        let page = Mapper::frame_to_page(frame);
        unsafe { &mut *page.as_mut_ptr().cast() }
    }

    pub fn map_page(&mut self, page: Page, frame: Frame, class: MemoryClass) {
        let (attr_idx, share) = match class {
            MemoryClass::Normal => (self.mair_idx.normal, Shareability::Inner),
            MemoryClass::Device => (self.mair_idx.device, Shareability::Outer),
        };

        let mut desc = Descriptor::new_page(frame);
        desc.set_attr_idx(attr_idx);
        desc.set_shareability(share);

        self.insert(page, desc);
    }

    pub fn map_region(
        &mut self,
        start_page: Page,
        start_frame: Frame,
        pages: usize,
        class: MemoryClass,
    ) {
        let mut page = start_page;
        let mut frame = start_frame;

        for _ in 0..pages {
            self.map_page(page, frame, class);
            page = page.next_page();
            frame = frame.next_frame();
        }
    }

    fn insert(&mut self, page: Page, desc: Descriptor) {
        let slot = self.lookup(page);

        // We require the existing descriptor to be invalid. Updating a valid entry requires a
        // "break-before-make" sequence, so the caller should unmap first.
        assert!(!slot.valid());

        *slot = desc;
    }

    fn lookup(&mut self, page: Page) -> &mut Descriptor {
        let table_index = |lvl: u64| {
            let va = page.base().into_u64() as usize;
            let shift = 39 - 9 * lvl;
            (va >> shift) & 0x1ff
        };

        let mut table_frame = self.root;
        for level in 0..LAST_LEVEL {
            let table = unsafe { self.view_table(table_frame) };
            let idx = table_index(level);
            let mut table_desc = table[idx];

            if !table_desc.valid() {
                let frame = Self::alloc_page_table();
                table_desc = Descriptor::new_table(frame);
                let table = unsafe { self.view_table_mut(table_frame) };
                table[idx] = table_desc;
            }

            table_frame = table_desc.output_frame();
        }

        let table = unsafe { self.view_table_mut(table_frame) };
        let idx = table_index(LAST_LEVEL);
        &mut table[idx]
    }

    fn walk(&self, mut f: impl FnMut(Page, Descriptor)) {
        self.walk_inner(self.root, Page::new(VA::new(0)), 0, &mut f);
    }

    fn walk_inner(
        &self,
        table_frame: Frame,
        mut page: Page,
        level: u64,
        f: &mut impl FnMut(Page, Descriptor),
    ) {
        let va_step: u64 = 1 << (39 - 9 * level);

        let table = unsafe { self.view_table(table_frame) };
        for desc in table {
            if desc.valid() {
                if level == LAST_LEVEL {
                    f(page, *desc)
                } else {
                    let frame = desc.output_frame();
                    self.walk_inner(frame, page, level + 1, f);
                }
            }

            page = Page::new(page.base() + va_step);
        }
    }

    pub fn clone_from(&mut self, other: &Self) {
        other.walk(|page, desc| {
            self.insert(page, desc);
        });
    }
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
    fn new_table(frame: Frame) -> Self {
        let addr = frame.base().into_u64();
        Self(addr | 0b11)
    }

    fn new_page(frame: Frame) -> Self {
        let addr = frame.base().into_u64();
        let mut desc = Self(addr | 0b11);

        // Prevent the generation of Access flag faults.
        desc.set_access_flag();

        desc
    }

    fn output_frame(&self) -> Frame {
        let addr = PA::new(self.0 & 0xfffffffff000);
        Frame::new(addr)
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

    fn valid(&self) -> bool {
        self.0 & 0b11 == 0b11
    }
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
        TTBR1_EL1::write(map.root.base());
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
    assert!(va.is_page_aligned(), "unaligned: {va:#}");

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
