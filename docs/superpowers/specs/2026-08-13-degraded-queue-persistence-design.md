# Degraded mode must not queue results where nothing will read them

**Date:** 2026-08-13
**Crate:** `refbox` only
**Follows:** PR #2319 (`fix/refbox/degraded-portal-startup-message`), merged 2026-08-13. That branch
fixed what degraded mode *says*; this one fixes where it *puts results*.
**Backlog origin:** `docs/backlog/degraded-queue-written-to-temp-dir/NOTE.md`

---

## The problem

A degraded-mode `PortalManager` stores its retry queue in `std::env::temp_dir()`. Results are still
queued in that mode — `enqueue_game_end` runs whenever an event is linked, with **no check for a
portal client** (`app/mod.rs:1389-1392`) — so a game finished during a degraded session is written
to the temp directory. A later healthy launch constructs the manager with the **real** config
directory and loads the queue from there, so it never sees those results. They are silently
abandoned.

Meanwhile the end-of-game banner tells the operator:

> *"Connection issue detected. Score will still be queued — find an admin to resolve."*

Which is true in letter and false in effect: it is queued, to a location nothing will ever read.

### The temp directory is not a decision — it is a default being reused out of context

`new_degraded()` was written for the case where **both** the config directory and the temp directory
reject I/O (`07466789`, 2026-04-21, added to stop a startup panic). For that case, pointing at the
temp directory is a reasonable "nowhere".

The **no-client** route was added later (`04eac281`, 2026-06-25, fixing finding M6 of the portal
review) and reuses the same constructor — even though in that case the disk is perfectly healthy and
`config_dir` is in scope, already created one line earlier (`app/mod.rs:2395`). Results land in the
temp directory because the constructor *assumes* the disk is broken, not because it is.

---

## Reachability: LOW, and recorded honestly

**This was weighed before building, and the decision to build anyway was Eric's, as insurance
against silent loss of tournament results.** The evidence is recorded here so nobody re-derives it
or later mistakes this for a frequent fault.

- **Never observed in the field.** Both routes into degraded mode came from reviews imagining
  failures, not from any tournament incident.
- **The queue-I/O route needs two independent filesystem failures** — the config dir *and* the temp
  dir both rejecting I/O. Effectively unreachable.
- **The no-client route is rarer than PR #2319's spec claims.** That spec calls a broken
  TLS/certificate store "the realistic trigger". On closer reading that is probably wrong:
  certificate *verification* happens per request, not at client construction, so a broken
  certificate store almost certainly produces per-request failures — which take the
  `TokenUnreachable` path that already reports an honest red. Every *construction* failure path is
  TLS-configuration related, and this call site sets none of those options (only `https_only` and
  `timeout`). No concrete field scenario reaching it has been identified.
- The Pi's root filesystem is a deliberately read-only overlay, which exists to prevent exactly the
  corruption that would be the likeliest cause.

**Correction owed to the earlier spec:** `2026-08-13-degraded-portal-startup-message-design.md`
overstates its trigger as realistic. That branch is still correct and worth keeping — it removes a
false instruction from a state the code can enter — but its likelihood wording should be softened.
Doing that is part of this work.

---

## THE TRAP: the obvious fix destroys results

Recorded first because it is the whole reason this design is not a one-line change.

`queue::save` is a **full atomic overwrite** of the queue file (`queue.rs:135` →
`write_atomic`, temp file then rename over the target). And a degraded manager starts from
`QueueFile::empty()` (`mod.rs:634`) — it never loads an existing queue, because it was built on the
assumption that the disk is unusable.

So simply pointing degraded mode at the real config directory produces this:

> A healthy session queues two games that could not be sent. The refbox restarts; this time the
> portal client fails to build, so it starts degraded. The operator finishes a third game. The save
> writes a **one-game file over the two-game file.** Two real results destroyed — by the change
> meant to protect them.

Today's behaviour is *safe* precisely because it writes somewhere useless. Any fix must therefore
load the existing queue before it can be allowed to write.

**And the corollary, missed in the first draft and caught by code review: if the load FAILED, it must
not write either.** A queue file that cannot be *read* can still be *replaced* — `queue::save`
renames over it, and rename needs write permission on the **directory**, not on the target file.
Verified: a `chmod 000` queue file is unreadable to `cat` yet a rename over it succeeds and the
contents are gone. So falling back to an empty queue while keeping the same write target destroys
real results, with no `portal_queue.corrupt.*` backup, and reports success.

That made the first implementation *more* destructive than the healthy path, which already handles
this case correctly: `new()` returns `Err`, and `app/mod.rs` falls back to the temp directory,
leaving the file intact. **Fix:** on a failed load the write target moves elsewhere and the
unreadable file is preserved byte-for-byte. The first attempt used `std::env::temp_dir()`, which a
second review showed was the same bug one branch over — see "the guards were positional" below. It is
now a fresh per-process directory.

