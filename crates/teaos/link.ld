/* the kernel lives in high memory */
__KERNEL_START        = 0xffff000000000000;

/* 64 KiB kernel stack */
__STACK_START         = 0xffff000040000000;
__STACK_END           = 0xffff000040010000;

/* 2 GiB kernel heap */
__HEAP_START          = 0xffff000080000000;
__HEAP_END            = 0xffff000100000000;

/* 240 TiB for directly mapping physical memory */
__LINEAR_REGION_START = 0xffff100000000000; 

SECTIONS {
    . = __KERNEL_START;

    .text   : ALIGN(0x1000) { *(.text); *(.text.*) }
    .data   : ALIGN(0x1000) { *(.data); *(.data.*) }
    .rodata : ALIGN(0x1000) { *(.rodata); *(.rodata.*) }
    .bss    : ALIGN(0x1000) { *(.bss); *(.bss.*) }

    __KERNEL_END  = .;
}
