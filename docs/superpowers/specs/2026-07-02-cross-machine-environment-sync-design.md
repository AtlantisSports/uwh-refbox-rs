# Cross-Machine Environment Sync — Design

**Date:** 2026-07-02
**Status:** Approved by Eric (revised design); refined 2026-07-03 into a version-pinned software & settings reference. Ready for implementation plan.
**Author:** Claude + Eric

> **Where this file lives:** this is a personal-workflow document, not a refbox or portal
> feature. Its permanent home is Eric's private `e-straily/claude-workspace` repo (see Part 1).
> It is kept here as an *untracked local note* for now and moved into `claude-workspace` when
> that repo is created — it is deliberately **not** committed to the shared `uwh-refbox-rs` repo.

---

## Goal

Set up a **second Windows 11 machine** so its development environment matches the current one,
and keep the two matched going forward with a **simple, explicit switch routine**. This is a
**machine-level** design: it covers *both* projects Eric works in — `uwh-refbox-rs` and
`uwh-portal` — not just one.

The outcome: Eric can sit down at either machine and pick up exactly where he left off — same
code, same tools, same Claude assistant (memory and settings), same unfinished work — for
refbox and portal alike.

Eric is a non-programmer domain expert. All steps in the eventual plan must be plain-English
and walked through; Claude runs the technical commands.

---

## What we are replicating (the current machine)

- **Base:** Windows 11 → WSL2 → **Ubuntu 24.04.4 LTS**, with Claude Code installed inside it.

- **Windows-side apps** (these live on the Windows side and are **not** part of the Linux copy —
  they are installed separately on machine 2):
  - WSL2 + the Ubuntu 24.04 distro
  - Docker Desktop (with WSL integration enabled) — required for LocalStack and local RavenDB
  - VS Code + the **WSL** extension and the **Claude Code** extension
  - Windows Terminal

- **Refbox toolchain (inside WSL)** — *match the pinned versions*:
  - Rust (rustc/cargo) **exactly 1.85.0** — the project's MSRV; a newer Rust can break the build
  - Rust build targets: `aarch64-unknown-linux-gnu` (Raspberry Pi), `x86_64-pc-windows-gnu`,
    `x86_64-unknown-linux-gnu`
  - `just` 1.49.0, GitHub CLI (`gh`) 2.45.0, git 2.43.0 (latest of these is fine)

- **Portal toolchain (inside WSL)** — installed *on demand*; versions pinned only where noted:
  - **.NET SDK 10** (currently 10.0.201) — the API
  - **Node.js 24** (currently v24.15) + **pnpm via `corepack`** (project pins its own pnpm) — the website
  - **Flutter via `fvm`** (fvm 4.0.5; Flutter auto-pinned by the repo to **3.41.5**) — the phone app
  - **Docker + docker-compose** (28.x, Docker Desktop with WSL integration) running **LocalStack** and
    a **local RavenDB** — the local test stack (images pulled on first run)
  - `backlog` (pnpm global) — task tracking
  - **Not installed on machine 1, so not part of the baseline** (add only on demand): Android
    SDK / phone emulator, `sentry-cli`, `bruno`, standalone `playwright` — these ship with the
    portal code's own dependencies or are installed when actually needed.

- **Code (already on GitHub, over SSH):**
  - `uwh-refbox-rs` → `git@github.com:AtlantisSports/uwh-refbox-rs.git`
  - `uwh-portal` → `git@github.com:zerotomvp/uwhportal.git`

- **Non-code bits NOT on GitHub today** (the parts this design exists to move):
  1. Claude's memory + settings (`~/.claude`) — global instructions, settings, hooks,
     keybindings, saved plans, and per-project memory folders.
  2. Unfinished work — uncommitted changes across the repos **and inside the extra working
     folders (worktrees)**, plus untracked local planning notes (`docs/superpowers/…`,
     `docs/backlog/…`).
  3. Logins/keys — GitHub, Claude, Sentry, portal token.