---

## Design

| # | Change | Why |
|---|--------|-----|
| 1 | `new_degraded` takes the config directory instead of hard-coding `std::env::temp_dir()` | Results land where the queue actually lives, so a later healthy start finds them |
| 2 | It **loads any existing queue, best-effort** — `queue::load_or_empty`, falling back to empty on `Err` | Closes the trap. A degraded session must never overwrite results already waiting |
| 3 | Both call sites pass the real `config_dir` (`app/mod.rs:2404` and `:2433`) | The no-client case has a healthy disk. The last-resort case is no worse off than today: saves fail, the error is logged, the in-memory queue still serves the session |
| 4 | Drop the documented "performs no disk I/O" property, replacing it with the reason | That property existed because the constructor served only the both-directories-failed case. Now that it also serves the healthy-disk case, refusing to touch the disk *is* the bug |

Best-effort is deliberate throughout: a degraded manager must never fail to construct, because the
whole point of degraded mode is that the game can still run.

**Deliberately NOT copied from `new()`: the expiry sweep.** `PortalManager::new` calls
`sweep_expired()`, which archives and removes items older than 120 hours — and that writes.

**Corrected after code review:** the first draft of this spec said degraded construction simply
omits the sweep and "the next healthy start sweeps instead". That was **false**. `ui_tick()` also
sweeps, and `Message::PortalUiTick` is an unconditional 1 Hz subscription with no health gate — so a
degraded session was sweeping the real config directory within a second of launch. Skipping it at
construction bought nothing.

The sweep is therefore **gated on `!startup_problem` inside `ui_tick()`**, which makes the intent
real. Two reasons it matters: `portal_queue.expired.json` is surfaced nowhere in the UI, so an
archived result is unrecoverable by the operator; and a refbox whose clock reads days ahead (a
documented Pi failure mode) would otherwise archive still-pending results in the first second. The
next healthy session sweeps — the only session that could have uploaded them anyway.

### Found in review: adopting results makes them destroyable, so the rows are made inert

Closing the trap above created a second one, caught by code review rather than by this design.

Once a degraded session **adopts** the queue, those results become visible on the detail page. Any
item older than 30 minutes renders as *"Game X Score send error, tap to fix"*, whose attention page
says *"You can Retry if connection is verified, or discard to clear the error"*. In degraded mode:

- **Retry cannot work.** `force_submit` writes and pushes a queue snapshot, but `new_degraded` drops
  the command receiver and no background task exists, so the send goes nowhere. It *does* reset
  `queued_at`, so the row disappears for 30 minutes and looks like it succeeded.
- **Discard deletes permanently and does not archive**, unlike the expiry sweep.

Before this change the degraded queue was always empty, so no pre-existing result could be reached
this way. Adopting them without this guard would mean the branch written to protect results instead
hands the operator an on-screen path to destroying them.

**Fix:** `Message::PortalRowTapped` returns immediately when `has_startup_problem()`. Rows stay
visible but inert, which matches everything else in that state — the startup-failure row is not
tappable and REFRESH declines to spin. This deliberately does **not** touch `is_item_stuck`, so the
"stuck" rule is unchanged as required.

**No unit test:** the guard lives in `update()`, which this crate does not exercise directly. Stated
here rather than covered by a test that only looks like coverage.

### Second review: the guards were positional, so the class was not closed

