use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Serialize)]
pub struct PublishedCrate {
    pub name: String,
    pub version: String,
    pub tag: String,
}

#[derive(Debug)]
pub struct PublishOptions {
    pub dry_run: bool,
    pub create_tags: bool,
    pub create_release: bool,
    pub token: Option<String>,
    pub allow_dirty: bool,
}

impl Default for PublishOptions {
    fn default() -> Self {
        Self {
            dry_run: false,
            create_tags: true,
            create_release: true,
            token: None,
            allow_dirty: false,
        }
    }
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

/// Publish crates to crates.io with optional git tagging and GitHub releases
pub fn publish_crates(opts: &PublishOptions) -> Result<Vec<PublishedCrate>> {
    println!("🚀 Starting publish process...\n");

    // Get workspace metadata
    let metadata = get_cargo_metadata()?;
    let packages = metadata.packages;

    if packages.is_empty() {
        println!("❌ No packages found to publish");
        return Ok(vec![]);
    }

    let is_workspace = packages.len() > 1;
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

        // Determine tag name
        let tag = if is_workspace {
            format!("{}-v{}", package.name, package.version)
        } else {
            format!("v{}", package.version)
        };

        // Check if tag already exists
        if opts.create_tags && tag_exists(&tag)? {
            println!("⚠️  Tag {} already exists, skipping...\n", tag);
            continue;
        }

        // Create and push git tag
        if opts.create_tags && !opts.dry_run {
            create_and_push_tag(&tag, &package.name, &package.version)?;
            println!("✅ Created and pushed tag: {}", tag);
        } else if opts.create_tags && opts.dry_run {
            println!("🔍 Would create tag: {}", tag);
        }

        // Create GitHub Release
        if opts.create_release && !opts.dry_run {
            if let Err(e) = create_github_release(&tag, &package.name, &package.version) {
                println!("⚠️  Failed to create GitHub release: {}", e);
            } else {
                println!("✅ Created GitHub release: {}", tag);
            }
        } else if opts.create_release && opts.dry_run {
            println!("🔍 Would create GitHub release: {}", tag);
        }

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
            tag,
        });

        println!();
    }

    // Print summary
    if !published.is_empty() {
        println!("✨ Summary:");
        for p in &published {
            println!("  • {} v{} ({})", p.name, p.version, p.tag);
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

fn tag_exists(tag: &str) -> Result<bool> {
    let output = Command::new("git")
        .args(["rev-parse", tag])
        .output()
        .context("Failed to check if git tag exists")?;

    Ok(output.status.success())
}

fn create_and_push_tag(tag: &str, name: &str, version: &str) -> Result<()> {
    // Create tag
    let message = format!("Release {} v{}", name, version);
    let status = Command::new("git")
        .args(["tag", "-a", tag, "-m", &message])
        .status()
        .context("Failed to create git tag")?;

    if !status.success() {
        anyhow::bail!("Failed to create git tag");
    }

    // Push tag
    let status = Command::new("git")
        .args(["push", "origin", tag])
        .status()
        .context("Failed to push git tag")?;

    if !status.success() {
        anyhow::bail!("Failed to push git tag");
    }

    Ok(())
}

fn create_github_release(tag: &str, name: &str, version: &str) -> Result<()> {
    // Check if gh CLI is available
    let gh_check = Command::new("gh").arg("--version").output();

    if gh_check.is_err() {
        anyhow::bail!("gh CLI not found (install from https://cli.github.com)");
    }

    let title = format!("{} v{}", name, version);
    let notes = format!("Release of {} version {}", name, version);

    let status = Command::new("gh")
        .args([
            "release",
            "create",
            tag,
            "--title",
            &title,
            "--notes",
            &notes,
            "--verify-tag",
        ])
        .status()
        .context("Failed to create GitHub release")?;

    if !status.success() {
        anyhow::bail!("gh release create failed");
    }

    Ok(())
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
