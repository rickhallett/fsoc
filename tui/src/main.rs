//! wargamezr - a terminal-native front-end for a self-hosted, Bandit-style
//! Linux wargame, wearing an fsociety mask. The TUI is the frame: target
//! briefs, a leak ladder, a creds dump, and a door into a real shell.
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
use std::io::{stdout, Write};
use std::path::PathBuf;
use std::process::Command;
use std::thread::sleep;
use std::time::Duration;

// palette - Mr. Robot grade: near-black, dim bone, amber phosphor, fsociety red
const AMBER: Color = Color::Rgb(217, 165, 33);
const RED: Color = Color::Rgb(196, 33, 30);
const GREEN: Color = Color::Rgb(63, 185, 80);
const FG: Color = Color::Rgb(200, 200, 180);
const DIM: Color = Color::Rgb(92, 92, 82);
const HILITE_BG: Color = Color::Rgb(38, 32, 12);

const BORDER: BorderType = BorderType::Plain; // squared, cold, "real terminal"

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
    tick: u64,
}

impl App {
    fn new(campaign: Campaign, world: World) -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));
        let world_running = world.is_running();
        let status = if world_running {
            "the box is breathing. pick a target, jack in.".into()
        } else {
            "box is dark. `make world-up`, then hit r.".into()
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
            "the box is breathing.".into()
        } else {
            "box is dark. `make world-up`.".into()
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
            self.status = "target 00 has no creds - just jack in (enter).".into();
            self.input.clear();
            self.input_mode = false;
            return;
        }
        if !self.world_running {
            self.status = "box is dark - can't verify.".into();
            self.input_mode = false;
            return;
        }
        match self.world.verify(n, &self.input) {
            Ok(true) => {
                self.save.mark_solved(n, self.input.trim().to_string());
                let _ = self.save.store();
                self.status = format!("[ok] you're in. bandit{n} owned.");
            }
            Ok(false) => self.status = "that's not it. try again, friend.".into(),
            Err(e) => self.status = format!("verify error: {e}"),
        }
        self.input.clear();
        self.input_mode = false;
    }

    fn enter_level(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        let n = self.current().n;
        if !self.save.is_unlocked(n) {
            self.status = "locked. own the one before it first.".into();
            return Ok(());
        }
        if !self.world_running {
            self.status = "box is dark - run `make world-up` then hit r.".into();
            return Ok(());
        }
        let (cmd, args) = self.world.shell_command(n);
        // suspend the TUI, hand the terminal to the child, then restore.
        disable_raw_mode()?;
        execute!(stdout(), LeaveAlternateScreen)?;
        let who = if n == 0 {
            "bandit0 over ssh (password: bandit0)".to_string()
        } else {
            format!("bandit{} (target {n})", n - 1)
        };
        println!("\n\x1b[1;38;2;196;33;30m-- jacking into {who} --\x1b[0m");
        println!("\x1b[2m(pull the creds, then `exit` / Ctrl-D to bail)\x1b[0m\n");
        let _ = Command::new(&cmd).args(&args).status();
        enable_raw_mode()?;
        execute!(stdout(), EnterAlternateScreen)?;
        terminal.clear()?;
        self.refresh_world();
        self.status = format!("back topside. hit p to drop the creds you pulled off {who}.");
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

    intro();

    let mut app = App::new(campaign, world);
    let mut terminal = ratatui::init();
    let res = run(&mut terminal, &mut app);
    ratatui::restore();
    res
}

// ---- boot splash ----------------------------------------------------------

