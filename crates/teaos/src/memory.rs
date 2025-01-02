extern "C" {
    pub static __KERNEL_START: u8;
    pub static __KERNEL_END: u8;

    pub static __STACK_START: u8;
    pub static __STACK_END: u8;

    pub static __HEAP_START: u8;
    pub static __HEAP_END: u8;

    pub static __LINEAR_REGION_START: u8;
}
