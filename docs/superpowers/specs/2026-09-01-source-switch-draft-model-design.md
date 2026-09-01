# Design — the settings editor drafts everything, APPLY commits it

**Status:** model agreed in principle by Eric 2026-09-01. NOT built. Own branch.
**Supersedes:** the 2026-08-28 amendment to ADR 017, which made the source buttons the commit
point. This puts them back to staging. Amend that ADR rather than contradicting it silently.
**Related:** PR #3082 (portal-token cross-site leak) — see "What this changes about #3082" below.

---

## The problem

The three source controls in one row on the Game page are not three of the same kind of control.

| Button | Message | Behaviour |
|---|---|---|
| MANUAL GAMES | `SelectGameSource` | Stages only. APPLY commits. CANCEL undoes it. |
| UWH PORTAL | `SwitchGameSource` | Commits, repoints, wipes the link, resets the clock, saves to disk, at the tap. CANCEL cannot undo it. |
| CUSTOM | `SwitchGameSource` | Same as PORTAL. |

Turning MANUAL GAMES *off* stages a move to Portal or Custom — the same destinations the other two
buttons reach immediately. So the same move is reversible or irreversible depending on which
button reaches it.

Four consequences, found by audit on 2026-09-01:

1. **ADR 017's 2026-08-28 amendment overstates.** It says "the source buttons are now the commit
   point" and "Game Options' APPLY no longer repoints the client". Both hold for two buttons out of
   three. Through MANUAL the Game page's APPLY still repoints: Custom -> MANUAL (client stays on the
   custom site) -> MANUAL off with Portal remembered -> APPLY repoints at `app/mod.rs:4470`.
2. **The mid-game rule is enforced twice, differently.** Tapping PORTAL/CUSTOM during a game is
   refused at the tap with a message; tapping MANUAL GAMES is accepted silently and only refused at
   APPLY. The rule is also written twice — `source_tap_outcome` and `refuse_repoint`.
3. **Tapping CUSTOM with no usable saved address commits the source but cannot move the client.**
   The refbox then reports CUSTOM while still talking to the Portal, with fetches suppressed and no
   explanation on screen.
4. **Three overlapping notions of "which site"** — `site_serves`, `site_generation`, `reply_source`
   — where the operator has one.

## The model

**Nothing is written until APPLY.** The settings editor holds a draft; the refbox goes on running
whatever it was running until APPLY commits the draft.

- All three source controls stage. None commits. They become the same kind of control.
- Changing the drafted source clears the drafted event/site/court/game. Returning to the source you
  came from lands on **blank** selections — there is no restore of what was cleared.
- Because the clearing is of a draft, CANCEL always means one thing: *forget the draft, keep
  running what I am running.* Today CANCEL cannot say that, which is why the code has to re-baseline
  the page after a tap so CANCEL cannot offer to undo it.
- The clock keeps showing the live game throughout. The event has not switched, so this is correct
  rather than confusing (Eric, 2026-09-01).

### The cascade

Each step is disabled and blank until the one above it is satisfied:

1. **EVENT** (Portal) or **SITE** (Custom) — nothing else is available until one is supplied.
2. **ACCESS TOKEN** — disabled and blank until an event/site is set.
3. **COURT** — disabled and blank until the access key is confirmed. Skipped when the event has
   only one court.
4. **GAME** — disabled and blank until a court is selected.
5. **APPLY** — disabled until a game is selected.

The greying is the instruction: the page reads top to bottom, each step lighting up the next. This
also fixes the defect Eric spotted on 2026-09-01 — COURT being tappable before any connection is
confirmed.

### The access key is not part of the draft

The login is the one step that cannot be staged: pressing DONE genuinely fetches a key from that
site and the pending link is consumed at the far end. So the key is filed against the
**(site, event)** that issued it immediately, and CANCEL does not take it back. Filing by event
alone is wrong: event ids collide between the Portal and a custom site by design.

Eric's framing, 2026-09-01: *"this is just like wifi passwords for multiple networks — it is fine
that you can connect to multiple events, so long as the event you are targeting is reachable."*
A key for a site the refbox is not currently using is never sent anywhere, and discarding it could
force a trip back to the Portal website for a fresh code mid-tournament.

## What this changes about #3082

**#3082 remains correct for the model the app has today, and stops being sufficient under this one.**

Its guard works by noting which site the *live* client was pointed at when the login went out, and
discarding the answer if the live client has moved since. That holds today because a login always
goes to the live client. Under the draft model a login goes to the **drafted** site while the live
client deliberately does not move — so the marker never goes stale, and a key from drafted site A
could be filed under whatever the live site is.

Under this model the login reply must carry **the identity of the site and event it was issued
to**, not a marker that tracks the live connection. That is strictly more precise, and it is what filing a key
per site requires anyway. Amend #3082's design note when this lands.

## Consequences to design around

- **A "look, don't move" connection is required.** Filling EVENT for a drafted site means talking to
  it without moving the live connection. Precedent exists: `request_event_list` already builds its
  own portal client for exactly this reason, so this generalises a pattern rather than inventing one.
- **Two sites are reachable at once.** Eric assessed the risk as near-zero in practice: switching
  Portal <-> Custom mid-event "borders on will not actually ever happen" — it is a setup-time action.
- **The key check has three answers, not two.** `verify_token` returns accepted, rejected, or
  could-not-reach. The cascade gates COURT on "the access key is confirmed", which leaves the
  unreachable case undefined — an operator returning to a known site with the wifi down is
  neither confirmed nor refused. The design must say what step 3 shows in that state. Raised
  2026-09-01 while ruling question 1; NOT yet decided.
