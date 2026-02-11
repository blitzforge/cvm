use anyhow::{Context, Result};
use semver::Version;
use std::collections::HashMap;
use std::fs;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use toml::{Table, Value};

use crate::project::analyze_project;
use crate::ui;

pub fn set_prerelease(enable: bool, identifier: Option<&str>) -> Result<()> {
    init_cvm_dir()?;
    let config_path = ".cvm/config.toml";
    let mut config: Table = if std::path::Path::new(config_path).exists() {
        let content = fs::read_to_string(config_path)?;
        toml::from_str(&content)?
    } else {
        Table::new()
    };
    let pre_section = config
        .entry("pre".to_string())
        .or_insert(Value::Table(Table::new()));
    let pre_table = pre_section.as_table_mut().unwrap();
    pre_table.insert("enabled".to_string(), Value::Boolean(enable));
    if enable {
        let identifier = if let Some(id) = identifier {
            id.to_string()
        } else if let Some(existing) = pre_table.get("identifier") {
            existing.as_str().unwrap_or("alpha").to_string()
        } else {
            ui::prompt_identifier()
        };
        pre_table.insert("identifier".to_string(), Value::String(identifier));

        // Store base versions for all crates when entering prerelease mode
        let all_crates = analyze_project()?;
        let mut base_versions = Table::new();
        for krate in &all_crates {
            let content = fs::read_to_string(&krate.path)
                .with_context(|| format!("Failed to read {}", krate.path))?;
            let toml: Table = toml::from_str(&content)
                .with_context(|| format!("Failed to parse {}", krate.path))?;

            if let Some(package) = toml.get("package").and_then(|p| p.as_table()) {
                if let Some(version_str) = package.get("version").and_then(|v| v.as_str()) {
                    let version = Version::parse(version_str)
                        .with_context(|| format!("Invalid version: {}", version_str))?;
                    let base = format!("{}.{}.{}", version.major, version.minor, version.patch);
                    base_versions.insert(krate.name.clone(), Value::String(base));
                }
            }
        }
        pre_table.insert("base_versions".to_string(), Value::Table(base_versions));
    } else {
        // Clear base_versions when exiting prerelease mode
        pre_table.remove("base_versions");
    }
    let content = toml::to_string(&config)?;
    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(config_path)?;
    file.write_all(content.as_bytes())?;
    file.sync_all()?;
    Ok(())
}

pub fn is_prerelease_enabled() -> bool {
    let config_path = ".cvm/config.toml";
    if std::path::Path::new(config_path).exists() {
        if let Ok(config_content) = fs::read_to_string(config_path) {
            if let Ok(config) = toml::from_str::<Table>(&config_content) {
                if let Some(pre_section) = config.get("pre") {
                    if let Some(table) = pre_section.as_table() {
                        return table
                            .get("enabled")
                            .and_then(|p| p.as_bool())
                            .unwrap_or(false);
                    }
                }
            }
        }
    }
    false
}

pub fn get_prerelease_identifier() -> Option<String> {
    let config_path = ".cvm/config.toml";
    if std::path::Path::new(config_path).exists() {
        if let Ok(config_content) = fs::read_to_string(config_path) {
            if let Ok(config) = toml::from_str::<Table>(&config_content) {
                if let Some(pre_section) = config.get("pre") {
                    if let Some(table) = pre_section.as_table() {
                        return table
                            .get("identifier")
                            .and_then(|i| i.as_str())
                            .map(|s| s.to_string());
                    }
                }
            }
        }
    }
    None
}

pub fn get_base_versions() -> HashMap<String, String> {
    let config_path = ".cvm/config.toml";
    let mut result = HashMap::new();

    if std::path::Path::new(config_path).exists() {
        if let Ok(config_content) = fs::read_to_string(config_path) {
            if let Ok(config) = toml::from_str::<Table>(&config_content) {
                if let Some(pre_section) = config.get("pre") {
                    if let Some(table) = pre_section.as_table() {
                        if let Some(base_versions) = table.get("base_versions") {
                            if let Some(versions_table) = base_versions.as_table() {
                                for (name, version) in versions_table {
                                    if let Some(v) = version.as_str() {
                                        result.insert(name.clone(), v.to_string());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    result
}

pub fn init_cvm_dir() -> Result<()> {
    let cvm_dir = std::path::Path::new(".cvm");
    if !cvm_dir.exists() {
        std::fs::create_dir(cvm_dir).with_context(|| "Failed to create .cvm directory")?;
    }
    let changes_dir = cvm_dir.join("changes");
    if !changes_dir.exists() {
        std::fs::create_dir(&changes_dir).with_context(|| "Failed to create changes directory")?;
    }
    let readme_path = cvm_dir.join("README.md");
    if !readme_path.exists() {
        let readme_content = r#"# Crate Version Manager (CVM)

Hello and welcome! This folder has been automatically generated by `cvm`, a build tool that works
with multi-crate repos, or single-crate repos to help you version and publish your code. You can
find the full documentation for it [in our repository](https://github.com/lucasaarch/cvm)"#;
        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&readme_path)?;
        file.write_all(readme_content.as_bytes())?;
        file.sync_all()?;
    }
    Ok(())
}

pub fn create_default_config() -> Result<()> {
    init_cvm_dir()?;
    let config_path = Path::new(".cvm/config.toml");

    if config_path.exists() {
        println!("⚠️  Config file already exists at .cvm/config.toml");
        return Ok(());
    }

    let default_config = r#"# CVM Configuration
"#;

    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(config_path)?;
    file.write_all(default_config.as_bytes())?;
    file.sync_all()?;

    Ok(())
}
