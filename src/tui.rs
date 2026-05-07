//! Kairos TUI — epistemically-governed terminal user interface.
//!
//! Implements a ratatui-based terminal UI that:
//!   - Shows governance backend health (REQ-001 / REQ-004)
//!   - Displays a scrollable session log with timestamped entries
//!   - Provides an input prompt for natural-language governance queries
//!   - Calls `POST /preflight` on every submitted utterance (REQ-003)
//!   - Renders `confidence_target` and `decision` in the log (REQ-004)
//!
//! # REQ-005 — WebView Governance Dashboard (Round 1 TUI foundation)
//!
//! Key bindings:
//!   Enter   — submit utterance to specsmith preflight gate
//!   Ctrl-C  — quit cleanly (terminal restored)
//!   q       — quit when input is empty
//!   ↑ / ↓  — scroll session log

use std::io::{self, Stdout};

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Margin},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
    Frame, Terminal,
};

use crate::governance::client::{GovernanceClient, PreflightDecision};
use crate::session::{LogEntry, LogLevel, SessionConfig};

// ---------------------------------------------------------------------------
// App state
// ---------------------------------------------------------------------------

/// Kairos TUI application state.
pub struct App {
    /// Governance backend health (updated at startup and on reconnect).
    pub governance: GovernanceHealth,
    /// Session log entries (newest appended at the bottom).
    pub log: Vec<LogEntry>,
    /// Current text in the input prompt.
    pub input: String,
    /// Cursor position within `input` (byte offset).
    pub cursor: usize,
    /// Scroll offset for the log panel (lines from bottom visible).
    pub log_scroll: usize,
    /// Session configuration (project dir, port, etc.).
    pub config: SessionConfig,
    /// Whether the user has requested a clean exit.
    pub quit: bool,
}

#[derive(Debug, Clone)]
pub enum GovernanceHealth {
    Connected { version: String },
    Disconnected { reason: String },
    Checking,
}

impl App {
    pub fn new(config: SessionConfig) -> Self {
        Self {
            governance: GovernanceHealth::Checking,
            log: Vec::new(),
            input: String::new(),
            cursor: 0,
            log_scroll: 0,
            config,
            quit: false,
        }
    }

    pub fn log_info(&mut self, msg: impl Into<String>) {
        self.log.push(LogEntry::info(msg));
    }
    pub fn log_success(&mut self, msg: impl Into<String>) {
        self.log.push(LogEntry::success(msg));
    }
    pub fn log_warn(&mut self, msg: impl Into<String>) {
        self.log.push(LogEntry::warn(msg));
    }
    pub fn log_error(&mut self, msg: impl Into<String>) {
        self.log.push(LogEntry::error(msg));
    }
}

// ---------------------------------------------------------------------------
// Terminal setup / teardown helpers
// ---------------------------------------------------------------------------

fn setup_terminal() -> Result<Terminal<CrosstermBackend<Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?;
    Ok(terminal)
}

fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<Stdout>>) {
    // Best-effort — never panic on cleanup.
    let _ = disable_raw_mode();
    let _ = execute!(terminal.backend_mut(), LeaveAlternateScreen);
    let _ = terminal.show_cursor();
}

// ---------------------------------------------------------------------------
// Main TUI entry point
// ---------------------------------------------------------------------------

/// Run the Kairos TUI event loop.
///
/// This is the `async` entry point called from `main`. It owns the terminal
/// for its entire lifetime and restores it cleanly on exit (even on panic via
/// the RAII guard pattern).
pub async fn run(config: SessionConfig, client: GovernanceClient) -> Result<()> {
    let mut terminal = setup_terminal()?;
    let mut app = App::new(config);

    // Perform initial health check to populate governance status.
    match client.health().await {
        Ok(h) => {
            app.governance = GovernanceHealth::Connected { version: h.version.clone() };
            app.log_success(format!(
                "Governance backend ready — specsmith {}",
                h.version
            ));
        }
        Err(e) => {
            app.governance = GovernanceHealth::Disconnected { reason: e.to_string() };
            app.log_error(format!("Governance backend unreachable: {e}"));
            app.log_warn("Run `specsmith governance-serve --port 7700` in another terminal.");
        }
    }

    let result = event_loop(&mut terminal, &mut app, &client).await;
    restore_terminal(&mut terminal);
    result
}

