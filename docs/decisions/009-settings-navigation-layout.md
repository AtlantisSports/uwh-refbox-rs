# 009 — Settings Navigation and Layout

**Date:** 2026-04-19
**Status:** accepted

## Context

The uwh-portal web refbox has reworked the settings flow into a shallow
hub-and-spoke layout that groups related options behind one entry point
each. The native Rust refbox still exposes its original flat Configuration
page, where every sub-category (Game, Sound, Display, App, Remotes,
Language) is reachable only by sideways button jumps or is nested
inconsistently.

The project's declared back-porting rule is that **the web refbox is the
new visual standard**; the native refbox should match its screens and
colours, and any deviation must be explicitly confirmed. (See
`memory/feedback_backport_web_is_standard.md`.)

Today's state in the Rust refbox:

- `Message::ChangeConfigPage(ConfigPage)` drives settings navigation.
  `ConfigPage` has variants `Main`, `Game`, `Sound`, `Display`, `App`,
  and `Remotes(usize, bool)`
  (`refbox/src/app/message.rs:409`).
- `ConfigPage::Main` renders a flat list of category buttons; there is
  no grouping page between Main and the leaf pages.
- Sound Options is already the home of the "Manage remotes" button
  (`refbox/src/app/view_builders/configuration.rs:728`).
- Language selection is handled by a dedicated full-page list view
  using `Message::SelectLanguage(Language)` and
  `Message::LanguageSelectComplete { canceled }`. This screen already
  gives every supported language its own row — not a cycle button.

Today's state in the web refbox (authoritative design source):

- The settings entry page is a **2×2 grid** of category tiles:
  `GAME OPTIONS` | `APP OPTIONS` on top, `USER OPTIONS` | `LANGUAGE` on
  the bottom.
  (`@underwater-web/components/refbox/pages/SettingsMainPage.tsx`)
- **User Options** is a grouping page with three tiles:
  `DISPLAY OPTIONS`, `VIEW MODE` (a cycle button that switches through
  Default / Dark / High Contrast in place), and `SOUND OPTIONS`.
  (`@underwater-web/components/refbox/pages/UserOptionsPage.tsx`)
- `Manage Remotes` remains inside Sound Options and has not been moved.
- Every sub-page carries the same chrome: timer bar across the top,
  `CANCEL` / `DONE` footer, and the timeout ribbon across the bottom.

The absence of a User Options grouping page in the Rust refbox means:

1. The main Configuration page is more cluttered than the web version.
2. Display-related and sound-related options live as siblings to game
   options, even though they are preference-level rather than
   rules-level choices.
3. Any future back-port of per-user display settings (see ADR 010) has
   nowhere natural to live.

## Decision

Restructure the Rust refbox's settings navigation so that its screen
layout matches the web refbox. Functionality is unchanged; buttons are
rearranged to fit the new hierarchy.

### New navigation tree

```
Main settings page (2×2 grid)
 ├── GAME OPTIONS        (existing ConfigPage::Game)
 ├── APP OPTIONS         (existing ConfigPage::App)
 ├── USER OPTIONS        (NEW grouping page)
 │    ├── DISPLAY OPTIONS  (existing ConfigPage::Display)
 │    ├── VIEW MODE        (see ADR 010)
 │    └── SOUND OPTIONS    (existing ConfigPage::Sound)
 │         └── MANAGE REMOTES  (existing ConfigPage::Remotes)
 └── LANGUAGE            (existing language-selector page)
```

### Concrete changes

- **`ConfigPage` gains a `User` variant.** This is the new grouping
  page between `Main` and the three leaf pages it owns.
- **Main settings page becomes a 2×2 grid.** Four tiles of equal size,
  same visual weight as the web version, labelled `GAME OPTIONS`,
  `APP OPTIONS`, `USER OPTIONS`, `LANGUAGE`.
- **User Options page** contains exactly three tiles in the same
  layout the web uses: `DISPLAY OPTIONS`, `VIEW MODE`, `SOUND OPTIONS`.
  `VIEW MODE` is a cycle button whose behaviour and palette are
  defined by ADR 010.
- **Manage Remotes stays where it is.** The existing
  `manage-remotes` button inside Sound Options
  (`refbox/src/app/view_builders/configuration.rs:728`) is not moved;
  it matches the web refbox's current layout.
