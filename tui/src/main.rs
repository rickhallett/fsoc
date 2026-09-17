//! wargamezr - a terminal-native, self-hosted Linux wargame.
//!
//! The surface *is* a terminal: a real PTY into the game-world container,
//! rendered faithfully (the box's own colours) in a dark frame. You live at
//! the prompt and pivot node to node yourself (ssh). The story arrives as
//! comms from a handler and narration at the seams - no menus, no panels,
//! no progress bars. The tool recedes.
mod config;
mod model;
mod term;

use anyhow::{Context, Result};
use config::{CampaignConfig, Theme};
use model::{Campaign, Level};
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Paragraph, Wrap};
use ratatui::{DefaultTerminal, Frame};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;
use term::Term;

enum Kind {
    Handler,
    Narr,
    Sys,
}
struct Comm {
    kind: Kind,
    text: String,
}

struct App {
    config: CampaignConfig,
    campaign: Campaign,
    theme: Theme,
    term: Term,
    comms: Vec<Comm>,
    hints_used: HashMap<u32, usize>,
    current_user: Option<u32>,
    narr_idx: usize,
    tick: u64,
    quit: bool,
}

impl App {
    fn push(&mut self, kind: Kind, text: impl Into<String>) {
        self.comms.push(Comm { kind, text: text.into() });
        let n = self.comms.len();
        if n > 200 {
            self.comms.drain(0..n - 200);
        }
    }

    fn level_by_n(&self, n: u32) -> Option<&Level> {
        self.campaign.levels.iter().find(|l| l.n == n)
    }

    /// When logged in as user K, the job is to recover user K+1's creds -
    /// that's the level defined with n == K+1.
    fn job_level_n(&self) -> u32 {
        self.current_user.unwrap_or(0) + 1
    }

    fn emit_job(&mut self) {
        let n = self.job_level_n();
        if let Some(lvl) = self.level_by_n(n) {
            let title = short_title(&lvl.title);
            let goal: String = lvl
                .goal
                .trim()
                .lines()
                .take(2)
                .collect::<Vec<_>>()
                .join(" ");
            self.push(Kind::Sys, format!("-- objective: {title} --"));
            self.push(Kind::Handler, goal);
        }
    }

    fn on_user_change(&mut self, k: u32, first: bool) {
        self.current_user = Some(k);
        if first {
            let opening: Vec<String> = self.theme.comms.opening.clone();
            for line in opening {
                self.push(Kind::Handler, line);
            }
        } else {
            let narr = self.theme.comms.narration.clone();
            if !narr.is_empty() {
                let line = narr[self.narr_idx % narr.len()].clone();
                self.narr_idx += 1;
                self.push(Kind::Narr, line);
            }
            self.push(Kind::Sys, format!("-- you are now {}{} --", self.config.user_prefix, k));
        }
        self.emit_job();
    }

    fn reveal_hint(&mut self) {
        let n = self.job_level_n();
        let (total, hint) = match self.level_by_n(n) {
            Some(l) => (l.hints.len(), l.hints.clone()),
            None => (0, vec![]),
        };
        if total == 0 {
            return;
        }
        let used = self.hints_used.entry(n).or_insert(0);
        if *used < total {
            let h = hint[*used].clone();
            *used += 1;
            self.push(Kind::Handler, format!("(leak) {h}"));
        } else {
            self.push(Kind::Handler, "that's everything i've got. you're on your own.");
        }
    }

    fn show_job(&mut self) {
        self.push(Kind::Sys, "-- the job --");
        self.emit_job();
    }
}

fn main() -> Result<()> {
    let name = campaign_name();
    let base = campaign_dir(&name)
        .with_context(|| format!("locating campaign folder for '{name}'"))?;
    let config = CampaignConfig::load(&base.join("campaign.toml"))?;
    let campaign = Campaign::load(&base.join("levels.toml"))?;
    let theme = Theme::load(&base.join("theme.toml"))?;

    if std::env::args().any(|a| a == "--check") {
        return check(&config, &campaign);
    }

    if !world_running(&config.container()) {
        anyhow::bail!(
            "world '{}' is not running.\nstart it first:  make world-up CAMPAIGN={}",
            config.container(),
            config.name
        );
    }

    let mut terminal = ratatui::init();
    let res = run(&mut terminal, config, campaign, theme);
    ratatui::restore();
    res
}

