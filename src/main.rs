use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "code-monkey",
    version,
    about = "Automated presentation assistant"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the demo agent (run on the demo machine)
    Agent {
        /// Script file path
        script: PathBuf,
        /// TCP port to listen on
        #[arg(long, default_value = "9876")]
        port: u16,
    },
    /// Run a presentation (run on the presenter's laptop)
    Present {
        /// Script file path
        script: PathBuf,
        /// Agent address (ip:port)
        #[arg(long)]
        agent: Option<String>,
        /// Show actions without connecting or executing
        #[arg(long)]
        dry_run: bool,
    },
    /// Parse and validate a script without running
    Check {
        /// Script file path
        script: PathBuf,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Check { script } => {
            let content = std::fs::read_to_string(&script)?;
            let parsed =
                code_monkey::parser::parse_script(&content).map_err(|e| anyhow::anyhow!("{e}"))?;
            let blocks = code_monkey::grouper::group_into_blocks(&parsed);
            println!(
                "Script '{}' is valid: {} directives, {} action blocks",
                script.display(),
                parsed.lines.len(),
                blocks.len()
            );
            if let Some(title) = &parsed.front_matter.title {
                println!("Title: {title}");
            }
            Ok(())
        }
        Commands::Present {
            script,
            dry_run,
            agent,
        } => {
            let content = std::fs::read_to_string(&script)?;
            let parsed =
                code_monkey::parser::parse_script(&content).map_err(|e| anyhow::anyhow!("{e}"))?;

            if dry_run {
                let blocks = code_monkey::grouper::group_into_blocks(&parsed);
                println!("=== Dry Run: {} ===\n", script.display());
                for (i, block) in blocks.iter().enumerate() {
                    println!("--- Block {} ---", i + 1);
                    if let Some(section) = &block.section {
                        println!("  Section: {section}");
                    }
                    if let Some(narration) = &block.narration {
                        for line in narration.lines() {
                            println!("  [SAY] {line}");
                        }
                    }
                    match &block.block_type {
                        code_monkey::grouper::BlockType::Action => {
                            for action in &block.actions {
                                println!("  {action}");
                            }
                        }
                        code_monkey::grouper::BlockType::Pause(None) => {
                            println!("  [PAUSE] (wait for Enter)");
                        }
                        code_monkey::grouper::BlockType::Pause(Some(secs)) => {
                            println!("  [PAUSE {secs}] (auto-continue)");
                        }
                        code_monkey::grouper::BlockType::NarrationOnly => {
                            println!("  (narration only)");
                        }
                    }
                    println!();
                }
                return Ok(());
            }

            // Non-dry-run present mode requires --agent
            let agent_str = agent.ok_or_else(|| {
                anyhow::anyhow!("--agent <ip:port> is required when not using --dry-run")
            })?;

            let agent_addr: std::net::SocketAddr = agent_str
                .parse()
                .map_err(|e| anyhow::anyhow!("Invalid agent address '{agent_str}': {e}"))?;

            let mut presenter = code_monkey::client::Presenter::new(parsed, agent_addr);

            println!("Connecting to agent at {agent_addr}...");
            match presenter.connect() {
                Ok(()) => println!("Connected!"),
                Err(e) => {
                    eprintln!(
                        "Warning: Could not connect to agent: {e}. TUI will show disconnected state."
                    );
                }
            }

            let mut app = code_monkey::tui::App::new(presenter);
            code_monkey::tui::run_tui(&mut app)?;
            Ok(())
        }
        Commands::Agent { script, port } => {
            let content = std::fs::read_to_string(&script)?;
            let _parsed =
                code_monkey::parser::parse_script(&content).map_err(|e| anyhow::anyhow!("{e}"))?;

            let executor = code_monkey::agent::AppleScriptExecutor;
            let agent = code_monkey::agent::Agent::new(Box::new(executor), port);
            agent.run()?;
            Ok(())
        }
    }
}
