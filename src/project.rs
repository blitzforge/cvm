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
        let workspace_version = workspace
            .get("package")
            .and_then(|p: &Value| p.as_table())
            .and_then(|package| package.get("version"))
            .and_then(|v: &Value| v.as_str());
        let members = workspace
            .get("members")
            .and_then(|m: &Value| m.as_array())
            .context("No members in [workspace]")?;
        let mut crates = Vec::new();
        for member in members {
            if let Some(path_str) = member.as_str() {
                let crate_toml = format!("{}/Cargo.toml", path_str);
                let crate_info = read_crate_info(&crate_toml, workspace_version)?;
                crates.push(crate_info);
            }
        }
        Ok(crates)
    } else if toml.contains_key("package") {
        // Single crate
        let crate_info = read_crate_info(root_toml, None)?;
        Ok(vec![crate_info])
    } else {
        Err(anyhow::anyhow!(
            "No [workspace] or [package] found in Cargo.toml"
        ))
    }
}

pub fn is_workspace_project() -> Result<bool> {
    let content = fs::read_to_string("Cargo.toml")
        .with_context(|| "Failed to read Cargo.toml".to_string())?;
    let toml: Table =
        toml::from_str(&content).with_context(|| "Failed to parse Cargo.toml".to_string())?;
    Ok(toml.contains_key("workspace"))
}

fn read_crate_info(path: &str, workspace_version: Option<&str>) -> Result<CrateInfo> {
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
    let version = if let Some(version) = package.get("version").and_then(|v: &Value| v.as_str()) {
        version.to_string()
    } else if package
        .get("version")
        .and_then(|v: &Value| v.as_table())
        .and_then(|v| v.get("workspace"))
        .and_then(|v: &Value| v.as_bool())
        == Some(true)
    {
        workspace_version
            .context("No version in [workspace.package] for crate using version.workspace = true")?
            .to_string()
    } else {
        return Err(anyhow::anyhow!("No version in [package]"));
    };
    Ok(CrateInfo {
        name,
        version,
        path: path.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::read_crate_info;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn create_temp_manifest(contents: &str) -> String {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("cvm-project-test-{}", nanos));
        fs::create_dir_all(&dir).unwrap();
        let manifest_path = dir.join("Cargo.toml");
        fs::write(&manifest_path, contents).unwrap();
        manifest_path.to_string_lossy().to_string()
    }

    #[test]
    fn resolves_workspace_inherited_version() {
        let manifest_path = create_temp_manifest(
            r#"
[package]
name = "foo"
version.workspace = true
"#,
        );

        let info = read_crate_info(&manifest_path, Some("0.1.0")).unwrap();
        assert_eq!(info.name, "foo");
        assert_eq!(info.version, "0.1.0");
    }

    #[test]
    fn errors_when_workspace_inherited_version_has_no_workspace_value() {
        let manifest_path = create_temp_manifest(
            r#"
[package]
name = "foo"
version.workspace = true
"#,
        );

        let err = read_crate_info(&manifest_path, None).unwrap_err();
        assert_eq!(
            err.to_string(),
            "No version in [workspace.package] for crate using version.workspace = true"
        );
    }
}