fn run(
    terminal: &mut DefaultTerminal,
    config: CampaignConfig,
    campaign: Campaign,
    theme: Theme,
) -> Result<()> {
    let size = terminal.size()?;
    let area = Rect::new(0, 0, size.width, size.height);
    let (_, term_rect, _) = content_rects(area);
    let user0 = format!("{}0", config.user_prefix);
    let term = Term::spawn(
        &config.container(),
        &user0,
        &format!("/home/{user0}"),
        term_rect.height.max(1),
        term_rect.width.max(1),
    )?;

    let mut app = App {
        config,
        campaign,
        theme,
        term,
        comms: Vec::new(),
        hints_used: HashMap::new(),
        current_user: None,
        narr_idx: 0,
        tick: 0,
        quit: false,
    };

    loop {
        app.tick = app.tick.wrapping_add(1);
        app.term.pump();

        // learn which node we're on from the live prompt; announce pivots.
        let contents = app.term.screen().contents();
        if let Some(k) = find_user(&contents, &app.config.user_prefix) {
            if app.current_user != Some(k) {
                let first = app.current_user.is_none();
                app.on_user_change(k, first);
            }
        }

        // keep the PTY sized to the visible canvas
        let a = Rect::new(0, 0, terminal.size()?.width, terminal.size()?.height);
        let (_, tr, _) = content_rects(a);
        app.term.resize(tr.height.max(1), tr.width.max(1));

        terminal.draw(|f| ui(f, &app))?;

        if app.quit || app.term.is_dead() {
            break;
        }

        if event::poll(Duration::from_millis(20))? {
            match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => handle_key(&mut app, key),
                _ => {}
            }
        }
    }
    Ok(())
}

fn handle_key(app: &mut App, key: ratatui::crossterm::event::KeyEvent) {
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    // meta keys for the tool - intercepted, never sent to the shell
    match key.code {
        KeyCode::F(10) => {
            app.quit = true;
            return;
        }
        KeyCode::Char('g') if ctrl => {
            // panic-escape: works even if a full-screen program is wedged
            app.quit = true;
            return;
        }
        KeyCode::F(1) => {
            app.reveal_hint();
            return;
        }
        KeyCode::F(2) => {
            app.show_job();
            return;
        }
        _ => {}
    }
    if let Some(bytes) = key_to_bytes(&key) {
        app.term.send(&bytes);
    }
}

fn key_to_bytes(key: &ratatui::crossterm::event::KeyEvent) -> Option<Vec<u8>> {
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    Some(match key.code {
        KeyCode::Char(c) if ctrl => {
            let b = (c.to_ascii_uppercase() as u8) & 0x1f;
            vec![b]
        }
        KeyCode::Char(c) => c.to_string().into_bytes(),
        KeyCode::Enter => vec![b'\r'],
        KeyCode::Backspace => vec![0x7f],
        KeyCode::Tab => vec![b'\t'],
        KeyCode::BackTab => vec![0x1b, b'[', b'Z'],
        KeyCode::Esc => vec![0x1b],
        KeyCode::Left => vec![0x1b, b'[', b'D'],
        KeyCode::Right => vec![0x1b, b'[', b'C'],
        KeyCode::Up => vec![0x1b, b'[', b'A'],
        KeyCode::Down => vec![0x1b, b'[', b'B'],
        KeyCode::Home => vec![0x1b, b'[', b'H'],
        KeyCode::End => vec![0x1b, b'[', b'F'],
        KeyCode::PageUp => vec![0x1b, b'[', b'5', b'~'],
        KeyCode::PageDown => vec![0x1b, b'[', b'6', b'~'],
        KeyCode::Delete => vec![0x1b, b'[', b'3', b'~'],
        _ => return None,
    })
}

