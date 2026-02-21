use std::io;
use std::time::Duration;

use anyhow::Result;
use crossterm::ExecutableCommand;
use crossterm::event::{self, Event, KeyCode};
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use crate::client::{Presenter, StepResult};

#[derive(Debug, PartialEq)]
#[allow(dead_code)]
enum ConnectionState {
    Connected,
    Disconnected,
    Reconnecting(u32),
}

pub struct App {
    presenter: Presenter,
    should_quit: bool,
    status_message: Option<String>,
    connection_state: ConnectionState,
    finished: bool,
}

impl App {
    pub fn new(presenter: Presenter) -> Self {
        let connection_state = if presenter.is_connected() {
            ConnectionState::Connected
        } else {
            ConnectionState::Disconnected
        };
        Self {
            presenter,
            should_quit: false,
            status_message: None,
            connection_state,
            finished: false,
        }
    }
}

pub fn run_tui(app: &mut App) -> Result<()> {
    // Install panic hook that restores terminal
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let _ = io::stdout().execute(LeaveAlternateScreen);
        original_hook(panic_info);
    }));

    enable_raw_mode()?;
    io::stdout().execute(EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;

    // Main event loop
    while !app.should_quit {
        terminal.draw(|frame| ui(frame, app))?;

        // Poll with timeout for responsive updates
        if event::poll(Duration::from_millis(250))?
            && let Event::Key(key) = event::read()?
        {
            match key.code {
                KeyCode::Char('q') => {
                    app.should_quit = true;
                }
                KeyCode::Char('b') => {
                    app.presenter.go_back();
                    app.status_message = None;
                    app.finished = false;
                }
                KeyCode::Char('s') => {
                    // Skip current block (useful when agent is not responding)
                    app.presenter.skip();
                    app.status_message = None;
                }
                KeyCode::Enter => {
                    if app.finished {
                        app.should_quit = true;
                        continue;
                    }

                    if app.connection_state != ConnectionState::Connected {
                        // Try reconnecting
                        match app.presenter.connect() {
                            Ok(()) => {
                                app.connection_state = ConnectionState::Connected;
                                app.status_message = Some("Reconnected!".into());
                            }
                            Err(e) => {
                                app.status_message = Some(format!("Reconnection failed: {e}"));
                                continue;
                            }
                        }
                    }

                    app.status_message = Some("Executing...".into());
                    terminal.draw(|frame| ui(frame, app))?;

                    match app.presenter.step() {
                        Ok(StepResult::Executed) => {
                            app.status_message = None;
                        }
                        Ok(StepResult::NarrationOnly) => {
                            app.status_message = None;
                        }
                        Ok(StepResult::Paused(None)) => {
                            app.status_message = None;
                            // Just advance — the next Enter will handle the next block
                        }
                        Ok(StepResult::Paused(Some(secs))) => {
                            app.status_message = Some(format!("Waiting {secs} seconds..."));
                            terminal.draw(|frame| ui(frame, app))?;
                            // Wait with interruptible polling
                            let deadline = std::time::Instant::now() + Duration::from_secs(secs);
                            while std::time::Instant::now() < deadline {
                                if event::poll(Duration::from_millis(100))?
                                    && let Event::Key(k) = event::read()?
                                    && (k.code == KeyCode::Enter || k.code == KeyCode::Char('q'))
                                {
                                    break;
                                }
                            }
                            app.status_message = None;
                        }
                        Ok(StepResult::Finished) => {
                            app.finished = true;
                            app.status_message =
                                Some("Presentation complete! Press Enter or q to exit.".into());
                        }
                        Ok(StepResult::AgentError(msg)) => {
                            app.status_message =
                                Some(format!("Agent error: {msg} (Enter=retry, s=skip)"));
                        }
                        Ok(StepResult::ConnectionLost) => {
                            app.connection_state = ConnectionState::Disconnected;
                            app.status_message =
                                Some("Connection lost. Press Enter to reconnect.".into());
                        }
                        Err(e) => {
                            app.status_message = Some(format!("Error: {e}"));
                        }
                    }
                }
                _ => {}
            }
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    io::stdout().execute(LeaveAlternateScreen)?;

    Ok(())
}

fn ui(frame: &mut Frame, app: &App) {
    let area = frame.area();

    // Layout: title, connection, narration, actions, status, footer
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // title + connection
            Constraint::Min(5),    // narration
            Constraint::Length(8), // actions
            Constraint::Length(3), // status
            Constraint::Length(1), // footer
        ])
        .split(area);

    // Title bar with progress
    let (current, total) = app.presenter.progress();
    let block = app.presenter.current_block();
    let section = block.and_then(|b| b.section.as_deref()).unwrap_or("");

    let title_text = format!(
        "  {}   [{} / {}]   {}",
        "Code Monkey",
        current + 1,
        total,
        section
    );

    let connection_indicator = match &app.connection_state {
        ConnectionState::Connected => "● Connected",
        ConnectionState::Disconnected => "○ Disconnected",
        ConnectionState::Reconnecting(n) => &format!("◌ Reconnecting ({n})..."),
    };

    let title_line = format!("{title_text}   {connection_indicator}");
    let title = Paragraph::new(title_line)
        .style(Style::default().fg(Color::White).bold())
        .block(Block::default().borders(Borders::BOTTOM));
    frame.render_widget(title, chunks[0]);

    // Narration pane
    let narration_text = block
        .and_then(|b| b.narration.as_deref())
        .unwrap_or("(no narration)");
    let narration = Paragraph::new(narration_text)
        .style(Style::default().fg(Color::White).bold())
        .wrap(Wrap { trim: false })
        .block(
            Block::default()
                .title(" SAY ")
                .title_style(Style::default().fg(Color::Yellow))
                .borders(Borders::ALL),
        );
    frame.render_widget(narration, chunks[1]);

    // Actions pane
    let actions_text = if let Some(block) = block {
        match &block.block_type {
            crate::grouper::BlockType::Action => block
                .actions
                .iter()
                .map(|a| format!("  {a}"))
                .collect::<Vec<_>>()
                .join("\n"),
            crate::grouper::BlockType::Pause(None) => "  [PAUSE] (wait for Enter)".into(),
            crate::grouper::BlockType::Pause(Some(s)) => {
                format!("  [PAUSE {s}] (auto-continue)")
            }
            crate::grouper::BlockType::NarrationOnly => "  (narration only)".into(),
        }
    } else {
        "(end of presentation)".into()
    };

    let actions = Paragraph::new(actions_text)
        .style(Style::default().fg(Color::Cyan))
        .block(
            Block::default()
                .title(" NEXT ACTION ")
                .title_style(Style::default().fg(Color::Yellow))
                .borders(Borders::ALL),
        );
    frame.render_widget(actions, chunks[2]);

    // Status bar
    let status_text = app.status_message.as_deref().unwrap_or("");
    let status_style = if status_text.contains("error") || status_text.contains("Error") {
        Style::default().fg(Color::Red)
    } else if status_text.contains("Executing") || status_text.contains("Waiting") {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::Green)
    };

    let status = Paragraph::new(status_text)
        .style(status_style)
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(status, chunks[3]);

    // Footer
    let footer_text = "  Enter = execute  │  b = back  │  s = skip  │  q = quit";
    let footer = Paragraph::new(footer_text).style(Style::default().fg(Color::DarkGray));
    frame.render_widget(footer, chunks[4]);
}