const MASK: &str = r#"
        .-"""""-.
      .'  _   _  '.
     /   (o) (o)   \
    :      .-.      :
    |     (   )     |
    :      '-'      :
     \   \_____/   /
      '.         .'
        '-.....-'
"#;

const WORDMARK: &str = "        f s o c i e t y";
const TAGLINE: &str = "        our democracy has been hacked.";

/// Typewriter-print a string, char by char.
fn type_out(s: &str, per_char_ms: u64) {
    let mut out = stdout();
    for ch in s.chars() {
        print!("{ch}");
        let _ = out.flush();
        sleep(Duration::from_millis(per_char_ms));
    }
}

/// The fsociety boot sequence. Skipped when WARGAMEZR_NO_INTRO is set.
fn intro() {
    if std::env::var("WARGAMEZR_NO_INTRO").is_ok() {
        return;
    }
    let amber = "\x1b[38;2;217;165;33m";
    let red = "\x1b[38;2;196;33;30m";
    let green = "\x1b[38;2;63;185;80m";
    let dim = "\x1b[38;2;92;92;82m";
    let rst = "\x1b[0m";

    print!("\x1b[2J\x1b[H\x1b[?25l"); // clear, home, hide cursor
    let _ = stdout().flush();

    type_out(&format!("{amber}hello, friend.{rst}\n"), 55);
    sleep(Duration::from_millis(500));

    for line in MASK.lines() {
        println!("{red}{line}{rst}");
        sleep(Duration::from_millis(35));
    }
    println!();
    println!("{amber}\x1b[1m{WORDMARK}{rst}");
    println!("{dim}{TAGLINE}{rst}");
    println!();

    let steps = [
        "establishing tor circuit",
        "spoofing mac address",
        "scanning target box",
        "opening a door",
    ];
    for s in steps {
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
    println!();
    type_out(&format!("{red}you're in.{rst}\n"), 55);
    sleep(Duration::from_millis(700));
    print!("\x1b[?25h"); // show cursor again
    let _ = stdout().flush();
}

// ---- glitch ---------------------------------------------------------------

/// Small deterministic PRNG (splitmix64) so glitches animate without a crate.
fn rng(seed: u64) -> u64 {
    let mut x = seed.wrapping_add(0x9E37_79B9_7F4A_7C15);
    x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^ (x >> 31)
}

/// Corrupt a few characters of `s`, `prob` percent per char. Spaces survive.
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

/// Look for the campaign TOML, walking up from the current dir.
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

/// Headless doctor: confirm the world is reachable and every provisioned
/// target has a recoverable password (exercises the Docker layer).
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
                println!("  level {:>2}  password reachable ({}...)", lvl.n, &pw[..4]);
            }
            Ok(_) => println!("  level {:>2}  password too short?", lvl.n),
            Err(_) => println!("  level {:>2}  not provisioned in this world build", lvl.n),
        }
    }
    println!("ok: {provisioned} targets provisioned and verifiable");
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
                app.status = "paste the creds, enter to submit, esc to bail.".into();
            }
            KeyCode::Enter => app.enter_level(terminal)?,
            _ => {}
        }
    }
    Ok(())
}

fn themed_block(title: &str) -> Block<'static> {
    Block::bordered()
        .title(Span::styled(
            format!(" {title} "),
            Style::default().fg(AMBER).add_modifier(Modifier::BOLD),
        ))
        .border_type(BORDER)
        .border_style(Style::default().fg(DIM))
}

fn ui(f: &mut Frame, app: &App) {
    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // header
            Constraint::Min(5),    // body
            Constraint::Length(4), // help (monologue + keys)
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
    // the brand flickers: corrupt "fsociety" for ~2 frames every ~7s
    let brand = if app.tick % 37 < 2 {
        glitch("fsociety", app.tick, 40)
    } else {
        "fsociety".to_string()
    };
    let (live_word, live_color) = if app.world_running {
        ("breathing", GREEN)
    } else {
        ("dark", RED)
    };
    let line = Line::from(vec![
        Span::styled(
            format!("  {brand} "),
            Style::default().fg(RED).add_modifier(Modifier::BOLD),
        ),
        Span::styled(format!("// {} ", app.campaign.campaign), Style::default().fg(FG)),
        Span::styled(format!("| {solved}/{total} owned "), Style::default().fg(GREEN)),
        Span::styled(format!("| {} creds ", app.save.treasure.len()), Style::default().fg(AMBER)),
        Span::styled("| box ".to_string(), Style::default().fg(DIM)),
        Span::styled(live_word, Style::default().fg(live_color)),
    ]);
    let p = Paragraph::new(line).block(
        Block::bordered()
            .border_type(BORDER)
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
                ("[x]", GREEN)
            } else if !unlocked {
                ("[-]", DIM)
            } else {
                ("[>]", AMBER)
            };
            let label = if lvl.n == 0 {
                "target 00 - get in".to_string()
            } else {
                format!("target {:02} - {}", lvl.n, short_title(&lvl.title))
            };
            ListItem::new(Line::from(vec![
                Span::styled(format!(" {glyph} "), Style::default().fg(color)),
                Span::styled(label, Style::default().fg(if unlocked { FG } else { DIM })),
            ]))
        })
        .collect();

    let list = List::new(items)
        .block(themed_block("targets"))
        .highlight_style(
            Style::default()
                .bg(HILITE_BG)
                .fg(AMBER)
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

    // --- the job ---
    let mut brief: Vec<Line> = Vec::new();
    // the job title flickers rarely
    let title = if app.tick % 53 < 1 {
        glitch(&lvl.title, app.tick, 25)
    } else {
        lvl.title.clone()
    };
    brief.push(Line::from(Span::styled(
        title,
        Style::default().fg(AMBER).add_modifier(Modifier::BOLD),
    )));
    brief.push(Line::from(""));
    for l in lvl.goal.trim().lines() {
        brief.push(Line::from(Span::styled(l.to_string(), Style::default().fg(FG))));
    }
    if !lvl.commands.is_empty() {
        brief.push(Line::from(""));
        brief.push(Line::from(vec![
            Span::styled("tools: ", Style::default().fg(DIM)),
            Span::styled(lvl.commands.join("  "), Style::default().fg(AMBER)),
        ]));
    }
    if let Some(pw) = app.save.treasure.get(&lvl.n) {
        brief.push(Line::from(""));
        brief.push(Line::from(vec![
            Span::styled("creds: ", Style::default().fg(GREEN)),
            Span::styled(pw.clone(), Style::default().fg(GREEN).add_modifier(Modifier::BOLD)),
        ]));
    }
    let brief_p = Paragraph::new(Text::from(brief))
        .wrap(Wrap { trim: false })
        .block(themed_block("the job"));
    f.render_widget(brief_p, rows[0]);

    // --- leaks / exploit / creds input ---
    let mut lower: Vec<Line> = Vec::new();
    if app.input_mode {
        let cur = if app.tick % 2 == 0 { "_" } else { " " };
        lower.push(Line::from(Span::styled(
            "// creds pulled off the box?",
            Style::default().fg(DIM),
        )));
        lower.push(Line::from(""));
        lower.push(Line::from(vec![
            Span::styled("root@target:~$ ", Style::default().fg(RED)),
            Span::styled(app.input.clone(), Style::default().fg(FG)),
            Span::styled(cur, Style::default().fg(AMBER)),
        ]));
    } else if app.show_solution {
        lower.push(Line::from(Span::styled(
            "// exploit [spoiler] - this is cheating, friend:",
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
            format!("// leaks  ({used}/{total} decrypted - press h for more)"),
            Style::default().fg(DIM),
        )));
        lower.push(Line::from(""));
        for (i, hint) in lvl.hints.iter().take(used).enumerate() {
            lower.push(Line::from(vec![
                Span::styled(format!("{}. ", i + 1), Style::default().fg(AMBER)),
                Span::styled(hint.clone(), Style::default().fg(FG)),
            ]));
            lower.push(Line::from(""));
        }
        if used == 0 {
            lower.push(Line::from(Span::styled(
                "nothing leaked yet. you're on your own, friend.",
                Style::default().fg(DIM),
            )));
        }
    }
    let title = if app.input_mode {
        "creds"
    } else if app.show_solution {
        "exploit"
    } else {
        "// leaks"
    };
    let lower_p = Paragraph::new(Text::from(lower))
        .wrap(Wrap { trim: false })
        .block(themed_block(title));
    f.render_widget(lower_p, rows[1]);
}

fn render_help(f: &mut Frame, area: Rect, app: &App) {
    let keys = "j/k move   enter jack in   h leak   s exploit   p creds   r rescan   q bail";
    let status = Line::from(Span::styled(
        format!(" {} ", app.status),
        Style::default().fg(AMBER),
    ));
    let help = Line::from(Span::styled(keys, Style::default().fg(DIM)));
    let p = Paragraph::new(Text::from(vec![status, help]))
        .alignment(Alignment::Left)
        .block(
            Block::bordered()
                .border_type(BORDER)
                .border_style(Style::default().fg(DIM)),
        );
    f.render_widget(p, area);
}

fn short_title(t: &str) -> String {
    // strip the "Level X -> Y - " prefix for a tidy list label
    t.rsplit(" - ").next().unwrap_or(t).to_string()
}
