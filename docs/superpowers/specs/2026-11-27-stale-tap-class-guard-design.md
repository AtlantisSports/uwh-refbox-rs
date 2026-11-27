# Stale-tap class guard: design

Status: APPROVED by the PO 2026-11-27 (engine lane). Branch base: origin/master 208b38bd. Approach 1 chosen by the PO
2026-11-27 (see `approaches.md`, kept in the PO's backlog notes for this item, not on this branch). Facts checked against origin/master 962d9990.
Branch once approved: `fix/refbox/stale-tap-class-guard`. Target: v0.5.6 (release before
2026-12-08).

## Intent

An operator tap can arrive after the screen it was drawn on has gone: a half ends, or the
score-confirm window closes on its timer, while the operator is mid-tap. The handlers assume
their screen is still up, so today such a tap crashes the app (`unreachable!()`) or acts on
the wrong screen. The PO ruled on 2026-11-24 to guard the whole class, not to add cases to a
list, and on 2026-11-27 picked the "stamp each tap with its screen" approach.

**Success:** no tap can crash the app or act on a screen after the screen it was drawn on has
gone, for every current button and for every future button without anyone having to
remember to list it. Plus the one known same-screen crash (section 6).

**Known limit (PO, 2026-11-27):** a tap that arrives on the *same kind* of screen just after
something in the background changed what that screen holds is deliberately let through, so
fast typing keeps working. Section 6 fixes the one known case. No search for others is part
of this work; any found later are fixed when found.

## Goals

- A tap drawn on one kind of screen and arriving on another kind is ignored and logged.
- Taps that must never be lost (the release half of a press-and-hold) always get through.
- Adding a new message or a new screen forces the author to classify it (the compiler
  refuses a build until they do).
- Fix the TIME EDIT crash when a team timeout ends while the page is open (own task).

## Non-goals

- Messages that do not come from the screen (clock ticks, portal replies, timers, the
  wireless remote, the spacebar and window-wide mouse release) are untouched and never
  dropped.
- The 0:00 "zero gap" gate (`tap_lands_in_the_zero_gap`) stays as it is. It guards a
  different problem (the clock having no reading) and is orthogonal.
- The existing `unreachable!()` calls stay. They become truly unreachable for taps.
- The cosmetic CANCEL TIMEOUT double-tap warning stays out (PO did not ask for it).

## Anti-goals (a failure even if the crash is gone)

- **A buzzer left sounding, or a hold left running,** because a release was dropped.
- **Ordinary fast tapping broken:** typing two digits quickly, or +/- repeated on TIME EDIT,
  must never be treated as a screen change.
- **TIME EDIT behaving differently on APPLY/CANCEL** (see trap 1 below).

## Approaches (decided)

1. **Stamp each tap with its screen kind (chosen).** One wrapper applied in `view()`; covers
   every current and future button, including a late tap that lands on a *different* screen
   that also accepts that message (a late digit onto another keypad, a late TIME EDIT +/-
   onto a length editor). Small to maintain.
2. Per-message list of allowed screens (the 2026-11-24 shape). Rejected: a message shared by
   several screens can only be checked as "one of them is up", so the cross-screen cases slip
   through, and a wrong entry silently brings the crash back.

## The design

### 1. Screen kind

A new `ScreenKind` enum and one function `screen_kind(&AppState) -> ScreenKind`, written as
an exhaustive `match` with no `_` arm, so a new `AppState` variant will not compile until it
is classified.

A screen kind is the *type* of screen, never the values on it.

| AppState | Kind includes | Kind excludes (changes while the screen stays up) |
|---|---|---|
| `KeypadPage(page, number)` | which keypad: AddScore, Penalty, GameNumber, TeamTimeouts, FoulAdd, WarningAdd, PortalLogin | the typed number, selected colour, kind, infraction, duration |
| `EditGameConfig(page)` | which config page | — |
| `ConfirmationPage(kind)` | which confirmation (variant only, not its payload) | the payload |
| `ParameterEditor(p, ..)`, `ParameterEditorHelp(p, ..)`, `ParameterList(p, ..)` | which parameter | duration, staged flag, scroll index |
| `BeepTestSettings(page)` | which sub-page | — |
| `ScoreEdit { is_confirmation, .. }` | `is_confirmation` | the scores |
| `Updates { .. }` | nothing beyond "Updates" | the update state (it advances by itself during a download; a BACK tap must still work) |
| `PortalAttentionAction { .. }` | nothing beyond the screen | item, `discard_armed` |
| every other variant | the variant | all its fields (scroll positions, staged values, durations) |

### 2. Tagging, in one place

`view()` maps its finished element (the `container(main_view)` at its end) through
`Element::map`, so every message any widget on screen produces is wrapped as
`Message::Tap { screen: ScreenKind, message: Box<Message> }` (exact name is the builder's
call). The timeout ribbon, the health tile and every other shared element are covered,
because they are part of that element tree. Subscriptions and tasks are not part of `view()`,
so their messages arrive untagged, as now.

### 3. Unwrapping and the check: the first thing `update()` does

At the very top of `update()`, before `time_edit_was_running` and
`closed_by_apply_or_cancel` are computed:

- An untagged message passes through unchanged (all background messages, and every existing
  test, which sends bare messages).
- A tagged message whose `screen` equals `screen_kind(&self.app_state)` is unwrapped and
  carries on as today.
- A tagged message whose `screen` differs is **dropped** if its inner message may be dropped
  (section 4): log at `warn` ("Ignoring {inner:?}: drawn on {screen:?}, now on {kind:?}"),
  return `Task::none()`, and leave `last_message` untouched. Otherwise it is unwrapped and
  carries on.

**Trap 1 — why it must be the very top of `update()`.** `update()` checks
`matches!(message, Message::TimeEditComplete { .. })` to tell APPLY/CANCEL apart from other
ways off TIME EDIT. If the message were still wrapped there, every APPLY and CANCEL would
look like a side exit and the clock would be resumed when it should not be. Nothing after
the unwrap may ever see a `Tap`.

`handle_message` therefore never receives a `Tap`. Its `match` still needs an arm for it;
that arm must not panic (log at `error` and handle the inner message). `is_repeatable()`
delegates to the inner message. The repeat filter keeps working on the inner message,
because it runs inside `handle_message`, after the unwrap.

### 4. Which taps are never dropped

One function, `may_drop_when_late(&self) -> bool` on `Message`, written as an exhaustive
`match` with no `_` arm (the same shape `is_repeatable()` already uses), so a new message
will not compile until it is classified.

- **Never dropped:** `AlarmReleased` and `TimeoutReviveReleased`. These are the release
  halves of the two press-and-hold buttons. The revive button also sends its release
  on `on_exit` (the pointer leaving the button). Losing either would leave the buzzer on or
  a hold running. Known limit, not covered by this guard: iced sends no `on_exit` when the
  button is removed from the screen, so a revive hold still down when the timeout ribbon
  leaves (e.g. the half ends) never gets its release.
- **Never dropped (and never tagged anyway):** every message that does not come from the
  screen. Classifying them "never drop" keeps the function safe even if one is ever wired
  to a button.
- **Dropped when late:** every other on-screen tap, including the press halves
  (`AlarmPressed`, `TimeoutRevivePressed`). A dropped press is harmless: the operator presses
  again.

### 5. The old hand-written list goes

`tap_outlived_its_screen` is superseded: every case on it is a tagged tap whose screen kind
changes, so the new check catches it. Remove it, and port each of its tests to the new check
(the same message, tagged with the old screen, arriving on the new one), so no case loses
its test.

### 6. Separate task: TIME EDIT with a timeout that ended

`EndTimeout` and `CancelTimeout` clear the TIME EDIT page's timeout value
(`AppState::TimeEdit(_, _, None)`), but a timeout +/- tap drawn before that redraw still
arrives. The screen kind has not changed (still TIME EDIT), so the guard rightly lets it
through, and `ChangeTime` panics on `timeout_dur.as_mut().unwrap()` (mod.rs ~5114). Fix: in
that arm, when the timeout value is `None`, log at `warn` and return `Task::none()`.

## Tests (each written to fail first, where a crash or wrong behaviour exists today)

**How the tests are built (engine-lane call, 2026-11-27, after the builder check):** no test
builds a whole `RefBoxApp` or calls `update()` today (its constructor brings in config, audio,
the portal and a runtime), and building that harness is out of scope. The rules are pinned
in pure functions instead, which `update()` calls:

- a decision function, given the screen a tap was drawn on, the screen now up and the
  message, that says pass or drop (and returns the unwrapped message on pass);
- `screen_kind`;
- for trap 1, the unwrap is a pure step whose output is what `closed_by_apply_or_cancel`
  is computed from, so a test can show a tagged APPLY comes out as `TimeEditComplete`;
- for section 6, the TIME EDIT target choice is factored so a `None` timeout returns
  "nothing to edit" instead of reaching the unwrap.

The cases below are written against those functions. Where a case states the effect on
the screen (2, 6, 7), the test asserts the decision (dropped, or passed through unchanged),
and the effect is covered by the existing handler tests and the walkthrough.

Cases:

1. Penalty, warning and foul keypads: DONE, CHANGE KIND and CHANGE INFRACTION drawn on the
   keypad, arriving on ConfirmScores, are ignored (no panic, screen unchanged).
2. Late CANCEL from a keypad arriving on ConfirmScores leaves the app on ConfirmScores.
3. `ChangeTime` drawn on TIME EDIT arriving on the main page: ignored, no panic.
4. A digit drawn on one keypad arriving on a different keypad: that keypad's number is
   unchanged.
5. A colour tap drawn on a keypad arriving elsewhere: ignored, no panic.
6. `AlarmReleased` and `TimeoutReviveReleased` drawn on one screen and arriving on another
   still take effect.
7. Same-kind taps pass: two digits tagged "Penalty keypad" both land while the number
   changes in between.
8. `screen_kind` ignores contents: the same screen with different values maps to the same
   kind (one case per row of the table that has exclusions).
9. TIME EDIT: after `EndTimeout` clears the timeout value, a timeout +/- tap does not panic
   and changes nothing (separate task).
10. Untagged messages are never dropped (background messages keep working).
11. Trap 1: a tagged TIME EDIT APPLY is still treated as APPLY (the clock is not resumed
    as a side exit).

**What a unit test cannot see:** that `view()` actually applies the tag (iced 0.13 has no
widget test harness). Proof goes in the human walkthrough: with refbox launched with `-v`
(it ignores `RUST_LOG`), a debug line per tagged tap ("Tap from {screen:?}") must appear when the operator taps buttons. That
step fails if the tagging was never wired in.

## Constraints

- Heavy process: this changes how every screen tap is delivered (the screen state machine).
  Per-task review, plus the full code review at the end.
- MSRV 1.85; no new dependencies; no `unwrap()` in production code.
- The panel simulator (`sim_app`) needs no change: it is a separate canvas app and does not
  wrap the refbox view. (Correction to `approaches.md`, which said it did.)

## Left to the builder

- Exact names (`ScreenKind`, `Tap`, `may_drop_when_late`) and where the new items live
  (mod.rs or message.rs).
- Whether `ScreenKind` holds small sub-enums or reuses existing payload-free enums
  (`LengthParameter`, `ListableParameter`, `BeepTestConfigPage`) directly. `ConfigPage` is NOT
  payload-free (its remotes and custom-site pages carry values that change by themselves), so it
  needs its own payload-free copy.
- How tests build tagged messages (a small helper is fine).
