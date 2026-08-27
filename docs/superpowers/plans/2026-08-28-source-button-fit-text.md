# Source Buttons Shrink To Fit — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development
> (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use
> checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the UWH PORTAL and CUSTOM source buttons on the Game Options page render their
labels with `fit_text`, so a long translation shrinks to fit instead of being clipped.

**Architecture:** These two buttons are the only hand-built tiles left on the page. Every other
tile goes through `make_chrome_button` / `make_tile_button` / `make_value_button`, all of which
build their label with `fit_text` — a widget that wraps at word gaps and then shrinks down a
size ladder until the label fits. The two source buttons instead build a plain
`text(...).size(MEDIUM_TEXT)`, which has one size and no way to get smaller, so a label wider
than the button is simply cut off. The fix swaps the label widget at these two call sites and
changes nothing else about the buttons.

**Tech Stack:** Rust 2024, iced 0.13, `refbox` crate only.

**Spec:** This plan is the spec. The defect was reported by Eric on 2026-08-28 against
`refbox/src/app/view_builders/configuration.rs` in `make_event_config_page`.

## Global Constraints

- MSRV 1.85, edition 2024. Do not change either.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` must stay clean.
- `refbox` only. Do not touch `uwh-common`, `overlay`, `wireless-remote`, or any wire format.
- Do not add dependencies.
- Do not introduce `unwrap()`/`expect()` in production code.
- **Do not change any translation wording, and do not add or remove translation keys.**
- **Do not change the buttons' height, width, padding, style or selected-state treatment.**
  This is about how the label is drawn inside the button, nothing else.
- Do not touch shared helpers in `shared_elements.rs` or `fit_text.rs`. PR #2620 (merged
  2026-08-27) has just reworked button heights across the settings screens; changing a shared
  helper now would ripple across every page it touched.
- Do not touch any other page.

---

## The defect, with the geometry

Both buttons build their label like this (`configuration.rs`, around lines 856–880 on
`origin/master`):

```rust
text(fl!("source-custom"))
    .size(MEDIUM_TEXT)          // 38.0, fixed — this is the whole problem
    .align_x(Horizontal::Center)
    .align_y(Vertical::Center)
    .width(Length::Fill)
    .height(Length::Fill)
```

**How much room the label actually has**, on the default 945 × 691 window
(`config.rs` defaults, and Eric's live config):

| Step | Arithmetic | Width |
|---|---|---|
| Window | — | 945 |
| Less outer page padding (`PADDING` = 8, both sides) | 945 − 16 | 929 |
| Row 1 is three equal `Fill` cells with two `SPACING` gaps | 929 − 16 | 913 |
| One cell | 913 / 3 | **304.3** |
| Less the button's own padding (iced `DEFAULT_PADDING`: 10 left + 10 right) | 304.3 − 20 | **≈ 284** |

So each label gets about **284 px**.

**What the labels need.** Measured, not estimated: advance widths taken from the bundled
`refbox/resources/Roboto-Medium.ttf` at 38 px, with no kerning, which is what `Shaping::Basic`
actually does.

| Locale | `source-custom` | Splittable? | Width at 38 px | Result after the fix |
|---|---|---|---|---|
| it-IT | PERSONALIZZATO | **no** — one 14-letter word | 322.6 → **clips** | shrinks to **33 px** (280.1) |
| es / pt-PT | PERSONALIZADO | no | 301.4 → **clips** | shrinks to **35 px** |
| fr | PERSONNALISÉ | no | 273.6 → fits, ~11 px spare | stays 38 px |
| en-US | CUSTOM | no | 155.1 | stays 38 px |
| ko-KR | 사용자 지정 | yes (space) | 201.4 | stays 38 px, one line |
| zh-CN | 自定义 | CJK char-break | 114.0 | stays 38 px, one line |

`source-portal` is `{ $portal } PORTAL` in every locale, and `$portal` resolves to `UWH` or
`UWR`: 237.8 px and 234.5 px at 38 px. Both fit the 284 px budget, so neither clips today.

**Conclusion:** Italian is the worst case, and it lands at 33 px — a full 14 px above
`fit_text`'s floor (`MIN_FIT_TEXT` = 19.0). Shrinking alone therefore fixes every locale with
room to spare: no new floor, no wrapping trick, and no wording change. Locales that already fit
stay at 38 px and look identical to today, because `fit_text` starts at the size it is given and
only steps down when it has to.

**`UWH PORTAL` will not become two lines.** It contains a space, so `best_split` *could* break
it — but `fit_layout` iterates size-outer / arrangement-inner (`fit_text.rs:145-151`), so at
38 px it tries the single line first, and 237.8 ≤ 284.3 means one line wins. Two lines only
become possible below a window width of about 805 px, which no deployed configuration uses.

Even so, the authoritative check is the visual walkthrough at the end of this plan.

## Why the `align_y(Center)` + `height(Fill)` pairing must not be carried across

Both labels currently pair `align_y(Vertical::Center)` with `height(Length::Fill)` on the
`text` widget. That is the iced 0.13 stale-paragraph pattern this repo has already removed
elsewhere — see the comment in `make_value_button` in `shared_elements.rs`, and the module
notes in `fit_text.rs`. A centre-anchored paragraph reports a dirty rectangle half a text-width
away from where it actually draws, so half the label survives as stale pixels when the text
changes.

`fit_text` solves this by always anchoring its paragraphs top-left and positioning each line
itself, then centring the whole block vertically inside its bounds. So the correct replacement
does **not** re-add either call:

- `align_x(Horizontal::Center)` — already `fit_text`'s default.
- `width(Length::Fill)` — already `fit_text`'s default.
- `height(Length::Fill)` — already `fit_text`'s default, and documented there as the right
  choice "right inside a button".
- `align_y(Vertical::Center)` — **`fit_text` has no such method**, by design. Vertical
  centring is done by its layout.

The whole label expression therefore reduces to `fit_text(...).size(MEDIUM_TEXT)`.

## Scope boundary

**In scope — the only file that changes:**

- `refbox/src/app/view_builders/configuration.rs` — the `portal_source_btn` and
  `custom_source_btn` label expressions in `make_event_config_page`, plus one added `use`.

**Explicitly out of scope:**

- `fit_text.rs`, `shared_elements.rs`, and every other shared helper.
- The `.ftl` translation files — no wording change, no key added or removed.
- The buttons' `.width()`, `.height()`, `.style()`, selected-state branch, `.on_press()`, and
  padding. All stay exactly as they are.
- `uwhportal_auth_text` immediately above these two buttons, which uses the same
  `text()` + `align_y(Center)` + `height(Fill)` pattern at `MEDIUM_TEXT`. It has the same
  latent problem, but it was not part of the report — see "Noticed but not fixed" below.
- Every other page.

## Acceptance criteria

Observable by the human, without reading code:

1. On the Game Options page in **Italian**, the CUSTOM tile reads **PERSONALIZZATO** in full,
   at a smaller size, with nothing cut off at either end.
2. In **Spanish** and **Portuguese** the CUSTOM tile reads **PERSONALIZADO** in full.
3. In **English** the two tiles look exactly as they do today — CUSTOM and UWH PORTAL at the
   same size as before, unchanged.
4. Both tiles are the same size, shape and colour as before, in both the selected and the
   unselected state, and selecting one still switches the page as it does today.
5. `just check` passes.

## Testing

**No test can fail before this change and pass after it.** The defect is a call-site choice —
these two buttons never reached the shrinking logic — and iced 0.13 exposes no way to assert a
rendered widget's size. The shrinking logic itself is already covered by `fit_text.rs`'s unit
tests, including `splitting_a_single_word_is_not_possible`, which is exactly the PERSONALIZZATO
case; those pass on both sides of this change.

**One guard test is added anyway**, at Eric's direction after the second code review pointed out
that `fit_text.rs` already contains `digit_ruler` — Roboto Medium's true advance — used to pin an
earlier fit bug against real geometry. `the_italian_source_label_shrinks_but_stays_clear_of_the_floor`
follows that precedent: with the real tile width and Roboto's real advance for PERSONALIZZATO,
`fit_layout` must return 33px.

Be clear about what it is and is not:

* It **does not** prove the fix. Revert the two call sites and it still passes.
* It **does** pin the arithmetic the fix depends on. If `PADDING`, `SPACING`, `MEDIUM_TEXT`,
  `MIN_FIT_TEXT` or the shipped window size later move such that Italian would be driven back to
  the floor, this fails — instead of nothing failing until someone runs the app in Italian.
* It was mutation-checked: at a 700px window it correctly computes 23px and fails.

The real verification of the fix remains compilation, `just check`, and looking at the screen in
Italian.

---

## Task 1: Draw both source-button labels with `fit_text`

**Files:**
- Modify: `refbox/src/app/view_builders/configuration.rs` (import block, line ~1; the two
  button expressions at ~856–880)

**Interfaces:**
- Consumes: `fit_text` and `FitText::size` from `refbox/src/app/view_builders/fit_text.rs`.
  Both are `pub(super)`, so they are already visible from `configuration.rs` — no visibility
  change is needed.
- Produces: nothing. No new public item.

- [x] **Step 1: Add the import**

`configuration.rs` does not import `fit_text` today. Add it as its own line at the top, matching
how `main_view.rs` does it:

```rust
use super::fit_text::fit_text;
```

Put it immediately above the existing `use super::{ViewData, fl, message::*, shared_elements::*, theme::*};`
line so the `super::` imports stay together.

- [x] **Step 2: Replace the portal button's label**

Find:

```rust
        let portal_source_btn = button(
            text(fl!("source-portal", portal = portal_name_for_mode(mode)))
                .size(MEDIUM_TEXT)
                .align_x(Horizontal::Center)
                .align_y(Vertical::Center)
                .width(Length::Fill)
                .height(Length::Fill),
        )
```

Replace with:

```rust
        let portal_source_btn = button(
            fit_text(fl!("source-portal", portal = portal_name_for_mode(mode))).size(MEDIUM_TEXT),
        )
```

Everything after that closing `)` — `.width(Length::Fill)`, `.height(Length::Fill)`, the
`.style(if settings.source == GameSource::Portal { ... })` branch and `.on_press(...)` — stays
exactly as it is. Do not add `.padding(...)`.

- [x] **Step 3: Replace the custom button's label**

Find:

```rust
        let custom_source_btn = button(
            text(fl!("source-custom"))
                .size(MEDIUM_TEXT)
                .align_x(Horizontal::Center)
                .align_y(Vertical::Center)
                .width(Length::Fill)
                .height(Length::Fill),
        )
