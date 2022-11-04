#[cfg(windows)]
fn main() {
    let mut res = winres::WindowsResource::new();
    res.set_icon("resources/AppIcon.ico");
    res.compile().unwrap();
}

#[cfg(not(windows))]
fn main() {}
