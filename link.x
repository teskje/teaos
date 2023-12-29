/*
 * Linker script for qemu's virt machine [1].
 *
 * [1]: https://qemu-project.gitlab.io/qemu/system/arm/virt.html
 */

ENTRY(start)

SECTIONS {
  . = 0x40000000;
  _dtb_start = .;
  . = . + 0x100000;
  _dtb_end = .;

  .text ALIGN(8): {
    *(.text .text.*)
  }
  .bss ALIGN(8): {
    *(.bss .bss.*)
  }
  .data ALIGN(8): {
    *(.data .data.*)
  }
  .rodata ALIGN(8): {
    *(.rodata .rodata.*)
  }

  _stack_top = .;
  . += 0x1000;
  _stack_bottom = .;
}