- **The game-number picker moves from Main to Game Options.** The
  Rust refbox currently shows the operator-editable game-number
  picker on the Main settings page. The web refbox places this
  control on Game Options. The Rust refbox matches the web: Main
  becomes a pure navigation hub, and the picker moves to Game
  Options alongside the other game parameters.
- **Each page carries chrome appropriate to what it does.** Every
  settings page still renders the same header (timer bar) and the
  same bottom timeout ribbon. Button chrome varies by page type:
  - **Navigation-only pages** — Main settings and User Options —
    carry a single `BACK` button. There is nothing to save or cancel
    on these pages; they only route to other pages.
  - **Editing pages** — Game Options, App Options, Display Options,
    Sound Options, Manage Remotes, Language — carry `CANCEL` and
    `APPLY`. Pressing `APPLY` writes that page's edits directly to
    the saved config and returns to the parent page. Pressing
    `CANCEL` discards that page's in-flight edits and returns to the
    parent page. `CANCEL` is always enabled and labelled the same
    regardless of whether the page has edits.
  - **`APPLY` is disabled when no changes have been made on that
    page.** The button is greyed out until at least one field on the
    page has moved from the value it had when the page was entered.
    This gives the operator immediate feedback that nothing would
    happen if they tapped it.

  `BACK` on Main settings exits settings and returns to the main
  game screen.

### Explicit deviations from the web

The Rust refbox deviates from the web refbox's settings design in
two places, each explicitly approved:

1. **Language screen layout.** The Rust refbox keeps its dedicated
   full-page list where every supported language is shown as its own
   row, reached by tapping the `LANGUAGE` tile on the main settings
   page. The web refbox uses a cycle button for this choice; the
   Rust refbox has more languages installed than the web version
   currently presents, and a list is easier to use than a
   many-position cycle button.

2. **Per-page save model.** The web refbox buffers every sub-page's
   edits into a single session and commits them only at the end. The
   Rust refbox commits each page's edits independently when its
   `APPLY` is pressed. A page's `CANCEL` discards only that page's
   in-flight edits; it does not roll back earlier pages. This makes
   each screen's effect obvious ("I pressed Apply on Display
   Options, so those changes are now live") and removes the
   ambiguity of when edits get persisted. It also removes the need
   for a global Cancel/Done pair on the settings entry page — which
   reduces to a simple `BACK` once it has no commits to gate.

Every other screen matches the web standard.

### What is changing beyond navigation

- **The save model moves from global to per-page.** Previously,
  every edit across every sub-page was held in an in-memory buffer
  (`EditableSettings`) and committed only when `DONE` was pressed
  on Main settings. Now each page commits its own edits
  independently when its `APPLY` is pressed. The operator loses
  the ability to make changes across several pages and then
  discard them all at once — once `APPLY` is pressed on a page,
  those changes are persisted.
- **Live side effects shift to per-page commits.** Today, updates
  to the sound controller and the LED panel's hide-time signal
  fire from a single global commit step. They now fire from each
  relevant page's `APPLY`.

### What is **not** changing

- No game rule, clock behaviour, hardware integration, or wire format
  is affected.
- No existing settings are removed, hidden, or renamed — they are
  regrouped.
- Translations for every button label already exist or will be added
  via the translation system (`translations/`); no hard-coded UI text.

### Decisions added during audit (2026-05-13)

The following decisions describe behaviour that the shipped implementation
adopted beyond what the original Decision section above specified. Each was
exercised under Unit 3 of the AI Code Audit and marked `@user_verified`. They
are folded in here so the ADR reflects shipped reality.

- **Apply on Game Options is disabled when portal state is incomplete.**
  When `using_uwhportal` is on and event/court/schedule is missing — or the
  current game is not in the active schedule for the current court — Apply
  stays disabled even if other edits exist. This prevents pressing Apply
  only to land on a dialog with no actionable choice.
  (`uwhportal_incomplete()` helper on `EditableSettings`.) [B3.9, B3.37]
- **Cancel on Game Options reverts portal-related App-slice fields too.**
  `PageEntrySnapshot::Game` captures `using_uwhportal`, `current_event_id`,
  `current_court`, and `schedule` in addition to `config` and `game_number`.
  Cancel reverts all of these. Without this expansion, edits to event/court/
  portal toggle on Game Options would survive Cancel silently. [B3.10]
