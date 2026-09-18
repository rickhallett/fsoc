# fsoc content spec

How to author an fsoc world: an inhabited real computing environment where
becoming a better terminal user is how you uncover the story. Distilled from
the mrrobot curriculum's linux-fu and grading rigor, stripped of its lab
scaffolding.

Status: design spec. The terminal, world container, campaign/theme system and
`fsoc` toolkit exist; the world daemon, multi-host topology and invariant
grader described here are not built yet.

## Non-negotiable principles

1. **Two progression axes only** — your real skill as a terminal user, and the
   story. No points, levels, XP, badges, streaks, completion screens, mission
   menu, or a numbered ladder. Progression shows up as *access, knowledge and
   trust*.
2. **No explicit hint mechanism.** No hint button, hint ladder, hint counter,
   solution reveal, or on-demand author hints. Help exists only *as the world*
   (see "Help, diegetically"). This is a hard rule.
3. **Legible action, ambiguous meaning.** The technical objective is always
   clear — you know what you're doing *to the machine*. The fiction carries the
   ambiguity (whose file, why they lied, what you just enabled). If the player
   must guess the *verb*, it's broken.
4. **Grade world-state invariants, never a command line.** Multiple valid
   approaches pass. Require a specific technique only when that technique is
   explicitly what's being practised, for a reason in the world.
5. **Real system = grader = teacher.** Because it's a real box, consequences
   are true, not scripted. Fix the cron and the report genuinely regenerates.
6. **Evidence is stable; interpretation grows.** Artifacts don't change; your
   reading of them does. A file you skimmed on day one becomes a gut-punch on
   day five. Revisiting must reward.
7. **Bug = evidence = lesson.** Author every artifact so that, where possible,
   the same object teaches a skill, causes a story problem, and carries a human
   trace.

## The skill spine (linux-fu backbone)

The competence a player accrues across the world. This is what the world
*requires*, never a syllabus shown to the player.

| Domain | Core linux-fu |
|---|---|
| Text & argv | quoting/argument boundaries, `"$@"`, stdout vs stderr, redirection order, exit & pipeline status |
| Search | `rg`/`grep` content vs filename, hidden vs ignored (`--hidden` ≠ `--no-ignore`), literal vs regex, `find -name/-type/-mtime` |
| Aggregation | `awk` fields/grouping, `sort` numeric vs lexicographic, `uniq` adjacency, `wc` semantics, CSV-aware tools vs `-F,` |
| Archives | regular vs hidden, mtime vs birth time, sym/hard links, `-print0`, checksum staging, collision-safe copy/rename |
| Permissions | u/g/o, file vs dir perms, dir-execute = traversal, umask, ACL awareness, read/search/write distinctions |
| Processes | job control, process trees, PIDs, signals, `wait`/`timeout`, bounded parallelism, background job ≠ supervised service |
| Services | journal filters, user services, env differences, ports, ownership, restart loops |
| Networks | addr/route/DNS, listener vs TCP vs HTTP error, `curl` status/body/timeout, loopback fixtures, SSH concepts (never scan public systems) |
| Reliable Bash | arrays, `getopts`, quoted heredocs, traps, temp ownership, `pipefail` caveats, idempotency, ShellCheck |
| Python bridge | argparse, pathlib, json/csv, `subprocess` argv, timeouts, atomic writes, tests |
| Evidence | `strace`, open files, space vs inodes, timestamps/timezones, reproducibility, minimal reproducers |

## Misconception catalog (puzzle seeds)

Each line is a seed. Every operation targets one primary skill + at most two
supporting, and encodes at least one of these as a **broken_state** the world
exposes:

- the shell expands argv *before* the command sees it; quoting moves the boundaries
- `rg pattern` searches content, not filenames; untracked ≠ ignored; `--hidden` and `--no-ignore` solve different problems; `--replace` changes display, not the file
- a no-match status is not a runtime error; `pipefail` is useful but not business logic
- `find -mtime` is modification age; `wc -l` counts newlines, not source lines
- `uniq` only groups *adjacent* equal lines; numeric sort ≠ lexicographic sort
- `awk -F,` is not a CSV parser
- directory *execute* means traversal; deleting a file depends on *directory* permissions
- a background job is not a supervised service; `SIGKILL` permits no cleanup
- `set -e` has contextual exceptions
- a symlink can redirect a host operation; a sandbox is not just a directory
- a printed "done" message says nothing about exit status

## Operation anatomy

An **operation** is the unit of authored content (fsoc's replacement for an
"exercise" — no attempt/completion record, no menu entry). Author it as data:

- **id**, **revision**
- **skills**: one primary + ≤2 supporting, from the spine/catalog
- **situation**: the diegetic reason it exists (who/what/why), delivered *in the
  world* — a message, a note, a machine's state — not a briefing panel
- **technical_objective**: the unambiguous verb (find the file / determine who
  logged in / get the service answering / restore the report). One sentence.
- **world_artifacts**: the fixtures — files, services, state — authored as
  *bug = evidence = lesson* (see template below). Bounded and seeded.
- **invariants**: typed checks on end state (the grader; see Assertion types).
  These fire **silently**.
- **consequences**: what the world *does* when the invariants hold — a delayed
  message arrives, a service starts answering, access opens, a *different*
  machine's state shifts. This is the only feedback. Never a toast.
- **broken_states**: ≥2 plausible wrong end-states + which invariant leaves each
  unresolved. The world stays diegetically "wrong" (a character still waiting, a
  service still down); it never prints an error verdict.
