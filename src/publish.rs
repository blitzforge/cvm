use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Serialize)]
pub struct PublishedCrate {
    pub name: String,
    pub version: String,
}

#[derive(Debug, Default)]
pub struct PublishOptions {
    pub dry_run: bool,
    pub token: Option<String>,
    pub allow_dirty: bool,
}

#[derive(Debug, Deserialize)]
struct CargoMetadata {
    packages: Vec<Package>,
}

#[derive(Debug, Deserialize)]
struct Package {
    name: String,
    version: String,
    manifest_path: PathBuf,
    publish: Option<Vec<String>>,
}

/// Publish crates to crates.io
pub fn publish_crates(opts: &PublishOptions) -> Result<Vec<PublishedCrate>> {
    println!("🚀 Starting publish process...\n");

    // Get workspace metadata
    let metadata = get_cargo_metadata()?;
    let packages = metadata.packages;

    if packages.is_empty() {
        println!("❌ No packages found to publish");
        return Ok(vec![]);
    }

    let mut published = Vec::new();

    for package in &packages {
        // Skip if publish = false
        if let Some(ref registries) = package.publish {
            if registries.is_empty() {
                println!("⏭️  Skipping {} (publish = false)", package.name);
                continue;
            }
        }

        println!("📦 Processing: {} v{}", package.name, package.version);

        // Publish to crates.io
        if !opts.dry_run {
            publish_to_crates_io(&package.manifest_path, opts)?;
            println!(
                "✅ Published {} v{} to crates.io",
                package.name, package.version
            );
        } else {
            println!(
                "🔍 Would publish {} v{} to crates.io",
                package.name, package.version
            );
        }

        published.push(PublishedCrate {
            name: package.name.clone(),
            version: package.version.clone(),
        });

        println!();
    }

    // Print summary
    if !published.is_empty() {
        println!("✨ Summary:");
        for p in &published {
            println!("  • {} v{}", p.name, p.version);
        }
    } else {
        println!("ℹ️  No crates were published");
    }

    Ok(published)
}

fn get_cargo_metadata() -> Result<CargoMetadata> {
    let output = Command::new("cargo")
        .args(["metadata", "--no-deps", "--format-version", "1"])
        .output()
        .context("Failed to run cargo metadata")?;

    if !output.status.success() {
        anyhow::bail!(
            "cargo metadata failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let metadata: CargoMetadata =
        serde_json::from_slice(&output.stdout).context("Failed to parse cargo metadata")?;

    Ok(metadata)
}

fn publish_to_crates_io(manifest_path: &Path, opts: &PublishOptions) -> Result<()> {
    let crate_dir = manifest_path
        .parent()
        .context("Failed to get crate directory")?;

    // Get token from options or environment
    let env_token = std::env::var("CARGO_REGISTRY_TOKEN").ok();
    let token = opts.token.as_deref().or(env_token.as_deref());

    if token.is_none() {
        anyhow::bail!(
            "CARGO_REGISTRY_TOKEN not set. Please set it in your environment or pass --token"
        );
    }

    let mut args = vec!["publish"];

    if let Some(token) = token {
        args.push("--token");
        args.push(token);
    }

    if opts.allow_dirty {
        args.push("--allow-dirty");
    }

    let status = Command::new("cargo")
        .args(&args)
        .current_dir(crate_dir)
        .status()
        .context("Failed to run cargo publish")?;

    if !status.success() {
        anyhow::bail!("cargo publish failed");
    }

    Ok(())
}
