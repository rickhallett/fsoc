//! Persistent player progress: which levels are solved and the treasure
//! (discovered passwords) collected so far. Stored as JSON in the user's
//! data dir so a run survives restarts.
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Save {
    /// levels the player has cleared (by level number)
    pub solved: Vec<u32>,
    /// discovered passwords, keyed by level number
    pub treasure: BTreeMap<u32, String>,
    /// how many hints the player has revealed per level
    pub hints_used: BTreeMap<u32, usize>,
}

impl Save {
    fn path() -> PathBuf {
        let base = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("wargamezr");
        let _ = std::fs::create_dir_all(&base);
        base.join("save.json")
    }

    pub fn load() -> Self {
        let p = Self::path();
        std::fs::read_to_string(&p)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn store(&self) -> Result<()> {
        let p = Self::path();
        std::fs::write(&p, serde_json::to_string_pretty(self)?)?;
        Ok(())
    }

    pub fn is_solved(&self, n: u32) -> bool {
        self.solved.contains(&n)
    }

    /// The intro (0) and first puzzle (1) are always open. Any later level
    /// is unlocked once the previous one is solved — because you need the
    /// previous level's treasure (a password) to log in for this one.
    pub fn is_unlocked(&self, n: u32) -> bool {
        n <= 1 || self.is_solved(n) || self.is_solved(n - 1)
    }

    pub fn mark_solved(&mut self, n: u32, password: String) {
        if !self.solved.contains(&n) {
            self.solved.push(n);
            self.solved.sort_unstable();
        }
        self.treasure.insert(n, password);
    }

    pub fn hints_used(&self, n: u32) -> usize {
        *self.hints_used.get(&n).unwrap_or(&0)
    }

    pub fn reveal_hint(&mut self, n: u32, max: usize) {
        let e = self.hints_used.entry(n).or_insert(0);
        if *e < max {
            *e += 1;
        }
    }
}
