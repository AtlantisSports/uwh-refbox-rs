# Portal Link Restore Across Restart — Design

**Date:** 2026-06-22
**Status:** Approved (design); local working doc — not committed to a branch/PR
**Crate scope:** `refbox` only (no `uwh-common`, wire-format, or hardware changes)

---

## Problem

Changing the on-screen language between two different alphabets (e.g. Korean ↔ English)
forces the whole refbox app to **relaunch** itself, because iced 0.13 can only choose its
font family once, at startup. The same self-relaunch also happens on a self-update.

The UWH Portal **token** is saved in the config file and *is* reloaded correctly after a
relaunch. But the **live link state** — which event you're connected to, which court you've
selected, which game you're on, and that "Using UWH Portal" is switched on — lives only in
memory and is reset to a dormant default on every startup
(`current_event_id: None`, `using_uwhportal: false`, `current_court: None` — see
`refbox/src/app/mod.rs` `RefBoxApp::new`).

Net effect: after a language-change relaunch the machine looks **unlinked** and the operator
is sent back through the linking flow (which can re-prompt for the token), even though the
saved token is still valid. The operator's mental model — "each language behaves like a
different, unrecognized machine" — is the visible symptom of this.

This is deliberate behavior today (ADR 011 amendment 2026-04-23 "dormant-until-linked";
ADR 017 "Portal Data Lifecycle"). This design relaxes it **only** for a recent-enough
remembered session.

## Goal

After a relaunch (and an overnight shutdown within a tournament), the machine comes back
**already recognized** — relinked to the same event, court, and game, showing the live
countdown to that game's scheduled start — with no token re-prompt, *unless the token has
genuinely expired*. A machine powered on weeks later for a brand-new event must **not** come
back fixated on the old, finished event.

### Explicitly out of scope

- Changing how the token is obtained or validated.
- Changing the font/restart mechanism (the relaunch is unavoidable for alphabet changes).
- Preserving a **live, in-progress game** (running clock, current period, current scores)
  across the relaunch — a restart resets those regardless. This feature restores the
  operator's *place in the schedule*, ready to start; it does not freeze a running game.
- Any change to `uwh-common`, the wire format, the wireless remote, LED panel, or overlay.

## Decisions (settled with the user)

| Question | Decision |
|---|---|
| Scope of "stay recognized" | Carry the link across **any restart within a freshness window** (covers same-session relaunch, overnight, cold boot). |
| Freshness window | **48 hours.** Older than that → start clean. |
| Remember court? | **Yes.** |
| Remember game selection? | **Yes** — the selected game number. |
| Show scheduled countdown on restore? | **Yes** — reproduce the normal "time to next game" countdown to the scheduled start. |
| Cross-portal (Hockey↔Rugby) | A note from one portal is **not** restored into the other. |

## The "link note" file

A small JSON file `portal_link.json` lives in the config directory, beside the existing
`portal_queue.json`. It is read/written with the **same robust pattern** as
`refbox/src/portal_manager/queue.rs`:

- Versioned envelope (`version: u32`, `CURRENT_VERSION`).
- Atomic write: write to `portal_link.json.tmp`, `flush` + `sync_all`, then `rename` over
  the target (a power loss mid-write cannot corrupt the live file).
- `load_or_empty`-style loader: missing file → `None`; unparseable / unknown-version file →
  rename to `portal_link.corrupt.<ts>.json`, log, return `None`. Never blocks startup.

### Contents

```
LinkSessionFile {
    version:     u32,                  // schema version, starts at 1
    event_id:    EventId,              // uwh_common::uwhportal::schedule::EventId (Serialize/Deserialize)
    court:       Option<String>,       // current_court is Option<String>
    game_number: Option<GameNumber>,   // the selected game; None if none selected yet
    mode:        Mode,                 // refbox::config::Mode (Serialize/Deserialize) — guards cross-portal
    last_active: OffsetDateTime,       // time-crate rfc3339, as in queue.rs
}
```

The token is **not** stored here — it stays in the config file exactly as today. The note's
*existence* is the signal "this machine was actively linked"; there is no separate
`using_portal` flag inside it.

> Open item for the plan: confirm `GameNumber` is `Serialize`/`Deserialize` (it appears in
> snapshots and `NextGameInfo`); if not, store its string form, matching `ItemId.game_number`.

## Lifecycle

A single helper on `RefBoxApp` — `persist_link_session()` — recomputes the note from current
state and either writes or deletes it:

- **Linked** (`using_uwhportal == true` **and** `current_event_id.is_some()`) → write the
  note with the current event, court, selected game number, `config.mode`, and
  `last_active = now`.
- **Not linked** → delete `portal_link.json` (no note = nothing to restore).

### When `persist_link_session()` is called

1. **On link / unlink / selection change** — after the Game-Options apply commits the portal
   fields (`apply_app_options` and the sibling apply path), and after `set_current_event_id`,
   court, and game selection settle. Calling the recompute-and-write/delete helper at these
   points keeps the note in lockstep with the operator's actual link.