```

Replace with:

```rust
        let custom_source_btn = button(fit_text(fl!("source-custom")).size(MEDIUM_TEXT))
```

Again, leave `.width()`, `.height()`, `.style()` and `.on_press()` untouched.

- [x] **Step 4: Update the comment above the pair**

The existing comment above `portal_source_btn` reads:

```rust
        // These two fill row 1's second and third cells, beside Manual Games. The
        // active one is marked with the existing selected-button style rather
        // than a new treatment.
```

Add one sentence recording why the label is a `fit_text` and not a `text`, so the next author
does not "simplify" it back:

```rust
        // These two fill row 1's second and third cells, beside Manual Games. The
        // active one is marked with the existing selected-button style rather
        // than a new treatment. Labels are `fit_text`, as every other tile's are:
        // Italian's PERSONALIZZATO is one 14-character word with nowhere to wrap,
        // so it can only be shown by shrinking it.
```

- [x] **Step 5: Check that the removed calls did not orphan an import**

`Vertical`, `Horizontal`, `text` and `MEDIUM_TEXT` are all still used elsewhere in this file
(`uwhportal_auth_text` just above uses the first three; `MEDIUM_TEXT` is used in several
places). So no import should need removing. Confirm by building rather than by eye — clippy's
`-D warnings` will fail on an unused import, which is the check.

Run: `cd <worktree> && just check` (its `cargo clippy --all -- -D warnings` leg is the one
that gates CI).

**Do not use `cargo clippy -p refbox --all-targets`.** That stricter form fails on this repo
today with two pre-existing errors that CI does not run into — `items after a test module` in
`player_grid.rs` and `field assignment outside of initializer` in `app/mod.rs` — neither of them
anything to do with this change. `just lint` is deliberately not `--all-targets`.

- [x] **Step 6: Run the full gate**

Run: `cd <worktree> && just check`
Expected: fmt, lint, tests and audit all clean.

A backgrounded command does not inherit a `cd`, so the `cd <worktree> &&` prefix is required.
Confirm from the log that the compiled paths are the worktree's, not the main checkout's.

- [x] **Step 7: Commit**

```
fix(refbox): shrink source-button labels to fit their tiles
```

---

## Verification

`cargo check` and `just check` do **not** refresh `target/debug/refbox` — `just check` builds a
*test* binary. Rebuild the app binary explicitly before looking at anything:

```bash
cd <worktree> && cargo build -p refbox
```

Then launch with a long-translation locale. Italian is the worst case:

```bash
cd <worktree> && UWH_PORTAL_URL_OVERRIDE=https://api.dev.uwhportal.com WAYLAND_DISPLAY= \
  ./target/debug/refbox --language it-IT
