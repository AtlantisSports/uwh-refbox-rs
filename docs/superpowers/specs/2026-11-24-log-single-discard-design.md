# Log a hand-discarded queued result — design

**Status:** approved by the PO 2026-11-24. Follow-up to #4298.

## Why

DISCARD THIS SUBMISSION (queued-results page) removes one waiting game result and writes
nothing to the log. SWITCH ANYWAY writes one line per result it drops. So after a tournament
the log cannot show that a score was thrown away by hand.

## What

`PortalManager::discard` (refbox/src/portal_manager/mod.rs) writes one `warn` line, in the
same wording `discard_all_queued` uses, after the removal has saved:

    discarded queued result: event 1889-B game 14 (score and stats not sent)

so a search for "discarded queued result" finds every dropped score, whichever button dropped
it. If the save fails, the row is kept and nothing is logged as discarded (the caller's
existing "discard failed" error line stays).

- Non-goals: no screen change, no new translation; no change to SWITCH ANYWAY's behaviour.
- Parked (PO 2026-11-24): CR5, SWITCH ANYWAY looking dead on a failed save. Rare tap × storage
  dying mid-session; not worth a screen. Re-open only on a field report.

## Approaches

1. **Log in `PortalManager::discard` (chosen).** It knows the event, game, whether the score was
   already sent, and whether the save succeeded.
2. Log in the app's `PortalDiscardTapped` handler. Rejected: it would have to look the item up
   again to get the event and game, for no gain.

## Calls left to the builder

- Share the line's wording with `discard_all_queued` rather than duplicating it (builder's call).
- No new test: the crate has no log-capture harness and adding one is a new dependency, not
  worth it for one log line. The existing discard tests must stay green.
- Walkthrough: nothing visible on screen; report it as such.