- **The refusals collapse to one place.** With staging harmless, a game in progress and queued
  results need only be checked at APPLY. `source_tap_outcome` and `refuse_repoint` become one rule.

## Open questions

1. **MANUAL — same treatment as the other two? RULED 2026-09-01: YES.** MANUAL is treated
   identically to Portal and Custom. `remembered_remote` is dropped: turning MANUAL off leaves no
   source drafted, and the operator taps PORTAL or CUSTOM. Established while ruling:
   event/court/game are ALREADY blanked on manual -> remote today (`clear_for_remote_switch`, ADR
   017, `app/mod.rs` ~4596) — `remembered_remote` (`config.rs:514`) only ever held which of
   Portal/Custom, indefinitely and with no timestamp. Returning to a site whose key is still held
   costs taps, not a re-login: `verify_token` re-confirms it. A 120h expiry on the memory (matching
   `link_session::FRESHNESS_WINDOW`, which is 120h — not 96 — and governs restart-restore,
   not this) was offered and declined.
2. **One key at a time, or a key per event? RULED 2026-09-01: A KEY PER EVENT.** (Question also
   CORRECTED 2026-09-01 — it was far wider than first written.)
   This question originally said the two token slots mean "pointing at a second custom site
   overwrites the first one's key". That understates it badly. The access key is issued AND
   verified **per event**: `login_to_portal` POSTs to `/api/events/{event_id}/access-keys/ref-box`
   and `verify_token` GETs `/api/events/{event_id}/access-keys/verify`
   (`uwh-common/src/uwhportal/mod.rs:219` and `:313`). The refbox stores exactly one Portal key
   (`UwhPortal { token: String }`, `config.rs:63`) and one custom-site key, both written at
   `app/mod.rs:5181`, each overwriting whatever was there. Switching events does not clear it
   (`app/mod.rs:1642`, "no logout"). **So connecting to a second event on the same Portal discards
   the first event's key**, and returning to that event needs a fresh login code from the Portal
   website. Eric's stated expectation on 2026-09-01 — "if they select an event that is still
   available and they have already connected to it before, they can just re-select the event and
   the connection would re-establish" — is NOT today's behaviour.
   **Ruling:** the refbox keeps a key per event, filed under **(site, event)** — never event
   alone, because event ids collide between the Portal and a custom site by design. Re-selecting
   any previously-used event re-establishes with no new login code. Accepted costs: a settings-file
   format change with a migration, and keys accumulating with no delete path. Deliberately NO
   automatic expiry of stored keys (a key silently vanishing is worse than a stale one sitting
   unused); each key is only ever sent to the event that issued it.
3. **Is the clearing confirmation still warranted? RULED 2026-09-01: YES, BUT ONLY WHEN THERE IS
   SOMETHING TO LOSE.** Tapping a different source is silent when no event/site, court or game has
   been drafted, and shows the confirmation when one has. Rationale: a confirmation that fires with
   nothing at stake trains the operator to tap straight through it, weakening the APPLY-time
   refusal that does matter. The mid-game refusal is a separate protection and stays regardless —
   it moves to APPLY.

   **Copy, ruled literally by Eric — do NOT reword, do NOT add a Portal/Custom variant:**

       This clears the selections you have made.

   One string covers both sources. The confirmation page is a statement plus labelled action
   buttons (`view_builders/confirmation.rs`), so the message deliberately does not ask a question —
   the buttons carry the choice. Button labels are NOT yet decided.

## Sequencing and delivery (ruled 2026-09-01)

- **Two branches, keys first.** Branch 1 delivers the per-event key store on its own: keys filed
  under (site, event), so re-selecting a previously-used event re-establishes with no new login
  code. It stands alone and is separately walkable. Branch 2 delivers the draft model on top —
  it needs per-event filing anyway to meet acceptance criterion 6.
- **Base:** both start from master after PR #3082 merges. #3082 is reviewed and walked; the only
  thing holding it is the `cargo audit` failure, fixed on `chore/deps/rtrb-advisory-fix`.
- **Rollback: new format only.** The settings file gets the new per-event key store and nothing
  is duplicated into the old `[uwhportal] token` / `[custom_site] token` slots for the benefit of
  an older binary. Reverting to an older refbox therefore means logging in again with a fresh code
  for the current event. Ruled by Eric 2026-09-01, having been shown that cost. An older binary
  does NOT crash on the new file — config parsing has no `deny_unknown_fields`, so unrecognised
  entries are ignored.
- **Deferred to the branch-2 plan, NOT yet ruled:** what the COURT step shows when the site cannot
  be reached at all (the third answer from `verify_token`), and the button labels on the new
  confirmation.

## Acceptance criteria

Each fails today.

1. On the Portal with an event, court and game applied, open settings, tap CUSTOM, then CANCEL.
   The refbox is still on the Portal with the same event, court and game, and the clock is
   undisturbed.
2. Same, but tap CUSTOM then tap UWH PORTAL again. EVENT, COURT and GAME are blank, and CANCEL
   still restores the applied selection.
3. Tapping any of the three source buttons during a game behaves identically to the other two.
4. COURT cannot be selected until the access key is confirmed.
5. Log in against a drafted site, then CANCEL. The key is still held for that site; returning to it
   does not require logging in again.
6. Draft site A, begin a login, switch the draft to site B before it answers. A's key is filed
   against A and is never attached to a request to B. (This is the criterion #3082's mechanism
   cannot meet; see above.)
