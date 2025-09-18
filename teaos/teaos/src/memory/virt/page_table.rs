//! Data structures for manipulating page tables.

use core::marker::PhantomData;
use core::mem::ManuallyDrop;
use core::ops::{Deref, DerefMut};

use aarch64::memory::paging::Shareability;
use aarch64::memory::{PA, PAGE_SIZE};

use crate::memory::phys::FrameNr;
use crate::memory::virt::PageNr;
use crate::memory::{pa_to_va, phys};

/// A page table at a given level `L`.
///
/// A `PageTable` owns its backing page frame as well as its children.
pub(super) struct PageTable<const L: u64> {
    base: PA,
}

impl<const L: u64> PageTable<L> {
    const LEN: usize = 512;

    pub fn new() -> Self {
        let frame = phys::alloc_zero();
        frame.inc_map();

        Self { base: frame.pa() }
    }

    pub fn base(&self) -> PA {
        self.base
    }

    fn desc(&self) -> TableDesc {
        TableDesc::new(self.base)
    }
}

macro_rules! pt_table_level {
    ($level:literal -> $next:literal) => {
        impl PageTable<$level> {
            fn get_desc<I>(&self, idx: I) -> Option<TableDesc>
            where
                I: PageTableIndex<$level>,
            {
                let ptr = pa_to_va(self.base).as_mut_ptr::<TableDesc>();
                let desc = unsafe { ptr.add(idx.index()).read() };
                desc.valid().then_some(desc)
            }

            pub fn get<I>(&self, idx: I) -> Option<PageTableRef<'_, $next>>
            where
                I: PageTableIndex<$level>,
            {
                let desc = self.get_desc(idx)?;
                unsafe { Some(PageTableRef::new(desc.output_addr())) }
            }

            pub fn get_mut<I>(&mut self, idx: I) -> Option<PageTableMut<'_, $next>>
            where
                I: PageTableIndex<$level>,
            {
                let desc = self.get_desc(idx)?;
                unsafe { Some(PageTableMut::new(desc.output_addr())) }
            }

            pub fn set<I>(&mut self, idx: I, pt: PageTable<$next>)
            where
                I: PageTableIndex<$level>,
            {
                let pt = ManuallyDrop::new(pt);
                let ptr = pa_to_va(self.base).as_mut_ptr::<TableDesc>();
                unsafe { ptr.add(idx.index()).write(pt.desc()) };
            }

            pub fn get_or_insert<I>(&mut self, idx: I) -> PageTableMut<'_, $next>
            where
                I: PageTableIndex<$level>,
            {
                if self.get(idx).is_none() {
                    self.set(idx, PageTable::new());
                }
                self.get_mut(idx).unwrap()
            }

            pub fn walk(&self, vpn: PageNr, mut f: impl FnMut(PageNr, PageDesc)) {
                let mut va = vpn.va();
                let va_step: u64 = 1 << (39 - 9 * $level);
                for idx in 0..Self::LEN {
                    if let Some(child) = self.get(idx) {
                        child.walk(PageNr::from_va(va), &mut f);
                    }
                    va += va_step;
                }
            }
        }
    };
}

pt_table_level!(0 -> 1);
pt_table_level!(1 -> 2);
pt_table_level!(2 -> 3);

impl PageTable<3> {
    pub fn get<I>(&self, idx: I) -> Option<PageDesc>
    where
        I: PageTableIndex<3>,
    {
        let ptr = pa_to_va(self.base).as_mut_ptr::<PageDesc>();
        let desc = unsafe { ptr.add(idx.index()).read() };
        desc.valid().then_some(desc)
    }

    pub fn set<I>(&mut self, idx: I, desc: PageDesc)
    where
        I: PageTableIndex<3>,
    {
        let ptr = pa_to_va(self.base).as_mut_ptr::<PageDesc>();
        unsafe { ptr.add(idx.index()).write(desc) }
    }

    pub fn walk(&self, vpn: PageNr, mut f: impl FnMut(PageNr, PageDesc)) {
        let mut va = vpn.va();
        for idx in 0..Self::LEN {
            if let Some(desc) = self.get(idx) {
                f(PageNr::from_va(va), desc);
            }
            va += PAGE_SIZE;
        }
    }
}

