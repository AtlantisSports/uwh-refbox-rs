# uwh-refbox-rs — Claude Session Guide

Read this file at the start of every session. It provides everything needed to work in this
workspace without requiring the human to re-explain context.

---

## Project Overview

This is the software system that manages underwater hockey (UWH) referee operations at
tournaments. It handles real-time game management (clock, scores, fouls, penalties) and
communicates with poolside hardware (LED scoreboard, wireless referee remote, stream overlay).

See `docs/domain.md` for a full plain-English explanation of the system and its components.

---

## Human Profile

**The human is a non-programmer and domain expert.** They are a tournament organizer with deep
knowledge of underwater hockey rules and operations, but no programming background.

This means:
- All explanations must be in plain English — no assumed programming knowledge
- All technical trade-offs must be framed in terms of outcomes and behaviour, not implementation
- Approval is required before creating branches, making commits, or pushing to the remote
- When CI fails, explain what failed in plain English before suggesting a fix
- Never assume intent — ask when a request is ambiguous

See `.claude/rules/communication.md` for full interaction rules.

---

## Session Startup Checklist

At the start of every session, before doing any work:

1. Check the current branch: `git rev-parse --abbrev-ref HEAD`
2. Check working state: `git status`
3. Check for active in-progress decisions: read `docs/decisions/` index
4. If there are uncommitted changes, ask the human what their status is before proceeding

---

## Workspace Map

| Crate | Role |
|-------|------|
| `refbox` | Main referee application — UI, game clock, scores, fouls |
| `uwh-common` | Shared library — core game types, portal API types, wire format |
| `schedule-processor` | CLI tool — processes tournament schedules before a tournament |
| `overlay` | Stream broadcast display — shows live game state on video stream |
| `beep-test` | Audio testing tool — tests buzzer/sound system independently |
| `matrix-drawing` | Drawing primitives for the LED panel display (no_std) |
| `fonts` | Embedded font data for the LED panel |
| `led-panel-sim` | LED panel simulator for testing without hardware |
| `alphagen` | Image processing utility for overlay assets |
| `wireless-modes` | LoRa radio mode definitions shared by refbox and wireless-remote |
| `wireless-remote` | **Embedded firmware** for the handheld referee button — separate workspace |

See `docs/workspace-map.md` for full details on each crate.

---

## Branch Naming Convention

**Format:** `type/scope/description`

| Types | Scopes |
|-------|--------|
| `fix` — bug fix | `refbox` |
| `feat` — new feature | `schedule-processor` |
| `chore` — maintenance | `uwh-common` |
| `refactor` — restructure | `overlay` |
| `docs` — documentation | `wireless-remote` |
| `hotfix` — urgent fix | `beep-test` |
| `wip` — work in progress | `ci`, `deps`, `workspace` |

**Examples:**
```
fix/refbox/confirm-score-timing
feat/schedule-processor/list-of-placements
hotfix/uwh-common/wire-format-version
chore/deps/update-iced-0-14
```

**Exempt:** `master`, `staging`, `pr/*` (grandfathered legacy branches)

---

## Commit Message Convention

**Format:** `type(scope): description`

- Lowercase, imperative mood, no trailing period, max ~72 characters
- Same `type` and `scope` values as branch names

**Examples:**
```
fix(refbox): end confirm pause before starting clock
feat(schedule-processor): support list of placements
chore(deps): update iced to 0.14
```

---

## Validation Commands

| What | Command |
|------|---------|
| Run everything (use before any PR) | `just check` |
| Format code | `just fmt` |
| Check formatting | `just fmt-check` |
| Run linter | `just lint` |
| Run tests | `just test` |
| Security scan | `just audit` |
| Build for Raspberry Pi | `just build-rpi` |
| Install pre-commit hook | `just install-hooks` |

Run `just` alone to see all available commands.

---

## Rules Reference

| File | Purpose |
|------|---------|
| `.claude/rules/scope.md` | No scope creep — flag before acting |
| `.claude/rules/communication.md` | Plain English; approval gates |
| `.claude/rules/workspace.md` | Which crates to touch for what |
| `.claude/rules/rust.md` | Rust patterns, no_std, iced, clippy |
| `.claude/rules/embedded.md` | wireless-remote special handling |
| `.claude/rules/pr-review.md` | PR checklist + non-programmer review |
| `.claude/rules/plan-execution.md` | Lean vs. heavy process; when to use which |

---

## Key Constraints

- **MSRV:** Rust 1.85 — do not use features from newer versions
- **Edition:** Rust 2024 for all crates
- **Clippy:** `-D warnings` — zero warnings, all platforms
- **no_std:** `uwh-common` and `matrix-drawing` must compile without std
- **wireless-remote:** separate toolchain, do not touch without discussion

---

## Active Work

Check `docs/decisions/` for recorded decisions and any in-progress context. The current state
of work is tracked there and in the git log.
