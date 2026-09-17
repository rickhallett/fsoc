//! wargamezr — a terminal-native front-end for a self-hosted, Bandit-style
//! Linux wargame. The TUI is the frame: level briefs, a hint ladder, a
//! treasure vault, and a door into a real shell inside the game container.
mod docker;
mod model;
mod save;

use anyhow::{Context, Result};
use docker::World;
use model::{Campaign, Level};
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::crossterm::execute;
use ratatui::crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, BorderType, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::{DefaultTerminal, Frame};
use save::Save;
use std::io::stdout;
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

// palette -------------------------------------------------------------------
const ACCENT: Color = Color::Rgb(94, 234, 212); // teal
const GOLD: Color = Color::Rgb(250, 204, 21);
const GREEN: Color = Color::Rgb(74, 222, 128);
const RED: Color = Color::Rgb(248, 113, 113);
const DIM: Color = Color::Rgb(120, 133, 150);
const FG: Color = Color::Rgb(226, 232, 240);

struct App {
    campaign: Campaign,
    save: Save,
    world: World,
    world_running: bool,
    selected: usize,
    list_state: ListState,
    input_mode: bool,
    input: String,
    show_solution: bool,
    status: String,
}

impl App {
    fn new(campaign: Campaign, world: World) -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));
        let world_running = world.is_running();
        let status = if world_running {
            "World is up. Pick a level and press Enter to drop into a shell.".into()
        } else {
            "World not running. Start it with `make world-up`, then press r.".into()
        };
        App {
            campaign,
            save: Save::load(),
            world,
            world_running,
            selected: 0,
            list_state,
            input_mode: false,
            input: String::new(),
            show_solution: false,
            status,
        }
    }

    fn current(&self) -> &Level {
        &self.campaign.levels[self.selected]
    }

    fn move_sel(&mut self, delta: isize) {
        let len = self.campaign.levels.len() as isize;
        let mut i = self.selected as isize + delta;
        if i < 0 {
            i = 0;
        }
        if i >= len {
            i = len - 1;
        }
        self.selected = i as usize;
        self.list_state.select(Some(self.selected));
        self.show_solution = false;
    }

    fn refresh_world(&mut self) {
        self.world_running = self.world.is_running();
        self.status = if self.world_running {
            "World is up.".into()
        } else {
            "World not running. Start it with `make world-up`.".into()
        };
    }

    fn reveal_hint(&mut self) {
        let n = self.current().n;
        let max = self.current().hints.len();
        self.save.reveal_hint(n, max);
        let _ = self.save.store();
    }

    fn submit_password(&mut self) {
        let n = self.current().n;
        if n == 0 {
            self.status = "Level 0 has no password — just SSH in (Enter).".into();
            self.input.clear();
            self.input_mode = false;
            return;
        }
        if !self.world_running {
            self.status = "World not running — cannot verify.".into();
            self.input_mode = false;
            return;
        }
        match self.world.verify(n, &self.input) {
            Ok(true) => {
                self.save.mark_solved(n, self.input.trim().to_string());
                let _ = self.save.store();
                self.status = format!("✓ Correct! Level {n} cleared, bandit{n} unlocked.");
            }
            Ok(false) => self.status = "✗ Not the right password. Keep digging.".into(),
            Err(e) => self.status = format!("verify error: {e}"),
        }
        self.input.clear();
        self.input_mode = false;
    }

    fn enter_level(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        let n = self.current().n;
        if !self.save.is_unlocked(n) {
            self.status = "That level is locked. Clear the previous one first.".into();
            return Ok(());
        }
        if !self.world_running {
            self.status = "World not running — run `make world-up` then press r.".into();
            return Ok(());
        }
        let (cmd, args) = self.world.shell_command(n);
        // suspend the TUI, hand the terminal to the child, then restore.
        disable_raw_mode()?;
        execute!(stdout(), LeaveAlternateScreen)?;
        let who = if n == 0 {
            "bandit0 over SSH (password: bandit0)".to_string()
        } else {
            format!("bandit{} (level {n})", n - 1)
        };
        println!("\n\x1b[1;36m── entering {who} ──\x1b[0m");
        println!("\x1b[2m(find the next password, then `exit` / Ctrl-D to return)\x1b[0m\n");
        let _ = Command::new(&cmd).args(&args).status();
        enable_raw_mode()?;
        execute!(stdout(), EnterAlternateScreen)?;
        terminal.clear()?;
        self.refresh_world();
        self.status = format!("Back from {who}. Press p to enter the password you found.");
        Ok(())
    }
}

fn main() -> Result<()> {
    let campaign_path = find_campaign().context("locating levels/bandit.toml")?;
    let campaign = Campaign::load(&campaign_path)?;
    let container = std::env::var("WARGAMEZR_CONTAINER").unwrap_or_else(|_| "bandit".into());
    let world = World::new(container);

    if std::env::args().any(|a| a == "--check") {
        return check(&campaign, &world);
    }

    let mut app = App::new(campaign, world);

    let mut terminal = ratatui::init();
    let res = run(&mut terminal, &mut app);
    ratatui::restore();
    res
}

