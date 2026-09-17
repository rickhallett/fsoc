//! Thin wrapper over the `docker` CLI for talking to the game-world
//! container. The container runs real bandit users; the TUI shells into
//! them and verifies passwords against the world.
use anyhow::{anyhow, Result};
use std::process::{Command, Stdio};

pub struct World {
    pub container: String,
}

impl World {
    pub fn new(container: impl Into<String>) -> Self {
        World { container: container.into() }
    }

    /// Is the container up and reachable?
    pub fn is_running(&self) -> bool {
        Command::new("docker")
            .args(["inspect", "-f", "{{.State.Running}}", &self.container])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim() == "true")
            .unwrap_or(false)
    }

    /// The stored password for banditN, read as root inside the world.
    /// Used to verify a player's answer and to unlock progression.
    pub fn password_of(&self, n: u32) -> Result<String> {
        let out = Command::new("docker")
            .args([
                "exec",
                &self.container,
                "cat",
                &format!("/etc/bandit_pass/bandit{n}"),
            ])
            .output()?;
        if !out.status.success() {
            return Err(anyhow!(
                "could not read password for bandit{n}: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            ));
        }
        Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
    }

    /// Verify a candidate password against the world.
    pub fn verify(&self, n: u32, candidate: &str) -> Result<bool> {
        Ok(self.password_of(n)?.trim() == candidate.trim())
    }

    /// Build the argv that drops the player into the shell for a level.
    ///
    /// Level `n` (n >= 1) is solved while logged in as bandit(n-1) — that
    /// stage's goal is to recover bandit n's password. Level 0 is the
    /// bootstrap "get in over SSH" tutorial, so it launches a real SSH
    /// session as bandit0 (password: bandit0).
    pub fn shell_command(&self, n: u32) -> (String, Vec<String>) {
        if n == 0 {
            (
                "ssh".to_string(),
                vec![
                    "-o".into(),
                    "StrictHostKeyChecking=no".into(),
                    "-o".into(),
                    "UserKnownHostsFile=/dev/null".into(),
                    "-p".into(),
                    "2220".into(),
                    "bandit0@localhost".into(),
                ],
            )
        } else {
            let user = format!("bandit{}", n - 1);
            (
                "docker".to_string(),
                vec![
                    "exec".into(),
                    "-it".into(),
                    "-u".into(),
                    user.clone(),
                    "-w".into(),
                    format!("/home/{user}"),
                    self.container.clone(),
                    "bash".into(),
                    "-l".into(),
                ],
            )
        }
    }
}