/// Read the current node from the *prompt* - the last line that begins with
/// `<prefix><digits>@`. Anchoring to the line start means a typed command like
/// `ssh node1@localhost` doesn't count (that `node1@` isn't at column 0); only
/// the shell prompt does, so the objective changes only when you actually land.
fn find_user(contents: &str, prefix: &str) -> Option<u32> {
    let mut last = None;
    for line in contents.lines() {
        let s = line.trim_start();
        if let Some(after) = s.strip_prefix(prefix) {
            let digits: String = after.chars().take_while(|c| c.is_ascii_digit()).collect();
            if !digits.is_empty() && after[digits.len()..].starts_with('@') {
                if let Ok(n) = digits.parse::<u32>() {
                    last = Some(n);
                }
            }
        }
    }
    last
}

fn content_rects(area: Rect) -> (Rect, Rect, Rect) {
    let mx = (area.width / 12).clamp(3, 16);
    let inner = Rect {
        x: area.x + mx,
        y: area.y + 1,
        width: area.width.saturating_sub(2 * mx),
        height: area.height.saturating_sub(2),
    };
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4), // comms
            Constraint::Length(1), // air
            Constraint::Min(1),    // terminal
            Constraint::Length(1), // status
        ])
        .split(inner);
    (rows[0], rows[2], rows[3])
}

fn ui(f: &mut Frame, app: &App) {
    let t = &app.theme;
    // dark frame everywhere
    f.render_widget(Block::default().style(Style::default().bg(t.bg())), f.area());

    let (comms_rect, term_rect, status_rect) = content_rects(f.area());
    render_comms(f, comms_rect, app);
    render_term(f, term_rect, app);
    render_status(f, status_rect, app);
}

fn render_comms(f: &mut Frame, area: Rect, app: &App) {
    let t = &app.theme;
    let handler = t.comms.handler_name();
    let take = area.height as usize;
    let start = app.comms.len().saturating_sub(take);
    let mut lines: Vec<Line> = Vec::new();
    for c in &app.comms[start..] {
        let (prefix, style) = match c.kind {
            Kind::Handler => (
                format!("<{handler}> "),
                Style::default().fg(t.accent()),
            ),
            Kind::Narr => ("// ".to_string(), Style::default().fg(t.dim()).add_modifier(Modifier::ITALIC)),
            Kind::Sys => (String::new(), Style::default().fg(t.dim())),
        };
        lines.push(Line::from(vec![
            Span::styled(prefix, style),
            Span::styled(c.text.clone(), style),
        ]));
    }
    let p = Paragraph::new(Text::from(lines))
        .wrap(Wrap { trim: true })
        .style(Style::default().bg(t.bg()));
    f.render_widget(p, area);
}

/// Convert a vt100 colour to a ratatui one, mapping "default" to the theme.
fn conv(c: vt100::Color, dflt: Color) -> Color {
    match c {
        vt100::Color::Default => dflt,
        vt100::Color::Idx(i) => Color::Indexed(i),
        vt100::Color::Rgb(r, g, b) => Color::Rgb(r, g, b),
    }
}

fn render_term(f: &mut Frame, area: Rect, app: &App) {
    let t = &app.theme;
    let screen = app.term.screen();
    let (rows, cols) = screen.size();
    let (crow, ccol) = screen.cursor_position();
    let blink = (app.tick / 15) % 2 == 0;

    let mut lines: Vec<Line> = Vec::with_capacity(rows as usize);
    for r in 0..rows {
        let mut spans: Vec<Span> = Vec::new();
        let mut run = String::new();
        let mut run_style: Option<Style> = None;

        for c in 0..cols {
            let cell = screen.cell(r, c);
            let ch = cell
                .map(|c| {
                    let s = c.contents();
                    if s.is_empty() {
                        " ".to_string()
                    } else {
                        s
                    }
                })
                .unwrap_or_else(|| " ".to_string());

            // faithfully carry the box's own colours/attrs, graded to the theme
            let mut fg = cell.map(|c| conv(c.fgcolor(), t.fg())).unwrap_or_else(|| t.fg());
            let mut bg = cell.map(|c| conv(c.bgcolor(), t.bg())).unwrap_or_else(|| t.bg());
            let mut modifier = Modifier::empty();
            if let Some(c) = cell {
                if c.inverse() {
                    std::mem::swap(&mut fg, &mut bg);
                }
                if c.bold() {
                    modifier |= Modifier::BOLD;
                }
                if c.italic() {
                    modifier |= Modifier::ITALIC;
                }
                if c.underline() {
                    modifier |= Modifier::UNDERLINED;
                }
            }
            let is_cursor = blink && r == crow && c == ccol;
            let style = if is_cursor {
                Style::default().fg(t.bg()).bg(t.accent())
            } else {
                Style::default().fg(fg).bg(bg).add_modifier(modifier)
            };

            if run_style != Some(style) {
                if let Some(s) = run_style.take() {
                    spans.push(Span::styled(std::mem::take(&mut run), s));
                }
                run_style = Some(style);
            }
            run.push_str(&ch);
        }
        if let Some(s) = run_style {
            spans.push(Span::styled(run, s));
        }
        lines.push(Line::from(spans));
    }
    let p = Paragraph::new(Text::from(lines)).style(Style::default().bg(t.bg()));
    f.render_widget(p, area);
}

