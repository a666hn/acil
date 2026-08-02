use std::borrow::Cow;

use clap::Parser;
use nu_ansi_term::Style;
use reedline::{
    Color, ColumnarMenu, DefaultCompleter, DefaultHinter, Emacs, ExampleHighlighter,
    FileBackedHistory, KeyCode, KeyModifiers, MenuBuilder, Prompt, PromptEditMode,
    PromptHistorySearch, Reedline, ReedlineEvent, ReedlineMenu, Signal, default_emacs_keybindings,
};

use crate::client::ApiClient;
use crate::commands::{confluence, jira, profile};
use crate::config::Config;
use crate::error::Result;
use crate::output;

const PAGE_SIZE: usize = 20;

fn gradient_color(t: f32) -> nu_ansi_term::Color {
    // cyan -> purple
    let start = (0.0, 255.0, 255.0);
    let end = (170.0, 80.0, 255.0);
    let lerp = |a: f64, b: f64| (a + (b - a) * t as f64).round() as u8;
    nu_ansi_term::Color::Rgb(
        lerp(start.0, end.0),
        lerp(start.1, end.1),
        lerp(start.2, end.2),
    )
}

fn print_banner(version: &str) {
    if let Ok(font) = figleter::FIGfont::standard()
        && let Some(figure) = font.convert("ACIL")
    {
        let art = figure.to_string();
        let lines: Vec<&str> = art.lines().filter(|l| !l.trim().is_empty()).collect();
        let total = lines.len();
        for (i, line) in lines.iter().enumerate() {
            let t = if total > 1 {
                i as f32 / (total - 1) as f32
            } else {
                0.0
            };
            println!(
                "{}",
                output::paint(line, Style::new().fg(gradient_color(t)).bold())
            );
        }
    }
    println!(
        "{}",
        output::dim(&format!("v{}  ·  github.com/a666hn/acil", version))
    );
}

fn command_words() -> Vec<String> {
    [
        "login",
        "profile",
        "list",
        "switch",
        "current",
        "rm",
        "jira",
        "get",
        "create",
        "transition",
        "confluence",
        "search",
        "pages",
        "pull",
        "push",
        "update",
        "help",
        "exit",
        "quit",
    ]
    .into_iter()
    .map(String::from)
    .collect()
}

struct AcilPrompt {
    profile: String,
}

impl Prompt for AcilPrompt {
    fn render_prompt_left(&self) -> Cow<'_, str> {
        Cow::Owned(format!("acil({})", self.profile))
    }

    fn render_prompt_right(&self) -> Cow<'_, str> {
        Cow::Borrowed("")
    }

    fn render_prompt_indicator(&self, _mode: PromptEditMode) -> Cow<'_, str> {
        Cow::Borrowed("> ")
    }

    fn render_prompt_multiline_indicator(&self) -> Cow<'_, str> {
        Cow::Borrowed("::: ")
    }

    fn render_prompt_history_search_indicator(&self, search: PromptHistorySearch) -> Cow<'_, str> {
        Cow::Owned(format!("(reverse-search: {}) ", search.term))
    }

    fn get_prompt_color(&self) -> Color {
        Color::Cyan
    }

    fn get_indicator_color(&self) -> Color {
        Color::Green
    }
}

fn build_editor() -> Reedline {
    let mut keybindings = default_emacs_keybindings();
    keybindings.add_binding(
        KeyModifiers::NONE,
        KeyCode::Tab,
        ReedlineEvent::UntilFound(vec![
            ReedlineEvent::Menu("completion_menu".to_string()),
            ReedlineEvent::MenuNext,
        ]),
    );
    let edit_mode = Box::new(Emacs::new(keybindings));

    let completion_menu = Box::new(ColumnarMenu::default().with_name("completion_menu"));

    let mut editor = Reedline::create()
        .with_hinter(Box::new(
            DefaultHinter::default()
                .with_style(Style::new().italic().fg(nu_ansi_term::Color::DarkGray)),
        ))
        .with_highlighter(Box::new(ExampleHighlighter::new(command_words())))
        .with_completer(Box::new(DefaultCompleter::new_with_wordlen(
            command_words(),
            2,
        )))
        .with_menu(ReedlineMenu::EngineCompleter(completion_menu))
        .with_edit_mode(edit_mode);

    let history_path = dirs::home_dir().map(|h| h.join(".config").join("acil").join("history.txt"));
    if let Some(path) = history_path
        && let Ok(history) = FileBackedHistory::with_file(1000, path)
    {
        editor = editor.with_history(Box::new(history));
    }

    editor
}

pub async fn run(_initial_config: &Config) -> Result<()> {
    let mut config = Config::load()?;

    print_banner(env!("CARGO_PKG_VERSION"));
    if !config.active_profile.is_empty() {
        println!(
            "Profile: {}",
            output::paint(
                &config.active_profile,
                Style::new().fg(nu_ansi_term::Color::Green).bold(),
            )
        );
    }
    println!(
        "{}",
        output::dim("Type 'help' for commands, 'exit' to quit")
    );
    println!();

    let mut line_editor = build_editor();

    loop {
        let prompt = AcilPrompt {
            profile: config.active_profile.clone(),
        };
        match line_editor.read_line(&prompt) {
            Ok(Signal::Success(line)) => {
                let line = line.trim().to_string();
                if line.is_empty() {
                    continue;
                }

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
            Ok(Signal::CtrlC) | Ok(Signal::CtrlD) => break,
            Ok(_) => {}
            Err(e) => {
                eprintln!("Readline error: {}", e);
                break;
            }
        }
    }

    let _ = line_editor.sync_history();

    Ok(())
}

fn print_help() {
    println!("Commands:");
    println!("  login --name <name> --url <url> --email <email>   Add a new profile");
    println!("  profile list                                      List all profiles");
    println!("  profile switch <name>                             Switch active profile");
    println!("  profile current                                   Show active profile");
    println!("  profile rm <name>                                 Remove a profile");
    println!(
        "  jira list [--query <jql>] [--assigned <email>] [--status <status>] [--max <n>] [--limit <n>] [--subtasks] [--tree]"
    );
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