- **retest**: the transfer variation and where/when it re-surfaces, unannounced.
- **story_finding**: what becomes knowable — supported by current evidence, no
  future spoilers.
- **author_solution** / **author_alternative**: author-only, kept *out of the
  world*. There is no in-world reference solution and no hint field.

## Assertion types (the invisible grader)

Typed invariants, checked against real end state, run in a clean context so a
background process can't spoof them. No `feedback` string is shown to the
player — an author note only.

| Type | Asserts |
|---|---|
| `regular_file` | a path exists and is a regular file |
| `exact_text` / `text_lines` | byte-exact or line-set content (whitespace/order explicit per assertion) |
| `json_value` | parsed structure equals expected |
| `unchanged_digest` | evidence survived (protect the original) — central to recovery ops |
| `exit_status` | a script/command's status |
| `stdout_text` / `stderr_contains` | stream contents, kept separate |
| `listener_present` / `service_responds` / `process_state` | live-world state for the services/networks/processes domains |

Rules: minimum necessary invariants; never an exact command line; normalize
whitespace/order only when explicitly allowed (byte-exact when whitespace is the
lesson); snapshot state at evaluation; preserve stderr and exit status
separately when taught; author invariants **independently** — never use a
candidate solution as the oracle.

**Firing without a HUD:** the world daemon watches real state, debounces, and
when an operation's invariants hold, triggers its `consequences`. There is no
"checking…" UI and no pass/fail screen. When invariants are unmet, *nothing is
announced* — the world simply remains in its broken state, and the fiction is
the feedback.

## Help, diegetically (the no-hint contract)

**Forbidden:** hint buttons, hint ladders, hint counters, solution reveals,
`F1`/"leak", an `-- objective --` ticker, or graduated author hints surfaced on
demand.

**Allowed, because they *are* the world:**

- `man` / `tldr` / `--help` — documentation literacy is competence.
- a predecessor's notes in a home dir or `/tmp`, maintenance logs, colleague
  correspondence, worked examples that exist for an in-world reason.
- a character (handler/colleague) who, when messaged, sharpens the *question*
  or points at *what to wonder about* — never a command. They may be wrong or
  evasive; the *technical* objective must stay discoverable regardless.
- the tools' own real defaults and error messages (why `rg` skipped an ignored
  file is a lesson the tool itself teaches).

**Re-orientation is allowed** (re-read a message you already received, scroll
back, consult your own notes) — that is memory, not a hint. Difficulty tuning is
the author's job: the world's own materials must be *sufficient* to solve
without any hint system.

## Retests

Transfer variations re-enter as **new circumstances**, unannounced, later in the
world — same mechanism, different dressing and symptom, no label. Recognition is
the assessment. Reproducible seeds vary incidental identifiers and counts only;
never the culprit, motive, or story facts.

## The world (topology & state)

- **Few hosts, deep relationships.** Machines have a purpose, an operational
  history, and traces of the people who use them. Later discoveries change how
  earlier evidence reads.
- **Your workstation is another machine in the world**, not the host — its own
  VM/container that persists across a campaign. Your aliases, notes, scripts and
  `fsoc` choices accumulate *there*; the real host is never touched. Remote
  machines stay constrained; tool scarcity is a diegetic fact (a stripped
  recovery box has only BusyBox), never a nag.
- **Machines have a hardware profile.** Each host is allocated resources that
  fit what it *is* — memory, CPU, disk/inode budget, pid limit — set per
  container (compose `mem_limit` / `cpus` / `pids_limit` / sized tmpfs). A
  stripped recovery box is genuinely small; a developer workstation is roomy; an
  old abandoned server is cramped and slow. This is primarily **realism**: the
  world feels like real machines with real constraints, and `free`, `df -i`,
  `nproc`, `ulimit` and `btop` report the truth. It occasionally becomes
  load-bearing (a box actually out of inodes, a job that OOMs under a real
  memory cap) — but a profile never needs a story reason to exist; realism is
  reason enough.
- **The world daemon** is the one new engine: watches real state, fires
  consequences in the fiction. Evidence that must survive is mounted read-only
  and asserted with `unchanged_digest`.

## Authoring checklist

1. State the technical objective as one unambiguous verb.
2. Map skills to the spine + ≤2 supporting; encode ≥1 misconception as a
   `broken_state`.
3. Author artifacts as bug = evidence = lesson; bound and seed fixtures.
4. Author invariants independently; verify one correct *and* one alternative
   approach pass, and that each `broken_state` fails for the intended reason.
5. Wire `consequences` — what the world does on satisfaction. Must be diegetic.
6. Ensure sufficient help exists *in-world* (a note, a man page a character
   points at) to solve without any hint system.
7. Author the retest and where it resurfaces.
8. Confirm narrative-evidence consistency; no future spoilers in `story_finding`.
9. Safety: fixtures are regular files inside the world; reject path traversal;
   host reads of results are bounded and typed; never execute player strings on
   the host.

## Implications for the current build

To move from the ladder to this spec:

- **Remove** the `F1` "leak" hint and the `-- objective --` line from the TUI.
  Objectives and help become diegetic (messages, notes, man pages).
- Keep the comms feed but reframe it as in-world channels (mail/chat), not an
  objective ticker. `F2` becomes "re-read the last message" (memory) or is
  dropped.
- Add the **world daemon** (state-watch → consequence) and **multi-host
  topology** (compose: a workstation + a few constrained machines).
- Adopt the **assertion set** as the invariant grader, fired silently by the
  daemon.
- Retire "assisted mode" language in the `fsoc` tiers — choosing and
  understanding good tools is part of the skill.
