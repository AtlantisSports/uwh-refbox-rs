#[cfg(feature = "prost")]
use std::io::Result;

#[cfg(feature = "prost")]
fn main() -> Result<()> {
    let protos = ["src/snapshot.proto"];

    prost_build::compile_protos(&protos, &["src/"])?;

    Ok(())
}

#[cfg(not(feature = "prost"))]
fn main() {}
