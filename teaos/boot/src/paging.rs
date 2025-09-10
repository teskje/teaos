use aarch64::memory::paging::{AddrMapper, FrameAlloc};
use aarch64::memory::{PA, VA};

use crate::uefi;

pub(crate) type PageMap = aarch64::memory::paging::PageMap<UefiAlloc, IdMapper>;

pub(crate) struct UefiAlloc;

impl FrameAlloc for UefiAlloc {
    fn alloc_frame() -> PA {
        // `allocate_page` zero-fills the returned page
        let buffer = uefi::allocate_page();
        PA::new(buffer.as_mut_ptr() as u64)
    }
}

pub(crate) struct IdMapper;

impl AddrMapper for IdMapper {
    fn pa_to_va(pa: PA) -> VA {
        VA::new(pa.into())
    }
}
