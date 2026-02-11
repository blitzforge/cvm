use anyhow::{Context, Result};
use semver::Version;
use std::collections::HashMap;
use std::fs;
use std::fs::OpenOptions;
use std::io::Write;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};
use toml::{Table, Value};

use crate::config;
use crate::project::{analyze_project, CrateInfo};

pub fn save_pending(
    summary: &str,
    major: &[CrateInfo],
    minor: &[CrateInfo],
    patch: &[CrateInfo],
) -> Result<()> {
    config::init_cvm_dir()?;
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let filename = format!(".cvm/changes/{}.toml", timestamp);
    let mut config = Table::new();
    let mut update_table = Table::new();
    update_table.insert("summary".to_string(), Value::String(summary.to_string()));
    update_table.insert(
        "major".to_string(),
        Value::Array(
            major
                .iter()
                .map(|c| Value::String(c.name.clone()))
                .collect(),
        ),
    );
    update_table.insert(
        "minor".to_string(),
        Value::Array(
            minor
                .iter()
                .map(|c| Value::String(c.name.clone()))
                .collect(),
        ),
    );
    update_table.insert(
        "patch".to_string(),
        Value::Array(
            patch
                .iter()
                .map(|c| Value::String(c.name.clone()))
                .collect(),
        ),
    );
    let is_prerelease = config::is_prerelease_enabled();
    update_table.insert("pre".to_string(), Value::Boolean(is_prerelease));
    config.insert("update".to_string(), Value::Table(update_table));
    let config_content = toml::to_string(&config).with_context(|| "Failed to serialize config")?;
    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&filename)?;
    file.write_all(config_content.as_bytes())?;
    file.sync_all()?;
    Ok(())
}

