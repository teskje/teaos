pub mod paging;

mod address;

pub use self::address::{PA, VA};

pub const PAGE_SHIFT: u64 = 12;
pub const PAGE_SIZE: usize = 1 << PAGE_SHIFT;
pub const PAGE_MAP_LEVELS: u64 = 3;
