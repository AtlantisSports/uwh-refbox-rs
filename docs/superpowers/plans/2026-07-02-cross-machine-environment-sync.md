# Cross-Machine Environment Sync — Implementation Plan

> **For agentic workers:** Execute task-by-task with review checkpoints. Steps use checkbox
> (`- [ ]`) syntax for tracking. Claude runs every command; Eric is a non-programmer, so each
> task states its plain-English intent first. **Any action that creates a GitHub repo, pushes,
> or is otherwise hard to undo is an explicit approval gate — show Eric what will happen and
> wait for "go" before running it.** Never offer to push; Eric initiates.

**Goal:** Stand up a second Windows 11 machine that matches the current one for both
`uwh-refbox-rs` and `uwh-portal`, and keep the two matched with a save/restore switch routine.

**Spec:** `~/.claude/plans/2026-07-02-cross-machine-environment-sync-design.md` (currently the
untracked note at `uwh-refbox-rs/docs/superpowers/specs/2026-07-02-cross-machine-environment-sync-design.md`).

**RFC / core-team reference pattern:** N/A — this is personal developer-workflow infrastructure
(shell + git + tool installs), not a portal/refbox product feature. The clubs-rewrite
preconditions in the project-tuned writing-plans skill (RFC lookup, `dotnet test` tasks,
spike-review, rewrite-tracker) do not apply and are intentionally omitted.

**Architecture:** A private `e-straily/claude-workspace` repo tracked in place at `~/.claude`
(default-deny ignore + safe-list) carries Claude's memory/settings. Two scripts —
`switch-out` and `switch-in` — save/restore that repo and park/restore unfinished work for a
**curated list of active worktrees** (`carry-list.tsv`) using throwaway `eric/carry/<branch>` branches,
never touching real branches or open PRs. Machine 2's toolchain is installed via the existing portal setup docs
(`docs/getting-started.md`, `tools/doctor.sh`, `/citizen-setup`) plus the refbox tools, matched
to versions captured from machine 1.

**Platforms:** Local dev environment only (WSL2/Ubuntu on Windows 11). No product code changes.

**Where the plan + spec live:** both are untracked local notes in the refbox repo (see the
Task 1.4 deviation in STATUS below) and are **not** committed to the shared refbox/portal repos.

---

## STATUS (updated 2026-07-03)

**Machine-1 side EXECUTED.** Phases 0–2 done; `e-straily/claude-workspace` created **private**
and pushed (`~/.claude` @ `bb89203`); the **10 curated `eric/carry/*` branches** pushed (9 →
`zerotomvp/uwhportal`, 1 → `AtlantisSports/uwh-refbox-rs`), including the previously-local-only
`eric/carry/refbox/integration`. Source worktrees left untouched. This plan now serves **Phases 3–6
(machine 2)** + **Phase 7 (ongoing switch)**.

**Deviations from the plan as first written (all intentional):**
- **Curated carry-list, not a blind sweep.** Machine 1 has ~55 worktrees; only the **10 active**
  ones are carried (`~/.claude/plans/scripts/carry-list.tsv`). The rest stay on machine 1,
  retrievable. See design doc Part 3.
- **As-built scripts supersede the Phase 2 drafts below.** Real scripts at
  `~/.claude/plans/scripts/{switch-out,switch-in,carry-list.tsv,scan-secrets,test-switch.sh}`
  use `commit-tree`/`cherry-pick -n` (not commit-onto-branch + `reset HEAD~1`); `test-switch.sh`
  passes **8/8**.
- **`scan-secrets` credential-shape gate added** (replaces the keyword "token" grep): catches
  JWT/Bearer, private-key headers, password/secret assignments incl. escaped-JSON, and 32+ char
  entropy (config files only). `switch-out` aborts the config push unless `SWITCH_ACK_SECRETS=1`,
  and the EXTRAS hatch needs `SWITCH_ACK_EXTRAS=1`. settings.json legitimately holds 2 dev bearer
  tokens + 3 mock passwords (localhost — accepted in a private repo).
- **Task 1.4 DEFERRED.** The design doc + this plan were **not** moved into `~/.claude/plans/`
  nor deleted from the refbox repo — a second Claude session is co-editing the design doc there,
  so moving it would disrupt that. Both remain in `uwh-refbox-rs/docs/superpowers/` and reach
  machine 2 when that repo is cloned.
