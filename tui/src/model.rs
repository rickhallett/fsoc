//! Level content, loaded from the campaign TOML.
use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
pub struct Level {
    pub n: u32,
    pub title: String,
    pub goal: String,
    #[serde(default)]
    pub commands: Vec<String>,
    #[serde(default)]
    pub hints: Vec<String>,
    #[serde(default)]
    pub solution: String,
}

#[derive(Debug, Deserialize)]
pub struct Campaign {
    pub campaign: String,
    #[serde(rename = "level")]
    pub levels: Vec<Level>,
}

impl Campaign {
    pub fn load(path: &Path) -> Result<Self> {
        let text = std::fs::read_to_string(path)
            .with_context(|| format!("reading campaign file {}", path.display()))?;
        let mut c: Campaign = toml::from_str(&text)
            .with_context(|| format!("parsing campaign file {}", path.display()))?;
        c.levels.sort_by_key(|l| l.n);
        Ok(c)
    }

    /// The highest level index this campaign defines content for.
    #[allow(dead_code)]
    pub fn max_n(&self) -> u32 {
        self.levels.iter().map(|l| l.n).max().unwrap_or(0)
    }
}
