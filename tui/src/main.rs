//! wargamezr - a terminal-native front-end for a self-hosted, Bandit-style
//! Linux wargame. The mechanics live in a Docker world; the narrative and
//! look are a swappable "campaign" (a folder of levels + theme). The plain
//! `bandit` skin and the `fsociety` (Mr. Robot) skin drive the same world.
mod config;
mod docker;
mod model;
mod save;

use anyhow::{Context, Result};
use config::{CampaignConfig, Theme};
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
use ratatui::widgets::{Block, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::{DefaultTerminal, Frame};
use save::Save;
use std::io::{stdout, Write};
use std::path::PathBuf;
use std::process::Command;
use std::thread::sleep;
use std::time::Duration;

struct App {
    campaign: Campaign,
    theme: Theme,
    prefix: String,
    word_target: String,
    save: Save,
    world: World,
    world_running: bool,
    selected: usize,
    list_state: ListState,
    input_mode: bool,
    input: String,
    show_solution: bool,
    status: String,
    tick: u64,
}

impl App {
    fn new(campaign: Campaign, theme: Theme, world: World, prefix: String) -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));
        let world_running = world.is_running();
        let word_target = theme.lbl("word_target", "level");
        let status = if world_running {
            theme.lbl("status_up", "world is up. pick a level, press enter.")
        } else {
            theme.lbl("status_down", "world not running. `make world-up`, then r.")
        };
        App {
            campaign,
            theme,
            prefix,
            word_target,
            save: Save::load(),
            world,
            world_running,
            selected: 0,
            list_state,
            input_mode: false,
            input: String::new(),
            show_solution: false,
            status,
            tick: 0,
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
            self.theme.lbl("status_up", "world is up.")
        } else {
            self.theme.lbl("status_down", "world not running. `make world-up`.")
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
            self.status = self.theme.lbl("status_level0", "level 0 has no password - just log in (enter).");
            self.input.clear();
            self.input_mode = false;
            return;
        }
        if !self.world_running {
            self.status = self.theme.lbl("status_no_verify", "world not running - can't verify.");
            self.input_mode = false;
            return;
        }
        match self.world.verify(n, &self.input) {
            Ok(true) => {
                self.save.mark_solved(n, self.input.trim().to_string());
                let _ = self.save.store();
                let user = format!("{}{}", self.prefix, n);
                self.status = self
                    .theme
                    .lbl("status_correct", "[ok] correct. {user} cleared.")
                    .replace("{user}", &user);
            }
            Ok(false) => {
                self.status = self.theme.lbl("status_wrong", "wrong password. keep digging.")
            }
            Err(e) => self.status = format!("verify error: {e}"),
        }
        self.input.clear();
        self.input_mode = false;
    }

    fn who(&self, n: u32) -> String {
        if n == 0 {
            format!("{p}0 over ssh (password: {p}0)", p = self.prefix)
        } else {
            format!("{}{} ({} {n})", self.prefix, n - 1, self.word_target)
        }
    }

    fn enter_level(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        let n = self.current().n;
        if !self.save.is_unlocked(n) {
            self.status = self.theme.lbl("status_locked", "locked. clear the previous one first.");
            return Ok(());
        }
        if !self.world_running {
            self.status = self.theme.lbl("status_no_verify", "world not running - run `make world-up`.");
            return Ok(());
        }
        let (cmd, args) = self.world.shell_command(n);
        let who = self.who(n);
        disable_raw_mode()?;
        execute!(stdout(), LeaveAlternateScreen)?;
        let banner = self
            .theme
            .lbl("enter_banner", "-- entering {who} --")
            .replace("{who}", &who);
        println!("\n{}\x1b[1m{banner}\x1b[0m", ansi_fg(self.theme.red()));
        println!("\x1b[2m(find the next password, then `exit` / Ctrl-D to return)\x1b[0m\n");
        let _ = Command::new(&cmd).args(&args).status();
        enable_raw_mode()?;
        execute!(stdout(), EnterAlternateScreen)?;
        terminal.clear()?;
        self.refresh_world();
        self.status = self
            .theme
            .lbl("status_back", "back. press p to enter the password you found.")
            .replace("{who}", &who);
        Ok(())
    }
}

