fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=wrapper.h");
    println!("cargo:rerun-if-changed=wrapper.cc");

    #[cfg(feature = "link")]
    build_with_ortools();
}

#[cfg(feature = "link")]
fn build_with_ortools() {
    use std::{env, path::PathBuf};

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let workspace_root = manifest_dir.parent().unwrap().parent().unwrap();

    // Allow override via FERROX_ORTOOLS_ROOT env var; default to vendor/ortools/build
    let ortools_build = env::var("FERROX_ORTOOLS_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|_| workspace_root.join("vendor/ortools/build"));

    let ortools_src = ortools_build.parent().unwrap().to_path_buf();

    if !ortools_build.exists() {
        panic!(
            "OR-Tools build not found at {:?}.\n\
             Run `make ortools` from the ferrox workspace root, or set FERROX_ORTOOLS_ROOT.",
            ortools_build
        );
    }

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
    println!("cargo:rustc-link-lib=dylib=ortools");

    let absl_libs = [
        "absl_log_internal_message", "absl_log_internal_check_op",
        "absl_log_internal_nullguard", "absl_log_internal_conditions",
        "absl_log_internal_format", "absl_log_internal_globals",
        "absl_log_internal_log_sink_set", "absl_log_globals", "absl_log_entry",
        "absl_log_severity", "absl_raw_logging_internal", "absl_examine_stack",
        "absl_stacktrace", "absl_symbolize", "absl_debugging_internal",
        "absl_demangle_internal", "absl_demangle_rust", "absl_decode_rust_punycode",
        "absl_time", "absl_time_zone", "absl_civil_time", "absl_strings",
        "absl_strings_internal", "absl_string_view", "absl_int128",
        "absl_throw_delegate", "absl_base", "absl_spinlock_wait",
        "absl_synchronization", "absl_malloc_internal", "absl_graphcycles_internal",
        "absl_kernel_timeout_internal", "absl_hashtablez_sampler",
        "absl_exponential_biased", "absl_hash", "absl_city", "absl_raw_hash_set",
        "absl_status", "absl_statusor", "absl_cord", "absl_cord_internal",
        "absl_cordz_functions", "absl_cordz_handle", "absl_cordz_info",
        "absl_cordz_sample_token", "absl_crc32c", "absl_crc_cord_state",
        "absl_crc_cpu_detect", "absl_crc_internal", "absl_strerror",
        "absl_random_internal_seed_material", "absl_random_seed_gen_exception",
        "absl_random_seed_sequences", "absl_random_distributions",
        "absl_random_internal_randen", "absl_random_internal_randen_hwaes",
        "absl_random_internal_randen_hwaes_impl", "absl_random_internal_randen_slow",
        "absl_random_internal_platform", "absl_random_internal_entropy_pool",
        "absl_str_format_internal", "absl_tracing_internal", "absl_vlog_config_internal",
    ];
    for lib in absl_libs {
        println!("cargo:rustc-link-lib=dylib={lib}");
    }
    println!("cargo:rustc-link-lib=dylib=protobuf");

    #[cfg(target_os = "macos")]
    println!("cargo:rustc-link-lib=c++");
    #[cfg(target_os = "linux")]
    println!("cargo:rustc-link-lib=stdc++");
}
