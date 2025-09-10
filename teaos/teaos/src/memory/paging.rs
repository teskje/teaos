use aarch64::memory::paging::{AddrMapper, FrameAlloc, PAGE_SIZE};
use aarch64::memory::{PA, VA};

use crate::memory::{alloc_frame, pa_to_va};

pub(super) type PageMap = aarch64::memory::paging::PageMap<Alloc, PhysMapper>;

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
