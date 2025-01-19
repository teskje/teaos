kernel_start = 0xffff000000000000;

SECTIONS {
    . = kernel_start;
    .text   : ALIGN(0x1000) { *(.text); *(.text.*) }
    .data   : ALIGN(0x1000) { *(.data); *(.data.*) }
    .rodata : ALIGN(0x1000) { *(.rodata); *(.rodata.*) }
    .bss    : ALIGN(0x1000) { *(.bss); *(.bss.*) }
    kernel_end = .;
}
