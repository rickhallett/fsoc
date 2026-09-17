# wargamezr

A terminal-native recreation of the [OverTheWire **Bandit**](https://overthewire.org/wargames/bandit/)
"linux-fu" wargame: a gorgeous Rust TUI on the outside, a **real** Linux
game world in a Docker container on the inside.

The puzzles are the genuine article - progression is enforced by actual Unix
permissions (setuid binaries, group-readable files, cron jobs running as the
next user, SSH keys, local daemons), not faked by the UI. **Every password is
generated fresh at build time**, so nothing here redistributes anyone's
answers.

The narrative and look are a swappable **campaign** (a "skin"). Ships with
two over the *same* mechanics:

- **`bandit`** - the plain OverTheWire ladder, neutral naming.
- **`fsociety`** - a Mr. Robot re-skin: you pivot node to node across E Corp's
  network, with an amber/red phosphor grade, a boot splash, and terse
  paranoid copy.

```
+- fsociety | fsociety | 3/33 owned | 3 creds | box breathing +
| [x] node 01 - left in the open   | node 04 - buried               |
| [x] node 02 - a file named dash  | ...the job...                  |
| [>] node 04 - buried             | tools: ls cd cat find          |
| [-] node 05 - only one is real   | // leaks (1/2 decrypted)       |
+----------------------------------+--------------------------------+
```

Status glyphs are plain ASCII: `[x]` cleared, `[>]` current, `[-]` locked.

## How it fits together

| Piece | What it is |
|---|---|
| `world/` | One parameterized Dockerfile + provisioning that builds a Debian box with real `<prefix>0..N` users, planted puzzle files, and (later levels) daemons and cron jobs. `USER_PREFIX`/`PASS_DIR` pick the skin's naming. |
| `campaigns/<name>/` | A campaign: `campaign.toml` (identity + world coordinates), `levels.toml` (34 briefs, tools, hint ladder, spoiler), `theme.toml` (palette, chrome copy, splash, glitch). |
| `tui/` | The Rust/ratatui front-end: browse levels, reveal hints, drop into a real shell, bank passwords. Loads a campaign, applies its theme. |

**The mechanics are the invariant** - the same setuid/find/grep/cron/nc/git
techniques solve every skin. Only names, copy, and colors change. That's what
keeps it fundamentally a wargame no matter how it's dressed.

The TUI runs on your Mac and talks to the container over `docker exec` (and
SSH for level 0). Passwords you find are verified against the world and unlock
the next level.

## Quick start

Requires Docker (OrbStack or Docker Desktop) and a Rust toolchain.

```sh
make world-up             # build + run the fsociety world (sshd on :2220)
make check                # headless doctor: verifies the world
make play                 # launch the TUI

make world-up CAMPAIGN=bandit   # the plain skin instead
make play    CAMPAIGN=bandit
```

Run one world at a time on `:2220` (or pass `PORT=` to co-host). Each campaign
gets its own container (`wargamezr-<name>`).

In the TUI:

- `j`/`k` (or arrows) - move between levels
- `Enter` - drop into a shell for the selected level (find the next password)
- `h` - reveal the next hint; `s` - toggle the spoiler solution
- `p` - type the password you found; correct answers unlock the next level
- `r` - re-check the world; `q` - quit

You can always play it raw:

```sh
ssh node0@localhost -p 2220       # fsociety, password: node0
ssh bandit0@localhost -p 2220     # bandit,   password: bandit0
```

## Scope

**Vertical slice**: the world provisions levels **0-11** (the pure
filesystem/searching puzzles). The TUI, hint system, treasure vault,
progression, and theming are complete, and all 34 level briefs exist for both
campaigns. Levels 12-33 have full briefs/hints/solutions and are wired into
the UI; their world-side setup (SSH keys, TLS daemons, cron, git repos, shell
escapes) is the next tranche - `world/setup/plant-levels.sh` is guarded by
`MAX_LEVEL`, so raising it is additive.

## Look & feel (fsociety skin)

Near-black / amber-phosphor / fsociety-red grade, squared borders, terse
lowercase copy (targets, leaks, creds, "jack in"), a subtly glitching header,
and a boot splash (`hello, friend.` -> ASCII mask -> fake tor/handshake log).
`WARGAMEZR_NO_INTRO=1` skips the splash. The `bandit` skin is the neutral
counterpart (teal/gold, rounded borders, no splash).

## Making your own skin

1. `cp -r campaigns/bandit campaigns/mine`
2. Edit `campaign.toml` (`name`, `user_prefix`, story) and `theme.toml`
   (palette + `[labels]` copy + optional `[splash]`).
3. Re-flavor `levels.toml` - change the *framing* freely, but keep the
   *operational facts* (filenames, sizes, ports, user names) matching the
   world, or the puzzle breaks. `make check` catches a dead world; keep facts
   single-sourced to avoid drift.
4. `make world-up CAMPAIGN=mine && make play CAMPAIGN=mine`

Only `user_prefix` touches the container (it renames the users); everything
else is pure presentation.

## Design notes

- **Fresh secrets.** `world/setup/provision.sh` generates a random 32-char
  password per user into `$PASS_DIR/<prefix>N` (mode `0400`, owned by that
  user) and sets it as the user's SSH password. `<prefix>N` genuinely cannot
  read `<prefix>(N+1)`'s password except via the intended technique.
- **Safety.** Everything runs inside the container; nothing touches your host
  filesystem. `make world-down` removes it entirely.
