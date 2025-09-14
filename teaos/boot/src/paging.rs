use aarch64::memory::{AddrMapper, Frame, FrameAlloc, PA, VA};

use crate::uefi;

pub(crate) type PageMap = aarch64::memory::paging::PageMap<UefiAlloc, IdMapper>;

pub(crate) struct UefiAlloc;

impl FrameAlloc for UefiAlloc {
    fn alloc_frame() -> Frame {
        let buffer = uefi::allocate_page(uefi::sys::LoaderData);
        let pa = PA::new(buffer.as_mut_ptr() as u64);
        Frame::new(pa)
    }
}

pub(crate) struct IdMapper;

impl AddrMapper for IdMapper {
    fn pa_to_va(pa: PA) -> VA {
        VA::new(pa.into())
    }
}
