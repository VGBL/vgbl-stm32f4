use std::env;

fn main() {
    // Copyup has its own minimal linker script instead of cortex-m-rt's link.x
    println!("cargo:rustc-link-search={}", env::var("CARGO_MANIFEST_DIR").unwrap());
    println!("cargo:rustc-link-arg-bins=-Tcopyup.x");
    println!("cargo:rerun-if-changed=copyup.x");
}
