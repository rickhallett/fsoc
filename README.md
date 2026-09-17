# wargamezr

A terminal-native, self-hosted Linux wargame - a faithful recreation of the
[OverTheWire **Bandit**](https://overthewire.org/wargames/bandit/) "linux-fu"
ladder, wearing an fsociety mask.

**The surface is a terminal, not an app.** You get a real shell inside a
Docker "world" and pivot node to node yourself (`ssh`), exactly like the real
thing. The screen is a real PTY in a dark frame - the box ships with a themed shell
(coloured prompt, tuned ls colours), rendered faithfully. The story
arrives as comms from a handler and narration at the seams. No menus, no
panels, no progress bars - the tool recedes.

The puzzles are the genuine article: progression is enforced by real Unix
permissions (setuid, group-readable files, cron jobs running as the next user,
SSH keys, local daemons), not faked by the UI. **Every password is generated
fresh at build time**, so nothing here redistributes anyone's answers.

```
<darlene> you're on the jump box. don't linger.
-- objective: left in the open --
<darlene> the creds for the next node are in a file called readme...

  node0@ecorp:~$ cat readme
  Ms93nOIWY5RCkTOkccjnQiMEDTCYLEvt
  node0@ecorp:~$ ssh node1@localhost -p 2220
  ...

  node1  .  fsociety  .  F1 leak  .  F2 job  .  ^G quit  .  or type `exit`
```

## Two skins, one engine

The narrative and look are a swappable **campaign** over the *same* mechanics:

- **`fsociety`** (default) - a Mr. Robot re-skin: pivot node to node across
  E Corp's network. Amber phosphor on near-black, terse paranoid comms.
- **`bandit`** - the plain OverTheWire naming (`bandit0..`), understated.

Switch the mechanics' user naming is the *only* thing a skin changes in the
container; everything else is presentation. That's what keeps it fundamentally
a wargame no matter how it's dressed.

## How it fits together

| Piece | What it is |
|---|---|
| `world/` | One parameterized Dockerfile + provisioning: a Debian box with real `<prefix>0..N` users, planted puzzle files, `sshd` on 2220. `USER_PREFIX`/`PASS_DIR` pick the skin's naming. |
| `campaigns/<name>/` | A campaign: `campaign.toml` (identity + world coords), `levels.toml` (34 objectives + hint ladder + spoiler), `theme.toml` (palette, comms voice, handler). |
| `tui/` | The Rust front-end: a real PTY terminal (`portable-pty` + `vt100`) rendered faithfully (the box's own colours) in a dark frame, with a comms feed. It watches your prompt to know which node you're on and feeds you the matching objective. |

## Quick start

Requires Docker (OrbStack or Docker Desktop) and a Rust toolchain.

```sh
make world-up             # build + run the fsociety world (sshd on :2220)
make check                # headless doctor: verifies the world
make play                 # drop into the terminal

make world-up CAMPAIGN=bandit   # the plain skin instead
make play    CAMPAIGN=bandit
```

Once you're in:

- You're at a real shell as `node0` (or `bandit0`). Solve the objective,
  find the next node's password, and `ssh node1@localhost -p 2220` to pivot.
- Comms and the next objective arrive automatically as you move.
- **F1** - leak a hint from your handler.  **F2** - re-read the current job.
- **^G** (or type `exit`) - leave.

It's a genuine terminal: pipes, `cd`, background jobs, `ssh`, `vi` all work,
because it's a real shell in the box. You can also skip the wrapper entirely:

```sh
ssh node0@localhost -p 2220       # fsociety, password: node0
ssh bandit0@localhost -p 2220     # bandit,   password: bandit0
```

## Scope

**Vertical slice**: the world provisions levels **0-11** (the pure
filesystem/searching puzzles). The terminal, comms, node-detection, and both
campaigns are complete, and all 34 objectives exist for each. Levels 12-33
have full objectives/hints and are wired in; their world-side setup (SSH keys,
TLS daemons, cron, git repos, shell escapes) is the next tranche -
`world/setup/plant-levels.sh` is guarded by `MAX_LEVEL`, so raising it is
additive.

Known edge: full-screen curses programs (`vi`, `more`) render through the vt100
emulator; line-oriented work (the whole current slice) is exact.

## Making your own skin

1. `cp -r campaigns/bandit campaigns/mine`
2. Edit `campaign.toml` (`name`, `user_prefix`, story) and `theme.toml`
   (palette + `[comms]` handler/opening/narration).
3. Re-flavor `levels.toml` - change the *framing* freely, but keep the
   *operational facts* (filenames, sizes, ports, user names) matching the
   world, or the puzzle breaks.
4. `make world-up CAMPAIGN=mine && make play CAMPAIGN=mine`

Only `user_prefix` touches the container; everything else is presentation.

## Design notes

- **Fresh secrets.** `world/setup/provision.sh` generates a random 32-char
  password per user into `$PASS_DIR/<prefix>N` (mode `0400`, owned by that
  user) and sets it as the user's SSH password. `<prefix>N` genuinely cannot
  read `<prefix>(N+1)`'s password except via the intended technique.
- **Safety.** Everything runs inside the container; nothing touches your host
  filesystem. `make world-down` removes it entirely.