- **Editor + shell env added** (`~/.claude/plans/env-snapshot/`): a VS Code extensions list (25)
  + shell/git dotfiles (`.bashrc`/`.profile`/`.gitconfig`, credential-scanned clean). Restore
  steps (extensions reinstall, dotfile copy, VS Code Settings Sync toggle) are in
  `~/.claude/plans/machine2-handover.md` step 8.

---

## Phase 0 — Capture machine 1's exact state (run on machine 1)

Purpose: record what "matching" actually means, so machine 2 targets real values instead of
guesses. Outputs are plain notes that become part of the backup repo.

### Task 0.1: Record tool versions

**Files:** Create `~/.claude/plans/machine1-versions.md`

- [ ] **Step 1: Collect versions**
```bash
{
  echo "# Machine 1 tool versions — $(date -u +%FT%TZ)"
  echo
  for c in "rustc --version" "cargo --version" "rustup show" "just --version" \
           "gh --version" "git --version" "node --version" "pnpm --version" \
           "$HOME/.dotnet/dotnet --version" "fvm --version" "docker --version"; do
    echo "## $c"; eval "$c" 2>&1 | head -20; echo
  done
  echo "## flutter (via fvm)"; fvm flutter --version 2>&1 | head -5; echo
  echo "## docker images (ravendb / localstack)"; docker images | grep -Ei 'raven|localstack' || true
} > ~/.claude/plans/machine1-versions.md
```
- [ ] **Step 2: Show Eric the file and confirm it looks complete.** Add anything missing
  (e.g. Android SDK level) by hand.

### Task 0.2: Record the worktree map

**Files:** Create `~/.claude/plans/machine1-worktrees.md`

- [ ] **Step 1: List worktrees for both repos**
```bash
{
  echo "# Machine 1 worktree map — $(date -u +%FT%TZ)"
  for r in "$HOME/projects/uwh-refbox-rs" "$HOME/projects/uwh-portal"; do
    echo; echo "## $r"; git -C "$r" worktree list
  done
} > ~/.claude/plans/machine1-worktrees.md
```
- [ ] **Step 2:** Confirm every extra working folder Eric cares about appears. This map drives
  worktree recreation in Task 6.1.

### Task 0.3: List important ignored local config

**Files:** Create `~/.claude/plans/machine1-ignored-config.md`

- [ ] **Step 1: Find gitignored-but-present files that machine 2 will need re-created** (local
  workarounds and dev configs — **not** secrets):
```bash
{
  echo "# Ignored local config to reproduce on machine 2 (NO secrets) — $(date -u +%FT%TZ)"
  for r in "$HOME/projects/uwh-portal" "$HOME/projects/uwh-refbox-rs"; do
    echo; echo "## $r"
    git -C "$r" status --ignored --porcelain | grep '^!!' | grep -Ei \
      'jest\.local|appsettings\.Development|\.env|default-config' || echo "(none matched)"
  done
} > ~/.claude/plans/machine1-ignored-config.md
```
- [ ] **Step 2:** For each listed file, note in the doc whether it holds a secret (if yes → it
  is re-entered fresh on machine 2, contents NOT copied) or is safe boilerplate (copy the
  contents into the note). Include refbox `~/.config/refbox/default-config.toml`.

---

## Phase 1 — Create the `claude-workspace` backup repo (run on machine 1)

### Task 1.1: Create the private GitHub repo

- [ ] **Step 1 (APPROVAL GATE): Create the repo.** Show Eric this command, wait for "go":
```bash
gh repo create e-straily/claude-workspace --private \
  --description "Personal Claude Code config, memory, plans, and cross-machine switch scripts"
```
Expected: repo created, private.

### Task 1.2: Track `~/.claude` in place with a default-deny ignore

**Files:** Create `~/.claude/.gitignore`

