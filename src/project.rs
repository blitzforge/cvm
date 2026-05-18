use anyhow::{Context, Result};
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
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

fn workspace_package_version_from_manifest(manifest_path: &Path) -> Result<Option<String>> {
    let content = fs::read_to_string(manifest_path)
        .with_context(|| format!("Failed to read {}", manifest_path.display()))?;
    let toml: Table = toml::from_str(&content)
        .with_context(|| format!("Failed to parse {}", manifest_path.display()))?;

    let Some(workspace) = toml.get("workspace").and_then(|w| w.as_table()) else {
        return Ok(None);
    };

    let version = workspace
        .get("package")
        .and_then(|p| p.as_table())
        .and_then(|package| package.get("version"))
        .and_then(|v| v.as_str())
        .map(str::to_string);

    Ok(version)
}

/// Returns `[workspace.package].version` when the root manifest defines a workspace package table.
pub fn workspace_package_version() -> Result<Option<String>> {
    workspace_package_version_from_manifest(Path::new("Cargo.toml"))
}

/// Walks upward from a crate manifest until a workspace root `Cargo.toml` is found.
pub fn find_workspace_root(crate_manifest_path: &Path) -> Option<PathBuf> {
    let mut dir = crate_manifest_path.parent()?;
    loop {
        let manifest = dir.join("Cargo.toml");
        if manifest.is_file() {
            if let Ok(content) = fs::read_to_string(&manifest) {
                if let Ok(toml) = toml::from_str::<Table>(&content) {
                    if toml.contains_key("workspace") {
                        return Some(dir.to_path_buf());
                    }
                }
            }
        }
        dir = dir.parent()?;
    }
}

/// Single release version for CI tags and summaries.
///
/// Prefers `[workspace.package].version` in workspaces; otherwise requires every crate
/// in the project to share the same version string.
pub fn release_version() -> Result<String> {
    if let Some(version) = workspace_package_version()? {
        return Ok(version);
    }

    let crates = analyze_project()?;
    if crates.is_empty() {
        return Err(anyhow::anyhow!("No crates found in project"));
    }

    let version = crates[0].version.clone();
    if crates.iter().all(|c| c.version == version) {
        return Ok(version);
    }

    Err(anyhow::anyhow!(
        "Crates have different versions; use `cvm info` for per-crate JSON or set [workspace.package].version"
    ))
}

/// True when the crate manifest inherits its version from `[workspace.package]`.
pub fn uses_workspace_version(crate_manifest_path: &str) -> Result<bool> {
    let content = fs::read_to_string(crate_manifest_path)
        .with_context(|| format!("Failed to read {}", crate_manifest_path))?;
    let toml: Table = toml::from_str(&content)
        .with_context(|| format!("Failed to parse {}", crate_manifest_path))?;
    let package = toml
        .get("package")
        .and_then(|p| p.as_table())
        .context("No [package] section")?;
    let version_value = package.get("version");
    Ok(version_value
        .and_then(|v: &Value| v.as_table())
        .and_then(|v| v.get("workspace"))
        .and_then(|v: &Value| v.as_bool())
        == Some(true))
}