```

Three things in that line are not optional:

* **`UWH_PORTAL_URL_OVERRIDE`** — launching bare sends refbox at the *production* portal, where
  a dev event's token is invalid, and refbox then **deletes** `~/.config/refbox/portal_link.json`.
  A run that has nothing to do with the portal still does this. Carry the override every time.
* **`WAYLAND_DISPLAY=`** unset — forces X11 under WSL, which is what actually renders.
* **`--language it-IT`** — confirmed against the `Cli` struct in `refbox/src/main.rs:174-176`
  (`#[clap(long)] language: Option<LanguageIdentifier>`).

Only one refbox may run at a time: every build and worktree shares `~/.config/refbox`.

Walk to **Settings → Game Options** and check, against the acceptance criteria above:

1. Italian: CUSTOM tile reads PERSONALIZZATO in full, smaller, nothing clipped.
2. Switch to Spanish: PERSONALIZADO in full.
3. Switch to English: both tiles look exactly as they did before the change.
4. Tap each tile: the selected-state highlight and the page switch both still work.
5. Back in Italian, look at the two tiles **together**: PERSONALIZZATO sits at 33 px beside
   UWH PORTAL at 38 px. Confirm that size difference reads as acceptable. `fit_text` can be told
   to make a pair agree on one size (`shared_with`), and deliberately is not asked to here —
   every other tile on the page shrinks on its own, and matching them would drag UWH PORTAL down
   to 33 px for no reason of its own.