fn main() -> Result<()> {
    let name = campaign_name();
    let base = campaign_dir(&name)
        .with_context(|| format!("locating campaign folder for '{name}'"))?;
    let config = CampaignConfig::load(&base.join("campaign.toml"))?;
    let campaign = Campaign::load(&base.join("levels.toml"))?;
    let theme = Theme::load(&base.join("theme.toml"))?;
    let world = World::new(config.container(), &config.user_prefix, config.pass_dir());

    if std::env::args().any(|a| a == "--check") {
        return check(&config, &campaign, &world);
    }

    intro(&theme);

    let mut app = App::new(campaign, theme, world, config.user_prefix.clone());
    let mut terminal = ratatui::init();
    let res = run(&mut terminal, &mut app);
    ratatui::restore();
    res
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

fn ansi_fg(c: Color) -> String {
    if let Color::Rgb(r, g, b) = c {
        format!("\x1b[38;2;{r};{g};{b}m")
    } else {
        String::new()
    }
}

// ---- boot splash ----------------------------------------------------------

fn type_out(s: &str, per_char_ms: u64) {
    let mut out = stdout();
    for ch in s.chars() {
        print!("{ch}");
        let _ = out.flush();
        sleep(Duration::from_millis(per_char_ms));
    }
}

/// The campaign's boot splash. No-op unless the theme enables it (and
/// WARGAMEZR_NO_INTRO is unset).
fn intro(theme: &Theme) {
    if !theme.splash.enabled || std::env::var("WARGAMEZR_NO_INTRO").is_ok() {
        return;
    }
    let sp = &theme.splash;
    let amber = ansi_fg(theme.accent());
    let red = ansi_fg(theme.red());
    let green = ansi_fg(theme.green());
    let dim = ansi_fg(theme.dim());
    let rst = "\x1b[0m";

    print!("\x1b[2J\x1b[H\x1b[?25l");
    let _ = stdout().flush();

    if !sp.hello.is_empty() {
        type_out(&format!("{amber}{}{rst}\n", sp.hello), 55);
        sleep(Duration::from_millis(500));
    }
    for line in sp.mask.lines() {
        println!("{red}{line}{rst}");
        sleep(Duration::from_millis(35));
    }
    if !sp.wordmark.is_empty() {
        println!();
        println!("{amber}\x1b[1m{}{rst}", sp.wordmark);
    }
    if !sp.tagline.is_empty() {
        println!("{dim}{}{rst}", sp.tagline);
    }
    println!();
    for s in &sp.steps {
        print!("{dim} > {s}");
        let _ = stdout().flush();
        for _ in 0..8 {
            print!(".");
            let _ = stdout().flush();
            sleep(Duration::from_millis(70));
        }
        println!(" {green}ok{rst}");
        sleep(Duration::from_millis(120));
    }
    if !sp.final_line.is_empty() {
        println!();
        type_out(&format!("{red}{}{rst}\n", sp.final_line), 55);
        sleep(Duration::from_millis(700));
    }
    print!("\x1b[?25h");
    let _ = stdout().flush();
}

// ---- glitch ---------------------------------------------------------------

fn rng(seed: u64) -> u64 {
    let mut x = seed.wrapping_add(0x9E37_79B9_7F4A_7C15);
    x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^ (x >> 31)
}

fn glitch(s: &str, seed: u64, prob: u64) -> String {
    const G: &[u8] = b"!@#$%&*<>/\\|=+~^";
    s.chars()
        .enumerate()
        .map(|(i, c)| {
            if c == ' ' {
                return c;
            }
            let r = rng(seed ^ (i as u64).wrapping_mul(0x0100_0000_01B3));
            if r % 100 < prob {
                G[(r as usize / 7) % G.len()] as char
            } else {
                c
            }
        })
        .collect()
}

/// Apply the theme's glitch to `s` when a burst is active on this tick.
fn maybe_glitch(app: &App, s: &str) -> String {
    let g = &app.theme.glitch;
    if g.enabled && g.period > 0 && app.tick % g.period < g.burst {
        glitch(s, app.tick, g.prob)
    } else {
        s.to_string()
    }
}

/// Headless doctor: confirm the world is reachable and every provisioned
/// user has a recoverable password (exercises the Docker layer).
fn check(config: &CampaignConfig, campaign: &Campaign, world: &World) -> Result<()> {
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
    print!("world    : {} ... ", world.container);
    if !world.is_running() {
        println!("NOT RUNNING");
        anyhow::bail!("start the world first: make world-up CAMPAIGN={}", config.name);
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
                println!("  {}{:<2}  password reachable ({}...)", config.user_prefix, lvl.n, &pw[..4]);
            }
            Ok(_) => println!("  {}{:<2}  password too short?", config.user_prefix, lvl.n),
            Err(_) => println!("  {}{:<2}  not provisioned in this world build", config.user_prefix, lvl.n),
        }
    }
    println!("ok: {provisioned} users provisioned and verifiable");
    Ok(())
}

