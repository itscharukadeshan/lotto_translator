use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;

const DICT_FILE: &str = "dictionary.json";

#[derive(Serialize, Deserialize, Debug)]
struct Dictionary {
    map: HashMap<String, String>,
}

fn main() {
    // Load dictionary file
    let content = fs::read_to_string(DICT_FILE).expect("❌ Failed to read dictionary.json");
    let dict: Dictionary =
        serde_json::from_str(&content).expect("❌ Failed to parse dictionary.json");

    let mut cleaned: HashMap<String, String> = HashMap::new();

    for (key, value) in dict.map {
        let trimmed_key = key.trim().to_string();
        // Insert only if not already present (keeps first entry)
        cleaned.entry(trimmed_key).or_insert(value);
    }

    let cleaned_dict = Dictionary { map: cleaned };

    // Save back to JSON (pretty format)
    let json = serde_json::to_string_pretty(&cleaned_dict).unwrap();
    fs::write(DICT_FILE, json).expect("❌ Failed to write dictionary.json");

    println!("✅ dictionary.json cleaned and duplicates removed!");
}
