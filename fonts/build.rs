use std::{
    error::Error,
    fs::{read_dir, read_to_string, File},
    io::Write,
    ops::Shl,
    path::Path,
};

fn main() -> Result<(), Box<dyn Error>> {
    let src_path = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/src"));

    eprintln!("Reading {}", src_path.to_str().unwrap());

    for res in read_dir(src_path)? {
        let file = res?;
        if file.file_type()?.is_file() {
            let name = file.file_name().into_string().unwrap();
            let input;
            if name.starts_with("font_") && name.ends_with(".txt") {
                eprintln!("Reading {}", file.path().to_str().unwrap());
                input = read_to_string(file.path())?;
            } else {
                continue;
            }

            let mut out_path = Path::new(&std::env::var("OUT_DIR").unwrap()).to_path_buf();
            out_path.push(&name);
            out_path.set_extension("raw");
            let mut writer = File::create(out_path)?;

            let bits: Vec<bool> = input
                .chars()
                .filter_map(|c| match c {
                    '0' => Some(false),
                    '1' => Some(true),
                    ' ' => None,
                    '\n' => None,
                    '\r' => None,
                    _ => panic!("Bad character in file {}: {}", name, c),
                })
                .collect();
            assert_eq!(bits.len() % 8, 0);

            let bytes: Vec<u8> = bits
                .chunks(8)
                .map(|chunk| chunk.iter().fold(0u8, |acc, x| acc.shl(1) + u8::from(*x)))
                .collect();

            writer.write_all(&bytes)?;

            println!("cargo:rerun-if-changed={}", file.path().to_str().unwrap());
        }
    }
    Ok(())
}
