use std::{env, fs, path::PathBuf};

fn main() {
    // Copyup builds run from RAM, so they need the RAM layout
    let layout = if env::var_os("CARGO_FEATURE_COPYUP").is_some() { "memory-copyup.x" } else { "memory.x" };

    // Put the layout where the cortex-m-rt linker script can find it as memory.x
    let out = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    fs::copy(layout, out.join("memory.x")).unwrap();
    println!("cargo:rustc-link-search={}", out.display());
    println!("cargo:rerun-if-changed=memory.x");
    println!("cargo:rerun-if-changed=memory-copyup.x");

    // link.x comes from cortex-m-rt and INCLUDEs memory.x
    println!("cargo:rustc-link-arg-bins=-Tlink.x");
}
