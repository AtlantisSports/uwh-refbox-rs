use sha2::{Digest, Sha256};

/// Stream the file through SHA-256 and compare (case-insensitive hex) to `expected`.
pub fn verify_sha256(path: &std::path::Path, expected: &str) -> std::io::Result<bool> {
    let mut f = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    std::io::copy(&mut f, &mut hasher)?;
    let got = hasher.finalize();
    let got_hex = got.iter().fold(String::with_capacity(64), |mut s, b| {
        use std::fmt::Write as _;
        // infallible: writing hex digits to a String never fails
        let _ = write!(s, "{b:02x}");
        s
    });
    Ok(got_hex.eq_ignore_ascii_case(expected.trim()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verifies_good_and_rejects_tampered() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("f");
        std::fs::write(&p, b"hello").unwrap();
        // sha256("hello")
        let good = "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824";
        assert!(verify_sha256(&p, good).unwrap());
        assert!(
            !verify_sha256(
                &p,
                "0000000000000000000000000000000000000000000000000000000000000000"
            )
            .unwrap()
        );
    }
}
