fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=highs_wrapper.h");
    println!("cargo:rerun-if-changed=highs_wrapper.cc");

    #[cfg(feature = "link")]
    build_with_highs();
}

#[cfg(feature = "link")]
fn build_with_highs() {
    use std::{env, path::PathBuf};

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let workspace_root = manifest_dir.parent().unwrap().parent().unwrap();

    let highs_build = env::var("FERROX_HIGHS_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|_| workspace_root.join("vendor/highs/build"));

    let highs_src = highs_build.parent().unwrap().to_path_buf();

    if !highs_build.exists() {
        panic!(
            "HiGHS build not found at {:?}.\n\
             Run `make highs` from the ferrox workspace root, or set FERROX_HIGHS_ROOT.",
            highs_build
        );
    }

    cc::Build::new()
        .cpp(true)
        .std("c++17")
        .file("highs_wrapper.cc")
        .include(highs_src.join("src"))
        .include(highs_build.join("generated"))
        .flag_if_supported("-Wno-unused-parameter")
        .flag_if_supported("-Wno-deprecated-declarations")
        .compile("highs_wrapper");

    let lib_dir = highs_build.join("lib");
    println!("cargo:rustc-link-search=native={}", lib_dir.display());
    println!("cargo:LIB_DIR={}", lib_dir.display());
    println!("cargo:rustc-link-lib=dylib=highs");

    #[cfg(target_os = "macos")]
    println!("cargo:rustc-link-lib=c++");
    #[cfg(target_os = "linux")]
    println!("cargo:rustc-link-lib=stdc++");
}
