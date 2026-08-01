use clap::Parser;
use rustyline::DefaultEditor;
use rustyline::error::ReadlineError;

use crate::client::ApiClient;
use crate::commands::{confluence, jira, profile};
use crate::config::Config;
use crate::error::Result;

const PAGE_SIZE: usize = 20;

pub async fn run(_initial_config: &Config) -> Result<()> {
    let mut config = Config::load()?;
    if !config.active_profile.is_empty() {
        println!("Active profile: {}", config.active_profile);
    }

    let mut rl =
        DefaultEditor::new().map_err(|e| crate::error::AppError::Readline(e.to_string()))?;
    let history_path = dirs::home_dir().map(|h| h.join(".config").join("acil").join("history.txt"));

    if let Some(ref path) = history_path {
        let _ = rl.load_history(path);
    }

    println!("acil REPL — type 'help' for commands, 'exit' to quit");

    loop {
        let prompt = format!("acil({})> ", config.active_profile);
        match rl.readline(&prompt) {
            Ok(line) => {
                let line = line.trim().to_string();
                if line.is_empty() {
                    continue;
                }
                let _ = rl.add_history_entry(&line);

                match line.as_str() {
                    "exit" | "quit" | "q" => break,
                    "help" => print_help(),
                    _ => {
                        if let Err(e) = dispatch(&mut config, &line).await {
                            eprintln!("Error: {}", e);
                        }
                    }
                }
            }
            Err(ReadlineError::Interrupted) | Err(ReadlineError::Eof) => break,
            Err(e) => {
                eprintln!("Readline error: {}", e);
                break;
            }
        }
    }

    if let Some(ref path) = history_path {
        let _ = rl.save_history(path);
    }

    Ok(())
}

fn print_help() {
    println!("Commands:");
    println!("  login --name <name> --url <url> --email <email>   Add a new profile");
    println!("  profile list                                      List all profiles");
    println!("  profile switch <name>                             Switch active profile");
    println!("  profile current                                   Show active profile");
    println!("  profile rm <name>                                 Remove a profile");
    println!("  jira list [--query <jql>] [--max <n>] [--limit <n>] [--subtasks] [--tree]");
    println!("  jira get <key>");
    println!(
        "  jira create --project <key> --summary <text> [--issue-type <type>] [--priority <p>] [--labels <l>] [--assignee <a>] [--parent <key>] [--description <md>]"
    );
    println!("  jira transition <key> --status <status>");
    println!("  confluence search <query> [--max <n>] [--limit <n>]");
    println!("  confluence pages --space <key> [--tree] [--limit <n>]");
    println!("  confluence get <id>");
    println!("  confluence pull <id> [--output <dir>]              Download page as markdown");
    println!(
        "  confluence push <file>                              Push local markdown to Confluence"
    );
    println!("  confluence create --space <key> --title <text> [--file <path>] [--parent <id>]");
    println!("  confluence update <id> [--title <text>] [--body <text>]");
    println!("  help");
    println!("  exit");
}

async fn dispatch(config: &mut Config, input: &str) -> Result<()> {
    let parts: Vec<&str> = input.split_whitespace().collect();
    if parts.is_empty() {
        return Ok(());
    }

    match parts[0] {
        "login" => {
            let args = std::iter::once("login").chain(parts[1..].iter().copied());
            match LoginArgs::try_parse_from(args) {
                Ok(args) => {
                    crate::login(args.name, args.url, args.email).await?;
                    *config = Config::load()?;
                }
                Err(e) => eprintln!("{}", e),
            }
            Ok(())
        }
        "profile" => {
            // Handle switch manually (REPL-only)
            if parts.len() >= 2 && parts[1] == "switch" {
                if parts.len() >= 3 {
                    let out = profile::switch(config, parts[2])?;
                    out.paged_print(PAGE_SIZE);
                } else {
                    eprintln!("Usage: profile switch <name>");
                }
                return Ok(());
            }
            let args = std::iter::once("profile").chain(parts[1..].iter().copied());
            match profile::ProfileCommand::try_parse_from(args) {
                Ok(cmd) => {
                    let out = profile::execute(config, cmd)?;
                    out.paged_print(PAGE_SIZE);
                    Ok(())
                }
                Err(e) => {
                    eprintln!("{}", e);
                    Ok(())
                }
            }
        }
        "jira" => {
            let profile = config.active_profile()?;
            let client = ApiClient::new(&profile.jira, false);
            let args = std::iter::once("jira").chain(parts[1..].iter().copied());
            match jira::JiraCommand::try_parse_from(args) {
                Ok(cmd) => {
                    let out = jira::execute(&client, cmd).await?;
                    out.paged_print(PAGE_SIZE);
                    Ok(())
                }
                Err(e) => {
                    eprintln!("{}", e);
                    Ok(())
                }
            }
        }
        "confluence" => {
            let profile = config.active_profile()?;
            let client = ApiClient::new(&profile.confluence, false);
            let args = std::iter::once("confluence").chain(parts[1..].iter().copied());
            match confluence::ConfluenceCommand::try_parse_from(args) {
                Ok(cmd) => {
                    let out = confluence::execute(&client, cmd).await?;
                    out.paged_print(PAGE_SIZE);
                    Ok(())
                }
                Err(e) => {
                    eprintln!("{}", e);
                    Ok(())
                }
            }
        }
        other => {
            eprintln!("Unknown command: {}", other);
            Ok(())
        }
    }
}

#[derive(Parser)]
#[command(name = "login")]
struct LoginArgs {
    #[arg(long)]
    name: String,
    #[arg(long)]
    url: String,
    #[arg(long)]
    email: String,
}
