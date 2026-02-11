use crate::project::CrateInfo;
use inquire::{MultiSelect, Text};
use std::process;

pub fn select_crates(prompt: &str, remaining: &mut Vec<CrateInfo>) -> Vec<CrateInfo> {
    if remaining.is_empty() {
        return vec![];
    }

    let items: Vec<String> = remaining
        .iter()
        .map(|c| format!("{} v{} at {}", c.name, c.version, c.path))
        .collect();
    let selections: Vec<String> = match MultiSelect::new(prompt, items).prompt() {
        Ok(sel) => sel,
        Err(_) => {
            println!("\nCancelled.");
            process::exit(0);
        }
    };
    let mut selected_indices = vec![];
    for (i, c) in remaining.iter().enumerate() {
        let item = format!("{} v{} at {}", c.name, c.version, c.path);
        if selections.contains(&item) {
            selected_indices.push(i);
        }
    }
    let mut selected = vec![];
    selected_indices.sort_by(|a, b| b.cmp(a));
    for i in selected_indices {
        selected.push(remaining.remove(i));
    }
    selected.reverse();
    selected
}

pub fn prompt_summary() -> String {
    loop {
        let s = match Text::new("Enter a summary for this update:")
            .with_help_message(
                "Brief description of what is being updated, e.g., 'Fix bug in parser'",
            )
            .prompt()
        {
            Ok(text) => text,
            Err(_) => {
                println!("\nCancelled.");
                process::exit(0);
            }
        };
        let trimmed = s.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
        println!("Summary is required. Please enter a description.");
    }
}

pub fn prompt_identifier() -> String {
    match Text::new("Enter prerelease identifier (e.g., alpha, canary, rc):").prompt() {
        Ok(id) => id,
        Err(_) => {
            println!("\nCancelled.");
            process::exit(0);
        }
    }
}