- **Picking a game from the picker returns to Game Options.** Previously
  the picker's complete-message routed back to Main. The audit-shipped
  routing returns to Game Options so the operator can continue editing
  without re-navigating. Applies to both portal-mode (game list) and
  non-portal-mode (keypad). [B3.14]
- **Picking a new event auto-clears court, game number, and cached
  schedule.** Picker-driven field clearing prevents the stale-state path
  where game_number retains a value from a previous event's filtered
  list. [B3.15]
- **Picking a new court auto-clears the game number.** Same family as the
  event-clearing rule. [B3.16]
- **Game Options' picker lives in the action row.** The action row reads
  `CANCEL | Game: N | APPLY` (picker between the two buttons in the middle
  slot). Final placement after the layout sweep in commit `ce6cfeb`.
  [B3.21]
- **Game Options' "Using UWH Portal" toggle is in the left-hand column of
  the first content row.** Layout-sweep change. [B3.19]
- **Game Options uses the canonical 5-row layout** (time bar + 4
  Fill-height content rows + fixed action row at bottom) shared with all
  other Editing pages. [B3.20]
- **The Single Half tile is temporarily absent from Game Options.** The
  `BoolGameParameter::SingleHalf` variant is retained with
  `#[expect(dead_code)]` so re-wiring it raises a compile-time alert. A
  planned follow-up branch will reintroduce single-half toggling inside
  the Half Length parameter editor. Until then, single-half cannot be
  enabled from the UI — accepted as a transitional state. [B3.18, B3.39]
- **Mid-game Apply on Game Options raises the existing Keep/End/Discard
  confirmation.** New `*FromApply` ConfirmationKind variants
  (`GameConfigChangedFromApply`, `GameNumberChangedFromApply`,
  `UwhPortalIncompleteFromApply`) share the dialog UI with the prior
  global-Done flow but commit only the Game slice and route back to
  settings. Mid-game safety preserved. [B3.12, B3.36]

## Consequences

**Becomes easier:**

- A referee who uses both the web refbox and the native refbox sees
  the same screens in the same places. Muscle memory transfers.
- The main settings page is visibly less crowded — three tiles of
  preference-level options move behind `USER OPTIONS`.
- Future preference-level back-ports (starting with ADR 010's display
  modes) have a natural home instead of adding more clutter to Main.

**Becomes harder / constrained:**

- Reaching Display, Sound, or View Mode now costs one extra tap
  (Main → User → leaf). This is accepted as the cost of matching the
  web standard; these pages are visited rarely during a game.
- The refbox code must carry a new `ConfigPage::User` variant and the
  view builder that renders it. This is a small addition but touches
  the central message enum.
- Any future divergence between the web refbox and the Rust refbox
  layout requires a new ADR and explicit user approval, per the
  back-port rule.
- **Live preview of sound volumes and starting sides is not part of
  this work.** Under today's model, a change to sound or display
  settings only takes effect when the final commit runs. Under the
  new per-page model, a change takes effect when that page's `APPLY`
  is pressed — not while the operator is still editing. True live
  preview (audibly testable volume changes, visibly swapped starting
  sides before `APPLY`) is a distinct UX enhancement deferred to a
  separate ADR.

**Scope:**

