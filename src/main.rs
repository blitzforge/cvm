mod changes;
mod config;
mod project;
mod publish;
mod ui;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "cvm")]
#[command(about = "Crate Version Manager - Manages versions of Cargo packages")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize CVM in the current project
    Setup,
    /// Applies versioning changes
    Apply {
        /// Show what would be applied without making changes
        #[arg(long)]
        dry_run: bool,
        /// Create git tags for each updated crate (overrides config)
        #[arg(long, conflicts_with = "no_git_tags")]
        git_tags: bool,
        /// Do not create git tags (overrides config)
        #[arg(long, conflicts_with = "git_tags")]
        no_git_tags: bool,
    },
    /// Check for pending changes
    Status,
    /// Manage prerelease mode
    Pre {
        #[command(subcommand)]
        action: PreAction,
    },
    /// Publish crates to crates.io with git tags and GitHub releases
    Publish {
        /// Show what would be published without making changes
        #[arg(long)]
        dry_run: bool,
        /// Do not create git tags
        #[arg(long)]
        no_tag: bool,
        /// Do not create GitHub releases
        #[arg(long)]
        no_release: bool,
        /// Cargo registry token (overrides CARGO_REGISTRY_TOKEN env var)
        #[arg(long)]
        token: Option<String>,
        /// Allow publishing with uncommitted changes
        #[arg(long)]
        allow_dirty: bool,
    },
}

#[derive(Subcommand)]
enum PreAction {
    /// Start prerelease mode
    Start {
        /// Optional prerelease identifier
        identifier: Option<String>,
    },
    /// Exit prerelease mode
    Exit,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Setup) => {
            config::create_default_config()?;
            println!("✅ CVM initialized successfully!");
            println!("\nConfiguration file created at: .cvm/config.toml");
            println!("Change files will be stored in: .cvm/changes/");
            println!("\nNext steps:");
            println!("  1. Run 'cvm' to create version changes interactively");
            println!("  2. Run 'cvm apply' to apply pending changes");
            println!("  3. Run 'cvm status' to check for pending changes");
            Ok(())
        }
        None => {
            // Check if CVM is initialized
            if !std::path::Path::new(".cvm/config.toml").exists() {
                eprintln!("❌ CVM is not initialized in this project.");
                eprintln!("\nRun 'cvm setup' to initialize CVM.");
                std::process::exit(1);
            }
            // Analyze the current project (workspace or single crate)
            let crates = project::analyze_project()?;
            if crates.is_empty() {
                println!("No crates found.");
                return Ok(());
            }
            let mut remaining = crates;

            // Select for major bump (red)
            let major_selected = ui::select_crates(
                "Select crates for \x1b[91mmajor\x1b[0m bump:",
                &mut remaining,
            );

            // Select for minor bump if remaining (yellow)
            let minor_selected = if !remaining.is_empty() {
                ui::select_crates(
                    "Select crates for \x1b[93mminor\x1b[0m bump:",
                    &mut remaining,
                )
            } else {
                vec![]
            };

            // Select for patch bump if still remaining (blue)
            let patch_selected = if !remaining.is_empty() {
                ui::select_crates(
                    "Select crates for \x1b[94mpatch\x1b[0m bump:",
                    &mut remaining,
                )
            } else {
                vec![]
            };

            // Check if any were selected
            if major_selected.is_empty() && minor_selected.is_empty() && patch_selected.is_empty() {
                println!("No crates selected for bumping.");
                return Ok(());
            }

            // Ask for mandatory summary
            let summary = ui::prompt_summary();

            // Save pending updates
            changes::save_pending(&summary, &major_selected, &minor_selected, &patch_selected)?;
            Ok(())
        }
        Some(Commands::Apply {
            dry_run,
            git_tags,
            no_git_tags,
        }) => {
            // Check if CVM is initialized
            if !std::path::Path::new(".cvm").exists() {
                eprintln!("❌ CVM is not initialized in this project.");
                eprintln!("\nRun 'cvm setup' to initialize CVM.");
                std::process::exit(1);
            }
            // Determine whether to create tags: CLI flags override config
            let create_tags = if git_tags {
                true
            } else if no_git_tags {
                false
            } else {
                config::should_create_git_tags()
            };

            changes::load_and_apply_pending(dry_run, create_tags)?;
            Ok(())
        }
        Some(Commands::Status) => {
            // Check if CVM is initialized
            if !std::path::Path::new(".cvm").exists() {
                eprintln!("❌ CVM is not initialized in this project.");
                eprintln!("\nRun 'cvm setup' to initialize CVM.");
                std::process::exit(1);
            }
            changes::check_pending_changes()?;
            Ok(())
        }
        Some(Commands::Pre { action }) => {
            match action {
                PreAction::Start { identifier } => {
                    config::set_prerelease(true, identifier.as_deref())?;
                    println!("Prerelease mode started.");
                }
                PreAction::Exit => {
                    config::set_prerelease(false, None)?;
                    println!("Prerelease mode exited.");
                }
            }
            Ok(())
        }
        Some(Commands::Publish {
            dry_run,
            no_tag,
            no_release,
            token,
            allow_dirty,
        }) => {
            // Check if CVM is initialized
            if !std::path::Path::new(".cvm").exists() {
                eprintln!("❌ CVM is not initialized in this project.");
                eprintln!("\nRun 'cvm setup' to initialize CVM.");
                std::process::exit(1);
            }

            let opts = publish::PublishOptions {
                dry_run,
                create_tags: !no_tag,
                create_release: !no_release,
                token,
                allow_dirty,
            };

            let published = publish::publish_crates(&opts)?;

            // Output JSON for CI/CD integration
            if !published.is_empty() {
                let json = serde_json::to_string(&published)?;
                println!("\n📋 Published crates (JSON):");
                println!("{}", json);
            }

            Ok(())
        }
    }
}
