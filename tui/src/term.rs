//! A live terminal into the game-world container: a real PTY running a
//! login shell, parsed by vt100 so full-screen programs (vi, more, ssh,
//! readline) behave. The app renders the vt100 screen itself, in a single
//! phosphor colour - it's a terminal, not a dashboard.
use anyhow::Result;
use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtySize};
use std::io::{Read, Write};
use std::sync::mpsc::{channel, Receiver};
use std::thread;

pub struct Term {
    parser: vt100::Parser,
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    rx: Receiver<Vec<u8>>,
    child: Box<dyn portable_pty::Child + Send + Sync>,
    dead: bool,
}

impl Term {
    /// Open a PTY and start a login shell in the container as `user`,
    /// landing in `workdir`. The player pivots onward (ssh) from here.
    pub fn spawn(
        container: &str,
        user: &str,
        workdir: &str,
        rows: u16,
        cols: u16,
    ) -> Result<Term> {
        let pty = native_pty_system();
        let pair = pty.openpty(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })?;

        let mut cmd = CommandBuilder::new("docker");
        cmd.args([
            "exec",
            "-it",
            "-u",
            user,
            "-w",
            workdir,
            container,
            "bash",
            "-l",
        ]);
        cmd.env("TERM", "xterm-256color");

        let child = pair.slave.spawn_command(cmd)?;
        drop(pair.slave);

        let mut reader = pair.master.try_clone_reader()?;
        let writer = pair.master.take_writer()?;

        let (tx, rx) = channel::<Vec<u8>>();
        thread::spawn(move || {
            let mut buf = [0u8; 8192];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        if tx.send(buf[..n].to_vec()).is_err() {
                            break;
                        }
                    }
                }
            }
        });

        Ok(Term {
            parser: vt100::Parser::new(rows, cols, 0),
            master: pair.master,
            writer,
            rx,
            child,
            dead: false,
        })
    }

    /// Drain any pending PTY output into the screen model. Returns true if
    /// something changed (so the app can redraw).
    pub fn pump(&mut self) -> bool {
        let mut changed = false;
        while let Ok(chunk) = self.rx.try_recv() {
            self.parser.process(&chunk);
            changed = true;
        }
        if let Ok(Some(_)) = self.child.try_wait() {
            self.dead = true;
        }
        changed
    }

    pub fn is_dead(&self) -> bool {
        self.dead
    }

    pub fn send(&mut self, bytes: &[u8]) {
        let _ = self.writer.write_all(bytes);
        let _ = self.writer.flush();
    }

    pub fn resize(&mut self, rows: u16, cols: u16) {
        let (cr, cc) = self.parser.screen().size();
        if cr == rows && cc == cols {
            return;
        }
        self.parser.set_size(rows, cols);
        let _ = self.master.resize(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        });
    }

    pub fn screen(&self) -> &vt100::Screen {
        self.parser.screen()
    }
}