/// Headless doctor: confirm the world is reachable and that every level the
/// world provisions has a recoverable password (exercises the Docker layer).
fn check(campaign: &Campaign, world: &World) -> Result<()> {
    println!("wargamezr --check");
    println!("campaign : {} ({} levels)", campaign.campaign, campaign.levels.len());
    print!("world    : {} ... ", world.container);
    if !world.is_running() {
        println!("NOT RUNNING");
        anyhow::bail!("start the world first: make world-up");
    }
    println!("running");
    let mut provisioned = 0u32;
    for lvl in &campaign.levels {
        if lvl.n == 0 {
            continue;
        }
        match world.password_of(lvl.n) {
            Ok(pw) if pw.len() >= 8 => {
                provisioned += 1;
                println!("  level {:>2}  password reachable ({}…)", lvl.n, &pw[..4]);
            }
            Ok(_) => println!("  level {:>2}  password too short?", lvl.n),
            Err(_) => println!("  level {:>2}  not provisioned in this world build", lvl.n),
        }
    }
    println!("ok: {provisioned} levels provisioned and verifiable");
    Ok(())
}

/// Look for the campaign TOML next to the binary's project, walking up from
/// the current dir (dev) and checking a couple of sensible fallbacks.
fn find_campaign() -> Result<PathBuf> {
    let candidates = [
        PathBuf::from("levels/bandit.toml"),
        PathBuf::from("../levels/bandit.toml"),
        PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../levels/bandit.toml")),
    ];
    for c in candidates {
        if c.exists() {
            return Ok(c);
        }
    }
    anyhow::bail!("could not find levels/bandit.toml (run from the repo root)")
}

fn run(terminal: &mut DefaultTerminal, app: &mut App) -> Result<()> {
    loop {
        terminal.draw(|f| ui(f, app))?;
        if !event::poll(Duration::from_millis(200))? {
            continue;
        }
        let Event::Key(key) = event::read()? else {
            continue;
        };
        if key.kind != KeyEventKind::Press {
            continue;
        }

        if app.input_mode {
            match key.code {
                KeyCode::Enter => app.submit_password(),
                KeyCode::Esc => {
                    app.input.clear();
                    app.input_mode = false;
                }
                KeyCode::Backspace => {
                    app.input.pop();
                }
                KeyCode::Char(c) => app.input.push(c),
                _ => {}
            }
            continue;
        }

        match key.code {
            KeyCode::Char('q') => break,
            KeyCode::Down | KeyCode::Char('j') => app.move_sel(1),
            KeyCode::Up | KeyCode::Char('k') => app.move_sel(-1),
            KeyCode::Char('h') => app.reveal_hint(),
            KeyCode::Char('s') => app.show_solution = !app.show_solution,
            KeyCode::Char('r') => app.refresh_world(),
            KeyCode::Char('p') => {
                app.input_mode = true;
                app.status = "Type the password you found, Enter to submit, Esc to cancel.".into();
            }
            KeyCode::Enter => app.enter_level(terminal)?,
            _ => {}
        }
    }
    Ok(())
}

fn ui(f: &mut Frame, app: &App) {
    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // header
            Constraint::Min(5),    // body
            Constraint::Length(4), // help (status + keys)
        ])
        .split(f.area());

    render_header(f, root[0], app);

    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(34), Constraint::Percentage(66)])
        .split(root[1]);

    render_level_list(f, body[0], app);
    render_detail(f, body[1], app);
    render_help(f, root[2], app);
}

fn render_header(f: &mut Frame, area: Rect, app: &App) {
    let total = app.campaign.levels.len().saturating_sub(1); // exclude intro
    let solved = app.save.solved.len();
    let dot = if app.world_running { "●" } else { "○" };
    let world_color = if app.world_running { GREEN } else { RED };
    let line = Line::from(vec![
        Span::styled("  wargamezr ", Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)),
        Span::styled(
            format!("· {} campaign ", app.campaign.campaign),
            Style::default().fg(FG),
        ),
        Span::styled(
            format!("· {solved}/{total} cleared "),
            Style::default().fg(GREEN),
        ),
        Span::styled(
            format!("· {} treasure ", app.save.treasure.len()),
            Style::default().fg(GOLD),
        ),
        Span::styled(format!("· world {dot}"), Style::default().fg(world_color)),
    ]);
    let p = Paragraph::new(line).block(
        Block::bordered()
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(DIM)),
    );
    f.render_widget(p, area);
}

