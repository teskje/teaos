PHDRS {
    text   PT_LOAD;
    rodata PT_LOAD;
    data   PT_LOAD;
    stack  PT_LOAD;
}

SECTIONS {
    . = kernel_start;
    .text : ALIGN(0x1000) { *(.text*) } :text
    .rodata : ALIGN(0x1000) { *(.rodata*) } :rodata
    .data : ALIGN(0x1000) { *(.data*) } :data
    .bss : ALIGN(0x1000) { *(.bss*) } :data

    . = kstack_start;
    .kstack : ALIGN(16) { KEEP(*(.kstack)) } :stack
}
