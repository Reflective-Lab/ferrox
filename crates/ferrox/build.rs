fn main() {
    // Inject rpaths for vendor dylibs so tests can find them at runtime.
    // DEP_*_LIB_DIR is set by highs-sys/ortools-sys build scripts via
    // `cargo:LIB_DIR=...` (propagated because those crates use `links = ...`).
    for dep in ["DEP_HIGHS_LIB_DIR", "DEP_ORTOOLS_LIB_DIR"] {
        if let Ok(lib_dir) = std::env::var(dep) {
            println!("cargo:rustc-link-arg=-Wl,-rpath,{lib_dir}");
        }
    }
}
