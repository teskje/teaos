use core::arch::asm;

use cpu::vmem::PAGE_SIZE;
use kstd::memory::{PA, VA};

use crate::uefi;

const TABLE_LEN: usize = 512;

pub struct TranslationTable {
    level0: &'static mut [Descriptor; TABLE_LEN],
}

impl TranslationTable {
    pub fn new() -> Self {
        Self {
            level0: allocate_table(),
        }
    }

    pub fn map(&mut self, mut va: VA, mut pa: PA, mut size: usize) {
        assert_eq!(u64::from(va) >> 48, 0xffff, "only high memory supported");
        assert!(
            va.is_aligned_to(PAGE_SIZE),
            "va {va:#} not aligned to page size"
        );
        assert!(
            pa.is_aligned_to(PAGE_SIZE),
            "pa {pa:#} not aligned to page size"
        );

        while size > 0 {
            self.map_page(va, pa);
            va += PAGE_SIZE;
            pa += PAGE_SIZE;
            size = size.saturating_sub(PAGE_SIZE);
        }
    }

    fn map_page(&mut self, va: VA, pa: PA) {
        let desc = self.get_descriptor(va, 3);
        *desc = Descriptor::page(pa);
    }

    fn get_descriptor(&mut self, va: VA, level: usize) -> &mut Descriptor {
        let table_index = |va: VA, lvl: usize| {
            let shift = 39 - 9 * lvl;
            (usize::from(va) >> shift) & 0x1ff
        };

        let idx = table_index(va, 0);
        let mut desc = &mut self.level0[idx];

        for lvl in 1..=level {
            if desc.is_invalid() {
                let buffer = uefi::allocate_page();
                let pa = PA::new(buffer.as_mut_ptr() as u64);
                *desc = Descriptor::table(pa);
            }

            let entries = desc.table_entries();

            let idx = table_index(va, lvl);
            desc = &mut entries[idx];
        }

        desc
    }

    /// Install this translation table.
    ///
    /// This method must be called after [`uefi::exit_boot_services`], i.e. after UEFI has released
    /// control over the system's translation tables.
    pub fn install(&self) {
        let ttb = self.level0.as_ptr() as u64;

        let mut tcr: u64;
        unsafe {
            asm!("MRS {x}, TCR_EL1", x = out(reg) tcr);
        };

        // TCR.T1SZ = 16
        tcr &= !(0x3f << 16);
        tcr |= 16 << 16;
        // TCR.EPD1 = 0
        tcr &= !(1 << 23);
        // TCR.IRGN1 = 0b00 (normal memory, inner non-cacheable)
        tcr &= !(0x3 << 24);
        // TCR.ORGN1 = 0b00 (normal memory, outer non-cacheable)
        tcr &= !(0x3 << 26);
        // TCR.SH1 = 0b00 (non-shareable)
        tcr &= !(0x3 << 28);
        // TCR.TG1 = 0b10 (4KB)
        tcr &= !(0x3 << 30);
        tcr |= 0b10 << 30;

        unsafe {
            asm!(
                "MSR TTBR1_EL1, {ttb}",
                "MSR TCR_EL1, {tcr}",
                "ISB",
                ttb = in(reg) ttb,
                tcr = in(reg) tcr,
            );
        }
    }
}

fn allocate_table() -> &'static mut [Descriptor; TABLE_LEN] {
    let buffer = uefi::allocate_page();
    unsafe { &mut *buffer.as_mut_ptr().cast() }
}

#[derive(Clone, Copy)]
#[repr(transparent)]
struct Descriptor(u64);

impl Descriptor {
    fn table(pa: PA) -> Self {
        assert!(pa.is_aligned_to(PAGE_SIZE));
        Self(u64::from(pa) | 0b11)
    }

    fn page(pa: PA) -> Self {
        assert!(pa.is_aligned_to(PAGE_SIZE));

        let mut x = u64::from(pa) | 0b11;

        // Set the access flag, to prevent the generation of Access flag faults.
        x |= 1 << 10;

        Self(x)
    }

    fn is_invalid(&self) -> bool {
        (self.0 & 1) == 0
    }

    fn address(&self) -> PA {
        PA::new(self.0 & 0xfffffffff000)
    }

    fn table_entries(&mut self) -> &mut [Descriptor; TABLE_LEN] {
        assert_eq!(self.0 & 0b11, 0b11);

        let ptr = self.address().as_mut_ptr();
        unsafe { &mut *ptr }
    }
}
