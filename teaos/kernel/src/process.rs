use core::arch::asm;

use aarch64::memory::paging::load_ttbr0;
use aarch64::memory::{PAGE_SIZE, VA, paging};

use crate::memory;
use crate::memory::virt::{PageMap, PageNr};

struct Process {
    page_map: PageMap,
}

impl Process {
    fn new() -> Self {
        Self {
            page_map: PageMap::new(),
        }
    }
}

pub fn run() -> ! {
    let mut proc = Process::new();

    // load
    let code_ptr = user_mode as *const u8;
    let mut code_frame = memory::phys::alloc();
    code_frame.with_contents(|buf| unsafe {
        for i in 0..PAGE_SIZE {
            buf[i] = *code_ptr.add(i);
        }
    });
    let entry_va = VA::new(0x1000);
    let entry_vpn = PageNr::from_va(entry_va);
    let flags = paging::Flags::default().access_permissions(paging::AccessPermissions::UnprivRO);
    proc.page_map.map_ram_page(entry_vpn, code_frame, flags);

    // enter
    unsafe {
        load_ttbr0(proc.page_map.base(), 0);

        asm!(
            r#"
            msr spsr_el1, {spsr:x}
            msr elr_el1, {entry}
            eret
            "#,
            spsr = in(reg) 0,
            entry = in(reg) entry_va.into_u64(),
        );
    }

    unreachable!();
}

extern "C" fn user_mode() {
    unsafe { asm!("svc #0") };

    loop {}
}