- `refbox` — changes are limited to the settings UI layer:
  - `src/app/message.rs` — add `ConfigPage::User`.
  - `src/app/view_builders/configuration.rs` — rework the Main page
    to a 2×2 grid; add the User Options page; ensure uniform chrome
    on all sub-pages.
  - `src/app/mod.rs` — handle the new variant in `update()` so the
    tap on `USER OPTIONS` navigates to the grouping page and its
    three tiles navigate to their leaf pages.
  - `translations/` — add strings for the new labels
    (`user-options`, and any labels that change case to match the
    web's uppercase convention).
- `uwh-common`, `overlay`, `schedule-processor`, `wireless-remote` —
  no change.

## Verified by Unit 3 audit

This ADR's design was implemented in the 10 commits ending at `ce6cfeb` on
`refactor/refbox/settings-navigation` and audited under Unit 3 of the AI Code
Audit. The audit's catalog of operator-observable behaviours, the keep/delete
decisions, the manual-walkthrough record, and the Findings backlog are in
`AUDIT-PLAN.md`'s Unit 3 section.

**Outcome:** 39 of 40 catalog entries marked `@user_verified`; 1 entry
(B3.13 — end-game-and-apply landing page) marked `@redesign-followup` and
deferred to a separate branch; 0 deletions. Audit branch
`audit/refbox/adr-009-settings` carries 4 additional commits beyond the
original 10:

- `4eba8b6` — justify new unwraps and clippy allows (B3.40, B3.41)
- `0b6af14` — lock per-page Apply state invariants (12 regression tests)
- `f9a32f4` — restore parent snapshot after sub-page Apply/Cancel (operator-
  surfaced bug during S3.15 walkthrough; `navigate_to_parent` now re-captures
  the parent's snapshot when returning from a sub-page; regression test
  `sound_apply_requires_snapshot_present` documents the predicate)
- `4750acf` — record manual-walkthrough results in
  `refbox/tests/features/adr_009_settings.feature`

Status flipped from `proposed` to `accepted` on 2026-05-13.

### What was not verified

- **Mid-game Apply gate variant selection (B3.12 / B3.36).** `apply_game_options`
  selects between `GameConfigChangedFromApply`, `GameNumberChangedFromApply`,
  and `UwhPortalIncompleteFromApply` based on `current_period() != BetweenGames`
  plus per-edit comparisons. The supporting predicate (`uwhportal_incomplete`)
  is unit-tested exhaustively (7 tests). The variant-selection logic itself
  was not directly unit-tested because constructing a `RefBoxApp` in a test
  requires mocking tokio channels, the sound subsystem, and the portal
  client — out of audit scope per the "do not fake-test with mocks" rule.
  Verified by manual walkthrough only (S3.7).
- **Manual walkthrough of portal-incomplete Apply-disable (S3.5).** The
  operator skipped the manual walk because the 7 `uwhportal_incomplete_*`
  regression tests cover the predicate exhaustively and the manual walk
  would require setting up portal-incomplete state without adding
  predicate coverage.
- **Manual walkthrough of picker-driven field clearing (S3.10, S3.11).**
  Same rationale — the regression tests `select_event_sets_event_and_clears_*`
  and `select_court_sets_court_and_clears_*` lock the field-clearing
  behaviour; manual walk would require portal multi-event/multi-court
  test data and adds no new coverage.
- **End-game-and-apply landing page (B3.13).** The operator-observable
  behaviour was confirmed (mid-game Apply raises the three-option
  confirmation), but the destination after "End game and apply" was
  flagged for redesign: ending a game is a high-magnitude action and
  the operator should land on the main game screen (exit settings),
  not on Main settings. Deferred to branch
  `fix/refbox/end-game-and-apply-landing` per operator decision
  2026-05-13.

## References

- `refbox/src/app/message.rs:409` — `ConfigPage` enum where the new
  `User` variant is added.
- `refbox/src/app/view_builders/configuration.rs:728` — existing
  `manage-remotes` button that stays in place inside Sound Options.
- `memory/feedback_backport_web_is_standard.md` — back-porting rule:
  the web app is the authoritative source for UI design in back-port
  work.
- `@underwater-web/components/refbox/pages/SettingsMainPage.tsx` —
  web's 2×2 settings grid (authoritative layout).
- `@underwater-web/components/refbox/pages/UserOptionsPage.tsx` —
  web's User Options grouping page (authoritative layout).
- ADR 010 — defines the View Mode cycle button's content and
  behaviour.
- `refbox/src/app/mod.rs` — `PageEntrySnapshot` enum (per-page snapshot
  used by Cancel) and `capture_snapshot_for` / `navigate_to_parent`
  helpers that drive the snapshot/restore flow added during the audit.
- `refbox/src/app/view_builders/configuration.rs` —
  `EditableSettings::uwhportal_incomplete()` (Apply-gate predicate),
  `page_has_changes`, and `make_cancel_apply_footer` (per-page footer
  rendering with Apply enable/disable state).
- `refbox/tests/features/adr_009_settings.feature` — Gherkin scenarios
  recording the Unit 3 manual-walkthrough results.
- `AUDIT-PLAN.md` (Unit 3) — gitignored audit working file with the
  full catalog of operator-observable behaviours, keep/delete
  decisions, and Findings backlog for this ADR's shipped
  implementation.
