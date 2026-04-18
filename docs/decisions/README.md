# Architecture Decision Records

This directory records significant decisions made about the design, architecture, and direction
of this project. Each file is one decision.

These records exist so that future Claude sessions (and future contributors) can understand *why*
things are the way they are — not just *what* was decided, but the context that led to it. Without
this, decisions get revisited, re-debated, and sometimes accidentally reversed.

---

## When to Write a Decision Record

Write a new record when:

- A significant architectural choice was made (e.g., choosing a library, structuring a feature)
- A non-obvious trade-off was accepted (e.g., choosing simplicity over performance)
- Work was intentionally deferred or rejected
- A convention or process was established (like this conventions system)

You do not need to write a record for routine bug fixes or small features.

---

## Record Format

Create a new file named `NNN-short-description.md` (e.g., `002-portal-api-client.md`) and use
this template:

```markdown
# NNN — Title

**Date:** YYYY-MM-DD
**Status:** accepted

## Context

Why was this decision needed? What problem or question prompted it?

## Decision

What was decided?

## Consequences

What changes as a result of this decision? What becomes easier or harder?
What constraints does this create for future work?
```

**Status values:**
- `proposed` — under consideration, not yet acted on
- `accepted` — decided and in effect
- `deprecated` — was accepted but is no longer the current approach
- `superseded by NNN` — replaced by a later decision

---

## Index

| # | Title | Date | Status |
|---|-------|------|--------|
| [001](001-conventions-system.md) | Conventions and validation system | 2026-04-10 | accepted |
| [002](002-time-cve-msrv.md) | Time CVE and MSRV bump decision | 2026-04-10 | proposed |
| [003](003-scoresheet-style-architecture.md) | Scoresheet style architecture | 2026-04-12 | accepted |
| [004](004-xlsx-user-templates.md) | User-provided XLSX scoresheet templates | 2026-04-12 | accepted |
| [005](005-v040-feature-audit.md) | v0.4.0 feature audit and scope decisions | 2026-04-17 | accepted |
| [006](006-multi-remote-alarm-buttons.md) | Multi-remote alarm buttons | 2026-04-18 | proposed |
| [007](007-help-text-layout.md) | Help text layout and overflow | 2026-04-18 | proposed |
