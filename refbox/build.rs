#[cfg(target_os = "windows")]
fn main() {
    let mut res = winres::WindowsResource::new();
    res.set_icon("resources/AppIcon.ico");
    res.compile().unwrap();
}

#[cfg(target_os = "macos")]
fn main() {
    println!("cargo:rustc-env=MACOSX_DEPLOYMENT_TARGET=12");
}

#[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
fn main() {}