- [ ] **Step 1: Initialize (only if not already a repo)**
```bash
[ -d "$HOME/.claude/.git" ] || git -C "$HOME/.claude" init
git -C "$HOME/.claude" remote add origin git@github.com:e-straily/claude-workspace.git \
  2>/dev/null || git -C "$HOME/.claude" remote set-url origin git@github.com:e-straily/claude-workspace.git
```
- [ ] **Step 2: Write the safe-list ignore file** at `~/.claude/.gitignore`:
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
```

### Task 1.3: Verify only the safe-list is captured — SAFETY GATE

- [ ] **Step 1: Stage and inspect**
```bash
git -C "$HOME/.claude" add -A
git -C "$HOME/.claude" status --short
```
- [ ] **Step 2: Prove the danger files are ignored**
```bash
for f in .credentials.json history.jsonl sessions/ telemetry/ cache/ \
         mcp-needs-auth-cache.json settings.local.json; do
  printf '%s -> ' "$f"; git -C "$HOME/.claude" check-ignore -v "$f" || echo "NOT IGNORED"
done
```
Expected: every credential/cache/session path resolves as ignored. `settings.local.json` is
intentionally NOT ignored (it is on the safe-list). **Do not rely on eyeballing — run the
credential-shape scan** `bash ~/.claude/plans/scripts/scan-secrets --staged "$HOME/.claude"`
and review the redacted hits before continuing (machine 1: 2 dev bearer tokens + 3 mock
passwords, localhost — acceptable in a private repo). `switch-out` enforces this gate
automatically (aborts unless `SWITCH_ACK_SECRETS=1`).
- [ ] **Step 3: Confirm the staged list** contains only: `.gitignore`, `CLAUDE.md`,
  `settings.json`, `settings.local.json`, `keybindings.json`, and files under `hooks/`,
  `skills/`, `plans/`, and `projects/*/memory/`. **Show Eric this list. Do not proceed if
  anything else appears.**

### Task 1.4: Move the spec + this plan into the backup repo

- [ ] **Step 1: Relocate both notes into `~/.claude/plans/`**
```bash
mkdir -p ~/.claude/plans
git mv -k 2>/dev/null || true   # (no-op guard)
cp "$HOME/projects/uwh-refbox-rs/docs/superpowers/specs/2026-07-02-cross-machine-environment-sync-design.md" \
   ~/.claude/plans/2026-07-02-cross-machine-environment-sync-design.md
cp "$HOME/projects/uwh-refbox-rs/docs/superpowers/plans/2026-07-02-cross-machine-environment-sync.md" \
   ~/.claude/plans/2026-07-02-cross-machine-environment-sync.md
```
- [ ] **Step 2: Delete the refbox-repo copies** so they don't linger as stale untracked notes
  in a shared repo:
```bash
rm "$HOME/projects/uwh-refbox-rs/docs/superpowers/specs/2026-07-02-cross-machine-environment-sync-design.md" \
   "$HOME/projects/uwh-refbox-rs/docs/superpowers/plans/2026-07-02-cross-machine-environment-sync.md"
```

### Task 1.5: First commit + push

- [ ] **Step 1: Stage and show the diff summary**
```bash
git -C "$HOME/.claude" add -A
git -C "$HOME/.claude" status --short
```
- [ ] **Step 2 (APPROVAL GATE): Commit + push after Eric approves the staged list**
```bash
git -C "$HOME/.claude" commit -m "chore: seed claude-workspace (config, memory, plans, spec)"
git -C "$HOME/.claude" branch -M main
git -C "$HOME/.claude" push -u origin main
```
Expected: `claude-workspace` on GitHub now holds the safe-list only.

---

## Phase 2 — The switch-ritual scripts (write on machine 1, store in the backup repo)

> These scripts are the heart of the "keep them matched" routine.
>
> **✅ AS-BUILT (2026-07-03): the code blocks in Tasks 2.1–2.2 below are the original DRAFT and
> were superseded.** The real, tested scripts live at `~/.claude/plans/scripts/` (`switch-out`,
> `switch-in`, `carry-list.tsv`, `scan-secrets`, `test-switch.sh`). They use a curated
> `carry-list.tsv` (not a `git worktree list` sweep) and `commit-tree`/`cherry-pick -n` (not
> commit-onto-branch + `reset HEAD~1`), gated by the `scan-secrets` credential scan;
> `test-switch.sh` passes 8/8. Treat the drafts below as design notes only. They live under
> `~/.claude/plans/scripts/` so they sync to both machines.

### Task 2.1: Write `switch-out`

**Files:** Create `~/.claude/plans/scripts/switch-out`

- [ ] **Step 1: Write the script**
```bash
#!/usr/bin/env bash
# switch-out — run when LEAVING a machine.
# 1) backs up ~/.claude, 2) parks unfinished work in every repo + worktree onto throwaway
# eric/carry/<branch> branches (real branches/PRs untouched).
set -uo pipefail

REPOS=( "$HOME/projects/uwh-refbox-rs" "$HOME/projects/uwh-portal" )
# Ignored-but-actively-edited files to carry (relative to each repo root). NO secrets.
# Stable local config is set up at bootstrap instead; leave empty unless a real need appears.
EXTRAS=( )
HOST="$(hostname)"

echo "==> 1/2 Backing up ~/.claude"
git -C "$HOME/.claude" add -A
git -C "$HOME/.claude" diff --cached --quiet || \
  git -C "$HOME/.claude" commit -m "backup: $HOST $(date -u +%FT%TZ)"
git -C "$HOME/.claude" push origin HEAD

echo "==> 2/2 Parking unfinished work"
for repo in "${REPOS[@]}"; do
  [ -e "$repo/.git" ] || { echo "  skip (missing): $repo"; continue; }
  while read -r wt; do
    [ -n "$wt" ] || continue
    br="$(git -C "$wt" symbolic-ref --quiet --short HEAD || true)"
    [ -n "$br" ] || { echo "  [$wt] detached — skipped"; continue; }
    # keep the real branch synced when it tracks an upstream
    if git -C "$wt" rev-parse --abbrev-ref @{u} >/dev/null 2>&1; then
      git -C "$wt" push origin "$br" || echo "  [$wt] real-branch push skipped"
    fi
    git -C "$wt" add -A
    for f in ${EXTRAS[@]+"${EXTRAS[@]}"}; do [ -e "$wt/$f" ] && git -C "$wt" add -f "$f"; done
    if git -C "$wt" diff --cached --quiet; then
      git -C "$wt" reset -q; echo "  [$wt] clean"; continue
    fi
    tree="$(git -C "$wt" write-tree)"
    commit="$(git -C "$wt" commit-tree "$tree" -p HEAD -m "carry: $br @ $HOST")"
    git -C "$wt" branch -f "eric/carry/$br" "$commit"
    git -C "$wt" reset -q               # unstage; working tree untouched
    git -C "$wt" push -f origin "eric/carry/$br"
    echo "  [$wt] parked -> eric/carry/$br"
  done < <(git -C "$repo" worktree list --porcelain | awk '/^worktree /{print $2}')
done
echo "==> switch-out complete."
```

### Task 2.2: Write `switch-in`

**Files:** Create `~/.claude/plans/scripts/switch-in`

- [ ] **Step 1: Write the script**
```bash
#!/usr/bin/env bash
# switch-in — run when ARRIVING at a machine.
# 1) restores ~/.claude, 2) recreates worktrees + restores parked work exactly as left.
set -uo pipefail

REPOS=( "$HOME/projects/uwh-refbox-rs" "$HOME/projects/uwh-portal" )

echo "==> 1/2 Restoring ~/.claude"
git -C "$HOME/.claude" fetch origin
git -C "$HOME/.claude" reset --hard origin/main   # single-writer config repo: safe

echo "==> 2/2 Restoring parked work"
for repo in "${REPOS[@]}"; do
  [ -e "$repo/.git" ] || { echo "  skip (missing): $repo"; continue; }
  git -C "$repo" fetch origin --prune
  while read -r ref; do
    br="${ref#origin/eric/carry/}"
    # locate the worktree on $br; create a sibling one if none exists
    wt="$(git -C "$repo" worktree list --porcelain \
          | awk -v b="refs/heads/$br" '/^worktree /{p=$2} /^branch /{if($2==b)print p}')"
    if [ -z "$wt" ]; then
      wt="${repo}-$(echo "$br" | tr '/' '-')"
      git -C "$repo" worktree add "$wt" "$br" 2>/dev/null \
        || git -C "$repo" worktree add -b "$br" "$wt" "origin/$br"
    fi
    git -C "$wt" checkout "$ref" -- .   # bring carry contents into the working tree
    git -C "$wt" reset -q               # unstage -> modified/untracked exactly as left
    echo "  [$wt] restored from $ref"
  done < <(git -C "$repo" for-each-ref --format='%(refname:short)' 'refs/remotes/origin/carry')
done
echo "==> switch-in complete."
```
- [ ] **Step 2: Make both executable**
```bash
chmod +x ~/.claude/plans/scripts/switch-out ~/.claude/plans/scripts/switch-in
```

### Task 2.3: Dry-run round-trip test on a scratch repo — VERIFY GATE

Purpose: prove the park/restore logic reproduces state exactly, including the tricky cases,
before it ever touches Eric's real work.

- [ ] **Step 1: Build a scratch repo with a modified file, an untracked file, and a deletion**
```bash
S="$(mktemp -d)"; cd "$S"
git init -q origin.git --bare
git clone -q "$S/origin.git" work; cd work
printf 'keep\n' > tracked.txt; printf 'old\n' > todelete.txt
git add -A; git commit -qm base; git push -q origin HEAD:refs/heads/main
git branch -M main
echo "modified" >> tracked.txt        # modified tracked file
rm todelete.txt                        # deletion
printf 'new\n' > untracked.txt         # untracked file
git status --short
```
- [ ] **Step 2: Park it** using the same commands as `switch-out` (REPOS temporarily pointed at
  `$S/work`), then record `git status --short` output as the expected state.
- [ ] **Step 3: Wipe the working changes** (`git checkout -- . && git clean -fd`), then run the
  `switch-in` restore commands against `$S/work`.
- [ ] **Step 4: Compare** `git status --short` to the recorded expected state.
  Expected: `tracked.txt` modified, `untracked.txt` untracked, `todelete.txt` deleted — an
  exact match. **If deletions do not survive, fix the restore step** (use
  `git read-tree -u -m HEAD "$ref"` + `git reset -q` instead of `checkout -- .`) and re-test.
- [ ] **Step 5: Clean up** `rm -rf "$S"`.

### Task 2.4: Commit the scripts

- [ ] **Step 1: Stage + show diff**
```bash
git -C "$HOME/.claude" add plans/scripts
git -C "$HOME/.claude" status --short
```
- [ ] **Step 2 (APPROVAL GATE): Commit + push after approval**
```bash
git -C "$HOME/.claude" commit -m "feat: switch-out/switch-in cross-machine scripts (tested)"
git -C "$HOME/.claude" push origin main
```

---

## Phase 3 — Machine 2: Windows base

### Task 3.1: Enable WSL2 + Ubuntu 24.04

- [ ] **Step 1:** In an elevated Windows PowerShell:
```powershell
wsl --install -d Ubuntu-24.04
```
- [ ] **Step 2:** Reboot if prompted; launch Ubuntu; create the **same username `estraily`**
  (so `~/.claude/projects/...` memory paths and `~/projects/...` code paths match machine 1).
- [ ] **Step 3: Verify**
```bash
lsb_release -a    # Ubuntu 24.04; whoami -> estraily
```

### Task 3.2: Install the Windows-side apps

- [ ] **Step 1:** Install **Docker Desktop** (Windows) and enable *Settings → Resources → WSL
  integration* for Ubuntu-24.04.
- [ ] **Step 2:** Install **VS Code** (Windows) + the **WSL** and **Claude Code** extensions,
  and **Windows Terminal**.
- [ ] **Step 3: Verify** from inside WSL:
```bash
docker version    # client + server both respond
code --version
```

---

## Phase 4 — Machine 2: WSL toolchain (match machine1-versions.md)

### Task 4.1: Refbox toolchain

- [ ] **Step 1: Rust (rustup) at the pinned version + targets** (read exact version from
  `machine1-versions.md`):
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
. "$HOME/.cargo/env"
rustup toolchain install 1.85.0 && rustup default 1.85.0
rustup target add aarch64-unknown-linux-gnu x86_64-pc-windows-gnu x86_64-unknown-linux-gnu
```
- [ ] **Step 2: `just`, `gh`, git, Node v24**
```bash
cargo install just
sudo apt-get update && sudo apt-get install -y git
# gh: official apt repo (see https://github.com/cli/cli/blob/trunk/docs/install_linux.md)
# Node v24 via nvm:
curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.40.1/install.sh | bash
. "$HOME/.nvm/nvm.sh" && nvm install 24
```
- [ ] **Step 3: Verify** each version matches `machine1-versions.md`:
```bash
rustc --version; just --version; gh --version; node --version; git --version
```

### Task 4.2: Portal toolchain (reuse existing setup docs — do not re-derive)

- [ ] **Step 1:** In the cloned portal repo (after Task 5.2, or clone early), run the project's
  own setup path rather than hand-installing:
```bash
tools/doctor.sh          # reports what's installed / missing
```
  Then in Claude Code run **`/citizen-setup`** and follow `docs/getting-started.md` to install
  what doctor flags: **.NET 10 SDK** (`~/.dotnet`), **pnpm** (via corepack), **FVM + Flutter**
  (+ Android SDK + emulator), **backlog.md**, and the other CLIs (**sentry-cli**,
  **playwright-cli**, **bruno**, the **failure-baseline** tool).
- [ ] **Step 2: Start the local data services** (Docker):
```bash
# RavenDB (per docs/getting-started.md):
docker run -d -p 8080:8080 -p 38888:38888 \
  -v ~/RavenDB/Data:/opt/RavenDB/Server/RavenData ravendb/ravendb:7.2-ubuntu-latest
# LocalStack S3 (per tools/start-localstack.sh):
tools/start-localstack.sh
```
- [ ] **Step 3: Create the dev database + seed** (per getting-started §Database Setup):
  create database `uwhportal-dev` in RavenDB Studio (http://localhost:8080), run the API once
  to apply migrations, then `cd tools/seed-cli && $HOME/.dotnet/dotnet run -- reset`.
- [ ] **Step 4: Verify** `tools/doctor.sh` reports all-green (matching machine 1).

### Task 4.3: Claude Code + Superpowers

- [ ] **Step 1:** Install Claude Code (per its docs).
- [ ] **Step 2:** Re-install the Superpowers plugin via the plugin marketplace (it is not
  synced — it's a versioned cache).
- [ ] **Step 3: Verify** `/superpowers` skills are listed.

---

## Phase 5 — Machine 2: logins, code, and Claude's brain

### Task 5.1: Fresh logins (no secret files copied)

- [ ] **Step 1: New SSH key → GitHub**
```bash
ssh-keygen -t ed25519 -C "estraily@atlantissports.org"
gh auth login          # then: gh ssh-key add ~/.ssh/id_ed25519.pub
ssh -T git@github.com  # expect the success greeting
```
- [ ] **Step 2:** Log in to **Claude** (Claude Code) and **Sentry** (`sentry-cli login`).

### Task 5.2: Clone both repos (same paths as machine 1)

- [ ] **Step 1:**
```bash
mkdir -p ~/projects && cd ~/projects
git clone git@github.com:AtlantisSports/uwh-refbox-rs.git
git clone git@github.com:zerotomvp/uwhportal.git uwh-portal
```
- [ ] **Step 2: Verify** both clone and the paths match `machine1-worktrees.md` main entries.

### Task 5.3: Attach `claude-workspace` to the existing `~/.claude`

> `~/.claude` already exists after installing Claude Code, so this is **not** a plain
> `git clone` into an empty folder.

- [ ] **Step 1: Point the existing folder at the repo and check out, keeping local files**
```bash
cd "$HOME/.claude"
git init
git remote add origin git@github.com:e-straily/claude-workspace.git
git fetch origin
git checkout -f main            # brings memory/settings/plans/scripts down; ignore-list protects local caches
chmod +x plans/scripts/switch-out plans/scripts/switch-in
```
- [ ] **Step 2: Verify** memory is present:
```bash
ls ~/.claude/projects/*/memory/ | head; cat ~/.claude/CLAUDE.md | head
```

### Task 5.4: Reproduce ignored local config + portal token

- [ ] **Step 1:** For each file in `machine1-ignored-config.md` marked "safe boilerplate",
  create it with the recorded contents (e.g. the local jest config, `appsettings.Development.json`
  placeholders, refbox `~/.config/refbox/default-config.toml`).
- [ ] **Step 2:** Files marked "secret" are **entered fresh**, not copied — including the
  **portal token** (entered once in the refbox app).
- [ ] **Step 3: Verify** none of these were pulled from git (they are ignored by design).

---

## Phase 6 — Machine 2: restore work + verify (acceptance criteria)

### Task 6.1: Restore parked work + recreate worktrees

- [ ] **Step 1:** Recreate the worktrees listed in `machine1-worktrees.md` (or let `switch-in`
  create them from `eric/carry/*`), then run:
```bash
~/.claude/plans/scripts/switch-in
```
- [ ] **Step 2: Verify** each repo's `git status` shows the expected uncommitted/untracked work
  restored, and `git worktree list` matches `machine1-worktrees.md`.

### Task 6.2: Verify refbox

- [ ] **Step 1:**
```bash
cd ~/projects/uwh-refbox-rs && just check
```
Expected: PASS.
- [ ] **Step 2:** Launch refbox and confirm it runs.

### Task 6.3: Verify portal (full parity)

- [ ] **Step 1: Build**
```bash
cd ~/projects/uwh-portal/api && $HOME/.dotnet/dotnet build
cd ~/projects/uwh-portal && pnpm install && pnpm run typecheck
```
- [ ] **Step 2: Run the stack** — API (`cd api && $HOME/.dotnet/dotnet watch run`) and web
  (`pnpm --filter @underwater/web dev`); confirm:
```bash
curl -s localhost:5000/api/health; curl -s localhost:3000 >/dev/null && echo "web OK"
```
- [ ] **Step 3: Mobile**
```bash
cd ~/projects/uwh-portal/mobile-app && fvm dart analyze
```
Expected: analyze passes.

### Task 6.4: Verify Claude memory recall

- [ ] **Step 1:** In Claude Code on machine 2, ask about a known prior item (e.g. "what's the
  status of the dues void model / refbox parity engine?"). Expected: it recalls from synced
  memory — no re-explaining needed.

### Task 6.5: Full round-trip acceptance test

- [ ] **Step 1:** On machine 2, make a trivial uncommitted edit in a portal worktree, run
  `switch-out`, then on machine 1 run `switch-in`.
- [ ] **Step 2: Verify** the edit appears on machine 1 exactly, with **no `eric/carry/…` commit on
  any real branch or open PR** (`git log --oneline -5` on the real branch is unchanged).

---

## Phase 7 — Ongoing usage (reference, not one-time)

- **Leaving a machine:** run `~/.claude/plans/scripts/switch-out` (Claude does this when Eric
  says "I'm switching").
- **Arriving at a machine:** run `~/.claude/plans/scripts/switch-in` (Claude does this when Eric
  says "I'm back").
- **Rule:** one machine active at a time. If a tool version changes, update it on both machines
  and refresh `machine1-versions.md`.
- **Optional future add-on:** auto-`switch-out` on session end (deferred in the spec).

---

## Self-Review

- **Spec coverage:** every spec part maps to tasks — backup repo (Phase 1), switch scripts
  (Phase 2 + 7), Windows-side apps (Phase 3), refbox + full portal toolchain (Phase 4), fresh
  logins (Phase 5.1), code + Claude brain (Phase 5.2–5.3), ignored config (Phase 5.4), the
  worktree sweep and parking method A (Phase 2 + 6.1 + 6.5), and all acceptance criteria
  (Phase 6). No gaps.
- **Placeholder scan:** no "TBD"/"TODO". The `EXTRAS=( )` list is intentionally empty (stable
  ignored config is handled at bootstrap); the one conditional fix (deletions →
  `git read-tree`) is gated by the Task 2.3 test, which is the correctness check, not a
  placeholder.
- **Consistency:** repo paths, branch naming (`eric/carry/<branch>`), and the safe-list ignore
  pattern are identical across the scripts, Phase 1, and the spec. `switch-in` resets to
  `origin/main` matching the `branch -M main` in Task 1.5.
- **Safety:** every GitHub-affecting step (1.1, 1.5, 2.4) is an explicit approval gate; the
  safe-list is verified with `git check-ignore` before the first push (Task 1.3); no secret is
  ever committed or copied.
- **Non-programmer fit:** each task leads with plain-English intent; Claude runs all commands.
