use crate::project::CrateInfo;
use inquire::{MultiSelect, Text};

pub fn select_crates(prompt: &str, remaining: &mut Vec<CrateInfo>) -> Vec<CrateInfo> {
    if remaining.is_empty() {
        return vec![];
    }
    let items: Vec<String> = remaining
        .iter()
        .map(|c| format!("{} v{} at {}", c.name, c.version, c.path))
        .collect();
    let selections: Vec<String> = MultiSelect::new(prompt, items).prompt().unwrap_or(vec![]);
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
        let s = Text::new("Enter a summary for this update:")
            .with_help_message(
                "Brief description of what is being updated, e.g., 'Fix bug in parser'",
            )
            .prompt()
            .unwrap_or_default();
        let trimmed = s.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
        println!("Summary is required. Please enter a description.");
    }
}

pub fn prompt_identifier() -> String {
    Text::new("Enter prerelease identifier (e.g., alpha, canary, rc):")
        .prompt()
        .unwrap_or("alpha".to_string())
}
