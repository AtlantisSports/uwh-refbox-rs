# Editor top-spacing standardization — design

Date: 2026-06-17
Branch: `refactor/refbox/editor-top-spacing` (cut from `origin/master` @ 8f92bc8c)
Status: approved (Approach A — flush top-pin)

## Goal

Pin the time-editor box (the `make_time_editor` container: title + the value
and ▲▼/=0 keypad) to the **top** of the screen on the refbox time-editor pages,
removing the flexible spacer currently above it that pushes the box toward the
vertical middle. All editor screens should share one consistent shape:
controls packed at the top, a single flexible gap, footer pinned at the bottom.

This supersedes the never-built "reserve fixed space for the Game Block caution
note" idea: once the editor box is anchored to the top, a note appearing or
disappearing below it can no longer shift the box, so the note-shift problem is
fixed for free.

## Scope

In scope (refbox crate, view-builder layout only):

- `refbox/src/app/view_builders/configuration.rs` — `build_game_parameter_editor`
  (Game Block, Half Time, Half Length, breaks, overtime, etc.).
- `refbox/src/app/view_builders/time_edit.rs` — `build_time_edit_view`
  (the game-time / timeout-time edit screen). Also **move** the
  "The time is paused while on this screen…" note (`fl!("Note-Game-time-is-paused")`)
  from *above* the editor to *below* it, matching the other screens.

Explicitly out of scope:

- `refbox/src/app/view_builders/keypad_pages/team_timeout_edit.rs` — **no change.**
  PR #1188 already redesigned this screen; it no longer uses `make_time_editor`
  and its controls are already top-pinned with a single flexible spacer before
  the footer. It is the reference pattern, not a target. (The original SCOPE
  listed it, but that was based on a pre-#1188 view of the code.)
- The editor's internal layout/contents (buttons, ▲▼ controls, value
  formatting, the `?` help button, the 2-HALVES/1-PERIOD selector).
- The footer Cancel/Done(Apply) row and its logic (Apply-gate behaviour stays).
- The note/validity text content itself — only its position changes.

## Current state (origin/master)

`make_time_editor` returns a shrink-height `Container` with no internal
`Length::Fill`/anchor, so its vertical position is dictated entirely by the
parent `column`'s spacers. No iced text-anchor-bleed risk.

**`build_game_parameter_editor`** builds the column as:

```
column![ game_time_button ]            // status bar at top
   (+ 2-halves/1-period selector if Half Length)
   .push(vertical_space())             // ← FLOATING GAP (push editor to middle)
   .push(editor_row)                   // make_time_editor (+ ? button)
   .push(vertical_space())             // ← gap between editor and note
   (+ validity_note if Game Block)
   .push(vertical_space())
   .push(footer)
```

**`build_time_edit_view`**:

```
column![
   game_time_button,
   vertical_space(),                   // ← FLOATING GAP
   note ("The time is paused…"),       // ← note ABOVE editor
   vertical_space(),
   edit_row,                           // make_time_editor(s)
   vertical_space(),
   footer,
]
```

## Target state (Approach A — flush top-pin)

Mirror the already-top-pinned `team_timeout_edit` shape: pack everything at the
top with the standard `SPACING`, keep exactly **one** flexible `vertical_space()`
before the footer.

**`build_game_parameter_editor`**:

```
column![ game_time_button ]
   (+ selector if Half Length)
   .push(editor_row)                   // directly under top — no leading gap
   (+ validity_note if Game Block)     // directly under the editor
   .push(vertical_space())             // single flexible gap
   .push(footer)
```

**`build_time_edit_view`**:

```
column![
   game_time_button,
   edit_row,                           // directly under status bar
   note ("The time is paused…"),       // moved BELOW the editor
   vertical_space(),                   // single flexible gap
   footer,
]
```

Net change in both files: remove the leading flexible `vertical_space()` above
the editor and the spacer between the editor and its note; keep one flexible
spacer before the footer. In `time_edit.rs` the note is additionally relocated
from above the editor to below it.

## Acceptance criteria (operator-observable)

1. On every game-parameter editor (Game Block, Half Time, Half Length, breaks,
   overtime), the editor box sits just under the status bar (under the
   2-HALVES/1-PERIOD selector on Half Length), not floating in the middle.
2. On the game-time edit screen, the editor box sits just under the status bar,
   and the "time is paused" note appears *below* the editor, not above it.
3. On the Game Block editor, when the red "too short" / yellow "tight" note
   appears or disappears, the editor box does **not** move.
4. The footer (Cancel / Done / Apply) stays pinned to the bottom on all screens.
5. No behaviour change: Apply-gating, value editing, and the `?` help button all
   work exactly as before.

## Verification

- Rebuild the run binary first: `cargo build -p refbox` (note: `just check`
  builds a separate test binary and does NOT refresh `target/debug/refbox`).
- Launch and walk through: a normal parameter editor, the Half Length editor,
  the Game Block editor (watch the box stay put as the note toggles), and the
  game-time edit screen (note now below the editor).
- `just check` (fmt, lint, test, audit) clean before PR.

## Risk

Low. Layout-only change in two view-builder functions in the `refbox` UI crate;
no state-machine, wire-format, or shared-type changes. Lean process applies.
