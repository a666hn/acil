mod client;
mod commands;
mod config;
mod confluence_convert;
mod confluence_meta;
mod error;
mod jira_adf;
mod output;
mod repl;

use clap::{Parser, Subcommand};

use crate::commands::{confluence, jira, profile};
use crate::config::{Config, Profile, ServiceConfig};
use crate::error::Result;

#[derive(Parser, Debug)]
#[command(
    name = "acil",
    version,
    about = "Manage Jira and Confluence from the CLI"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Override active profile
    #[arg(short, long, global = true)]
    profile: Option<String>,

    /// Show API request details
    #[arg(short, long, global = true)]
    verbose: bool,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Manage profiles (list, current, remove)
    Profile {
        #[command(subcommand)]
        command: profile::ProfileCommand,
    },

    /// Jira operations
    Jira {
        #[command(subcommand)]
        command: jira::JiraCommand,
    },

    /// Confluence operations
    Confluence {
        #[command(subcommand)]
        command: confluence::ConfluenceCommand,
    },

    /// Update acil to the latest release
    Update {
        /// Skip the confirmation prompt
        #[arg(short, long)]
        yes: bool,
    },

    /// Uninstall acil
    Uninstall {
        /// Skip the confirmation prompt
        #[arg(short, long)]
        yes: bool,

        /// Also remove ~/.config/acil (profiles, API tokens, history)
        #[arg(long)]
        purge_config: bool,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Profile { command }) => {
            let mut config = Config::load()?;
            let out = profile::execute(&mut config, command)?;
            out.print_all();
            Ok(())
        }
        Some(Commands::Update { yes }) => tokio::task::spawn_blocking(move || update(yes))
            .await
            .map_err(|e| crate::error::AppError::Update(format!("Task error: {}", e)))?,
        Some(Commands::Uninstall { yes, purge_config }) => {
            tokio::task::spawn_blocking(move || uninstall(yes, purge_config))
                .await
                .map_err(|e| crate::error::AppError::Update(format!("Task error: {}", e)))?
        }
        None => {
            let config = Config::load()?;
            if let Some(ref p) = cli.profile {
                let mut config = config;
                config.switch_profile(p)?;
                repl::run(&config).await
            } else {
                repl::run(&config).await
            }
        }
        Some(cmd) => {
            let mut config = Config::load()?;
            if let Some(ref p) = cli.profile {
                config.switch_profile(p)?;
            }
            let profile = config.active_profile()?;
            let verbose = cli.verbose;
            match cmd {
                Commands::Jira { command } => {
                    let client = client::ApiClient::new(&profile.jira, verbose);
                    let out = jira::execute(&client, command).await?;
                    out.print_all();
                    Ok(())
                }
                Commands::Confluence { command } => {
                    let client = client::ApiClient::new(&profile.confluence, verbose);
                    let out = confluence::execute(&client, command).await?;
                    out.print_all();
                    Ok(())
                }
                Commands::Profile { .. } | Commands::Update { .. } | Commands::Uninstall { .. } => {
                    unreachable!()
                }
            }
        }
    }
}

async fn login(name: String, url: String, email: String) -> Result<()> {
    println!("Adding profile '{}' for {}", name, url);
    use std::io::Write;

    print!("Jira API Token: ");
    std::io::stdout().flush().ok();
    let jira_api_token =
        rpassword::read_password().map_err(|e| crate::error::AppError::Readline(e.to_string()))?;

    print!("Confluence API Token: ");
    std::io::stdout().flush().ok();
    let confluence_api_token =
        rpassword::read_password().map_err(|e| crate::error::AppError::Readline(e.to_string()))?;

    let url = url.trim_end_matches('/');
    let profile = Profile {
        jira: ServiceConfig {
            base_url: url.to_string(),
            email: email.clone(),
            api_token: jira_api_token,
        },
        confluence: ServiceConfig {
            base_url: url.to_string(),
            email,
            api_token: confluence_api_token,
        },
    };

    let mut config = Config::load()?;
    config.add_profile(name.clone(), profile);
    config.save()?;
    println!(
        "Profile '{}' saved. Active profile: '{}'",
        name, config.active_profile
    );
    Ok(())
}

fn update(yes: bool) -> Result<()> {
    use crate::error::AppError;
    use self_update::cargo_crate_version;

    let status = self_update::backends::github::Update::configure()
        .repo_owner("a666hn")
        .repo_name("acil")
        .bin_name("acil")
        .bin_path_in_archive("acil-v{{ version }}-{{ target }}/{{ bin }}")
        .target(self_update::get_target())
        .show_download_progress(true)
        .no_confirm(yes)
        .current_version(cargo_crate_version!())
        .build()
        .map_err(|e| AppError::Update(format!("Update failed: {}", e)))?
        .update()
        .map_err(|e| {
            AppError::Update(format!(
                "Update failed: {}. If acil is installed in a system directory, try running with sudo, or re-run install.sh.",
                e
            ))
        })?;

    if status.uptodate() {
        println!("\nAlready up to date (v{}).", status.version());
    } else {
        println!("\nUpdated to v{}.", status.version());
    }

    Ok(())
}

fn uninstall(yes: bool, purge_config: bool) -> Result<()> {
    use crate::error::AppError;
    use std::io::Write;

    let exe_path = std::env::current_exe()
        .map_err(|e| AppError::Update(format!("Could not locate the running binary: {}", e)))?;

    if !yes {
        print!(
            "Remove {}{}? [y/N] ",
            exe_path.display(),
            if purge_config {
                " and ~/.config/acil (profiles, API tokens, history)"
            } else {
                ""
            }
        );
        std::io::stdout().flush().ok();
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).ok();
        if !matches!(input.trim().to_lowercase().as_str(), "y" | "yes") {
            println!("Aborted.");
            return Ok(());
        }
    }

    if purge_config
        && let Some(config_dir) = dirs::home_dir().map(|h| h.join(".config").join("acil"))
        && config_dir.exists()
    {
        std::fs::remove_dir_all(&config_dir)?;
        println!("Removed {}", config_dir.display());
    }

    println!("Removing {}...", exe_path.display());
    self_replace::self_delete()
        .map_err(|e| AppError::Update(format!("Failed to remove binary: {}", e)))?;

    println!("acil has been uninstalled.");
    Ok(())
}
