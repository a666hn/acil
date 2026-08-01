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
            match cmd {
                Commands::Jira { command } => {
                    let client = client::ApiClient::new(&profile.jira);
                    let out = jira::execute(&client, command).await?;
                    out.print_all();
                    Ok(())
                }
                Commands::Confluence { command } => {
                    let client = client::ApiClient::new(&profile.confluence);
                    let out = confluence::execute(&client, command).await?;
                    out.print_all();
                    Ok(())
                }
                Commands::Profile { .. } => unreachable!(),
            }
        }
    }
}

async fn login(name: String, url: String, email: String) -> Result<()> {
    println!("Adding profile '{}' for {}", name, url);
    print!("API Token: ");
    let api_token =
        rpassword::read_password().map_err(|e| crate::error::AppError::Readline(e.to_string()))?;

    let url = url.trim_end_matches('/');
    let profile = Profile {
        jira: ServiceConfig {
            base_url: url.to_string(),
            email: email.clone(),
            api_token: api_token.clone(),
        },
        confluence: ServiceConfig {
            base_url: url.to_string(),
            email,
            api_token,
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
