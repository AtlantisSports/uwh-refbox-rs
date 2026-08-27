# PR Review Standards

These rules define what makes a pull request ready to open and what the human must verify before
merging. They apply to every PR opened from this workspace.

## The Three Pre-PR Checks

Every PR opened from this workspace, in any crate, has three checks. Two are mandatory and one is
discretionary. **State the status of all three, unprompted, whenever proposing a PR, calling work
"ready", or recommending a merge** — for each, exactly one of **done** (say when, and what it
found), **not done** (say why), or **unknown** (it could not be verified — say so rather than
guessing either way).

**1. Automated code review — mandatory, never skipped.** Run the built-in `code-review` skill
against the diff, and fix or explicitly answer every finding before the PR is opened. This is in
addition to `just check`, not a substitute for it: `just check` proves the code compiles, lints and
passes its tests, while the review reads the diff for defects no gate can see.

A review counts only if it covers the diff **as it now stands**. Commits that landed after it must
be disclosed either way, and there is exactly one exception to re-running: commits that implement
nothing but that review's own findings. Say so and state precisely what the delta is. Anything else
— a behaviour change of your own, a rebase, files added to or removed from the diff — means the
review no longer covers the diff and has to be run again. A review that ran but was never reported
has not been done.

**2. Manual walkthrough by the human — mandatory for any change with a visible effect.** Claude
launches the app; the human clicks through numbered steps Claude has written for them, and reports
back. Claude driving the app itself is check 3 and is never this one. Where a change is purely
internal with nothing to observe, say that explicitly instead of passing over the check in silence.

Before writing the steps, confirm which configuration, language or portal actually reaches the
screen in question — a walkthrough of the wrong state proves nothing. When more than one build of
the app can be running on this machine, confirm which binary owns the window before trusting what
is on screen (`readlink /proc/<pid>/exe`).

**3. Claude-driven verification — discretionary, and Claude recommends.** Claude driving the
running app itself: screenshots, a scripted UI pass, or a browser session. It suits anything
provable from the outside — a rendered state, a request that should or should not fire, a timing
window — and needs the human's explicit go-ahead before servers are started or an app is driven.
It is evidence *in addition to* check 2, never in place of it. Every disclosure should say whether
this one was worth running for this particular change, with a one-line reason.

**None of the three may be inferred from anything else.** A green CI run, passing tests, or GitHub
reporting `MERGEABLE` is not evidence for any of them. Where a check is missing, say so plainly and
give a recommendation on whether it matters for this change — do not proceed to the PR quietly.

These rules override any user-global instruction that treats the walkthrough as optional.

## Before Opening a PR

Verify all of the following before asking the human to review:

**Quality gates:**
- [ ] All three pre-PR checks above are reported (1 done; 2 done, or stated as nothing to
      observe; 3 answered either way)
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
- The branch is rebased onto a new base, or the base branch changes

The same triggers apply to the two mandatory checks above: a rebase or a behaviour commit stales
both the automated review and the human's walkthrough, and both have to be re-run and re-reported
rather than carried over — subject to the single exception stated in check 1, for commits that
implement nothing but that review's own findings.
