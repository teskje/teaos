use std::env;

fn main() {
    let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    println!("cargo::rustc-link-arg-bins=-T{crate_dir}/link.ld");
}
