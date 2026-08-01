use clap::Parser;

use crate::config::Config;
use crate::error::Result;
use crate::output::{self, CommandOutput};

#[derive(Parser, Debug)]
pub enum ProfileCommand {
    /// List all profiles
    List,

    /// Show current active profile
    Current,

    /// Remove a profile
    Rm {
        /// Profile name
        name: String,
    },
}

pub fn execute(config: &mut Config, command: ProfileCommand) -> Result<CommandOutput> {
    match command {
        ProfileCommand::List => list(config),
        ProfileCommand::Current => current(config),
        ProfileCommand::Rm { name } => rm(config, &name),
    }
}

pub fn switch(config: &mut Config, name: &str) -> Result<CommandOutput> {
    config.switch_profile(name)?;
    config.save()?;
    Ok(output::collect_single_line(format!(
        "Switched to profile '{}'",
        name
    )))
}

fn list(config: &Config) -> Result<CommandOutput> {
    if config.profiles.is_empty() {
        return Ok(output::collect_single_line(
            "No profiles configured. Run 'login' to add one.".to_string(),
        ));
    }

    let rows: Vec<Vec<String>> = config
        .profiles
        .iter()
        .map(|(name, profile)| {
            let active = if name == &config.active_profile {
                "*"
            } else {
                ""
            };
            vec![
                active.to_string(),
                name.clone(),
                profile.jira.base_url.clone(),
                profile.jira.email.clone(),
            ]
        })
        .collect();
    Ok(output::collect_table(
        &["", "Profile", "Jira URL", "Email"],
        &rows,
    ))
}

fn current(config: &Config) -> Result<CommandOutput> {
    if config.active_profile.is_empty() {
        Ok(output::collect_single_line(
            "No active profile. Run 'login' to add one.".to_string(),
        ))
    } else {
        Ok(output::collect_single_line(config.active_profile.clone()))
    }
}

fn rm(config: &mut Config, name: &str) -> Result<CommandOutput> {
    config.remove_profile(name)?;
    config.save()?;
    Ok(output::collect_single_line(format!(
        "Removed profile '{}'",
        name
    )))
}
