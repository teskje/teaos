use std::env;

fn main() {
    // The OS loader expects all ELF segments to be page-aligned. We supply a custom linker script
    // to enforce this.
    let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    println!("cargo::rustc-link-arg-bins=-T{crate_dir}/link.ld");
}
