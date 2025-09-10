use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::{self, BufRead, Write};

const DICT_FILE: &str = "dictionary.json";
const PAREN_DICT_FILE: &str = "paren_dictionary.json";

#[derive(Serialize, Deserialize, Debug)]
struct Dictionary {
    map: HashMap<String, String>,
}
#[derive(Serialize, Deserialize)]
struct Config {
    discord_webhook: String,
}

fn load_or_prompt_webhook() -> String {
    let path = "config.json";
    let mut config: Config = if let Ok(content) = fs::read_to_string(path) {
        serde_json::from_str(&content).unwrap_or(Config {
            discord_webhook: "".to_string(),
        })
    } else {
        Config {
            discord_webhook: "".to_string(),
        }
    };

    if config.discord_webhook.trim().is_empty() {
        print!("🔹 Enter Discord webhook URL: ");
        io::stdout().flush().unwrap();
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        let webhook = input.trim().to_string();
        config.discord_webhook = webhook.clone();
        // Save back
        let _ = fs::write(path, serde_json::to_string_pretty(&config).unwrap());
        webhook
    } else {
        config.discord_webhook
    }
}

fn send_to_discord(webhook_url: &str, message: &str) -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::blocking::Client::new();
    let payload = serde_json::json!({
        "content": message
    });
    client.post(webhook_url).json(&payload).send()?;
    Ok(())
}
fn format_lottery_output(raw: &str) -> String {
    let mut formatted = String::new();
    let mut lines: Vec<&str> = raw.lines().collect();

    // Remove lines starting with Rs. or just a dash
    lines.retain(|line| {
        let l = line.trim_start();
        !l.starts_with("Rs.") && l != "-"
    });

    let mut current_entry = String::new();

    for line in lines {
        let line = line.trim();

        if line.is_empty() {
            continue; // skip empty lines for now
        }

        // Detect date/header lines (e.g., "Jayamalla, 2025-09-10(බදාදා)")
        if Regex::new(r"\d{4}-\d{2}-\d{2}").unwrap().is_match(line) {
            if !current_entry.is_empty() {
                formatted.push_str(&current_entry);
                formatted.push_str("\n\n"); // extra blank line between lottery entries
                current_entry.clear();
            }
            // Extra spacing before date header
            formatted.push_str("\n");
            formatted.push_str(&format!("📅 **{}**\n\n", line));
            continue;
        }

        // Check if line starts a new lottery entry
        if let Some(caps) = Regex::new(r"^(.+?)(\s*\d*)?[:\-]\s*(.*)$")
            .unwrap()
            .captures(line)
        {
            // Flush previous entry with blank line after it
            if !current_entry.is_empty() {
                formatted.push_str(&current_entry);
                formatted.push_str("\n\n");
                current_entry.clear();
            }

            let name = caps.get(1).map(|m| m.as_str().trim()).unwrap_or("");
            let draw_number = caps.get(2).map(|m| m.as_str().trim()).unwrap_or("");
            let mut rest = caps.get(3).map(|m| m.as_str().trim()).unwrap_or("").to_string();

            // Bold parentheses terms
            rest = Regex::new(r"\((\w+)\)")
                .unwrap()
                .replace_all(&rest, |caps: &regex::Captures| {
                    format!("(**{}**)", &caps[1])
                })
                .to_string();

            let full_name = if !draw_number.is_empty() {
                format!("{} {}", name, draw_number)
            } else {
                name.to_string()
            };

            current_entry = format!("**{}**: {}", full_name, rest);
        } else {
            // Line does not start with name, so append numbers to current entry
            if !current_entry.is_empty() {
                current_entry.push(' ');
                current_entry.push_str(line);
            } else {
                // Edge case: line without a preceding name
                formatted.push_str(line);
                formatted.push_str("\n\n");
            }
        }
    }

    // Flush last entry
    if !current_entry.is_empty() {
        formatted.push_str(&current_entry);
        formatted.push_str("\n\n");
    }

    formatted
}





