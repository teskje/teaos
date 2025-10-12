//! The TeaOS boot loader.
//!
//! The boot loader is really just a thin shim between UEFI and the TeaOS kernel. It presents as a
//! UEFI application that loads the kernel and userimg from the boot disk, collects information
//! about the system required for the kernel to boot, then exits boot services and jumps into the
//! kernel.

#![cfg_attr(not(test), no_std)]

extern crate alloc;

pub mod log;

mod allocator;
mod paging;
mod uefi;

use aarch64::memory::paging::{AccessPermissions, Flags};
use aarch64::memory::{PA, PAGE_SIZE, VA};
use alloc::vec;
use alloc::vec::Vec;
use boot_info::{BootInfo, MemoryType};
use core::ffi::c_void;
use core::mem;
use elf::ElfFile;
use kstd::io::Read;

use crate::paging::KernelPager;

/// Initialize the UEFI wrapper.
///
/// # Safety
///
/// `system_table` must be a valid pointer to a [`sys::SYSTEM_TABLE`].
pub unsafe fn init_uefi(image_handle: *mut c_void, system_table: *mut c_void) {
    unsafe { uefi::init(image_handle, system_table.cast()) }
}

/// Run the boot loader.
///
/// This loads the kernel binary and userimg, retrieves all required boot information, and finally
/// passes control to the kernel.
pub fn load() -> ! {
    log!("entered UEFI boot loader");

    log!("loading kernel binary");
    let mut kernel = load_kernel();
    log!("  kernel.entry={:#?}", kernel.entry);
    log!("  kernel.userimg_start={:?}", kernel.userimg_start);
    log!("  kernel.physmap_start={:?}", kernel.physmap_start);

    log!("loading userimg");
    load_userimg(&mut kernel.pager, kernel.userimg_start);

    log!("retrieving ACPI RSDP pointer");
    let rsdp = find_acpi_rsdp();
    log!("  rsdp_ptr={rsdp:#?}");

    log!("retrieving UART config");
    let uart_info = unsafe { find_uart(rsdp) };
    log!("  uart={uart_info:?}");

    log!("creating phys mapping");
    let uart_base = uart_info.base();
    create_physmap(&mut kernel.pager, kernel.physmap_start, uart_base);

    log!("exiting boot services");
    let memory_info = exit_boot_services();

    // No (de)allocating or logging beyond this point!
    // We have lost access to the boot services and any attempt to invoke one will panic.

    kernel.pager.apply();

    let bootinfo = BootInfo {
        memory: memory_info,
        uart: uart_info,
        acpi_rsdp: PA::new(rsdp as u64),
    }
    .into_ffi();

    (kernel.entry)(bootinfo);
}

struct Kernel {
    entry: fn(boot_info::ffi::BootInfo) -> !,
    pager: KernelPager,
    userimg_start: VA,
    physmap_start: VA,
}

/// Memory type used by the loader for pages containing kernel code or data.
const KERNEL_MEMORY: uefi::sys::MEMORY_TYPE = 0x80000000;

/// Load the kernel binary.
///
/// The kernel binary is expected to be located in the boot file system at `\kernel`, and is
/// expected to be an ELF file. Its loadable segments are read into memory and mapped into the
/// returned page table.
fn load_kernel() -> Kernel {
    let boot_fs = uefi::get_boot_fs();
    let root = boot_fs.open_volume();
    let kernel_file = root.open("\\kernel");

    let mut elf = ElfFile::open(kernel_file);

    let entry = elf.entry();
    let entry = unsafe { mem::transmute::<u64, fn(boot_info::ffi::BootInfo) -> !>(entry) };

    let mut pager = KernelPager::new();
    let phdrs: Vec<_> = elf.program_headers().collect();
    for phdr in phdrs {
        if !phdr.is_load() {
            continue;
        }

        let size = phdr.memory_size() as usize;
        let buffer = uefi::allocate_page_memory(size, KERNEL_MEMORY);
        elf.read_segment(&phdr, buffer);

        let ap = if phdr.is_writable() {
            AccessPermissions::PrivRW
        } else {
            AccessPermissions::PrivRO
        };
        let xn = !phdr.is_executable();
        let flags = Flags::default()
            .access_permissions(ap)
            .privileged_execute_never(xn);

        let pa = PA::new(buffer.as_ptr() as u64);
        let va = VA::new(phdr.virtual_address());
        let count = buffer.len() / PAGE_SIZE;
        pager.map_ram_region(va, pa, count, flags);
        log!("  mapped {va:#} -> {pa:#} ({count} pages)");
    }

    let mut userimg_start = None;
    let mut physmap_start = None;
    if let Some(strtab) = elf.symbol_strtab() {
        for sym in elf.symbols().unwrap() {
            let name = sym.name(&strtab);
            if name == c"userimg_start" {
                userimg_start = Some(VA::new(sym.value()));
            } else if name == c"physmap_start" {
                physmap_start = Some(VA::new(sym.value()));
            }
        }
    }

    let userimg_start =
        userimg_start.unwrap_or_else(|| panic!("missing `userimg_start` kernel symbol"));
    let physmap_start =
        physmap_start.unwrap_or_else(|| panic!("missing `physmap_start` kernel symbol"));

    Kernel {
        entry,
        pager,
        userimg_start,
        physmap_start,
    }
}

