fn main() {
    let config = pyo3_build_config::get();
    if let Some(lib_dir) = &config.lib_dir {

        println!("cargo:rustc-link-arg=-Wl,-rpath,{}", lib_dir);
        println!("cargo:rustc-link-arg=-Wl,--enable-new-dtags");
    } else {
        println!("cargo:warning=Lib Dir could not be detected!");
    }
}