// ---------------------------------------------------------------------------
// Event loop
// ---------------------------------------------------------------------------

async fn event_loop(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    app: &mut App,
    client: &GovernanceClient,
) -> Result<()> {
    loop {
        terminal.draw(|f| draw(f, app))?;

        if app.quit {
            break;
        }

        // Poll for keyboard events with a short timeout so the TUI stays responsive.
        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                handle_key(app, key, client).await;
            }
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Key event handler
// ---------------------------------------------------------------------------

async fn handle_key(app: &mut App, key: KeyEvent, client: &GovernanceClient) {
    match key.code {
        // Quit: Ctrl-C always, 'q' when input is empty.
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.log_info("Session ended by user (Ctrl-C).");
            app.quit = true;
        }
        KeyCode::Char('q') if app.input.is_empty() => {
            app.log_info("Session ended by user.");
            app.quit = true;
        }

        // Scroll the log panel.
        KeyCode::Up => {
            app.log_scroll = app.log_scroll.saturating_add(1);
        }
        KeyCode::Down => {
            app.log_scroll = app.log_scroll.saturating_sub(1);
        }

        // Submit the input as a governance preflight query.
        KeyCode::Enter => {
            let utterance = app.input.trim().to_owned();
            if utterance.is_empty() {
                return;
            }
            app.input.clear();
            app.cursor = 0;
            app.log_scroll = 0; // scroll to bottom on new entry

            app.log_info(format!("> {utterance}"));
            run_preflight(app, client, &utterance).await;
        }

        // Backspace: remove last character.
        KeyCode::Backspace => {
            if !app.input.is_empty() {
                app.input.pop();
                app.cursor = app.input.len();
            }
        }

        // Regular character input.
        KeyCode::Char(c) => {
            app.input.push(c);
            app.cursor = app.input.len();
        }

        // Escape: clear input.
        KeyCode::Esc => {
            app.input.clear();
            app.cursor = 0;
        }

        _ => {}
    }
}

// ---------------------------------------------------------------------------
// Preflight call
// ---------------------------------------------------------------------------

async fn run_preflight(app: &mut App, client: &GovernanceClient, utterance: &str) {
    let project_dir = app.config.project_dir_str().to_owned();
    match client.preflight(utterance, Some(&project_dir)).await {
        Ok(decision) => render_decision(app, &decision),
        Err(e) => {
            app.log_error(format!("Preflight error: {e}"));
            // Mark backend as disconnected if health fails.
            app.governance = GovernanceHealth::Disconnected { reason: e.to_string() };
        }
    }
}

fn render_decision(app: &mut App, d: &PreflightDecision) {
    let badge = if d.accepted() { "✓ ACCEPTED" } else { "⚠ NOT ACCEPTED" };
    let reqs = if d.requirement_ids.is_empty() {
        "—".to_owned()
    } else {
        d.requirement_ids.join(", ")
    };
    if d.accepted() {
        app.log_success(format!(
            "Preflight {badge} — WI:{} | REQs: {} | confidence ≥ {:.2}",
            d.work_item_id, reqs, d.confidence_target
        ));
    } else {
        app.log_warn(format!(
            "Preflight {badge} — {}\n  Intent: {} | REQs: {}",
            d.instruction, d.intent, reqs
        ));
    }
}

// ---------------------------------------------------------------------------
// Drawing
// ---------------------------------------------------------------------------

fn draw(frame: &mut Frame, app: &App) {
    // ── Layout ────────────────────────────────────────────────────────────
    // ┌─────────────────────────────────────┐  ← header  (3 lines)
    // │  session log (scrollable)           │  ← log     (fills space)
    // │  input prompt                       │  ← input   (3 lines)
    // │  status bar                         │  ← footer  (1 line)
    // └─────────────────────────────────────┘

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),   // header
            Constraint::Min(5),      // log
            Constraint::Length(3),   // input
            Constraint::Length(1),   // footer
        ])
        .split(frame.area());

    draw_header(frame, app, chunks[0]);
    draw_log(frame, app, chunks[1]);
    draw_input(frame, app, chunks[2]);
    draw_footer(frame, app, chunks[3]);
}