/// Load the userimg binary.
///
/// The userimg binary is expected to be located in the boot file system at `\userimg`, and is
/// expected to be an ELF file. It is mapped verbatim into the given `pager` at `userimg_start`.
fn load_userimg(pager: &mut KernelPager, userimg_start: VA) {
    let boot_fs = uefi::get_boot_fs();
    let root = boot_fs.open_volume();
    let mut userimg_file = root.open("\\userimg");

    let size = userimg_file.get_size() as usize;
    let buffer = uefi::allocate_page_memory(size, KERNEL_MEMORY);
    userimg_file.read_exact(&mut buffer[..size]).unwrap();

    let pa = PA::new(buffer.as_ptr() as u64);
    let pages = buffer.len() / PAGE_SIZE;
    let flags = Flags::default()
        .access_permissions(AccessPermissions::PrivRO)
        .privileged_execute_never(true);
    pager.map_ram_region(userimg_start, pa, pages, flags);
    log!("  mapped {userimg_start:#} -> {pa:#} ({pages} pages)");
}

fn create_physmap(pager: &mut KernelPager, physmap_start: VA, uart_base: PA) {
    let mut map = |pa: PA, pages, type_| {
        let va = physmap_start + pa.into_u64();
        let flags = Flags::default()
            .access_permissions(AccessPermissions::PrivRW)
            .privileged_execute_never(true);

        if type_ == MemoryType::Mmio {
            pager.map_mmio_region(va, pa, pages, flags);
        } else {
            pager.map_ram_region(va, pa, pages, flags);
        }
    };

    let (buffer_size, _) = uefi::get_memory_map_size();
    // Allocating this `Vec` may add an entry to the memory map, so we need to overprovision.
    let buffer = vec![0; buffer_size + 1024];
    let memory_map = uefi::get_memory_map(buffer);

    for desc in memory_map.iter() {
        if let Some(block) = memory_bootinfo_from_uefi(desc) {
            map(block.start, block.pages, block.type_);
        };
    }

    // The UEFI memory map doesn't include all device MMIO regions, so map the UART one explicitly.
    map(uart_base, 1, MemoryType::Mmio);
}

/// Find the ACPI RSDP in the UEFI config table.
///
/// # Panics
///
/// Panics if no RSDP entry is found.
fn find_acpi_rsdp() -> *mut acpi::RSDP {
    for (guid, ptr) in uefi::config_table().iter() {
        if guid == uefi::sys::ACPI_TABLE_GUID {
            return ptr.cast();
        }
    }

    panic!("ACPI config table not found");
}

