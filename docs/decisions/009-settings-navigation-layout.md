# 009 — Settings Navigation and Layout

**Date:** 2026-04-19
**Status:** proposed

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
- **Sub-page chrome is made uniform.** Every settings sub-page —
  Main, Game, App, User, Display, View Mode preview, Sound, Remotes,
  Language — renders with the same header (timer bar), the same
  footer (`CANCEL` / `DONE`), and the same bottom timeout ribbon.
  This matches the web standard.

### One explicit deviation from the web

The Language screen stays as it is today in Rust: a dedicated
full-page list where every supported language is shown as its own row,
reached by tapping the `LANGUAGE` tile on the main settings page.

The web refbox uses a cycle button for this choice. The Rust refbox
has more languages installed than the web version currently presents,
and a list is easier to use than a many-position cycle button. This
deviation has been explicitly approved.

Every other screen matches the web standard.

### What is **not** changing

- No game rule, clock behaviour, hardware integration, or wire format
  is affected.
- No existing settings are removed, hidden, or renamed — they are
  regrouped.
- The save/cancel model is unchanged: edits live in `EditableSettings`
  until `DONE` is pressed on the leaf page; `CANCEL` discards.
- Translations for every button label already exist or will be added
  via the translation system (`translations/`); no hard-coded UI text.

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