fn run(terminal: &mut DefaultTerminal, app: &mut App) -> Result<()> {
    loop {
        app.tick = app.tick.wrapping_add(1);
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
                app.status = app.theme.lbl("status_enter_creds", "type the password, enter to submit, esc to cancel.");
            }
            KeyCode::Enter => app.enter_level(terminal)?,
            _ => {}
        }
    }
    Ok(())
}

fn themed_block(app: &App, title: &str) -> Block<'static> {
    Block::bordered()
        .title(Span::styled(
            format!(" {title} "),
            Style::default().fg(app.theme.accent()).add_modifier(Modifier::BOLD),
        ))
        .border_type(app.theme.border_type())
        .border_style(Style::default().fg(app.theme.dim()))
}

fn ui(f: &mut Frame, app: &App) {
    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(5),
            Constraint::Length(4),
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
    let t = &app.theme;
    let total = app.campaign.levels.len().saturating_sub(1);
    let solved = app.save.solved.len();
    let brand = maybe_glitch(app, &t.lbl("brand", "wargamezr"));
    let (live_word, live_color) = if app.world_running {
        (t.lbl("box_live", "up"), t.green())
    } else {
        (t.lbl("box_dead", "down"), t.red())
    };
    let line = Line::from(vec![
        Span::styled(format!("  {brand} "), Style::default().fg(t.red()).add_modifier(Modifier::BOLD)),
        Span::styled(format!("// {} ", app.campaign.campaign), Style::default().fg(t.fg())),
        Span::styled(
            format!("| {solved}/{total} {} ", t.lbl("word_owned", "cleared")),
            Style::default().fg(t.green()),
        ),
        Span::styled(
            format!("| {} {} ", app.save.treasure.len(), t.lbl("word_creds", "treasure")),
            Style::default().fg(t.gold()),
        ),
        Span::styled("| box ".to_string(), Style::default().fg(t.dim())),
        Span::styled(live_word, Style::default().fg(live_color)),
    ]);
    let p = Paragraph::new(line).block(
        Block::bordered()
            .border_type(t.border_type())
            .border_style(Style::default().fg(t.dim())),
    );
    f.render_widget(p, area);
}