2. **Refresh while linked** — on the app's periodic portal heartbeat. The background portal
   task already fires a `verify_token` health check (`GREEN_CADENCE` = 5 min) and the main
   app handles the resulting `PortalEvent` (`refbox/src/app/mod.rs` ~line 2371). Refreshing
   `last_active` on that event keeps the timestamp within ~5 minutes of "now" the whole time
   the machine is open and linked. So on shutdown, `last_active` is at most ~5 minutes stale —
   negligible against a 48-hour window.

## Startup restore sequence

In `RefBoxApp::new`, after the app struct is built and before the startup-task batch:

1. Load `portal_link.json`. If absent/corrupt → dormant start (today's behavior); delete a
   corrupt file via the loader's rename path.
2. **Freshness + portal gate:** restore only if **both**:
   - `now - last_active <= 48h`, and
   - the note's `mode` uses the same portal as the current `config.mode`
     (reuse `crosses_portal`).
   If the note is stale → delete it and start dormant. (Mode mismatch → leave dormant; the
   next real link overwrites the note.)
3. On restore:
   - set `using_uwhportal = true`,
   - `set_current_event_id(Some(event_id))` (this also mirrors into `portal_event_id`, so the
     background `verify_token` probe runs against the restored event),
   - `current_court = court`,
   - stash `game_number` in a one-shot `pending_restore_game: Option<GameNumber>` field to be
     consumed when the schedule arrives.
4. The existing `if new.using_uwhportal { startup_tasks.push(new.request_event_list()); }`
   (line ~1590) then fires; **also** push `request_schedule(event_id)` so the schedule loads.

### Re-selecting the game and showing the scheduled countdown

When the schedule arrives (`RecvSchedule` handler) and `pending_restore_game` is `Some`:

- Look the game up in the schedule, build `NextGameInfo { number, timing, start_time }`, and
  set it as the next game via the **same path the normal between-games transition uses**, so
  the between-games clock initializes to the scheduled-start countdown.
- Consume `pending_restore_game` (one-shot).

The countdown value comes from existing logic (`calc_time_to_next_game`,
`next_game_scheduled_start` in `tournament_manager/mod.rs`):
`time_to_next = max(scheduled_start - now, minimum_break)`, where `minimum_break` is currently
240 s (4 min). So:

- Boot at 7:42 for an 8:00 game → ~18 min counting down to 8:00.
- Boot within 4 min of, or after, the scheduled start → the 4-minute minimum break, counting
  down.

This is identical to what the refbox already shows between games when linked to a schedule.

> Open item for the plan: pin the exact call used to initialize the between-games scheduled
> countdown on restore (mirror the normal end-of-game → next-game path; do **not** hand-roll
> clock math).

## Edge cases

| Situation | Behavior |
|---|---|
| New event weeks later | Note stale (>48h) → ignored + deleted → clean dormant start. No dead-event fixation. |
| Token genuinely expired overnight | Restore reconnects; `verify_token` fails → normal "token expired" notice + re-link prompt. Correct, not a bug. |
| Mode changed (Hockey↔Rugby) | Different portal → note not restored → dormant for the new sport. |
| Latin-only language switch (English↔Spanish) | No relaunch occurs → nothing changes. (Regression to confirm.) |
| Corrupt / missing note | Treated as "no note" → dormant start. Never blocks startup. |
| Portal switched off before shutdown | Note was deleted at unlink → nothing to restore. |

## Acceptance criteria (operator-verifiable)

1. Link to an event, select court + game, switch **Korean ↔ English**: after the relaunch the
   machine is still linked to the same event + court + game and shows the live countdown to the
   scheduled start — **no token prompt**.
2. Switch **English ↔ Spanish**: no relaunch, nothing changes.
3. Link, then shut down and restart the program the **same day** (cold): it reconnects.
4. Back-date the note to **>48 h** old: next start is clean and dormant (no event, portal off).
5. Switch **Mode Hockey → Rugby** (cross-portal restart): the UWH link is **not** restored.
6. With an **expired** token in a fresh note: restore shows the re-link prompt.

## Testing

- **Pure decision function** `should_restore(last_active, now, window, note_mode, current_mode)`
  → bool, with table tests including the 48 h boundary and mode mismatch.
- **File round-trip / corrupt / missing** for `portal_link.json`, mirroring the `queue.rs`
  test suite (`save_then_load_round_trip`, `corrupted_file_is_renamed_and_empty_returned`,
  `loads_empty_when_file_missing`, `atomic_write_leaves_no_tmp_file_on_success`).
- **Serialization round-trip** of `LinkSessionFile`.
- Restore wiring (event/court/game set, schedule fetch queued) covered by the manual
  acceptance walkthrough above; unit-test the parts that are pure.

## ADR interaction

This relaxes the "dormant-until-linked at startup" contract (ADR 011 amendment 2026-04-23;
ADR 017) for a **recent-enough** remembered session. Cold start with a stale/absent note
preserves the dormancy contract. Record this as an amendment note to ADR 011/017 at PR time.

## Risks / open items

- Confirm `GameNumber` serializability (else store its string form).
- Pin the exact between-games scheduled-countdown initialization call on restore.
- Write frequency: atomic rewrites every ~5 min while linked are cheap and bounded; acceptable.
- Freshness uses wall-clock (`OffsetDateTime`); a large system-clock change could shift the
  window. Acceptable — the Pi syncs time and the window is generous.