fn render_level_list(f: &mut Frame, area: Rect, app: &App) {
    let items: Vec<ListItem> = app
        .campaign
        .levels
        .iter()
        .map(|lvl| {
            let unlocked = app.save.is_unlocked(lvl.n);
            let solved = app.save.is_solved(lvl.n);
            let (glyph, color) = if solved {
                ("✓", GREEN)
            } else if !unlocked {
                ("🔒", DIM)
            } else {
                ("▸", ACCENT)
            };
            let label = if lvl.n == 0 {
                "Level 0 · Getting in".to_string()
            } else {
                format!("Level {:>2} · {}", lvl.n, short_title(&lvl.title))
            };
            ListItem::new(Line::from(vec![
                Span::styled(format!(" {glyph} "), Style::default().fg(color)),
                Span::styled(label, Style::default().fg(if unlocked { FG } else { DIM })),
            ]))
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::bordered()
                .title(Span::styled(" levels ", Style::default().fg(ACCENT)))
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(DIM)),
        )
        .highlight_style(
            Style::default()
                .bg(Color::Rgb(30, 41, 59))
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("");

    let mut state = app.list_state.clone();
    f.render_stateful_widget(list, area, &mut state);
}

fn render_detail(f: &mut Frame, area: Rect, app: &App) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    let lvl = app.current();

    // --- brief ---
    let mut brief: Vec<Line> = Vec::new();
    brief.push(Line::from(Span::styled(
        lvl.title.clone(),
        Style::default().fg(GOLD).add_modifier(Modifier::BOLD),
    )));
    brief.push(Line::from(""));
    for l in lvl.goal.trim().lines() {
        brief.push(Line::from(Span::styled(l.to_string(), Style::default().fg(FG))));
    }
    if !lvl.commands.is_empty() {
        brief.push(Line::from(""));
        brief.push(Line::from(vec![
            Span::styled("useful: ", Style::default().fg(DIM)),
            Span::styled(lvl.commands.join("  "), Style::default().fg(ACCENT)),
        ]));
    }
    if let Some(pw) = app.save.treasure.get(&lvl.n) {
        brief.push(Line::from(""));
        brief.push(Line::from(vec![
            Span::styled("treasure: ", Style::default().fg(GOLD)),
            Span::styled(pw.clone(), Style::default().fg(GOLD).add_modifier(Modifier::BOLD)),
        ]));
    }
    let brief_p = Paragraph::new(Text::from(brief))
        .wrap(Wrap { trim: false })
        .block(
            Block::bordered()
                .title(Span::styled(" brief ", Style::default().fg(ACCENT)))
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(DIM)),
        );
    f.render_widget(brief_p, rows[0]);

    // --- hints / solution / input ---
    let mut lower: Vec<Line> = Vec::new();
    if app.input_mode {
        lower.push(Line::from(Span::styled(
            "Enter the password you recovered:",
            Style::default().fg(FG),
        )));
        lower.push(Line::from(""));
        lower.push(Line::from(vec![
            Span::styled("> ", Style::default().fg(ACCENT)),
            Span::styled(app.input.clone(), Style::default().fg(GOLD)),
            Span::styled("▏", Style::default().fg(ACCENT)),
        ]));
    } else if app.show_solution {
        lower.push(Line::from(Span::styled(
            "SPOILER — intended solution:",
            Style::default().fg(RED).add_modifier(Modifier::BOLD),
        )));
        lower.push(Line::from(""));
        for l in lvl.solution.trim().lines() {
            lower.push(Line::from(Span::styled(l.to_string(), Style::default().fg(FG))));
        }
    } else {
        let used = app.save.hints_used(lvl.n);
        let total = lvl.hints.len();
        lower.push(Line::from(Span::styled(
            format!("hints  ({used}/{total} revealed — press h for more)"),
            Style::default().fg(DIM),
        )));
        lower.push(Line::from(""));
        for (i, hint) in lvl.hints.iter().take(used).enumerate() {
            lower.push(Line::from(vec![
                Span::styled(format!("{}. ", i + 1), Style::default().fg(ACCENT)),
                Span::styled(hint.clone(), Style::default().fg(FG)),
            ]));
            lower.push(Line::from(""));
        }
        if used == 0 {
            lower.push(Line::from(Span::styled(
                "(no hints revealed yet)",
                Style::default().fg(DIM),
            )));
        }
    }
    let title = if app.show_solution { " solution " } else { " hints " };
    let lower_p = Paragraph::new(Text::from(lower))
        .wrap(Wrap { trim: false })
        .block(
            Block::bordered()
                .title(Span::styled(title, Style::default().fg(ACCENT)))
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(DIM)),
        );
    f.render_widget(lower_p, rows[1]);
}

fn render_help(f: &mut Frame, area: Rect, app: &App) {
    let keys = "↑/↓ move   ⏎ enter level   h hint   s solution   p password   r refresh   q quit";
    let text = Line::from(vec![
        Span::styled(format!(" {}  ", app.status), Style::default().fg(GOLD)),
    ]);
    let help = Line::from(Span::styled(keys, Style::default().fg(DIM)));
    let p = Paragraph::new(Text::from(vec![text, help]))
        .alignment(Alignment::Left)
        .block(
            Block::bordered()
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(DIM)),
        );
    f.render_widget(p, area);
}

fn short_title(t: &str) -> String {
    // strip the "Level X → Y — " prefix for a tidy list label
    t.rsplit("— ").next().unwrap_or(t).to_string()
}