6. Switch to **Français**: PERSONNALISÉ stays at 38 px, but with only ~11 px (4 %) to spare.
   Confirm it is not touching the tile edges.

## Noticed but not fixed

`uwhportal_auth_text` (the ACCESS TOKEN tile, immediately above these two in the same function)
builds its label the same way — `text(...).size(MEDIUM_TEXT)` with `align_y(Vertical::Center)`
and `height(Length::Fill)` — so it carries both the same clipping risk and the same
stale-paragraph pairing. It is a right-aligned label in a row beside a status indicator, not a
centred tile label, so it is not a copy-paste of this fix.

It was not part of the report and is not touched here. Worth its own branch if Eric wants it.

## Deviations

**`fit_text.rs` was modified, and the scope boundary above says it would not be.**

The boundary was written before either code review ran. The second review found that
`fit_text.rs` already contained `digit_ruler`, a real-font-metrics ruler used to pin an earlier
fit bug — precedent for a guard test I had argued was not worth writing. Eric was asked and chose
to add it. The test therefore lives in `fit_text.rs`'s own test module, alongside the functions it
exercises.

It could not live in `configuration.rs`, whose geometry it describes and which *is* in scope:
`fit_layout` and `size_ladder` are private to the `fit_text` module, and widening them purely to
relocate a test would be a worse change than the one recorded here.

Also corrected after review, both in this document rather than the code:

* The label budget read 294px. iced's default button padding is 10px horizontally, not 5, so the
  real figure is 284px. The estimates were replaced with measured Roboto-Medium advances.
* Task 1 Step 5 prescribed `cargo clippy -p refbox --all-targets -- -D warnings` and claimed it
  would be clean. It is not, and never was on this branch — see the corrected step.