/// Replace version in TOML content while preserving formatting
fn replace_version_in_toml(content: &str, old_version: &str, new_version: &str) -> Result<String> {
    // Find and replace version = "old" with version = "new"
    // This preserves all formatting, comments, and order
    let version_patterns = [
        format!(r#"version = "{}""#, old_version),
        format!(r#"version="{}""#, old_version),
        format!(r#"version = '{}'"#, old_version),
        format!(r#"version='{}'"#, old_version),
    ];

    let replacements = [
        format!(r#"version = "{}""#, new_version),
        format!(r#"version="{}""#, new_version),
        format!(r#"version = '{}'"#, new_version),
        format!(r#"version='{}'"#, new_version),
    ];

    for (pattern, replacement) in version_patterns.iter().zip(replacements.iter()) {
        if content.contains(pattern) {
            // Only replace the first occurrence (in [package] section)
            let new_content = content.replacen(pattern, replacement, 1);
            return Ok(new_content);
        }
    }

    Err(anyhow::anyhow!(
        "Could not find version = \"{}\" in Cargo.toml",
        old_version
    ))
}

/// Apply a single bump to a crate, respecting prerelease mode
fn apply_bump(
    crate_path: &str,
    crate_name: &str,
    bump_type: &str,
    is_prerelease: bool,
    prerelease_id: Option<&str>,
) -> Result<String> {
    let content =
        fs::read_to_string(crate_path).with_context(|| format!("Failed to read {}", crate_path))?;

    // Parse TOML to read current version
    let toml: Table =
        toml::from_str(&content).with_context(|| format!("Failed to parse {}", crate_path))?;

    let package = toml
        .get("package")
        .and_then(|p| p.as_table())
        .context("No [package] section")?;
    let current_version_str = package
        .get("version")
        .and_then(|v: &Value| v.as_str())
        .context("No version in [package]")?;

    let current_version = Version::parse(current_version_str)
        .with_context(|| format!("Invalid version: {}", current_version_str))?;

    let new_version_str = if is_prerelease {
        let tag = prerelease_id.context("Prerelease mode but no identifier")?;

        // Get the stored base version from when we entered prerelease mode
        let base_versions = config::get_base_versions();
        let stored_base = base_versions.get(crate_name);

        // Current version without prerelease
        let current_base = Version::new(
            current_version.major,
            current_version.minor,
            current_version.patch,
        );

        // Determine what the base version should be after this bump
        let (target_base, should_reset_number) = if bump_type == "patch" {
            // Patch in prerelease mode: always keep current base, increment prerelease number
            (current_base.clone(), false)
        } else if let Some(base_str) = stored_base {
            // Minor/Major: calculate new base from stored base
            let stored = Version::parse(base_str)
                .with_context(|| format!("Invalid stored base version: {}", base_str))?;

            let new_base = match bump_type {
                "major" => Version::new(stored.major + 1, 0, 0),
                "minor" => Version::new(stored.major, stored.minor + 1, 0),
                _ => current_base.clone(),
            };

            // If the new base is different from current, reset prerelease number
            (new_base.clone(), new_base != current_base)
        } else {
            // No stored base (shouldn't happen), calculate from current
            let new_base = match bump_type {
                "major" => Version::new(current_base.major + 1, 0, 0),
                "minor" => Version::new(current_base.major, current_base.minor + 1, 0),
                _ => current_base.clone(),
            };
            (new_base.clone(), new_base != current_base)
        };

        // Determine the prerelease number
        let prerelease_number = if should_reset_number {
            // Base changed, start from 0
            0
        } else {
            // Base stayed the same, increment the number
            if !current_version.pre.is_empty() && current_version_str.contains(&format!("-{}", tag))
            {
                // Parse current prerelease number
                let pre_str = current_version.pre.as_str();
                if let Some(num_part) = pre_str.strip_prefix(&format!("{}.", tag)) {
                    if let Some(dot_pos) = num_part.find('.') {
                        num_part[..dot_pos].parse::<u64>().unwrap_or(0) + 1
                    } else {
                        num_part.parse::<u64>().unwrap_or(0) + 1
                    }
                } else {
                    0
                }
            } else {
                0
            }
        };

        format!("{}-{}.{}", target_base, tag, prerelease_number)
    } else {
        // Regular bump (no prerelease)
        match bump_type {
            "major" => format!("{}.0.0", current_version.major + 1),
            "minor" => format!("{}.{}.0", current_version.major, current_version.minor + 1),
            "patch" => format!(
                "{}.{}.{}",
                current_version.major,
                current_version.minor,
                current_version.patch + 1
            ),
            _ => {
                return Err(anyhow::anyhow!(
                    "Invalid bump type: {}. Use major, minor or patch.",
                    bump_type
                ))
            }
        }
    };

    // Preserve formatting by doing in-place replacement of version line
    let new_content = replace_version_in_toml(&content, current_version_str, &new_version_str)?;

    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(crate_path)?;
    file.write_all(new_content.as_bytes())?;
    file.sync_all()?;

    Ok(new_version_str)
}

/// Create a git tag for a crate version
fn create_git_tag(crate_name: &str, version: &str) -> Result<()> {
    let tag_name = format!("v{}", version);

    // Check if tag already exists
    let check_tag = Command::new("git")
        .args(["tag", "-l", &tag_name])
        .output()
        .context("Failed to check existing tags")?;

    if !check_tag.stdout.is_empty() {
        println!("  Tag {} already exists, skipping", tag_name);
        return Ok(());
    }

    // Create the tag
    let tag_message = format!("Release {} v{}", crate_name, version);
    let result = Command::new("git")
        .args(["tag", "-a", &tag_name, "-m", &tag_message])
        .output()
        .context("Failed to create git tag")?;

    if result.status.success() {
        println!("  Created tag: {}", tag_name);

        // Push the tag
        let push_result = Command::new("git")
            .args(["push", "origin", &tag_name])
            .output()
            .context("Failed to push git tag")?;

        if push_result.status.success() {
            println!("  Pushed tag to origin");
        } else {
            eprintln!("  Warning: Failed to push tag to origin");
        }
    } else {
        eprintln!("  Warning: Failed to create tag {}", tag_name);
    }

    Ok(())
}

/// Check for pending changes and print summary
pub fn check_pending_changes() -> Result<()> {
    let changes_dir = std::path::Path::new(".cvm/changes");
    if !changes_dir.exists() {
        println!("No pending changes.");
        return Ok(());
    }

    let mut entries: Vec<_> = std::fs::read_dir(changes_dir)
        .with_context(|| "Failed to read changes directory")?
        .collect::<Result<Vec<_>, _>>()?;
    entries.sort_by_key(|e| e.file_name());

    let toml_entries: Vec<_> = entries
        .iter()
        .filter(|e| e.path().extension() == Some(std::ffi::OsStr::new("toml")))
        .collect();

    if toml_entries.is_empty() {
        println!("No pending changes.");
        return Ok(());
    }

    println!("Found {} pending change(s):\n", toml_entries.len());

    for (idx, entry) in toml_entries.iter().enumerate() {
        let path = entry.path();
        let content = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read {}", path.display()))?;
        let config: Table = toml::from_str(&content)
            .with_context(|| format!("Failed to parse {}", path.display()))?;

        let update = config
            .get("update")
            .and_then(|u| u.as_table())
            .context("No update section in config")?;

        let summary = update
            .get("summary")
            .and_then(|s| s.as_str())
            .unwrap_or("(no summary)");

        let is_pre = update.get("pre").and_then(|p| p.as_bool()).unwrap_or(false);

        println!("{}. {}", idx + 1, summary);
        if is_pre {
            println!("   Mode: prerelease");
        }

        let empty_array = vec![];
        let major_crates = update
            .get("major")
            .and_then(|m| m.as_array())
            .unwrap_or(&empty_array);
        let minor_crates = update
            .get("minor")
            .and_then(|m| m.as_array())
            .unwrap_or(&empty_array);
        let patch_crates = update
            .get("patch")
            .and_then(|p| p.as_array())
            .unwrap_or(&empty_array);

        if !major_crates.is_empty() {
            println!("   Major: {:?}", major_crates);
        }
        if !minor_crates.is_empty() {
            println!("   Minor: {:?}", minor_crates);
        }
        if !patch_crates.is_empty() {
            println!("   Patch: {:?}", patch_crates);
        }
        println!();
    }

    // Exit with code 1 to signal there are pending changes (useful for CI)
    std::process::exit(1);
}

pub fn load_and_apply_pending(dry_run: bool, create_tag: bool) -> Result<()> {
    let changes_dir = std::path::Path::new(".cvm/changes");
    if !changes_dir.exists() {
        println!("No pending updates found.");
        return Ok(());
    }

    let mut entries: Vec<_> = std::fs::read_dir(changes_dir)
        .with_context(|| "Failed to read changes directory")?
        .collect::<Result<Vec<_>, _>>()?;
    entries.sort_by_key(|e| e.file_name());

    if entries.is_empty() {
        println!("No pending updates found.");
        return Ok(());
    }

    if dry_run {
        println!("DRY RUN - No changes will be applied\n");
    }

    let all_crates = analyze_project()?;
    let crate_map: HashMap<String, &CrateInfo> =
        all_crates.iter().map(|c| (c.name.clone(), c)).collect();

    let mut updated_crates: HashMap<String, String> = HashMap::new();

    for entry in entries {
        let path = entry.path();
        if path.extension() == Some(std::ffi::OsStr::new("toml")) {
            let content = fs::read_to_string(&path)
                .with_context(|| format!("Failed to read {}", path.display()))?;
            let config: Table = toml::from_str(&content)
                .with_context(|| format!("Failed to parse {}", path.display()))?;

            let update = config
                .get("update")
                .and_then(|u| u.as_table())
                .context("No update section in config")?;

            let summary = update
                .get("summary")
                .and_then(|s| s.as_str())
                .context("No summary in update")?;

            let is_pre = update.get("pre").and_then(|p| p.as_bool()).unwrap_or(false);

            let prerelease_id = if is_pre {
                config::get_prerelease_identifier()
            } else {
                None
            };

            if dry_run {
                println!("[DRY RUN] Would apply: {}", summary);
            } else {
                println!("Applying update: {}", summary);
            }
            if is_pre {
                if let Some(id) = &prerelease_id {
                    println!("  (prerelease mode: {})", id);
                }
            }

            let empty_array = vec![];
            let major_crates = update
                .get("major")
                .and_then(|m| m.as_array())
                .unwrap_or(&empty_array);
            let minor_crates = update
                .get("minor")
                .and_then(|m| m.as_array())
                .unwrap_or(&empty_array);
            let patch_crates = update
                .get("patch")
                .and_then(|p| p.as_array())
                .unwrap_or(&empty_array);

            if !dry_run {
                for crate_name in major_crates {
                    if let Some(name_str) = crate_name.as_str() {
                        if let Some(c) = crate_map.get(name_str) {
                            let new_version = apply_bump(
                                &c.path,
                                &c.name,
                                "major",
                                is_pre,
                                prerelease_id.as_deref(),
                            )?;
                            println!("  {} major → {}", c.name, new_version);
                            updated_crates.insert(c.name.clone(), new_version);
                        }
                    }
                }

                for crate_name in minor_crates {
                    if let Some(name_str) = crate_name.as_str() {
                        if let Some(c) = crate_map.get(name_str) {
                            let new_version = apply_bump(
                                &c.path,
                                &c.name,
                                "minor",
                                is_pre,
                                prerelease_id.as_deref(),
                            )?;
                            println!("  {} minor → {}", c.name, new_version);
                            updated_crates.insert(c.name.clone(), new_version);
                        }
                    }
                }

                for crate_name in patch_crates {
                    if let Some(name_str) = crate_name.as_str() {
                        if let Some(c) = crate_map.get(name_str) {
                            let new_version = apply_bump(
                                &c.path,
                                &c.name,
                                "patch",
                                is_pre,
                                prerelease_id.as_deref(),
                            )?;
                            println!("  {} patch → {}", c.name, new_version);
                            updated_crates.insert(c.name.clone(), new_version);
                        }
                    }
                }

                std::fs::remove_file(&path)
                    .with_context(|| format!("Failed to remove {}", path.display()))?;
            } else {
                // Dry run: just show what would happen
                for crate_name in major_crates {
                    if let Some(name_str) = crate_name.as_str() {
                        if crate_map.contains_key(name_str) {
                            println!("  {} major", name_str);
                        }
                    }
                }
                for crate_name in minor_crates {
                    if let Some(name_str) = crate_name.as_str() {
                        if crate_map.contains_key(name_str) {
                            println!("  {} minor", name_str);
                        }
                    }
                }
                for crate_name in patch_crates {
                    if let Some(name_str) = crate_name.as_str() {
                        if crate_map.contains_key(name_str) {
                            println!("  {} patch", name_str);
                        }
                    }
                }
            }
        }
    }

    if dry_run {
        println!("\n[DRY RUN] No changes were made.");
    } else {
        println!("\nAll updates applied successfully!");

        // Create git tags if requested
        if create_tag && !updated_crates.is_empty() {
            println!("\nCreating git tags...");
            for (crate_name, version) in updated_crates {
                create_git_tag(&crate_name, &version)?;
            }
        }
    }
    Ok(())
}