fn render_level_list(f: &mut Frame, area: Rect, app: &App) {
    let t = &app.theme;
    let word = t.lbl("word_target", "level");
    let get_in = t.lbl("get_in_label", "get in");
    let items: Vec<ListItem> = app
        .campaign
        .levels
        .iter()
        .map(|lvl| {
            let unlocked = app.save.is_unlocked(lvl.n);
            let solved = app.save.is_solved(lvl.n);
            let (glyph, color) = if solved {
                ("[x]", t.green())
            } else if !unlocked {
                ("[-]", t.dim())
            } else {
                ("[>]", t.accent())
            };
            let label = if lvl.n == 0 {
                format!("{word} 00 - {get_in}")
            } else {
                format!("{word} {:02} - {}", lvl.n, short_title(&lvl.title))
            };
            ListItem::new(Line::from(vec![
                Span::styled(format!(" {glyph} "), Style::default().fg(color)),
                Span::styled(label, Style::default().fg(if unlocked { t.fg() } else { t.dim() })),
            ]))
        })
        .collect();

    let list = List::new(items)
        .block(themed_block(app, &t.lbl("panel_targets", "levels")))
        .highlight_style(
            Style::default()
                .bg(t.hilite())
                .fg(t.accent())
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("");

    let mut state = app.list_state.clone();
    f.render_stateful_widget(list, area, &mut state);
}

fn render_detail(f: &mut Frame, area: Rect, app: &App) {
    let t = &app.theme;
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    let lvl = app.current();

    // --- the job / brief ---
    let mut brief: Vec<Line> = Vec::new();
    brief.push(Line::from(Span::styled(
        maybe_glitch(app, &lvl.title),
        Style::default().fg(t.gold()).add_modifier(Modifier::BOLD),
    )));
    brief.push(Line::from(""));
    for l in lvl.goal.trim().lines() {
        brief.push(Line::from(Span::styled(l.to_string(), Style::default().fg(t.fg()))));
    }
    if !lvl.commands.is_empty() {
        brief.push(Line::from(""));
        brief.push(Line::from(vec![
            Span::styled(format!("{} ", t.lbl("tools_label", "useful:")), Style::default().fg(t.dim())),
            Span::styled(lvl.commands.join("  "), Style::default().fg(t.accent())),
        ]));
    }
    if let Some(pw) = app.save.treasure.get(&lvl.n) {
        brief.push(Line::from(""));
        brief.push(Line::from(vec![
            Span::styled(format!("{} ", t.lbl("creds_label", "treasure:")), Style::default().fg(t.green())),
            Span::styled(pw.clone(), Style::default().fg(t.green()).add_modifier(Modifier::BOLD)),
        ]));
    }
    let brief_p = Paragraph::new(Text::from(brief))
        .wrap(Wrap { trim: false })
        .block(themed_block(app, &t.lbl("panel_job", "brief")));
    f.render_widget(brief_p, rows[0]);

    // --- leaks / exploit / creds input ---
    let mut lower: Vec<Line> = Vec::new();
    let title;
    if app.input_mode {
        let cur = if app.tick % 2 == 0 { "_" } else { " " };
        lower.push(Line::from(Span::styled(
            t.lbl("creds_prompt_hint", "enter the password you recovered:"),
            Style::default().fg(t.dim()),
        )));
        lower.push(Line::from(""));
        lower.push(Line::from(vec![
            Span::styled(t.lbl("creds_prompt", "> "), Style::default().fg(t.red())),
            Span::styled(app.input.clone(), Style::default().fg(t.fg())),
            Span::styled(cur, Style::default().fg(t.accent())),
        ]));
        title = t.lbl("panel_creds", "password");
    } else if app.show_solution {
        lower.push(Line::from(Span::styled(
            t.lbl("exploit_header", "SPOILER - intended solution:"),
            Style::default().fg(t.red()).add_modifier(Modifier::BOLD),
        )));
        lower.push(Line::from(""));
        for l in lvl.solution.trim().lines() {
            lower.push(Line::from(Span::styled(l.to_string(), Style::default().fg(t.fg()))));
        }
        title = t.lbl("panel_exploit", "solution");
    } else {
        let used = app.save.hints_used(lvl.n);
        let total = lvl.hints.len();
        let header = t
            .lbl("leaks_header", "hints ({used}/{total} revealed - press h for more)")
            .replace("{used}", &used.to_string())
            .replace("{total}", &total.to_string());
        lower.push(Line::from(Span::styled(header, Style::default().fg(t.dim()))));
        lower.push(Line::from(""));
        for (i, hint) in lvl.hints.iter().take(used).enumerate() {
            lower.push(Line::from(vec![
                Span::styled(format!("{}. ", i + 1), Style::default().fg(t.accent())),
                Span::styled(hint.clone(), Style::default().fg(t.fg())),
            ]));
            lower.push(Line::from(""));
        }
        if used == 0 {
            lower.push(Line::from(Span::styled(
                t.lbl("leaks_empty", "(no hints revealed yet)"),
                Style::default().fg(t.dim()),
            )));
        }
        title = t.lbl("panel_leaks", "hints");
    }
    let lower_p = Paragraph::new(Text::from(lower))
        .wrap(Wrap { trim: false })
        .block(themed_block(app, &title));
    f.render_widget(lower_p, rows[1]);
}

fn render_help(f: &mut Frame, area: Rect, app: &App) {
    let t = &app.theme;
    let keys = t.lbl(
        "keys",
        "j/k move   enter open   h hint   s solution   p password   r refresh   q quit",
    );
    let status = Line::from(Span::styled(format!(" {} ", app.status), Style::default().fg(t.gold())));
    let help = Line::from(Span::styled(keys, Style::default().fg(t.dim())));
    let p = Paragraph::new(Text::from(vec![status, help]))
        .alignment(Alignment::Left)
        .block(
            Block::bordered()
                .border_type(t.border_type())
                .border_style(Style::default().fg(t.dim())),
        );
    f.render_widget(p, area);
}

fn short_title(t: &str) -> String {
    t.rsplit(" - ").next().unwrap_or(t).to_string()
}
