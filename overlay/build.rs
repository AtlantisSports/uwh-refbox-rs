fn main() {
    // With the `ndi` feature on Windows, load NDI's engine DLL only when NDI is first used rather
    // than when the program starts. That lets the overlay start (and offer to install the engine
    // if it's missing) on a PC without NDI. Once the engine is found, `src/ndi_runtime.rs` loads
    // the DLL by its full path before any NDI call, so this delayed load finds that copy already
    // loaded and never fails at the first NDI call. `delayimp.lib` is the MSVC helper that does
    // the deferred loading.
    let ndi = std::env::var_os("CARGO_FEATURE_NDI").is_some();
    let msvc = std::env::var("CARGO_CFG_TARGET_ENV").is_ok_and(|env| env == "msvc");
    let windows = std::env::var("CARGO_CFG_TARGET_OS").is_ok_and(|os| os == "windows");
    if ndi && msvc {
        println!("cargo:rustc-link-arg-bins=/DELAYLOAD:Processing.NDI.Lib.x64.dll");
        println!("cargo:rustc-link-arg-bins=delayimp.lib");
    } else if ndi && windows {
        println!(
            "cargo:warning=The ndi feature on a non-MSVC Windows target links NDI's DLL at start-up (no delayed load): overlay.exe won't start on a PC without NDI, so the Install NDI button can't appear. Build with the MSVC target."
        );
    }
    println!("cargo:rerun-if-changed=build.rs");
}
