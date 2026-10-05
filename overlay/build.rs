fn main() {
    // With the `ndi` feature on Windows, load NDI's engine DLL only when NDI is first used rather
    // than when the program starts. That lets the overlay start (and install the engine if it's
    // missing) on a PC without NDI; see `src/ndi_runtime.rs`. `delayimp.lib` is the MSVC helper
    // that does the deferred loading.
    let ndi = std::env::var_os("CARGO_FEATURE_NDI").is_some();
    let msvc = std::env::var("CARGO_CFG_TARGET_ENV").is_ok_and(|env| env == "msvc");
    if ndi && msvc {
        println!("cargo:rustc-link-arg-bins=/DELAYLOAD:Processing.NDI.Lib.x64.dll");
        println!("cargo:rustc-link-arg-bins=delayimp.lib");
    }
    println!("cargo:rerun-if-changed=build.rs");
}
