//! Thin wrapper over the `docker` CLI for talking to a game-world
//! container. The container runs real per-level users (named by the
//! campaign's prefix); the TUI shells into them and verifies passwords
//! against the world.
use anyhow::{anyhow, Result};
use std::process::{Command, Stdio};

pub struct World {
    pub container: String,
    prefix: String,
    pass_dir: String,
}

impl World {
    pub fn new(
        container: impl Into<String>,
        prefix: impl Into<String>,
        pass_dir: impl Into<String>,
    ) -> Self {
        World {
            container: container.into(),
            prefix: prefix.into(),
            pass_dir: pass_dir.into(),
        }
    }

    fn user(&self, n: u32) -> String {
        format!("{}{}", self.prefix, n)
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

    /// The stored password for level n's user, read as root inside the
    /// world. Used to verify a player's answer and unlock progression.
    pub fn password_of(&self, n: u32) -> Result<String> {
        let user = self.user(n);
        let out = Command::new("docker")
            .args([
                "exec",
                &self.container,
                "cat",
                &format!("{}/{user}", self.pass_dir),
            ])
            .output()?;
        if !out.status.success() {
            return Err(anyhow!(
                "could not read password for {user}: {}",
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
    /// Level `n` (n >= 1) is solved while logged in as <prefix>(n-1) - that
    /// stage's goal is to recover <prefix>n's password. Level 0 is the
    /// bootstrap "get in over SSH" tutorial, launching a real SSH session
    /// as <prefix>0 (whose password is <prefix>0).
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
                    format!("{}0@localhost", self.prefix),
                ],
            )
        } else {
            let user = self.user(n - 1);
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
