use aarch64::instruction::{dsb_ishst, isb};
use aarch64::memory::paging::{load_ttbr1, AddrMapper, FrameAlloc, MemoryClass, PAGE_SIZE};
use aarch64::memory::{PA, VA};
use aarch64::register::TTBR1_EL1;
use kstd::sync::Mutex;

use crate::memory::{alloc_frame, pa_to_va};

static ACTIVE_KERNEL_MAP: Mutex<Option<PageMap>> = Mutex::new(None);

type PageMap = aarch64::memory::paging::PageMap<Alloc, PhysMapper>;

pub(super) struct Alloc;

impl FrameAlloc for Alloc {
    fn alloc_frame() -> PA {
        let pa = alloc_frame();

        let va = pa_to_va(pa);
        unsafe {
            let frame = &mut *va.as_mut_ptr::<[u8; PAGE_SIZE]>();
            frame.fill(0);
        }

        pa
    }
}

pub(super) struct PhysMapper;

impl AddrMapper for PhysMapper {
    fn pa_to_va(pa: PA) -> VA {
        pa_to_va(pa)
    }
}

/// Initialize kernel paging.
///
/// This takes over the kernel (TTBR1) page tables from the boot loader by cloning them into a new
/// set of tables and then switching over to those. Doing so allows us to later free all boot
/// loader memory without having to make exceptions for the page tables.
pub(super) fn init() {
    let mut active = ACTIVE_KERNEL_MAP.lock();
    assert!(active.is_none(), "kernel map already initialized");

    let ttbr = TTBR1_EL1::read();
    let boot_ttb = PA::new(ttbr.BADDR() << 1);
    let boot_map = PageMap::with_root(boot_ttb);

    let mut kernel_map = PageMap::new();
    kernel_map.clone_from(&boot_map);

    // SAFETY: New map contains all existing mappings.
    unsafe { load_ttbr1(&kernel_map) };

    *active = Some(kernel_map);
}

pub fn map_page(va: VA, pa: PA, class: MemoryClass) {
    let mut active = ACTIVE_KERNEL_MAP.lock();
    let kernel_map = active.as_mut().expect("kernel map initialized");

    kernel_map.map_page(va, pa, class);

    // Wait for the new mapping to become visible.
    // Note that we don't need to TLBI here, since there wasn't a valid mapping for the VA before
    // (`PageMap::map_page` checks that).
    dsb_ishst();
    isb();
}
