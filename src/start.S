.global start

start:
    // Initialize the stack.
    ldr x0, =_stack_bottom
    mov sp, x0

    // Enable the FPU.
    mrs x0, CPACR_EL1
    orr x0, x0, #(0x3 << 20)
    msr CPACR_EL1, x0
    isb

    b main