impl<const L: u64> Drop for PageTable<L> {
    fn drop(&mut self) {
        fn drop_children<const L: u64>(base: PA) {
            let ptr = pa_to_va(base).as_mut_ptr::<TableDesc>();
            for idx in 0..PageTable::<L>::LEN {
                let desc = unsafe { ptr.add(idx).read() };
                if desc.valid() {
                    let child = PageTable::<L> {
                        base: desc.output_addr(),
                    };
                    drop(child);
                }
            }
        }

        match L {
            0 => drop_children::<1>(self.base),
            1 => drop_children::<2>(self.base),
            2 => drop_children::<3>(self.base),
            3 => (),
            _ => unreachable!(),
        }

        let pfn = FrameNr::from_pa(self.base);
        let frame = phys::get_alloc_frame(pfn).unwrap_or_else(|| {
            panic!("unallocated page table frame: {pfn:?}");
        });

        unsafe { frame.dec_map() };
    }
}

pub(super) struct PageTableRef<'a, const L: u64> {
    inner: ManuallyDrop<PageTable<L>>,
    _lifetime: PhantomData<&'a ()>,
}

impl<const L: u64> PageTableRef<'_, L> {
    pub unsafe fn new(base: PA) -> Self {
        Self {
            inner: ManuallyDrop::new(PageTable { base }),
            _lifetime: PhantomData,
        }
    }
}

impl<const L: u64> Deref for PageTableRef<'_, L> {
    type Target = PageTable<L>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

pub(super) struct PageTableMut<'a, const L: u64> {
    inner: ManuallyDrop<PageTable<L>>,
    _lifetime: PhantomData<&'a ()>,
}

impl<const L: u64> PageTableMut<'_, L> {
    pub unsafe fn new(base: PA) -> Self {
        Self {
            inner: ManuallyDrop::new(PageTable { base }),
            _lifetime: PhantomData,
        }
    }
}

impl<const L: u64> Deref for PageTableMut<'_, L> {
    type Target = PageTable<L>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<const L: u64> DerefMut for PageTableMut<'_, L> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

pub(super) trait PageTableIndex<const L: u64>: Copy {
    fn index(&self) -> usize;
}

impl<const L: u64> PageTableIndex<L> for usize {
    fn index(&self) -> usize {
        *self
    }
}

impl<const L: u64> PageTableIndex<L> for PageNr {
    fn index(&self) -> usize {
        let bits = self.va().into_u64() as usize;
        let shift = 39 - 9 * L;
        (bits >> shift) & 0x1ff
    }
}

/// A page descriptor.
#[derive(Clone, Copy, Debug, Default)]
#[repr(transparent)]
pub(super) struct PageDesc(u64);

impl PageDesc {
    pub fn new(base: PA) -> Self {
        Self(base.into_u64() | 0b11)
    }

    fn valid(&self) -> bool {
        self.0 & 0b11 == 0b11
    }

    pub fn output_addr(&self) -> PA {
        PA::new(self.0 & 0xfffffffff000)
    }

    pub fn set_access_flag(&mut self) {
        self.0 |= 1 << 10;
    }

    pub fn set_attr_idx(&mut self, attr_idx: u8) {
        const MASK: u64 = 0b111;
        const SHIFT: u64 = 2;

        self.0 &= !(MASK << SHIFT);
        self.0 |= u64::from(attr_idx) << SHIFT;
    }

    pub fn set_shareability(&mut self, share: Shareability) {
        const MASK: u64 = 0b11;
        const SHIFT: u64 = 8;

        self.0 &= !(MASK << SHIFT);
        self.0 |= (share as u64) << SHIFT;
    }
}

/// A table descriptor.
#[derive(Clone, Copy, Debug, Default)]
#[repr(transparent)]
struct TableDesc(u64);

impl TableDesc {
    fn new(base: PA) -> Self {
        Self(base.into_u64() | 0b11)
    }

    fn valid(&self) -> bool {
        self.0 & 0b11 == 0b11
    }

    fn output_addr(&self) -> PA {
        PA::new(self.0 & 0xfffffffff000)
    }
}