A second adversarial pass (after PR #2336 merged) found that each earlier fix closed the case it was
pointed at and left a sibling of the same class:

| Round | Closed | Sibling left open |
| --- | --- | --- |
| 1 | Row taps blocked in `update()` | **RETRY ALL still live** — its own button, no guard |
| 2 | `config_dir` clobber on a failed read | **`temp_dir` clobber** — the fallback wrote to a queue it never read |

**RETRY ALL was a genuine regression** introduced by adopting the queue. The button greys out unless
something is unsent, so with the old always-empty degraded queue it was inert. Adopted results make
it live — and `retry_all` resets `queued_at`, so red "Score send error" rows turn yellow with
"attempt 0". The operator gets *positive confirmation of a success that cannot happen* (no background
task exists), and the only record of when each game ended is destroyed — the input to both the
30-minute escalation and the 120-hour expiry. It also contradicted the sweep gate added in the same
commit, which exists precisely to stop a degraded session ageing the queue.

**The structural fix: gate at the chokepoint, not at the routes.** `retry_all`, `discard`,
`force_submit`, `force_immediate_retry` and `request_stats_retry` now return early while
`startup_problem` is set. That closes every present *and future* UI route by construction, and — the
reason it matters beyond tidiness — it is **unit-testable**, unlike the `update()` guard. The view
additionally greys RETRY ALL and stops rendering pressable rows, so the operator sees inert controls
rather than buttons that depress and do nothing.

**The temp-dir fallback now uses a fresh per-process directory** (`refbox-degraded-<pid>`) rather
than the bare temp dir, because `std::env::temp_dir()` is the healthy path's own second choice and can
hold real undelivered results. A per-process directory is empty by construction, so there is nothing
to destroy. This also stopped the test suite overwriting the real shared `/tmp/portal_queue.json` —
which it had been doing, leaving a plausible fake result that a later fallback session would have
adopted and tried to upload.

**Correction to the sweep-gate rationale above:** "the next healthy session sweeps" is true but not
harmless. A healthy session sweeps inside `new()`, *before* the background task attempts anything, so
an adopted result already past 120 hours is archived **unattempted**, with the indicator Green. The
gate is a real improvement while an outage is under 120 hours; past that, this promise ends quietly.

### What this delivers — stated precisely, not optimistically

**Results survive and become recoverable with one RETRY ALL press. They do not upload entirely on
their own.**

`is_item_retry_eligible` returns false for anything past the 30-minute stuck threshold —
*"Stuck items wait for operator action"* (`health.rs:31-39`). Any realistic outage lasts longer than
30 minutes, so on the next healthy start these games are stuck: they appear as red rows on the
connection page and go up when the operator presses RETRY ALL (which resets `queued_at` and returns
them to the auto-retry pool).

That is still the difference between "recoverable in one tap" and "silently gone". It is not
"automatic".

---

## Acceptance criteria

1. **Recovery, end to end:** a degraded manager given directory D queues a game; a fresh healthy
   `PortalManager::new(D, …)` finds that game in its queue.
2. **The clobber guard:** directory D already holds a queue with two games; a degraded manager on D
   queues a third; **all three** are present on disk afterwards. *This test fails on the naive
   version of the fix and must be written to prove that.*
3. **Construction never invents or destroys a queue:** constructing a degraded manager must not
   create a queue file where none existed, and must not overwrite a valid one. It may now *read* it —
   that is the change.
   *Deliberate exception:* `queue::load_or_empty` renames an **unparseable** file to
   `portal_queue.corrupt.<ts>.json` before returning empty. That write is acceptable and wanted — it
   is exactly what a healthy start does, it preserves the bad file rather than deleting it, and the
   alternative would be leaving a corrupt file to be silently overwritten by the first save.
4. A degraded manager still shows red, still reports `token_expired` false, still shows the
   startup-failure row, and still spawns no background task — i.e. PR #2319's behaviour is intact.
5. `just check` green.

6. **A queue that could not be read is left byte-identical**, and is not the write target.
7. **A degraded session does not archive expired items** — they stay on disk for the next healthy
   session, and no `portal_queue.expired.json` is created.

Each new guard must be mutation-tested: break the fix, confirm the test fails, restore. All five
guards were: restoring each defect made exactly the intended test fail.

### Also noted by review, accepted rather than fixed

- **Two refbox processes sharing one config directory now interfere across the degraded/healthy pair,
  where a degraded instance used to be inert.** Two *healthy* processes already do this, and the
  repo's standing rule is one refbox at a time, so this widens a documented hazard rather than
  creating a new class of bug.
- **`queue_dir()` now points at the real queue, so the cross-tenant flush empties it** on a mode
  change — unarchived. This is arguably a net improvement: previously the flush hit the temp
  directory and left old-tenant results in the config directory for the next healthy launch to adopt.
  It now matches healthy behaviour. Recorded so nobody rediscovers it as a surprise.

---

## Non-goals

- **Not changing what "stuck" means.** `is_item_stuck` keys purely on age and never asks whether an
  item was ever *attempted*, so a game queued while the subsystem was dead — zero attempts, failed
  at nothing — still displays as *"Game X Score send error, tap to fix"*. That wording is wrong for
  it, and fixing it would remove the RETRY ALL step entirely. But the rule is shared by every queued
  game on every path, so it is a separate design with its own risk. **Eric's decision, 2026-08-13:
  keep it separate.** Own backlog note.
- **Not rewording the end-of-game banner.** Once results genuinely persist, "Score will still be
  queued" becomes true for the reachable case.
- Not making the portal work in degraded mode; not ADR 011's missing failure counter; not the
  silent-login dead end (`docs/backlog/portal-login-silent-when-no-client/NOTE.md`).

## Risk

Low-to-moderate — higher than PR #2319, because this one can destroy data if the trap is not closed.
One crate, no shared types, no wire format, no state machine, so `.claude/rules/plan-execution.md`
still puts it in the **lean** process. The mitigation is acceptance criterion 2, which must be
written to fail on the naive implementation.

The change is confined to a constructor with exactly two production call sites, both in
`app/mod.rs`, and to tests.
