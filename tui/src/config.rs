//! Per-campaign configuration and theme, loaded from a campaign folder:
//!   campaigns/<name>/campaign.toml   - identity + world coordinates
//!   campaigns/<name>/theme.toml      - palette, chrome copy, splash, glitch
//!
//! Anything a theme omits falls back to neutral defaults, so the plain
//! "bandit" skin needs almost no theme file and a brand-new skin only
//! overrides what it wants to change.
use anyhow::{Context, Result};
use ratatui::style::Color;
use ratatui::widgets::BorderType;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Deserialize)]
pub struct CampaignConfig {
    pub name: String,
    #[serde(default)]
    pub story: String,
    pub user_prefix: String,
    #[serde(default)]
    pass_dir: String,
    #[serde(default)]
    container: String,
    #[serde(default)]
    image: String,
    #[serde(default = "default_max")]
    pub max_level: u32,
}

fn default_max() -> u32 {
    11
}

impl CampaignConfig {
    pub fn load(path: &Path) -> Result<Self> {
        let text = std::fs::read_to_string(path)
            .with_context(|| format!("reading {}", path.display()))?;
        toml::from_str(&text).with_context(|| format!("parsing {}", path.display()))
    }
    pub fn pass_dir(&self) -> String {
        if self.pass_dir.is_empty() {
            format!("/etc/{}_pass", self.user_prefix)
        } else {
            self.pass_dir.clone()
        }
    }
    pub fn container(&self) -> String {
        if self.container.is_empty() {
            format!("wargamezr-{}", self.name)
        } else {
            self.container.clone()
        }
    }
    #[allow(dead_code)]
    pub fn image(&self) -> String {
        if self.image.is_empty() {
            format!("wargamezr-{}:slice", self.name)
        } else {
            self.image.clone()
        }
    }
}

// ---- theme ---------------------------------------------------------------

#[derive(Debug, Deserialize, Default)]
#[allow(dead_code)]
pub struct Palette {
    #[serde(default)]
    accent: String,
    #[serde(default)]
    gold: String,
    #[serde(default)]
    green: String,
    #[serde(default)]
    red: String,
    #[serde(default)]
    fg: String,
    #[serde(default)]
    dim: String,
    #[serde(default)]
    hilite: String,
    #[serde(default)]
    bg: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct Glitch {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "g_prob")]
    pub prob: u64,
    #[serde(default = "g_period")]
    pub period: u64,
    #[serde(default = "g_burst")]
    pub burst: u64,
}
fn g_prob() -> u64 {
    40
}
fn g_period() -> u64 {
    37
}
fn g_burst() -> u64 {
    2
}
impl Default for Glitch {
    fn default() -> Self {
        Glitch { enabled: false, prob: g_prob(), period: g_period(), burst: g_burst() }
    }
}

#[derive(Debug, Deserialize, Default)]
#[allow(dead_code)]
pub struct Splash {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub hello: String,
    #[serde(default)]
    pub mask: String,
    #[serde(default)]
    pub wordmark: String,
    #[serde(default)]
    pub tagline: String,
    #[serde(default)]
    pub steps: Vec<String>,
    #[serde(default, rename = "final")]
    pub final_line: String,
}

#[derive(Debug, Deserialize, Default)]
pub struct Comms {
    #[serde(default)]
    pub handler: String,
    #[serde(default)]
    pub opening: Vec<String>,
    #[serde(default)]
    pub narration: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct Theme {
    #[serde(default = "default_border")]
    border: String,
    #[serde(default)]
    pub palette: Palette,
    #[serde(default)]
    pub glitch: Glitch,
    #[serde(default)]
    pub splash: Splash,
    #[serde(default)]
    pub comms: Comms,
    #[serde(default)]
    labels: HashMap<String, String>,
}
fn default_border() -> String {
    "rounded".into()
}
impl Default for Theme {
    fn default() -> Self {
        Theme {
            border: default_border(),
            palette: Palette::default(),
            glitch: Glitch::default(),
            splash: Splash::default(),
            comms: Comms::default(),
            labels: HashMap::new(),
        }
    }
}

impl Comms {
    pub fn handler_name(&self) -> String {
        if self.handler.is_empty() {
            "handler".into()
        } else {
            self.handler.clone()
        }
    }
}

fn hex_or(s: &str, fb: Color) -> Color {
    let s = s.trim().trim_start_matches('#');
    if s.len() == 6 {
        if let (Ok(r), Ok(g), Ok(b)) = (
            u8::from_str_radix(&s[0..2], 16),
            u8::from_str_radix(&s[2..4], 16),
            u8::from_str_radix(&s[4..6], 16),
        ) {
            return Color::Rgb(r, g, b);
        }
    }
    fb
}

#[allow(dead_code)]
impl Theme {
    pub fn load(path: &Path) -> Result<Self> {
        match std::fs::read_to_string(path) {
            Ok(text) => toml::from_str(&text)
                .with_context(|| format!("parsing {}", path.display())),
            Err(_) => Ok(Theme::default()), // no theme file -> neutral defaults
        }
    }

    // colors (neutral fallbacks if the palette omits a key)
    pub fn accent(&self) -> Color {
        hex_or(&self.palette.accent, Color::Rgb(217, 165, 33))
    }
    pub fn gold(&self) -> Color {
        hex_or(&self.palette.gold, self.accent())
    }
    pub fn green(&self) -> Color {
        hex_or(&self.palette.green, Color::Rgb(74, 222, 128))
    }
    pub fn red(&self) -> Color {
        hex_or(&self.palette.red, Color::Rgb(248, 113, 113))
    }
    pub fn fg(&self) -> Color {
        hex_or(&self.palette.fg, Color::Rgb(226, 232, 240))
    }
    pub fn dim(&self) -> Color {
        hex_or(&self.palette.dim, Color::Rgb(120, 133, 150))
    }
    pub fn hilite(&self) -> Color {
        hex_or(&self.palette.hilite, Color::Rgb(30, 41, 59))
    }
    pub fn bg(&self) -> Color {
        hex_or(&self.palette.bg, Color::Rgb(10, 10, 10))
    }

    pub fn border_type(&self) -> BorderType {
        match self.border.to_ascii_lowercase().as_str() {
            "plain" | "square" => BorderType::Plain,
            "double" => BorderType::Double,
            "thick" => BorderType::Thick,
            _ => BorderType::Rounded,
        }
    }

    /// A chrome string by key, falling back to a neutral default.
    pub fn lbl(&self, key: &str, default: &str) -> String {
        self.labels.get(key).cloned().unwrap_or_else(|| default.to_string())
    }
}