impl Dictionary {
    fn load(path: &str) -> Self {
        if let Ok(content) = fs::read_to_string(path) {
            if let Ok(dict) = serde_json::from_str(&content) {
                return dict;
            }
        }
        Dictionary {
            map: HashMap::new(),
        }
    }

    fn save(&self, path: &str) {
        if let Ok(json) = serde_json::to_string_pretty(&self) {
            let _ = fs::write(path, json);
        }
    }

    fn translate(&mut self, key: &str) -> String {
        let key_trim = key.trim();
        if let Some(translated) = self.map.get(key_trim) {
            translated.clone()
        } else {
            self.map
                .insert(key_trim.to_string(), format!("<<<{}>>>", key_trim));
            key_trim.to_string()
        }
    }
    
    


}

fn main() {
    // Load dictionaries
    let mut dict = Dictionary::load(DICT_FILE);
    let mut paren_dict = Dictionary::load(PAREN_DICT_FILE);

    println!("👉 Paste your lottery results (end with a blank line):");

    // Read input line by line until blank line
    let stdin = io::stdin();
    let mut input = String::new();
    for line in stdin.lock().lines() {
        let line = line.unwrap();
        if line.trim().is_empty() {
            break; // stop on blank line
        }
        input.push_str(&line);
        input.push('\n');
    }

    // -------------------------------
    // Step 1: Replace lottery names
    // Regex captures: name + optional draw number + separator (- or :)
    let re_name = Regex::new(r"(?m)^([A-Za-z .]+?)(?:\s+\d+)?\s*([-:])").unwrap();

    let mut output = re_name
        .replace_all(&input, |caps: &regex::Captures| {
            // caps[1] = name, caps[2] = separator
            let translated = dict.translate(&caps[1]);
            // Rebuild original line with translated name
            let full_match = caps.get(0).unwrap().as_str();
            full_match.replacen(&caps[1], &translated, 1)
        })
        .to_string();

    // -------------------------------
    // Step 2: Replace "lakhs" with "ලක්ෂ"
    let re_lakhs = Regex::new(r"\blakhs\b").unwrap();
    output = re_lakhs.replace_all(&output, "ලක්ෂ").to_string();

    // -------------------------------
    // Step 3: Replace parentheses terms
    let re_paren = Regex::new(r"\((\w+)\)").unwrap();
    output = re_paren
        .replace_all(&output, |caps: &regex::Captures| {
            let translated = paren_dict.translate(&caps[1]);
            format!("({})", translated)
        })
        .to_string();

    // -------------------------------
    // Save updated dictionaries
    dict.save(DICT_FILE);
    paren_dict.save(PAREN_DICT_FILE);

    let formatted_output = format_lottery_output(&output);

    let webhook_url = load_or_prompt_webhook();

    if let Err(e) = send_to_discord(&webhook_url, &formatted_output) {
    eprintln!("❌ Failed to send to Discord: {}", e);
} else {
    println!("✅ Sent translation to Discord webhook!");
}

    println!("\n✅ Translated Output:\n");
    println!("{}", formatted_output);


    let new_names: Vec<_> = dict
        .map
        .iter()
        .filter(|(_, v)| v.starts_with("<<<"))
        .map(|(k, _)| k)
        .collect();
    let new_parens: Vec<_> = paren_dict
        .map
        .iter()
        .filter(|(_, v)| v.starts_with("<<<"))
        .map(|(k, _)| k)
        .collect();

    if !new_names.is_empty() {
        println!("⚠️ New lottery names found (add translations to dictionary.json):");
        for name in new_names {
            println!("  - {}", name);
        }
    }

    if !new_parens.is_empty() {
        println!("⚠️ New parenthetical terms found (add translations to paren_dictionary.json):");
        for p in new_parens {
            println!("  - {}", p);
        }
    }
}