---

## Decisions already made (with Eric)

1. **Fresh logins on the new machine.** No secret files are copied. The new machine gets its
   own GitHub key, its own GitHub-tool login, and its own Claude login; the portal token is
   re-entered once in refbox. (A Sentry login is set up only if the on-demand Sentry tooling is
   used — see Decision #4.)
2. **Non-code bits travel via a private GitHub backup repo** (not a cloud-synced folder, not
   manual copy).
3. **No auto-save-on-session-end.** Considered and rejected for now — it would cover only
   memory (not code), can fail silently (offline / many concurrent sessions), and produces a
   confusing half-matched state when the switch step is forgotten. Recorded as an optional
   future add-on if Eric finds he forgets the manual step in practice.
4. **Parity = a version-pinned software & settings *reference*, installed on demand.** Rather
   than bulk-installing everything up front, machine 2 works from a reference list (see "What we
   are replicating"): tools are installed when Eric actually needs them, and versions are pinned
   only where it matters (Rust 1.85, Node 24, .NET 10, the fvm-pinned Flutter). Verified against
   machine 1: Android SDK/emulator, `sentry-cli`, `bruno`, and standalone `playwright` are **not**
   installed here and are not part of the baseline. `default-config.toml` holds a token and is
   re-entered fresh, never copied. The software list covers *tools*; Claude's memory/settings and
   unfinished work travel via the backup repo (Parts 1 and 3), not the list.
5. **Unfinished work travels via "parking branches" + a curated carry-list of the active
   worktrees** (method A below), not by committing onto the real working branch. Machine 1 has
   ~55 worktrees total, most of them stale/experimental; a blind sweep of all of them is
   impractical, so only an explicit list of active worktrees is carried (the rest stay on
   machine 1, retrievable anytime).
6. **This design document lives in `claude-workspace`** (Eric's personal repo), not in a shared
   project repo.

---

## Part 1 — The backup repo (the non-code bits)

A **new private repository** under Eric's own GitHub account: `e-straily/claude-workspace`
(private / invite-only). It is the canonical home for everything that makes Claude behave
identically on both machines — and for this design document itself.

- **Where it lives:** directly at `~/.claude`, tracked in place as a git repo — no copying.
- **Safety model — ignore everything by default.** The repo's ignore rules exclude *all* of
  `~/.claude` and then explicitly re-include only this safe list:
  - `CLAUDE.md` (global instructions)
  - `settings.json`, `settings.local.json` (permissions, hooks, env — reviewed once for any
    embedded secret before first upload)
  - `keybindings.json`
  - `hooks/` (custom hook scripts)
  - `skills/` (custom user skills)
  - `plans/` (saved plans — **this design doc, plus the switch scripts under `plans/scripts/`**
    (`switch-out`, `switch-in`, `carry-list.tsv`, `test-switch.sh`), are stored here so they
    travel to both machines)
  - `projects/*/memory/` (every project's memory folder)
- **Never uploaded** (stays ignored): `.credentials.json`, `history.jsonl`, `sessions/`,
  `session-env/`, `file-history/`, `telemetry/`, `ide/`, `cache/`, `downloads/`,
  `paste-cache/`, `shell-snapshots/`, `backups/`, `plugins/` (re-installed instead),
  `mcp-needs-auth-cache.json`, `policy-limits.json`, `remote-settings.json`, `.last-cleanup`,
  and compiled python caches (`hooks/__pycache__/`).
- **Verification before the first upload:** confirm with `git status` and `git check-ignore`
  that only the safe list is staged and no secret/cache file is included, **and scan the staged
  `settings.json` / `settings.local.json` for credential *shapes*** — login/bearer tokens, key
  headers, `password`, long high-entropy strings — not merely the word "token". This check is
  shown to Eric before anything is pushed. *(Machine-1 status: config + ~376 memory files across
  4 projects staged; all credential/session/cache/plugin paths confirmed ignored. `settings.json`
  **does** contain 2 dev bearer tokens and 3 test-password strings — all localhost / expired /
  mock, so acceptable in a **private** repo, but recorded here accurately rather than claimed
  absent. Nothing pushed yet — awaiting Eric's go after the private-visibility check + the
  credential-shape scan above.)*
- **Plugins** (Superpowers etc.) are **re-installed** on the new machine via the plugin
  marketplace rather than synced, since they are versioned caches.
- **Handling the doc during bootstrap:** because `claude-workspace` does not exist yet, this
  design is written now as an untracked local note and moved under `~/.claude/plans/` when the
  repo is created.

---

## Part 2 — One-time bootstrap of the second machine

A plain-English checklist Claude writes out and walks Eric through, in order. Exact install
commands and versions are finalized in the implementation plan; this is the shape.

1. **Windows base:** enable WSL2 and install Ubuntu 24.04.
2. **Windows-side apps:** install Docker Desktop (enable WSL integration), VS Code + the WSL
   and Claude Code extensions, and Windows Terminal.
3. **Refbox toolchain (WSL):** install Rust via rustup (set 1.85.0 as default; add the three
   build targets), plus `just`, `gh`, Node v24, git — matching current versions.
4. **Portal toolchain (WSL) — baseline:** install the .NET 10 SDK; enable pnpm via corepack;
   install Flutter + fvm (repo-pinned Flutter 3.41.5); wire up LocalStack and a local RavenDB via
   Docker Desktop; install `backlog` (pnpm global) and Eric's failure-baseline tool; set up Dart
   MCP. **On demand only** (absent on machine 1, per Decision #4 — skip unless a task needs them):
   Android SDK + emulator, `sentry-cli`, `bruno`, standalone `playwright`.
5. **Claude Code:** install it; then re-install the Superpowers add-ons via the plugin
   marketplace.
6. **Fresh logins:** generate a new SSH key and add it to GitHub; `gh auth login`; log in to
   Claude; (log in to Sentry only if/when the on-demand Sentry tooling is set up); (the portal
   token is entered in refbox in a later step).
7. **Get the code:** clone `uwh-refbox-rs` and `uwh-portal`.
8. **Get Claude's brain:** set up the `claude-workspace` backup repo into `~/.claude` so memory
   and settings are present. Because `~/.claude` already exists after installing Claude Code,
   this is **not** a plain `git clone` into an empty folder — the implementation plan handles
   the exact mechanics (initialize/point the repo at the existing folder, then check out).
9. **Non-secret config:**
   - Refbox: `~/.config/refbox/default-config.toml` **contains a push-stats token** — do **not**
     copy it whole. Re-set the non-token preferences in the app and re-enter the token fresh.
     `portal_link.json` / `portal_queue.json` are **not** carried (token re-entered in-app).
   - Portal: re-create the stable local config files that git ignores (e.g. the local jest
     config workaround and `appsettings.Development.json` with placeholder values). These are
     set up **once at bootstrap**, not carried on every switch. No real secrets are copied.
10. **Verify:**
    - Refbox: `just check` passes, and refbox launches.
    - Portal: the web app and API build; the local stack runs (API health check responds, web
      loads on `localhost:3000`); `fvm dart analyze` passes for mobile.

---

## Part 3 — The switching habit (going forward)

Two moves; **Claude runs the commands** — Eric just says the word. Only one machine is used at
a time, and **machine 1 is kept** — so anything not carried is never lost; it simply stays on
machine 1, retrievable later with the same mechanism.

**Scope — a curated carry-list, not a blind sweep.** Machine 1 has ~55 worktrees (9 in refbox,
~46 in portal), most of them stale or experimental. Sweeping all of them would push dozens of
backup branches to the shared remotes for no benefit. So `switch-out` works from a small,
editable **carry-list** (`~/.claude/plans/scripts/carry-list.tsv`) naming only the *active*
worktrees to carry. Adding or dropping an active project later = edit that one file.

- **Leaving a machine → "I'm switching."** Claude:
  1. Saves and uploads the backup repo (memory + settings).
  2. For each worktree in the carry-list, snapshots its committed **and** uncommitted state onto
     a throwaway **`eric/carry/<branch>`** branch and pushes it. The real working branch is left
     exactly as it was, so **open pull requests are never touched** and no forced overwrite of a
     real branch is ever needed. (Local-only branches — e.g. the never-pushed `refbox/integration`
     parity engine — get their commits safely onto GitHub this way too.)

- **Arriving at the other → "I'm back."** Claude:
  1. Downloads the latest backup repo.
  2. For each carry-list entry, recreates the worktree at its true branch tip and re-applies the
     uncommitted work exactly as it was left (modified files modified, untracked files untracked,
     deletions preserved).

This round-trip is covered by a dry-run test (`plans/scripts/test-switch.sh`, **8/8 passing**)
checking modified + untracked + deleted files, a local-only branch with an unpushed commit, a
clean pushed branch, and that `switch-out` leaves the source working tree untouched. Because only
one machine is used at a time, there is no conflict risk, and the full git history means even a
mistake is recoverable.

---

## Explicitly out of scope

- **Auto-save on session end** (recorded above as a possible future add-on).
- **Running both machines at the same time** — the whole design assumes one active machine at a
  time; simultaneous use is not supported.
- **wireless-remote embedded toolchain** — its separate Rust toolchain/target is not part of
  this setup; add later only if firmware work is needed on the second machine.
- **Session history, caches, and any secrets** — never synced.
- **Stale/experimental worktrees (~35 of them)** — not carried to machine 2; they remain on
  machine 1 and can be pulled later via the same mechanism if ever needed.

> **Changed from the earlier draft:** the switch does **not** blindly sweep all ~55 worktrees
> (that would spray dozens of backup branches across the shared remotes). It carries only the
> worktrees named in the curated carry-list (Part 3); the rest stay safely on machine 1. Nothing
> unsaved is lost, because machine 1 is kept.

---

## Acceptance criteria (what Eric can observe)

- `just check` passes in `uwh-refbox-rs` on the second machine, and refbox launches and runs.
- The portal web app and API build on the second machine; the local stack runs (API health
  check responds, web loads on `localhost:3000`); `fvm dart analyze` passes for mobile.
- Claude on the second machine recalls project context (memory is present — verifiable by
  asking Claude about prior work).
- A full switch round-trip (switch out on machine A → switch in on machine B) restores
  unfinished work intact — **including work that lived in a worktree, and including any carried
  ignored files** — with no `eric/carry/…` commit appearing on a real branch or in an open PR.

---

## Technical appendix (for implementation — not user-facing)

- **Backup repo ignore pattern** (default-deny at `~/.claude`):
  ```gitignore
  /*
  !/.gitignore
  !/CLAUDE.md
  !/settings.json
  !/settings.local.json
  !/keybindings.json
  !/hooks/
  !/skills/
  !/plans/
  !/projects/
  /projects/*
  !/projects/*/
  /projects/*/*
  !/projects/*/memory/

  # never track compiled python caches inside re-included dirs
  hooks/__pycache__/
  ```
  (Verified on machine 1 with `git check-ignore`: only the safe list stages — config + ~376
  memory files across 4 projects — and every credential/session/cache/plugin path is ignored.
  The `projects/*/memory/` nesting works as written.)

- **Pre-upload credential scan** (`~/.claude/plans/scripts/scan-secrets`): a credential-*shape*
  scanner (not a keyword grep) run over the staged files before any push. It flags private-key
  headers, `Bearer`/JWT tokens, `password`/`secret`/`api-key` assignments (**including
  escaped-JSON forms** like `"password\":\"…`), and 32+ char high-entropy runs (the entropy rule
  is limited to config-type files so the ~376 memory `.md` files' 40-char git SHAs don't flood
  it). Hits print **redacted** for Eric to eyeball. `switch-out` calls it as a gate and aborts
  before pushing unless `SWITCH_ACK_SECRETS=1`. Confirmed it surfaces the 2 dev bearer tokens + 3
  mock passwords in `settings.json`/`settings.local.json` that the earlier keyword grep missed.
  It is a heuristic safety net for eyeballing, not an exhaustive guarantee.

- **Parking-branch carry (method A) — as implemented and tested** (`~/.claude/plans/scripts/`):
  - Driven by a curated **`carry-list.tsv`** (columns: repo root, worktree path, branch, is-main)
    — **not** a blind `git worktree list` sweep. Current list = 10 active worktrees.
  - **Snapshot without disturbing the branch:** `git add -A` → `git write-tree` →
    `git commit-tree -p HEAD` yields a `eric/carry/<branch>` commit whose parent is the real tip;
    `git reset` then leaves the working tree exactly as it was; force-push `eric/carry/<branch>`
    (disposable, so force is fine). The real branch pointer never moves → open PRs untouched.
  - **Restore (deterministic + self-verified):** real tip = `eric/carry/<branch>^`; put the
    worktree there pristine (`git worktree add -B` / `checkout -B` + `reset --hard` + `clean -fd`),
    **stashing any pre-existing local changes first** (recoverable via `git stash`). Then
    `git read-tree -u --reset eric/carry/<branch>` forces the working tree to the snapshot and
    `git reset --mixed <real>` leaves branch+index at the real tip, so the diff surfaces as your
    WIP. No 3-way merge is used, so it never balks with "would be overwritten"/"not uptodate".
    Finally it **verifies**: stage the result into a throwaway index, `write-tree`, and compare to
    the snapshot tree — printing `[restored ✓ verified]` or `[RESTORE FAILED — VERIFY MISMATCH]`
    and exiting non-zero on any failure. **Never reports a blind success.**
  - **Ignored files (EXTRAS):** empty by default; populate via `SWITCH_EXTRAS="relpath …"`.
    Each named file is credential-scanned first — a clean file is carried, a credential-shaped
    one is **skipped** unless `SWITCH_ACK_EXTRAS=1` is set, because `eric/carry/<branch>` force-pushes
    to the **shared** org remotes where a leak is org-wide. Stable ignored config is set up at
    bootstrap (Part 2) instead.
  - **Tested:** `test-switch.sh` round-trips two clones through `switch-out`/`switch-in`, **11/11**
    assertions passing — WIP (modified + untracked + deleted), a local-only branch with an unpushed
    commit, a clean pushed branch, source-working-tree-untouched, **plus** the self-verify markers
    and a pre-existing-untracked-file case (the scenario that made an earlier build falsely report
    success on machine 2 — now handled and verified).

- **Tool install specifics** to be enumerated in the plan:
  - Refbox: `rustup` default 1.85 + `rustup target add …`; Node v24 (nvm/nodesource); `just`
    and `gh` via apt or upstream installers.
  - Portal (baseline): .NET 10 SDK; pnpm via `corepack enable`; Flutter + fvm (repo-pinned
    Flutter 3.41.5); LocalStack and RavenDB via Docker; `backlog` (pnpm global); the
    failure-baseline tool; Dart MCP wiring.
  - Portal (on demand — NOT baseline, absent on machine 1): Android SDK + emulator, `sentry-cli`,
    `bruno`, standalone `playwright` (per the reconciled inventory / Decision #4).
  - Windows-side: Docker Desktop, VS Code + WSL/Claude Code extensions, Windows Terminal.

- **Second Rust toolchain (1.96.0)** present on the current machine is not required for the
  project build; install only if a specific need surfaces.