/// Retrieve information about the serial port.
///
/// Finds the SPCR in the ACPI tables and extracts the UART type and base address.
///
/// # Safety
///
/// `rsdp` must be a valid pointer to an [`acpi::RSDP`].
unsafe fn find_uart(rsdp_ptr: *mut acpi::RSDP) -> boot_info::Uart {
    let rsdp = unsafe { &*rsdp_ptr };

    assert_eq!(rsdp.signature, *b"RSD PTR ");
    assert_eq!(rsdp.revision, 2);

    let xsdt_ptr = rsdp.xsdt_address as *const acpi::XSDT;
    let xsdt = unsafe { &*xsdt_ptr };
    assert_eq!(xsdt.header.signature, *b"XSDT");
    assert_eq!(xsdt.header.revision, 1);

    let xsdt_size = xsdt.header.length as usize;
    let mut entry_size = xsdt_size - mem::offset_of!(acpi::XSDT, entry);
    let mut entry_ptr = xsdt.entry.as_ptr();

    let mut spcr: Option<&acpi::SPCR> = None;
    const ADDR_SIZE: usize = mem::size_of::<usize>();
    while entry_size >= ADDR_SIZE {
        let addr_bytes_ptr = entry_ptr as *mut [u8; ADDR_SIZE];
        let addr_bytes = unsafe { *addr_bytes_ptr };
        let addr = usize::from_le_bytes(addr_bytes);
        let desc_ptr = addr as *mut acpi::DESCRIPTION_HEADER;
        let desc = unsafe { &*desc_ptr };
        if desc.signature == *b"SPCR" {
            spcr = Some(unsafe { &*desc_ptr.cast() });
            break;
        }

        entry_ptr = unsafe { entry_ptr.add(ADDR_SIZE) };
        entry_size -= ADDR_SIZE;
    }

    let spcr = spcr.expect("SPCR table present");
    assert_eq!(spcr.header.revision, 2);

    let base = PA::new(spcr.base_address.address);

    match spcr.interface_type {
        acpi::UART_TYPE_16550 => boot_info::Uart::Uart16550 { base },
        acpi::UART_TYPE_PL011 => boot_info::Uart::Pl011 { base },
        value => unimplemented!("UART type: {value:#x}"),
    }
}

/// Exit the UEFI boot services.
///
/// Returns information about the physical memory in the system.
fn exit_boot_services() -> boot_info::Memory<'static> {
    let (buffer_size, desc_size) = uefi::get_memory_map_size();
    let len = buffer_size / desc_size;

    // Allocating these `Vec`s may add entries to the memory map, so we need to overprovision.
    let buffer = vec![0; buffer_size + 1024];
    let mut block_info = Vec::with_capacity(len + 5);

    let memory_map = uefi::get_memory_map(buffer);

    uefi::exit_boot_services(memory_map.map_key);

    for desc in memory_map.iter() {
        if let Some(block) = memory_bootinfo_from_uefi(desc) {
            block_info.push(block);
        }
    }

    // We can't deallocate anymore, so we must avoid dropping the `MemoryMap`.
    mem::forget(memory_map);

    boot_info::Memory::new(block_info)
}

fn memory_bootinfo_from_uefi(
    desc: &uefi::sys::MEMORY_DESCRIPTOR,
) -> Option<boot_info::MemoryBlock> {
    use uefi::sys::*;

    #[allow(non_upper_case_globals)]
    let type_ = match desc.type_ {
        ConventionalMemory | PersistentMemory => MemoryType::Unused,
        LoaderCode | LoaderData | BootServicesCode | BootServicesData | RuntimeServicesCode
        | RuntimeServicesData => MemoryType::Boot,
        ACPIReclaimMemory | ACPIMemoryNVS => MemoryType::Acpi,
        MemoryMappedIO | MemoryMappedIOPortSpace => MemoryType::Mmio,
        KERNEL_MEMORY => MemoryType::Kernel,
        ReservedMemoryType | UnusableMemory | PalCode | UnacceptedMemoryType => return None,
        _ => return None,
    };

    let block = boot_info::MemoryBlock {
        type_,
        start: desc.physical_start.into(),
        pages: desc.number_of_pages as usize,
    };
    Some(block)
}

/// Validate the given pointer.
///
/// # Panics
///
/// Panics if the given pointer is NULL.
/// Panics if the given pointer is not correctly aligned.
fn validate_ptr<T>(ptr: *const T) {
    assert!(!ptr.is_null());
    assert!(ptr.is_aligned());
}

/// Validate the given pointer.
///
/// # Panics
///
/// Panics if the given pointer is NULL.
/// Panics if the given pointer is not correctly aligned.
fn validate_mut_ptr<T>(ptr: *mut T) {
    assert!(!ptr.is_null());
    assert!(ptr.is_aligned());
}