/// Path to edit and the resolved semver string for version bumps.
///
/// Workspace-inherited crates bump the root `Cargo.toml` `[workspace.package].version`.
pub fn resolve_version_for_crate(crate_manifest_path: &str) -> Result<(String, String)> {
    let manifest_path = Path::new(crate_manifest_path);
    let workspace_version = find_workspace_root(manifest_path)
        .map(|root| workspace_package_version_from_manifest(&root.join("Cargo.toml")))
        .transpose()?
        .flatten();
    let info = read_crate_info(crate_manifest_path, workspace_version.as_deref())?;
    let edit_path = if uses_workspace_version(crate_manifest_path)? {
        let root = find_workspace_root(manifest_path)
            .context("workspace root not found for crate using version.workspace = true")?;
        root.join("Cargo.toml").to_string_lossy().into_owned()
    } else {
        crate_manifest_path.to_string()
    };
    Ok((edit_path, info.version))
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
    let version_value = package.get("version");
    let version = if let Some(version) = version_value.and_then(|v: &Value| v.as_str()) {
        version.to_string()
    } else if version_value
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
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn create_temp_manifest(contents: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let counter = TEST_COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "cvm-project-test-{}-{}-{}",
            std::process::id(),
            nanos,
            counter
        ));
        fs::create_dir_all(&dir).unwrap();
        let manifest_path = dir.join("Cargo.toml");
        fs::write(&manifest_path, contents).unwrap();
        manifest_path
    }

    fn cleanup_temp_manifest(path: &Path) {
        if let Err(err) = fs::remove_file(path) {
            assert_eq!(
                err.kind(),
                std::io::ErrorKind::NotFound,
                "failed to remove test manifest {}: {}",
                path.display(),
                err
            );
        }
        if let Some(parent) = path.parent() {
            if let Err(err) = fs::remove_dir_all(parent) {
                assert_eq!(
                    err.kind(),
                    std::io::ErrorKind::NotFound,
                    "failed to remove test temp dir {}: {}",
                    parent.display(),
                    err
                );
            }
        }
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

        let info =
            read_crate_info(manifest_path.to_string_lossy().as_ref(), Some("0.1.0")).unwrap();
        assert_eq!(info.name, "foo");
        assert_eq!(info.version, "0.1.0");
        cleanup_temp_manifest(&manifest_path);
    }

    #[test]
    fn workspace_package_version_reads_workspace_package_table() {
        let dir = std::env::temp_dir().join(format!(
            "cvm-workspace-version-test-{}",
            TEST_COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("Cargo.toml"),
            r#"
[workspace]
members = ["crates/foo"]

[workspace.package]
version = "1.2.3"
"#,
        )
        .unwrap();

        let original = std::env::current_dir().unwrap();
        std::env::set_current_dir(&dir).unwrap();
        let version = super::workspace_package_version().unwrap();
        std::env::set_current_dir(original).unwrap();
        fs::remove_dir_all(&dir).unwrap();

        assert_eq!(version.as_deref(), Some("1.2.3"));
    }

    #[test]
    fn release_version_uses_workspace_package_version() {
        let dir = std::env::temp_dir().join(format!(
            "cvm-release-version-test-{}",
            TEST_COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("Cargo.toml"),
            r#"
[workspace]
members = ["crates/foo", "crates/bar"]

[workspace.package]
version = "2.0.0"
"#,
        )
        .unwrap();
        fs::create_dir_all(dir.join("crates/foo")).unwrap();
        fs::create_dir_all(dir.join("crates/bar")).unwrap();
        fs::write(
            dir.join("crates/foo/Cargo.toml"),
            r#"
[package]
name = "foo"
version.workspace = true
"#,
        )
        .unwrap();
        fs::write(
            dir.join("crates/bar/Cargo.toml"),
            r#"
[package]
name = "bar"
version.workspace = true
"#,
        )
        .unwrap();

        let original = std::env::current_dir().unwrap();
        std::env::set_current_dir(&dir).unwrap();
        let version = super::release_version().unwrap();
        std::env::set_current_dir(original).unwrap();
        fs::remove_dir_all(&dir).unwrap();

        assert_eq!(version, "2.0.0");
    }

    #[test]
    fn resolve_version_for_crate_points_at_workspace_root() {
        let dir = std::env::temp_dir().join(format!(
            "cvm-resolve-version-test-{}",
            TEST_COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(dir.join("crates/lemon")).unwrap();
        fs::write(
            dir.join("Cargo.toml"),
            r#"
[workspace]
members = ["crates/lemon"]

[workspace.package]
version = "0.1.0"

[workspace.dependencies]
lemon = { path = "crates/lemon", version = "0.1.0" }
"#,
        )
        .unwrap();
        fs::write(
            dir.join("crates/lemon/Cargo.toml"),
            r#"
[package]
name = "lemon"
version.workspace = true
"#,
        )
        .unwrap();

        let crate_manifest = dir.join("crates/lemon/Cargo.toml");
        let (edit_path, version) =
            super::resolve_version_for_crate(crate_manifest.to_str().unwrap()).unwrap();
        fs::remove_dir_all(&dir).unwrap();

        assert_eq!(edit_path, dir.join("Cargo.toml").to_string_lossy());
        assert_eq!(version, "0.1.0");
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

        let err = read_crate_info(manifest_path.to_string_lossy().as_ref(), None).unwrap_err();
        assert_eq!(
            err.to_string(),
            "No version in [workspace.package] for crate using version.workspace = true"
        );
        cleanup_temp_manifest(&manifest_path);
    }
}
