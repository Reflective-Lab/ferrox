fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Propagate rpath from highs-sys / ortools-sys to this binary.
    for dep in ["DEP_HIGHS_LIB_DIR", "DEP_ORTOOLS_LIB_DIR"] {
        if let Ok(lib_dir) = std::env::var(dep) {
            println!("cargo:rustc-link-arg=-Wl,-rpath,{lib_dir}");
        }
    }

    tonic_build::configure()
        .build_server(true)
        .build_client(false)
        .compile_protos(
            &[concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../proto/ferrox.proto"
            )],
            &[concat!(env!("CARGO_MANIFEST_DIR"), "/../../proto")],
        )?;

    Ok(())
}
