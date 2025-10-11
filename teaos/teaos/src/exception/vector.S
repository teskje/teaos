.macro vector name
    .balign 128
.vector\@:
    stp x18, x30, [sp, #-16]!
    stp x16, x17, [sp, #-16]!
    stp x14, x15, [sp, #-16]!
    stp x12, x13, [sp, #-16]!
    stp x10, x11, [sp, #-16]!
    stp  x8,  x9, [sp, #-16]!
    stp  x6,  x7, [sp, #-16]!
    stp  x4,  x5, [sp, #-16]!
    stp  x2,  x3, [sp, #-16]!
    stp  x0,  x1, [sp, #-16]!

    mrs x10, spsr_el1
    mrs x11, elr_el1
    stp x10, x11, [sp, #-16]!

    mov x0, sp
    bl handle_\name

    ldp x10, x11, [sp], #16
    msr elr_el1, x11
    msr spsr_el1, x10

    ldp  x0,  x1, [sp], #16
    ldp  x2,  x3, [sp], #16
    ldp  x4,  x5, [sp], #16
    ldp  x6,  x7, [sp], #16
    ldp  x8,  x9, [sp], #16
    ldp x10, x11, [sp], #16
    ldp x12, x13, [sp], #16
    ldp x14, x15, [sp], #16
    ldp x16, x17, [sp], #16
    ldp x18, x30, [sp], #16

    eret

// protect against vector overflow
.org .vector\@ + 128
.endm

.section .text.exception, "ax"
.balign 2048
.global exception_vectors
exception_vectors:
    vector unhandled       // Synchronous EL1 with SP_EL0
    vector unhandled       // IRQ EL1 with SP_EL0
    vector unhandled       // FIQ EL1 with SP_EL0
    vector unhandled       // SError EL1 with SP_EL0

    vector exception_el1   // Synchronous EL1 with SP_ELx
    vector unhandled       // IRQ EL1 with SP_ELx
    vector unhandled       // FIQ EL1 with SP_ELx
    vector unhandled       // SError EL1 with SP_ELx

    vector exception_el0   // Synchronous 64-bit EL0
    vector unhandled       // IRQ 64-bit EL0
    vector unhandled       // FIQ 64-bit EL0
    vector unhandled       // SError 64-bit EL0

    vector unhandled       // Synchronous 32-bit EL0
    vector unhandled       // IRQ 32-bit EL0
    vector unhandled       // FIQ 32-bit EL0
    vector unhandled       // SError 32-bit EL0
