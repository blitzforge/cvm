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
    },
    /// Check for pending changes
    Status,
    /// Get crate information as JSON
    Info {
        /// Print a single release version line (for CI tags and scripts)
        #[arg(long, conflicts_with = "format")]
        version: bool,
        /// Output format
        #[arg(long, default_value = "json", value_parser = ["json"])]
        format: String,
    },
    /// Manage prerelease mode
    Pre {
        #[command(subcommand)]
        action: PreAction,
    },
    /// Publish crates to crates.io
    Publish {
        /// Show what would be published without making changes
        #[arg(long)]
        dry_run: bool,
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
        Some(Commands::Apply { dry_run }) => {
            // Check if CVM is initialized
            if !std::path::Path::new(".cvm").exists() {
                eprintln!("❌ CVM is not initialized in this project.");
                eprintln!("\nRun 'cvm setup' to initialize CVM.");
                std::process::exit(1);
            }

            changes::load_and_apply_pending(dry_run)?;
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
        Some(Commands::Info { format, version }) => {
            if version {
                println!("{}", project::release_version()?);
                return Ok(());
            }

            // Get all crates in the project
            let crates = project::analyze_project()?;
            let is_workspace = project::is_workspace_project()?;

            if format == "json" {
                // If there's only one crate, return just the version string (no JSON formatting)
                if crates.len() == 1 && !is_workspace {
                    // Use write! to avoid any automatic formatting or whitespace
                    use std::io::{self, Write};
                    let stdout = io::stdout();
                    let mut handle = stdout.lock();
                    write!(handle, "{}", crates[0].version)?;
                    handle.flush()?;
                } else {
                    // For multiple crates, return the full array
                    use std::io::{self, Write};
                    let json = serde_json::to_string(&crates)?;
                    let stdout = io::stdout();
                    let mut handle = stdout.lock();
                    write!(handle, "{}", json)?;
                    handle.flush()?;
                }
            } else {
                eprintln!("❌ Unsupported format: {}", format);
                std::process::exit(1);
            }
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
                token,
                allow_dirty,
            };

            let published = publish::publish_crates(&opts)?;

            // Output JSON for CI/CD integration (always output, even if empty)
            let json = serde_json::to_string(&published)?;
            println!("\n::cvm-output-json::{}", json);

            Ok(())
        }
    }
}
