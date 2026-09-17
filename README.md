# wargamezr

A terminal-native recreation of the [OverTheWire **Bandit**](https://overthewire.org/wargames/bandit/)
"linux-fu" wargame: a gorgeous Rust TUI on the outside, a **real** Linux
game world in a Docker container on the inside.

The puzzles are the genuine article - level progression is enforced by
actual Unix permissions (setuid binaries, group-readable files, cron jobs
running as the next user, SSH keys, local daemons), not faked by the UI.
The only thing that isn't OverTheWire's is the answers: **every password is
generated fresh at build time**, so nothing here redistributes their secrets.

```
+- wargamezr | bandit campaign | 3/33 cleared | 3 treasure | world up +
| [x] Level  1 - reading a file    | Level 4 - hidden file          |
| [x] Level  2 - a file named dash | ...goal...                     |
| [>] Level  4 - hidden file       | useful: ls cd cat find         |
| [-] Level  5 - human-readable    | hints (1/2 revealed)           |
+----------------------------------+------------------------------+
```

Status glyphs are plain ASCII: `[x]` cleared, `[>]` current, `[-]` locked.

## How it fits together

| Piece            | What it is                                                        |
|------------------|-------------------------------------------------------------------|
| `world/`         | Dockerfile + provisioning that builds a Debian box with real `bandit0..N` users, planted puzzle files, and (later levels) daemons and cron jobs. |
| `levels/bandit.toml` | All 34 level briefs, useful commands, a progressive hint ladder, and a spoiler solution. |
| `tui/`           | The Rust/ratatui front-end: browse levels, reveal hints, drop into a real shell, and bank passwords in the treasure vault. |

The TUI runs on your Mac and talks to the container over `docker exec`
(and SSH for level 0). Passwords you find are verified against the world
and unlock the next level.

## Quick start

Requires Docker (OrbStack or Docker Desktop) and a Rust toolchain.

```sh
make world-up      # build + run the game world (sshd on localhost:2220)
make check         # optional: headless doctor, verifies every level
make play          # launch the TUI
```

Then, in the TUI:

- `j`/`k` (or arrow keys) - move between levels
- `Enter` - drop into a shell for the selected level (find the next password)
- `h` - reveal the next hint; `s` - toggle the spoiler solution
- `p` - type the password you found; correct answers unlock the next level
- `r` - re-check the world; `q` - quit

You can always play it the raw way too:

```sh
ssh bandit0@localhost -p 2220     # password: bandit0
```

## Scope

This build is a **vertical slice**: the world provisions levels **0-11**
(the pure filesystem/searching puzzles) and the TUI, hint system, treasure
vault, and progression are complete for all 34 level briefs. Levels 12-33
have full briefs/hints/solutions and are wired into the UI; their world-side
setup (SSH keys, TLS daemons, cron jobs, git repos, shell escapes) is the
next tranche - see `world/setup/plant-levels.sh`, which is already guarded
by `MAX_LEVEL` so raising it is additive.

## Look & feel

The front-end wears an fsociety mask: a near-black / amber-phosphor / fsociety-red
grade, squared terminal borders, terse lowercase copy (targets, leaks, creds,
"jack in"), a subtly glitching header, and a boot splash (`hello, friend.` ->
ASCII mask -> fake tor/handshake log). Set `WARGAMEZR_NO_INTRO=1` to skip the
splash.

## Design notes

- **Fresh secrets.** `world/setup/provision.sh` generates a random 32-char
  password per user into `/etc/bandit_pass/banditN` (mode `0400`, owned by
  `banditN`) and sets that as the user's SSH password. `banditN` genuinely
  cannot read `banditN+1`'s password except via the intended technique.
- **Safety.** Everything runs inside the container; nothing touches your
  host filesystem. `make world-down` removes it entirely.
- **Extending.** Add a level by appending to `levels/bandit.toml` and its
  setup to `plant-levels.sh`, then rebuild with a higher `MAX_LEVEL`.
