use fluent_syntax::parser::parse;
use std::collections::{HashMap, HashSet};
use std::fs;

fn extract_message_ids(content: &str) -> HashSet<String> {
    let mut ids = HashSet::new();
    if let Ok(ast) = parse(content) {
        for entry in ast.body {
            if let fluent_syntax::ast::Entry::Message(message) = entry {
                ids.insert(message.id.name.to_string());
            }
        }
    }
    ids
}

fn main() {
    #[cfg(target_os = "macos")]
    println!("cargo:rustc-env=MACOSX_DEPLOYMENT_TARGET=12");

    #[cfg(target_os = "windows")]
    {
        let mut res = winres::WindowsResource::new();
        res.set_icon("resources/AppIcon.ico");
        res.compile().unwrap();
    }

    // Path to your localization files
    let l10n_dir = "translations";
    let mut file_message_map: HashMap<String, HashSet<String>> = HashMap::new();

    // Load all .ftl files from subdirectories and extract message IDs
    for entry in fs::read_dir(l10n_dir).expect("Could not read directory") {
        let entry = entry.expect("Could not read directory entry");
        let path = entry.path();
        if path.is_dir() {
            for file_entry in fs::read_dir(&path).expect("Could not read subdirectory") {
                let file_entry = file_entry.expect("Could not read file entry");
                let file_path = file_entry.path();
                if file_path.extension().and_then(|ext| ext.to_str()) == Some("ftl") {
                    println!("cargo:rerun-if-changed={}", file_path.display());
                    let content = fs::read_to_string(&file_path).expect("Could not read file");
                    let message_ids = extract_message_ids(&content);
                    file_message_map.insert(file_path.display().to_string(), message_ids);
                }
            }
        }
    }

    // Compare sets of message IDs
    let all_keys: HashSet<_> = file_message_map
        .values()
        .flat_map(|set| set.iter().cloned())
        .collect();
    let mut missing_keys = HashMap::new();

    for (file, ids) in &file_message_map {
        let missing_in_file: Vec<_> = all_keys.difference(ids).cloned().collect();
        if !missing_in_file.is_empty() {
            missing_keys.insert(file, missing_in_file);
        }
    }

    assert_eq!(
        missing_keys,
        HashMap::new(),
        "Some translations are missing keys"
    );
}