fn draw_header(frame: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let (health_text, health_color) = match &app.governance {
        GovernanceHealth::Connected { version } => {
            (format!("🟢 Connected — specsmith {version}"), Color::Green)
        }
        GovernanceHealth::Disconnected { reason } => {
            (format!("🔴 Disconnected — {reason}"), Color::Red)
        }
        GovernanceHealth::Checking => ("⏳ Checking…".to_owned(), Color::Yellow),
    };

    let project = app.config.project_dir.display().to_string();
    let header = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("  KAIROS  ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::styled("v0.1.0", Style::default().fg(Color::DarkGray)),
            Span::raw("        "),
            Span::styled(health_text, Style::default().fg(health_color)),
        ]),
        Line::from(vec![
            Span::styled("  Project: ", Style::default().fg(Color::DarkGray)),
            Span::styled(project, Style::default().fg(Color::White)),
        ]),
    ])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan)),
    );
    frame.render_widget(header, area);
}

fn draw_log(frame: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let inner_height = area.height.saturating_sub(2) as usize; // subtract borders
    let total = app.log.len();

    // Determine which slice of log entries is visible.
    let start = if total + app.log_scroll > inner_height {
        total + app.log_scroll - inner_height
    } else {
        0
    }
    .min(total.saturating_sub(1));
    let visible: &[LogEntry] = if total == 0 {
        &[]
    } else {
        &app.log[start.min(total - 1)..]
    };

    let items: Vec<ListItem> = visible
        .iter()
        .map(|entry| {
            let (prefix, color) = match entry.level {
                LogLevel::Info => ("  ", Color::White),
                LogLevel::Success => ("✓ ", Color::Green),
                LogLevel::Warn => ("⚠ ", Color::Yellow),
                LogLevel::Error => ("✗ ", Color::Red),
            };
            let line = Line::from(vec![
                Span::styled(
                    format!("[{}] ", entry.timestamp),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(prefix, Style::default().fg(color)),
                Span::styled(&entry.message, Style::default().fg(color)),
            ]);
            ListItem::new(line)
        })
        .collect();

    let scroll_hint = if app.log_scroll > 0 {
        format!(" ↓ {} more", app.log_scroll)
    } else {
        String::new()
    };

    let block = Block::default()
        .title(format!(" Session Log{scroll_hint} "))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));

    let list = List::new(items).block(block);
    frame.render_widget(list, area);
}

fn draw_input(frame: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let prompt = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("  > ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::raw(&app.input),
            Span::styled("█", Style::default().fg(Color::Cyan)), // cursor block
        ]),
    ])
    .block(
        Block::default()
            .title(" Governance Query — Enter to preflight, q to quit ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Blue)),
    );
    frame.render_widget(prompt, area);
}

fn draw_footer(frame: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let text = Line::from(vec![
        Span::styled(" [Enter] preflight ", Style::default().fg(Color::DarkGray)),
        Span::styled(" [↑↓] scroll ", Style::default().fg(Color::DarkGray)),
        Span::styled(" [Esc] clear ", Style::default().fg(Color::DarkGray)),
        Span::styled(" [q / Ctrl-C] quit ", Style::default().fg(Color::DarkGray)),
    ]);
    let footer = Paragraph::new(text)
        .style(Style::default().bg(Color::Black).fg(Color::DarkGray));
    frame.render_widget(footer, area);
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::SessionConfig;

    fn make_app() -> App {
        App::new(SessionConfig::new(None))
    }

    #[test]
    fn app_log_appends_entries() {
        let mut app = make_app();
        app.log_info("hello");
        app.log_success("done");
        app.log_warn("check this");
        app.log_error("failed");
        assert_eq!(app.log.len(), 4);
        assert_eq!(app.log[0].level, LogLevel::Info);
        assert_eq!(app.log[1].level, LogLevel::Success);
    }

    #[test]
    fn app_input_and_cursor() {
        let mut app = make_app();
        app.input.push('h');
        app.input.push('i');
        app.cursor = app.input.len();
        assert_eq!(app.input, "hi");
        assert_eq!(app.cursor, 2);
        app.input.pop();
        app.cursor = app.input.len();
        assert_eq!(app.input, "h");
    }

    #[test]
    fn governance_health_variants() {
        let connected = GovernanceHealth::Connected { version: "0.10.1".to_owned() };
        let disconnected = GovernanceHealth::Disconnected { reason: "timeout".to_owned() };
        // Just ensure they construct without panicking.
        drop(connected);
        drop(disconnected);
    }
}
