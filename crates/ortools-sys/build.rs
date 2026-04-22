fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=wrapper.h");
    println!("cargo:rerun-if-changed=wrapper.cc");

    if std::env::var("CARGO_FEATURE_LINK").is_ok() {
        build_with_ortools();
    }
}

fn build_with_ortools() {
    use std::{env, path::PathBuf};

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let workspace_root = manifest_dir
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();

    let ortools_build = env::var("FERROX_ORTOOLS_ROOT").map_or_else(
        |_| workspace_root.join("vendor/ortools/build"),
        PathBuf::from,
    );

    let ortools_src = ortools_build.parent().unwrap().to_path_buf();

    assert!(
        ortools_build.exists(),
        "OR-Tools build not found at {}.\nRun `make ortools` from the ferrox workspace root, or set FERROX_ORTOOLS_ROOT.",
        ortools_build.display()
    );

    cc::Build::new()
        .cpp(true)
        .std("c++17")
        .file("wrapper.cc")
        .include(&ortools_src)
        .include(&ortools_build)
        .include(ortools_build.join("_deps/absl-src"))
        .include(ortools_build.join("_deps/protobuf-src/src"))
        .include(ortools_build.join("_deps/protobuf-src/third_party/utf8_range"))
        .define("OR_PROTO_DLL", "")
        .flag_if_supported("-Wno-unused-parameter")
        .flag_if_supported("-Wno-deprecated-declarations")
        .compile("ortools_wrapper");

    let lib_dir = ortools_build.join("lib");
    println!("cargo:rustc-link-search=native={}", lib_dir.display());
    println!("cargo:LIB_DIR={}", lib_dir.display());

    // libortools.dylib already embeds absl/protobuf — no need to list them separately.
    println!("cargo:rustc-link-lib=dylib=ortools");
    println!("cargo:rustc-link-arg=-Wl,-rpath,{}", lib_dir.display());

    #[cfg(target_os = "macos")]
    println!("cargo:rustc-link-lib=c++");
    #[cfg(target_os = "linux")]
    println!("cargo:rustc-link-lib=stdc++");
}
