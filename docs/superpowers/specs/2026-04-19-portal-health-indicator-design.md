# Portal Health Indicator — Design Spec

**Date:** 2026-04-19
**Scope:** `refbox` crate only (no changes to `uwh-common`, `overlay`, or any other crate)
**Branch type:** `feat/refbox/` (implementation branch to be cut from `master`)
**Source of truth for "what and why":** [ADR 011](../../decisions/011-portal-health-indicator.md)

> **Superseded in part — see [ADR 011 Amendment (2026-04-21)](../../decisions/011-portal-health-indicator.md#2026-04-21--conflict-handling-refined-after-api-verification).**
> The conflict-resolution design in this spec (dedicated `Conflict` state, `KEEP REFBOX VALUE` /
> `KEEP PORTAL VALUE` buttons, rows showing both portal and refbox values) was refined after
> verifying the `uwh-common` API — the portal client collapses all failure modes into a single
> generic error and does not return portal-side score values. The implemented design replaces
> the conflict-specific flow with a single "attention" state and `FORCE THIS GAME RESULT` /
> `DISCARD THIS SUBMISSION` buttons; pending rows are yellow rather than orange. See the
> amendment for the full refined shape. The implementation plan at
> `docs/superpowers/plans/2026-04-19-portal-health-indicator.md` reflects the refined design;
> this spec remains as historical context for the original Option A direction.

This spec is the "how" companion to ADR 011. The ADR decided the observable behaviour and module
boundary. This spec captures the concrete visual design, module layout, queue schema,
state-machine shape, and testing approach.

---

## Overview

Add a portal health indicator to the refbox time banner so tournament operators can see at a
glance whether game scores are reaching the UWH Portal. Silent submission failures become
visible, transient network failures auto-retry from a persisted on-disk queue, and the operator
has dedicated pages to resolve conflicts and expired logins.

The feature introduces a new `refbox/src/portal_manager/` module and modifies six existing
screens. No other crates change. The `uwh-common` portal client API is preserved exactly.

---

## What Is Not Changing

- The `uwh-common` portal client API — `verify_token`, `post_game_scores(force)`, and
  `post_game_stats` are used unchanged
- Game clock, state machine, scoring, penalties, and timeouts
- Overlay output, LED-panel output, and wireless-remote behaviour
- Wire format between refbox and overlay
- Existing configuration file schema (the queue file is a separate, new file)
- Any crate other than `refbox`

---

## Visual Design: Health Tile

A new clickable tile is prepended to the time-banner row on every page that shows the banner.

### Shape and size

- **Shape:** square.
- **Size:** **130 px × 130 px** (exact).
- **Background:** `LIGHT_GRAY` (theme constant `LIGHT_GRAY`, RGB 0.7/0.7/0.7 = `#B3B3B3`)
- **Corner radius:** matches the banner's existing `BORDER_RADIUS` (9 px)
- **Position:** inside the banner row, to the left of the existing period-name + clock content
- **Banner height:** today's time banner is at least `MIN_BUTTON_SIZE` (89 px) tall. The
  banner's minimum height grows to 130 px so the tile fits every screen that shows the banner.

### Content layout (vertical stack, centred)

- **Top:** UWH Portal compact logo, 100 px wide with aspect ratio preserved.
- **Bottom:** status dot, 50 px in diameter, horizontally centred.
- **Spacing:** standard `SPACING` (8 px) between logo and dot; remaining vertical space split
  as equal top/bottom padding inside the tile (approximately 8 px each, depending on the
  logo's native aspect ratio).

### Colour states of the status dot

| Base colour | Theme constant | Meaning |
|---|---|---|
| Green | `GREEN` | Last portal interaction succeeded; queue is empty; token is valid |
| Yellow | `YELLOW` | A health check is in flight, or the last call was slow but succeeded |
| Red | `RED` | At least one item needs attention (queued retry, conflict, or expired token) |

### Overlay icons (on top of the dot)

- **Green checkmark** — replaces the dot for exactly 10 seconds after a successful score
  submission, then **snaps** back to the plain dot. No fade animation.
- **Red exclamation mark** — persists on top of the dot while any item needs attention. Clears
  instantly when every attention item is resolved.
- **Mutex:** if both would apply (a successful submission happens while other items still need
  attention), the red exclamation wins. The green checkmark is suppressed — it would be
  misleading while red is still appropriate.

### Tap behaviour

The entire tile (logo area + dot area) is a single tap target. Tapping anywhere on the tile
in any state opens the Detail Page. The tile uses a minimal-chrome `button` style (no border
override beyond the tile background itself; `LIGHT_GRAY` pressed state = `LIGHT_GRAY_PRESSED`).

### Asset

- **File:** `UWH Portal Compact Logo.png` (provided by the user)
- **Storage:** copied into `refbox/resources/UWH_Portal_Compact_Logo.png`
- **Loading:** embedded at compile time via `include_bytes!` (matches how existing SVG and font
  assets are handled in `refbox/resources/`)

---

## Portal Manager Module

New module: `refbox/src/portal_manager/`.

### Files

- **`mod.rs`** — public `PortalManager` struct owned by the app; public API methods listed
  below; orchestrates the other two files.
- **`queue.rs`** — on-disk persistence (`QueueFile`, `QueuedItem`, `ItemState` types; `load`,
  `save`, atomic-write, corruption handling).
- **`health.rs`** — background async task driving the health-check and retry timers; sends
  `PortalEvent`s back to the main app via an `mpsc` channel that feeds an iced `Subscription`.

### Public API (`PortalManager`)

| Method | Purpose |
|---|---|
| `new(config_dir, portal_client) -> Self` | Load or create the queue file; start the health task |
| `subscription() -> Subscription<Message>` | Returns an iced subscription that delivers `PortalEvent`s as `Message`s |
| `enqueue_game_end(event_id, game_number, scores, stats)` | Called from `app/mod.rs` at game end; writes queue record then attempts submit |
| `retry(item_id)` | Operator tapped `RETRY NOW` on the pending-item action page |
| `discard(item_id)` | Operator tapped `DISCARD` on the pending-item action page |
| `resolve_conflict(item_id, keep_refbox_value: bool)` | Operator chose on the conflict action page |
| `token_refreshed()` | Called after a successful re-login; clears the token-expired row and retries blocked items |
| `verify_now()` | Forces a health check (useful for tests; may also be exposed to UI later) |

### Health state

An enum with three variants: `Green`, `Yellow`, `Red`. The manager computes the current state
from its internal data:

- `Green` → queue has zero attention-items AND last health check succeeded
- `Yellow` → a health check is in flight, or the last call was slow-but-successful
- `Red` → queue has at least one attention-item (pending retry, conflict, or expired token)

### Overlay state

Separate from health state. An enum with three variants:

- `None` → plain dot visible
- `RecentSuccess(deadline: Instant)` → green checkmark visible until `deadline`
- `AttentionNeeded` → red exclamation visible

The red-exclamation state overrides the green-checkmark state (the "mutex" rule from the Visual
Design section). The manager recomputes overlay state whenever items are added, resolved, or
the 10-second success timer expires.

### Health checks

- **Endpoint:** the existing `verify_token` HTTP call on the portal client
  (`GET /api/events/{event}/access-keys/verify`), reused unchanged from `uwh-common`. 200 OK
  means healthy; non-200 or a request timeout is a failure.
- **Cadence:** driven by a `tokio::time::interval` inside `health.rs`:
  - **5 minutes** while `Green`
  - **15 seconds** while `Yellow` or `Red`
  - Cadence resets after every successful portal interaction (health check OR score submit) —
    a just-sent score is not immediately followed by a redundant health check.
- **First check** fires immediately at app startup, so the indicator reflects real status
  within seconds of opening the app rather than waiting for the first interval tick.

### Retry cap

Each `QueuedItem` tracks an `attempts` counter. Auto-retry gives up after **10 consecutive
failed attempts** for the same item. The item then remains in the queue in `pending` state, but
the background loop stops retrying it. Operator must tap `RETRY NOW` on its action page to
re-enable automatic retries (this resets `attempts` to 0).

---

## Queue File

### Location and name

- **Directory:** same directory `confy` resolves to for the existing refbox config
  (OS-dependent: `%APPDATA%\uwh-refbox\config\` on Windows; `~/.config/uwh-refbox/` on Linux;
  `~/Library/Application Support/uwh-refbox/` on macOS).
- **File name:** `portal_queue.json`

### Schema

```json
{
  "version": 1,
  "items": [
    {
      "event_id": "2026-spring-nationals",
      "game_number": 27,
      "black_score": 3,
      "white_score": 2,
      "stats": { /* existing portal stats blob shape, verbatim */ },
      "queued_at": "2026-04-19T14:22:03Z",
      "attempts": 2,
      "last_attempt_at": "2026-04-19T14:23:15Z",
      "state": "pending",
      "force": false
    }
  ]
}
```

- `state` values: `"pending"` | `"conflict"` | `"token_expired"`
- `force` is used when resubmitting after a conflict where the operator picked `KEEP REFBOX VALUE`
- Timestamps are ISO-8601 UTC strings (chrono `DateTime<Utc>` serialises this way by default)

### What persists

Only items whose state is `pending`, `conflict`, or `token_expired`. Green "recently
submitted" rows are **in-memory only** and clear on app restart (per ADR / Q2 resolution:
last 5, clear on restart).

### Atomic write

Standard write-temp-then-rename pattern:

1. Write the new content to `portal_queue.json.tmp` in the same directory
2. Fsync the temp file
3. `std::fs::rename` over `portal_queue.json` (atomic on Windows/Linux/macOS)

### Corruption handling on startup

If `portal_queue.json` fails to deserialise (invalid JSON, unknown version, or any other
parse error):

1. Rename to `portal_queue.corrupt.json` (append a timestamp suffix if one already exists to
   avoid overwriting a previous backup)
2. Start with an empty queue
3. **Log loudly** via the existing `log` crate (`error!` level) — include the original error
4. Do **not** show any UI indication; the health tile remains in whatever state it would be in
   for an empty queue (Green if next check passes)

### Write failure handling

If the atomic write fails (disk full, permission denied, etc.):

1. Log the error at `error!` level
2. The live submit attempt still proceeds (may succeed in-flight)
3. Do **not** change the health-state indicator; the operator should not be alarmed by disk
   issues during a game
4. If the app crashes before the next successful write, the failed item is lost. This is an
   accepted failure mode — a non-writable disk is rare enough that a second fallback is not
   worth the complexity

---

## Detail Page

### Chrome and layout

Same chrome as every other page: time banner (with health tile) on top, timeout ribbon on
bottom. Between them, the Select Event layout verbatim:

- **List area (4/5 width):**
  - Title slot at top shows the summary: coloured dot + text. Example texts:
    - Green: `PORTAL — CONNECTED · All clear`
    - Yellow: `PORTAL — CHECKING…`
    - Red: `PORTAL — ISSUES · Last OK 4 min ago`
  - 4 visible row buttons with the Select-Event scroll-column on the right (`▲` / `▼`)
  - Each row is a coloured button (see Row types below)
- **Side column (1/5 width):**
  - Single `BACK` button pushed to the bottom by `vertical_space`
  - **No `VERIFY NOW` button.** (The mockup `.superpowers/brainstorm/32014-1776565793/content/detail-page-v3.html`
    shows one; the ADR intentionally dropped it. Tap the tile itself to open this page, which
    already triggers a fresh health check.)

### Row ordering

Rows are displayed top-to-bottom in this order:

1. **Token-expired row** (if present) — at most one, always at the very top
2. **Conflict rows** (red)
3. **Pending rows** (orange), sorted by `queued_at` ascending (oldest first)
4. **Recent success rows** (green), sorted by submit time descending (newest first), capped at 5

### Row types

| Type | Background colour | Label format | Tappable? |
|---|---|---|---|
| Token expired | `RED` | `"Portal login expired — tap to re-login"` | Yes |
| Conflict | `RED` | `"G{game_number} · Conflict · portal {p_b}–{p_w}, refbox {r_b}–{r_w}"` | Yes |
| Pending | `ORANGE` | `"G{game_number} · Pending · {n} attempts · retry in 0:MM"` | Yes |
| Pending (cap reached) | `ORANGE` | `"G{game_number} · Pending · {n} attempts · tap to retry"` | Yes |
| Pending (stats only) | `ORANGE` | `"G{game_number} · Pending · stats only · retry in 0:MM"` | Yes |
| Recent success | `GREEN` | `"G{game_number} · Submitted {N} min ago"` | No |

Only green rows are non-tappable. Red and orange rows all tap to their respective action
pages.

---

## Per-Item Action Pages

Three new view builders, one per tappable row type. All three share the same outer chrome (time
banner with health tile on top, timeout ribbon on bottom). Between them, a two-button or
three-button centred layout using standard refbox button theme colours.

### Conflict action page

Title area (top of the body): summary text — `"Game {game_number} · Conflict"` with the
portal's score vs. the refbox's score displayed prominently.

Buttons:

- `KEEP REFBOX VALUE` (`GREEN` theme) — calls `resolve_conflict(item_id, keep_refbox_value=true)`,
  which resubmits with `force=true`. On success, item becomes a green recent-success row.
- `KEEP PORTAL VALUE` (`BLUE` theme) — calls `resolve_conflict(item_id, keep_refbox_value=false)`,
  which silently removes the item from the queue (no resubmit; portal value wins).
- `BACK` (`RED` theme) — returns to the detail page.

### Pending action page

Title area: `"Game {game_number} · Pending"` with retry count and time of last attempt.

Buttons:

- `RETRY NOW` (`GREEN` theme) — calls `retry(item_id)`, resetting `attempts` to 0 and forcing
  an immediate submit attempt.
- `DISCARD` (`RED` theme) — calls `discard(item_id)`, which removes the item without
  submitting. Requires tapping twice (the second tap confirms) to prevent accidental data loss.
- `BACK` (`GRAY` theme) — returns to the detail page.

### Token-expired action page

Title area: `"Portal login expired"` with short explanation text.

Buttons:

- `GO TO LOGIN` (`BLUE` theme) — navigates to the existing portal-login view. **After a
  successful login, the app returns to the Detail Page** (per ADR / Q3 resolution: A1
  landing). Any items blocked by the expired token then retry automatically via the normal
  15-second cadence.
- `BACK` (`RED` theme) — returns to the detail page.

---

## End-Game Confirm Advisory Banner

### Trigger

- **Shown** when `config.confirm_score == true` AND current health state is `Red` AND the
  operator is on the confirm-score screen at game end.
- **Not shown** when health state is `Green` or `Yellow` (yellow is informational only).
- **Not shown** when `config.confirm_score == false` — the advisory gap in this mode is
  accepted per Q4 resolution; the persistent red exclamation mark on the time banner is the
  signal.

### Appearance

A thin red strip across the top of the confirm-score screen's body area, below the time
banner. Not a modal. Non-blocking.

### Copy

> Portal issue detected. Score will still be queued — find an admin to resolve.

(Copy per Q4 resolution. Added to all translation files — English/Spanish/French and the ~13
other supported languages. Translation key: `portal-advisory-at-game-end`.)

---

## Messages (`refbox/src/app/message.rs`)

New `Message` variants:

- `Message::OpenPortalDetailPage` — fired by tapping the health tile
- `Message::PortalEvent(PortalEvent)` — carries an event from the portal_manager subscription
  (variants listed below)
- `Message::PortalRowTapped(ItemId)` — fired by tapping a detail-page row; dispatches to the
  correct action-page state based on row type
- `Message::PortalKeepRefboxValue(ItemId)` — conflict page's `KEEP REFBOX VALUE` button
- `Message::PortalKeepPortalValue(ItemId)` — conflict page's `KEEP PORTAL VALUE` button
- `Message::PortalRetryNow(ItemId)` — pending page's `RETRY NOW` button
- `Message::PortalDiscardTapped(ItemId)` — pending page's `DISCARD` button; the handler checks
  the current `discard_armed` flag on the `AppState` and either arms or fires based on state
- `Message::PortalGoToLogin` — token-expired page's `GO TO LOGIN` button

`PortalEvent` variants (delivered via the subscription):

- `HealthChanged(HealthState)`
- `ItemAdded(ItemId)`
- `ItemResolved(ItemId)` (success, conflict accepted, or discard)
- `ItemUpdated(ItemId)` (retry count changed, state changed)
- `OverlayChanged(OverlayState)`

---

## App-State Additions (`refbox/src/app/mod.rs`)

New `AppState` variants:

- `AppState::PortalDetailPage`
- `AppState::PortalConflictAction(ItemId)`
- `AppState::PortalPendingAction { item_id: ItemId, discard_armed: bool }`
- `AppState::PortalTokenExpiredAction`

The `discard_armed` flag implements the two-tap `DISCARD` confirmation on the pending page.

The existing `App` struct gains one new field:

- `portal_manager: PortalManager`

The existing `handle_game_end()` is rerouted: instead of spawning fire-and-forget
`post_game_score` / `post_game_stats` tasks, it calls
`self.portal_manager.enqueue_game_end(...)`.

The existing portal-login success path is extended to call
`self.portal_manager.token_refreshed()` and then transition to `AppState::PortalDetailPage`.

---

## Time-Banner Helper (`refbox/src/app/view_builders/shared_elements.rs`)

The existing `make_game_time_button(snapshot, tall, editing_time, mode, clock_running)` helper
gains one new parameter:

```rust
pub(super) fn make_game_time_button<'a>(
    snapshot: &GameSnapshot,
    tall: bool,
    editing_time: bool,
    mode: Mode,
    clock_running: bool,
    portal_indicator: PortalIndicatorState,  // new
) -> Row<'a, Message>
```

`PortalIndicatorState` is a small value type:

```rust
pub struct PortalIndicatorState {
    pub health: HealthState,      // Green / Yellow / Red
    pub overlay: OverlayState,    // None / RecentSuccess / AttentionNeeded
}
```

The helper prepends the health tile (logo + dot + overlay) to the banner row before the
existing time-button.

Every existing call site of `make_game_time_button` must be updated to pass the new
parameter. At time of spec-writing that is 20 call sites across 12 files in
`refbox/src/app/view_builders/`. Each call site reads the current state from
`self.portal_manager.indicator_state()` (a synchronous getter returning the combined
`PortalIndicatorState`).

---

## Configuration: Dev-Portal Override

For testing against `dev.uwhportal.com`, add one override mechanism:

- **Environment variable:** `UWH_PORTAL_URL_OVERRIDE`
- If set, replaces the default production portal base URL at app startup
- If unset (the default in production builds), uses the existing hard-coded production URL
- Logged at `info!` level when the override is active so the operator-of-last-resort can tell
  they're not hitting production

No new CLI flag. No persisted setting. This is an environment-variable-only override intended
for development and testing.

A second debug-only env var `UWH_PORTAL_SCRAMBLE_TOKEN=1` scrambles the in-memory access token
on next request, to exercise the token-expired path during testing. Stripped out of release
builds via `#[cfg(debug_assertions)]`.

---

## Translations

New keys to add to every file in `refbox/translations/` (including the non-English/Spanish/French
languages — the project supports ~13 languages):

- `portal-summary-connected` → `"PORTAL — CONNECTED · All clear"`
- `portal-summary-checking` → `"PORTAL — CHECKING…"`
- `portal-summary-issues` → `"PORTAL — ISSUES · Last OK {duration} ago"`
- `portal-row-token-expired` → `"Portal login expired — tap to re-login"`
- `portal-row-conflict` → `"G{game} · Conflict · portal {p_b}–{p_w}, refbox {r_b}–{r_w}"`
- `portal-row-pending` → `"G{game} · Pending · {attempts} attempts · retry in 0:{secs}"`
- `portal-row-pending-capped` → `"G{game} · Pending · {attempts} attempts · tap to retry"`
- `portal-row-pending-stats-only` → `"G{game} · Pending · stats only · retry in 0:{secs}"`
- `portal-row-recent` → `"G{game} · Submitted {duration} ago"`
- `portal-action-keep-refbox` → `"KEEP REFBOX VALUE"`
- `portal-action-keep-portal` → `"KEEP PORTAL VALUE"`
- `portal-action-retry-now` → `"RETRY NOW"`
- `portal-action-discard` → `"DISCARD"`
- `portal-action-discard-confirm` → `"TAP AGAIN TO CONFIRM"`
- `portal-action-go-to-login` → `"GO TO LOGIN"`
- `portal-page-title-conflict` → `"Game {game} · Conflict"`
- `portal-page-title-pending` → `"Game {game} · Pending"`
- `portal-page-title-token-expired` → `"Portal login expired"`
- `portal-advisory-at-game-end` → `"Portal issue detected. Score will still be queued — find an admin to resolve."`

English and Spanish strings are added with the spec implementation. Other languages follow the
existing "unverified label" pattern from the translation workflow.

---

## Testing

### Automated (`just test` and CI)

- **Queue file round-trip:** write, read, assert equal
- **Atomic-write crash simulation:** inject a failure between temp-write and rename; assert
  original file is intact
- **Corruption handling:** write a deliberately malformed queue file; assert startup renames
  it, logs, and starts with empty queue
- **10-attempt retry cap:** simulate 10 failures; assert the item stops auto-retrying
- **State-machine transitions:** assert Green→Yellow→Red on the right triggers
- **Overlay state:** assert green-checkmark visibility for exactly 10s; assert red-exclamation
  wins when both would apply
- **Row ordering:** construct a queue with one of each state; assert display order matches the
  spec

### Manual against `dev.uwhportal.com`

Set `UWH_PORTAL_URL_OVERRIDE=https://dev.uwhportal.com` and walk through the six scenarios
from the ADR's behaviour-over-time description:

1. **Happy path:** end a mock game; observe checkmark flash for 10s, then snap to plain green
   dot; confirm the score appears in the dev portal web UI
2. **Transient blip:** block `dev.uwhportal.com` at the OS hosts-file level for 30 seconds
   during a submit; observe red exclamation; unblock; observe automatic retry and recovery
3. **Conflict:** submit a score; edit it on the dev portal web UI to something different; open
   the refbox detail page; observe the conflict row; walk through both `KEEP REFBOX` and
   `KEEP PORTAL` outcomes on separate runs
4. **Token expired:** set `UWH_PORTAL_SCRAMBLE_TOKEN=1`; make a submit; observe the token-expired
   row; walk the re-login flow; observe landing on the detail page and the blocked items
   retrying
5. **Restart with items queued:** close the refbox with pending items; reopen; observe the
   queue file is re-loaded and retries resume
6. **End game while red:** with pending items outstanding, confirm-scores on, end a game;
   observe the advisory banner with the exact copy from the spec

### Pre-merge gate

- `just check` passes (fmt, lint, all tests, audit)
- All six manual scenarios pass against `dev.uwhportal.com`
- The logo asset file is committed
- Every translation file has the new keys (using the unverified-label pattern where
  translations are not yet available)

### No production-portal pre-merge testing

All pre-merge testing is against `dev.uwhportal.com`. Production portal is only exercised
after the PR merges to `master` and the operator hits it during a real tournament.

---

## Implementation Touchpoints Summary

### New files

- `refbox/src/portal_manager/mod.rs`
- `refbox/src/portal_manager/queue.rs`
- `refbox/src/portal_manager/health.rs`
- `refbox/src/app/view_builders/portal_detail.rs`
- `refbox/src/app/view_builders/portal_conflict_action.rs`
- `refbox/src/app/view_builders/portal_pending_action.rs`
- `refbox/src/app/view_builders/portal_token_expired_action.rs`
- `refbox/resources/UWH_Portal_Compact_Logo.png`

### Modified files

- `refbox/src/app/mod.rs` — new `AppState` variants, `portal_manager` field, reroute
  `handle_game_end()`, new message handlers, confirm-score advisory banner hook
- `refbox/src/app/message.rs` — new `Message` variants listed above
- `refbox/src/app/view_builders/shared_elements.rs` — new `portal_indicator` parameter on
  `make_game_time_button`
- `refbox/src/app/view_builders/mod.rs` — expose the four new view builders
- All 13 call sites of `make_game_time_button` — pass new `portal_indicator` argument
- `refbox/src/app/view_builders/confirmation.rs` — advisory banner on confirm-score screen
- `refbox/src/config.rs` — no schema change; queue file lives separately
- `refbox/translations/*.ftl` — new translation keys
- `refbox/Cargo.toml` — no new dependencies expected; `serde_json` is already present

---

## Out of Scope

- Any change to `uwh-common`, `overlay`, `schedule-processor`, or `wireless-remote`
- Changing the HTTP wire shape of the portal API
- Adding a "Verify Now" button to the detail page (intentionally dropped per ADR)
- Showing a popup when confirm-scores is off and portal is red at game end
- Syncing the queue file across multiple refbox instances
- UI for the refbox operator to edit translations for the new strings
