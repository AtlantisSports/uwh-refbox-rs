# 001 — Conventions and Validation System

**Date:** 2026-04-10
**Status:** accepted

## Context

The `uwh-refbox-rs` workspace had strong automated quality checks in CI (formatting, linting,
security audits, MSRV verification) but no documented conventions, no AI collaboration guidance,
and no protection against scope creep or poorly-directed AI-generated changes.

The primary developer is a non-programmer domain expert who relies entirely on Claude for all
implementation work. Without a structured system:

- Each Claude session starts with no shared context about the project's purpose, conventions, or
  active work
- There is no standard for branch naming or commit messages, making the git history hard to
  navigate
- There are no guardrails preventing Claude from making changes beyond what was requested
- There is no documented process for the non-programmer to review Claude's work before merging
- Important decisions and their reasoning are not recorded anywhere

The `uwh-portal` project (a related project by the same team) has an established system using
CLAUDE.md files, `.claude/rules/`, and a `docs/` directory that serves as the reference model.

## Decision

Establish a conventions and validation system for this workspace, modelled on the uwh-portal
system but tailored to:

- A Rust workspace (vs. a .NET/React/Flutter monorepo)
- Embedded targets (wireless-remote requires special handling)
- A non-programmer lead who reviews work without reading code

The system consists of:

1. **`CLAUDE.md`** (root + per-crate) — Session context loaded at the start of every Claude
   session. Contains human profile, workspace map, all conventions, and key commands.

2. **`.claude/rules/`** — Behaviour rules for Claude: scope enforcement, communication style,
   workspace navigation, Rust-specific patterns, and embedded target rules.

3. **`docs/`** — Human-readable documentation: domain knowledge, workspace map, conventions
   reference, development guide, and a non-programmer review checklist.

4. **`docs/decisions/`** — Architecture decision records (this directory).

5. **`Justfile`** — Task runner providing single-command access to all common operations,
   mirroring exactly what CI runs.

6. **`scripts/pre-commit`** — Pre-commit hook that validates branch naming and code formatting
   before every commit.

## Consequences

- Every new Claude session has immediate context about the project without needing the user to
  re-explain it
- Branch names follow a consistent `type/scope/description` format, making the git history
  navigable at a glance
- Claude is explicitly constrained to the stated scope of each change
- The non-programmer has a concrete checklist for reviewing Claude's work before merging
- `just check` is the single command to validate the full quality suite locally
- Legacy `pr/*` branches are grandfathered and exempt from the naming convention
- The wireless-remote embedded firmware has explicit handling rules that prevent accidental
  breakage
- Future decisions are recorded here, making them queryable by future Claude sessions
