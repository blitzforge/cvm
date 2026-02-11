mod changes;
mod config;
mod project;
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
    /// Applies versioning changes
    Apply {
        /// Show what would be applied without making changes
        #[arg(long)]
        dry_run: bool,
    },
    /// Check for pending changes
    Status,
    /// Manage prerelease mode
    Pre {
        #[command(subcommand)]
        action: PreAction,
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
        None => {
            // Analyze the current project (workspace or single crate)
            let crates = project::analyze_project()?;
            if crates.is_empty() {
                println!("No crates found.");
                return Ok(());
            }
            let mut remaining = crates;

            // Select for major bump
            let major_selected = ui::select_crates("Select crates for major bump:", &mut remaining);

            // Select for minor bump if remaining
            let minor_selected = if !remaining.is_empty() {
                ui::select_crates("Select crates for minor bump:", &mut remaining)
            } else {
                vec![]
            };

            // Select for patch bump if still remaining
            let patch_selected = if !remaining.is_empty() {
                ui::select_crates("Select crates for patch bump:", &mut remaining)
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
            changes::load_and_apply_pending(dry_run)?;
            Ok(())
        }
        Some(Commands::Status) => {
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
    }
}
