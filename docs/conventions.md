# Conventions

This is the authoritative reference for all naming and formatting conventions in this workspace.
When in doubt, check here first.

---

## Branch Naming

**Format:** `type/scope/description`

### Types

| Type | When to use |
|------|-------------|
| `fix` | Correcting a bug or unintended behaviour |
| `feat` | Adding new functionality |
| `chore` | Maintenance work: dependency updates, config changes, CI tweaks |
| `refactor` | Restructuring existing code without changing behaviour |
| `docs` | Documentation only changes |
| `hotfix` | Urgent fix for a production or legacy deployment |
| `wip` | Work in progress — not intended for a PR yet |
| `audit` | Auditing existing code (AI-assisted or otherwise) to catalog, prune, and document behaviour |

### Scopes

| Scope | Corresponds to |
|-------|---------------|
| `refbox` | The main refbox application crate |
| `schedule-processor` | The schedule processing CLI crate |
| `uwh-common` | The shared types and game logic library |
| `overlay` | The stream overlay application |
| `wireless-remote` | The embedded remote firmware |
| `alphagen` | The image processing utility |
| `fonts` | The embedded font definitions |
| `led-panel-sim` | The LED panel simulator |
| `matrix-drawing` | The matrix drawing primitives |
| `wireless-modes` | The LoRa mode definitions |
| `ci` | CI/CD workflow changes |
| `deps` | Dependency updates across the workspace |
| `workspace` | Changes affecting the workspace as a whole |

### Description

- Use kebab-case (words separated by hyphens)
- Keep it short — aim for under 40 characters
- Use imperative mood (describe what the branch *does*, not what it *did*)
- Lowercase only

### Examples

```
fix/refbox/confirm-score-timing
feat/schedule-processor/list-of-placements
hotfix/uwh-common/wire-format-version
chore/deps/update-iced-0-14
refactor/uwh-common/game-snapshot-fields
docs/workspace/add-conventions
wip/refbox/new-timeout-ui
```

### Exempt branches

The following branch names are not subject to this convention:
- `master`
- `staging`
- `pr/*` — legacy branches from before this convention was established; grandfathered in

---

## Commit Messages

**Format:** `type(scope): description`

- Same `type` and `scope` values as branch naming above
- Description: lowercase, imperative mood, no trailing period, max ~72 characters total
- The scope is optional if the change truly spans multiple areas (rare)

### Examples

```
fix(refbox): end confirm pause before starting clock
feat(schedule-processor): support list of placements
fix(uwh-common): correct half_time_duration detection
chore(deps): update iced to 0.14
docs(workspace): add conventions and development guide
refactor(overlay): simplify network reconnection logic
```

### Multi-line commits

For changes that need more explanation, add a blank line after the subject and write a body:

```
fix(refbox): end confirm pause before starting clock

Without ending the confirm pause first, the auto-confirm timeout fires
after the game has already transitioned to BetweenGames, causing a
spurious state transition. Ending the pause before starting the clock
prevents this.
```

---

## Pull Requests

Every PR must include:

1. **Title** — matches commit format: `type(scope): description`
2. **Plain-language summary** — what changed, in plain English (not programmer jargon)
3. **Scope statement** — which crate(s) were modified and why
4. **How to verify** — what to look for to confirm the fix or feature works

PRs must:
- Pass all CI checks before merging
- Have no files changed outside the stated scope
- Be reviewed by the human using `docs/review-checklist.md`

---

## What Never to Do

- Never force-push to `master`
- Never use `--no-verify` to skip pre-commit hooks
- Never amend a commit that has already been pushed to the remote
- Never add `git add -A` or `git add .` — always stage files explicitly by name
- Never open a PR without a plain-language description
- Never merge without CI passing

---

## Code Formatting

All Rust code must be formatted with `cargo fmt --all` before committing. The pre-commit hook
enforces this automatically. Run `just fmt` to format, or `just fmt-check` to check without
modifying files.

---

## Linting

All code must pass `cargo clippy --workspace --all-targets --all-features -- -D warnings` with
zero warnings before committing. Run `just lint` to check. CI enforces this on all platforms
(Linux, Windows, macOS).
