use std::env;

fn main() {
    println!("cargo:rerun-if-changed=compat.c");

    let os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let target_env = env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
    if os != "linux" || target_env != "gnu" {
        return;
    }

    for obj in cc::Build::new().file("compat.c").compile_intermediates() {
        println!("cargo:rustc-link-arg={}", obj.display());
    }
}
