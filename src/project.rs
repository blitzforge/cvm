use anyhow::{Context, Result};
use serde::Serialize;
use std::fs;
use toml::{Table, Value};

#[derive(Debug, Clone, Serialize)]
pub struct CrateInfo {
    pub name: String,
    pub version: String,
    pub path: String,
}

pub fn analyze_project() -> Result<Vec<CrateInfo>> {
    let root_toml = "Cargo.toml";
    let content =
        fs::read_to_string(root_toml).with_context(|| format!("Failed to read {}", root_toml))?;
    let toml: Table =
        toml::from_str(&content).with_context(|| format!("Failed to parse {}", root_toml))?;

    if toml.contains_key("workspace") {
        // It's a workspace
        let workspace = toml
            .get("workspace")
            .and_then(|w: &Value| w.as_table())
            .context("Invalid [workspace] section")?;
        let members = workspace
            .get("members")
            .and_then(|m: &Value| m.as_array())
            .context("No members in [workspace]")?;
        let mut crates = Vec::new();
        for member in members {
            if let Some(path_str) = member.as_str() {
                let crate_toml = format!("{}/Cargo.toml", path_str);
                let crate_info = read_crate_info(&crate_toml)?;
                crates.push(crate_info);
            }
        }
        Ok(crates)
    } else if toml.contains_key("package") {
        // Single crate
        let crate_info = read_crate_info(root_toml)?;
        Ok(vec![crate_info])
    } else {
        Err(anyhow::anyhow!(
            "No [workspace] or [package] found in Cargo.toml"
        ))
    }
}

fn read_crate_info(path: &str) -> Result<CrateInfo> {
    let content = fs::read_to_string(path).with_context(|| format!("Failed to read {}", path))?;
    let toml: Table =
        toml::from_str(&content).with_context(|| format!("Failed to parse {}", path))?;
    let package = toml
        .get("package")
        .and_then(|p: &Value| p.as_table())
        .context("No [package] section")?;
    let name = package
        .get("name")
        .and_then(|n: &Value| n.as_str())
        .context("No name in [package]")?
        .to_string();
    let version = package
        .get("version")
        .and_then(|v: &Value| v.as_str())
        .context("No version in [package]")?
        .to_string();
    Ok(CrateInfo {
        name,
        version,
        path: path.to_string(),
    })
}
