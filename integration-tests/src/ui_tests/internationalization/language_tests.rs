#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    fn ftl_path(lang: &str) -> String {
        format!(
            "{}/{}",
            env!("CARGO_MANIFEST_DIR"),
            format!("../refbox/translations/{}/refbox.ftl", lang)
        )
    }

    #[test]
    fn translation_keys_exist_across_languages() {
        let langs = ["en-US", "es", "fr"];
        let required_keys = ["last-game-next-game", "two-games", "ref-list", "next-game"];

        for lang in langs {
            let path = ftl_path(lang);
            assert!(
                Path::new(&path).exists(),
                "missing translations file for {}",
                lang
            );
            let content = fs::read_to_string(&path).expect("read ftl");
            for key in required_keys {
                assert!(
                    content.contains(&format!("{} =", key)),
                    "missing key '{}' in {}",
                    key,
                    lang
                );
            }
        }
    }
}