fn render_status(f: &mut Frame, area: Rect, app: &App) {
    let t = &app.theme;
    let node = app
        .current_user
        .map(|k| format!("{}{}", app.config.user_prefix, k))
        .unwrap_or_else(|| "...".into());
    let line = Line::from(Span::styled(
        format!("{node}  .  {}  .  F1 leak  .  F2 job  .  ^G quit  .  or type `exit`", app.campaign.campaign),
        Style::default().fg(t.dim()),
    ));
    f.render_widget(
        Paragraph::new(line).style(Style::default().bg(t.bg())),
        area,
    );
}

// ---- helpers --------------------------------------------------------------

fn world_running(container: &str) -> bool {
    Command::new("docker")
        .args(["inspect", "-f", "{{.State.Running}}", container])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim() == "true")
        .unwrap_or(false)
}

fn campaign_name() -> String {
    let args: Vec<String> = std::env::args().collect();
    for (i, a) in args.iter().enumerate() {
        if a == "--campaign" {
            if let Some(v) = args.get(i + 1) {
                return v.clone();
            }
        }
        if let Some(v) = a.strip_prefix("--campaign=") {
            return v.to_string();
        }
    }
    std::env::var("WARGAMEZR_CAMPAIGN").unwrap_or_else(|_| "fsociety".into())
}

fn campaign_dir(name: &str) -> Result<PathBuf> {
    let candidates = [
        PathBuf::from(format!("campaigns/{name}")),
        PathBuf::from(format!("../campaigns/{name}")),
        PathBuf::from(format!(concat!(env!("CARGO_MANIFEST_DIR"), "/../campaigns/{}"), name)),
    ];
    for c in candidates {
        if c.join("levels.toml").exists() {
            return Ok(c);
        }
    }
    anyhow::bail!("could not find campaigns/{name}/ (run from the repo root)")
}

/// Headless doctor: confirm the world is reachable and every provisioned
/// user has a recoverable password.
fn check(config: &CampaignConfig, campaign: &Campaign) -> Result<()> {
    println!("wargamezr --check  (campaign: {})", config.name);
    if !config.story.is_empty() {
        println!("story    : {}", config.story);
    }
    println!(
        "levels   : {} ({} defined, world provisions 0..{})",
        campaign.campaign,
        campaign.levels.len(),
        config.max_level
    );
    let container = config.container();
    print!("world    : {container} ... ");
    if !world_running(&container) {
        println!("NOT RUNNING");
        anyhow::bail!("start the world first: make world-up CAMPAIGN={}", config.name);
    }
    println!("running");
    let mut provisioned = 0u32;
    for lvl in &campaign.levels {
        if lvl.n == 0 {
            continue;
        }
        let user = format!("{}{}", config.user_prefix, lvl.n);
        let out = Command::new("docker")
            .args(["exec", &container, "cat", &format!("{}/{user}", config.pass_dir())])
            .output()?;
        if out.status.success() && String::from_utf8_lossy(&out.stdout).trim().len() >= 8 {
            provisioned += 1;
            let pw = String::from_utf8_lossy(&out.stdout);
            println!("  {user:<10} reachable ({}...)", &pw.trim()[..4]);
        } else {
            println!("  {user:<10} not provisioned in this world build");
        }
    }
    println!("ok: {provisioned} users provisioned and verifiable");
    Ok(())
}

fn short_title(t: &str) -> String {
    t.rsplit(" - ").next().unwrap_or(t).to_string()
}
