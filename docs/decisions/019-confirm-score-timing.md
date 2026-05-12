# ADR 019 — Confirm-Score Timing Fix

**Status:** Accepted (retroactive)
**Date:** 2026-05-12
**Audit unit:** 1 — Confirm-score timing fix
**Audit PR:** *(to be filled at Final Integration)*

## Context

The refbox supports a "Confirm Score Required" mode controlled by an operator
setting. When turned ON, the end of the second half pauses for the operator to
confirm scores before the game transitions to between-games. When turned OFF,
the pause is brief and the operator can dismiss it immediately.

In the OFF case, the production handler for the dismiss action had a latent
state-machine bug: it started the next clock without first clearing the
confirm-pause state. The clock then advanced and the period transitioned to
`BetweenGames`, while `time_pause_confirmation` remained `Some`. About 90
seconds later, a background "is the pause over?" loop in `refbox/src/app/mod.rs`
called `end_confirm_pause` from `GamePeriod::BetweenGames` — a case the
function's `match` did not cover. The `unreachable!()` arm panicked, poisoned
the `tm` mutex, and made the refbox unusable until restart. The bug was
observed six times at tournaments on Jan 13, Jan 19, and Feb 24 2026.

The fix shipped in commit `0d895ca` (2026-04-10) with a follow-on Gherkin
specification in `54973a8` (2026-04-17). This ADR records what survived after
the AI Code Audit on 2026-05-12.

## Decision

The following behaviours are kept in the codebase, audited 2026-05-12.

### 1. Primary fix — pause state cleared before clock start (UI-facing)

In the dismiss-confirm-pause handler (`Message::ConfirmScores`, `confirm_score
== false` branch in [refbox/src/app/mod.rs](../../refbox/src/app/mod.rs)),
`end_confirm_pause` is called before `start_clock`. This mirrors the working
pattern already in `Message::ScoreConfirmation`.

```gherkin
Feature: Confirm-score timing fix

  @user_verified @tested_pass
  Scenario: Clock starts cleanly after the second half ends with confirm-score off
    Given the operator has "Confirm Score Required" set to OFF in Game Settings
    And a game has been configured and started
    And the second half has ended
    When the operator dismisses the score-confirmation prompt
    Then the refbox moves to the between-games period
    And the refbox remains fully responsive for at least 120 seconds afterwards
```

Verified on 2026-05-12 by manual reproduction in a running refbox: confirm-score
OFF, ~5-second half, operator dismissed the score-confirmation prompt; refbox
transitioned cleanly to between-games, stayed responsive through the full
120-second confirm-pause window and beyond (normal "Resetting game" message
logged at +2m11s). No panic, no mutex poison, no defensive-recovery warning.

### 2. Defensive fix in `end_confirm_pause` — graceful recovery (backend)

If `end_confirm_pause` is ever called when the game is not in a state where
score confirmation makes sense, the function now logs a warning, clears the
pause state silently, and returns `Ok(())` — instead of hitting the previous
`unreachable!()` branch and panicking the process.

The defensive fix is a belt-and-braces backstop: with the primary fix (item 1
above) in place, the regression path that exercised this branch is no longer
reachable from the production code. But a future bug in the same area that
once again left the pause state dangling will now log a warning rather than
poison the mutex and require a refbox restart at a live tournament.

Verified by a regression test in
[refbox/src/tournament_manager/mod.rs](../../refbox/src/tournament_manager/mod.rs)
test module: `test_end_confirm_pause_recovers_gracefully_from_unexpected_period`.
The test constructs the broken precondition (period `BetweenGames` + stale
`time_pause_confirmation`) and calls `end_confirm_pause` directly, mirroring
the async loop. With the defensive fix in place the test passes; with the
defensive arm temporarily reverted to `_ => unreachable!()` the test fails
with `internal error: entered unreachable code` — the exact signature of the
original tournament bug.

### 3. Justified `.unwrap()` calls in score-confirm handlers (code hygiene)

Both `tm.end_confirm_pause(now).unwrap()` calls in
[refbox/src/app/mod.rs](../../refbox/src/app/mod.rs) (the new one in
`Message::ConfirmScores` introduced by the fix, and the pre-existing
pattern-mate in `Message::ScoreConfirmation`) carry a two-line comment
explaining why they cannot panic: `end_confirm_pause`'s only `Err` is
`NotPaused`, and both message handlers are only dispatched while a
confirm-pause is active, so `time_pause_confirmation` is always `Some` when
control reaches the call. This satisfies the project rule in
`.claude/rules/rust.md` requiring justifying comments for any production
`.unwrap()` or `.expect()`.

### 4. Feature-spec documentation convention (workspace)

The directory [refbox/tests/features/](../../refbox/tests/features/) holds
Gherkin-style `.feature` specifications of observable behaviour, documented
in a README. The files are documentation-only at present; standing up a
cucumber harness to make them runnable is tracked as a post-v0.4.0 follow-up.
This is the project's chosen location for behaviour specifications — the
audit playbook's originally-prescribed `docs/audit-scenarios/` location
was rejected in favour of this existing convention.

### 5. `confirm_score_timing.feature` content (workspace)

A `.feature` file at
[refbox/tests/features/confirm_score_timing.feature](../../refbox/tests/features/confirm_score_timing.feature)
holds the Gherkin scenario embedded above (item 1), with a comment header
documenting the audit lineage and test sessions, and the tags
`@user_verified @tested_pass`. The file's content matches the scenario in
this ADR verbatim.

## Consequences

- Tournament operators can disable "Confirm Score Required" without risking
  refbox lockup at the end of the second half.
- The defensive fix in `end_confirm_pause` permanently raises the floor on
  this class of bug — any future regression that leaves the pause state
  dangling will surface as a warning log line rather than a mid-tournament
  process crash.
- The Gherkin scenario in `refbox/tests/features/confirm_score_timing.feature`
  is now the canonical operator-facing behaviour record, ready to become a
  runnable test when the cucumber harness lands.
- The regression test in `tournament_manager`'s test module prevents the
  defensive-recovery branch from being silently removed in future refactors.
- The justifying comments above the two score-confirm `.unwrap()` calls make
  the no-unwrap-without-justification project rule auditable: the rule's
  intent is now enforced by reading the code, not by trusting an absence.

## What was removed during audit

Nothing was removed during this audit. Every behaviour in the original diff
(commits `0d895ca` and `54973a8`) was reviewed in Task 4 and decided
`@user_verified`. The audit added a regression test, added two justifying
comments, and tightened the wording of the feature-spec file, but did not
delete any code or documentation that was in the original commits.

## Audit reference

- Audit branch: `audit/refbox/confirm-score-timing`
- Audit PR: *(to be filled at Final Integration)*
- Original commits:
  - `0d895ca` 2026-04-10 — `fix(refbox): call end_confirm_pause before start_clock when confirm_score is false`
  - `54973a8` 2026-04-17 — `docs(refbox): add confirm-score timing feature specification`
- Audit commits on this branch:
  - `2a8dcbc` — `chore(workspace): allow audit branch type in pre-commit hook and convention docs` *(workspace infrastructure caught up to the playbook)*
  - `0da3580` — `fix(refbox): document why end_confirm_pause unwraps are safe in score-confirm handlers`
  - `cc7aa42` — `docs(refbox): align confirm-score-timing scenario with audit unit 1`
  - `786cb5b` — `test(refbox): add regression test for confirm-score timing defensive fix`
  - `beeb552` — `docs(refbox): record test session 1 for confirm-score timing`
