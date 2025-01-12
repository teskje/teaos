.macro vector_entry
    .balign 128
.vector_entry\@:
    stp  x0,  x1, [sp, #-16]!
    stp  x2,  x3, [sp, #-16]!
    stp  x4,  x5, [sp, #-16]!
    stp  x6,  x7, [sp, #-16]!
    stp  x8,  x9, [sp, #-16]!
    stp x10, x11, [sp, #-16]!
    stp x12, x13, [sp, #-16]!
    stp x14, x15, [sp, #-16]!
    stp x16, x17, [sp, #-16]!
    stp x18, x30, [sp, #-16]!

    bl default_handler

    ldp x18, x30, [sp], #16
    ldp x16, x17, [sp], #16
    ldp x14, x15, [sp], #16
    ldp x12, x13, [sp], #16
    ldp x10, x11, [sp], #16
    ldp  x8,  x9, [sp], #16
    ldp  x6,  x7, [sp], #16
    ldp  x4,  x5, [sp], #16
    ldp  x2,  x3, [sp], #16
    ldp  x0,  x1, [sp], #16

    eret

// protect against vector overflow
.org .vector_entry\@ + 128
.endm

.section .text.vector, "ax"
.balign 2048
.global exception_vectors
exception_vectors:
    vector_entry    // Synchronous EL1t
    vector_entry    // IRQ EL1t
    vector_entry    // FIQ EL1t
    vector_entry    // SError EL1t
    vector_entry    // Synchronous EL1h
    vector_entry    // IRQ EL1h
    vector_entry    // FIQ EL1h
    vector_entry    // SError EL1h
    vector_entry    // Synchronous 64-bit EL0
    vector_entry    // IRQ 64-bit EL0
    vector_entry    // FIQ 64-bit EL0
    vector_entry    // SError 64-bit EL0
    vector_entry    // Synchronous 32-bit EL0
    vector_entry    // IRQ 32-bit EL0
    vector_entry    // FIQ 32-bit EL0
    vector_entry    // SError 32-bit EL0
