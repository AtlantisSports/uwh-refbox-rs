# PR Review Standards

These rules define what makes a pull request ready to open and what the human must verify before
merging. They apply to every PR opened from this workspace.

## Before Opening a PR

Verify all of the following before asking the human to review:

**Quality gates:**
- [ ] `just check` passes locally (fmt, lint, tests, audit — all clean)
- [ ] No files changed outside the stated scope
- [ ] No `unwrap()` or `expect()` added without justification
- [ ] No new dependencies added without discussion

**Documentation:**
- [ ] PR title follows commit format: `type(scope): description`
- [ ] PR body contains a plain-language summary (see format below)
- [ ] PR body contains a scope statement (which crate(s) and why)
- [ ] PR body contains a "how to verify" section

**Git hygiene:**
- [ ] Branch name follows convention (or is a known legacy exception)
- [ ] Commits follow the commit message format
- [ ] No merge commits (rebase if needed)
- [ ] No force-pushes to `master`

## PR Body Format

Every PR must use this structure:

```
## What changed
[Plain English description of what this PR does — what behaviour changed or was added]

## Why
[Why this change was needed — the problem it solves or the feature it adds]

## Scope
[Which crates were modified: e.g., "Changes are limited to refbox/src/tournament_manager/"]

## How to verify
[Specific steps the reviewer can take to confirm the change works correctly]
```

## Non-Programmer Review Gate

The human reviews every PR using `docs/review-checklist.md` before merging. The PR body must
be written so that a non-programmer can complete that checklist without asking follow-up questions.

If the plain-language summary is unclear, Claude must rewrite it before the human reviews.

## Hotfix PRs

Hotfixes (branch type `hotfix/`) are for urgent fixes to production or legacy deployments. They
follow all the same rules but may skip waiting for full CI on intermediate commits (not on the
final merge commit). Document clearly what the hotfix addresses and what version of the software
it targets.

## What Triggers a Re-Review

If any of the following happen after a review has started, the PR must be re-reviewed from
the beginning:
- New commits are pushed that change behaviour (not just formatting)
- Files are added or removed from the diff
- CI status changes from green to red
