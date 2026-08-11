# Design: auto-fit button text (shrink to fit, keep the line split)

**Date:** 2026-08-11
**Branch:** `feat/refbox/auto-fit-button-text` (off `origin/master` @ `22d33a43`)
**Supersedes for buttons:** `docs/backlog/auto-fit-button-text/NOTE.md` (the approved backlog item)
and `docs/backlog/translation-fit-audit/NOTE.md` (which audited instances of this bug rather than
fixing the class).

---

## Goal

Button labels shrink to fit the space the button actually has, so translated labels stop being cut
off. The two-line split stays exactly as it is, and when a button has two lines both lines get the
**same** size.

## The bug being fixed (measured, not inferred)

Rendered with `./target/debug/refbox --language de-DE`, warnings page:

- `team-warning-line-1` = `MANNSCHAFT` is wider than the button, so iced word-wraps it. Line 1
  renders `MANNSCHAF` and the orphaned `T` drops to line 2.
- `team-warning-line-2` = `VERWARNUNG` is pushed past the bottom of the 89px-high button and never
  renders at all.
- The wrapped text also renders left-aligned, because `Length::Shrink` expands to the full width
  once it wraps, defeating the `container(t).center_x(Length::Fill)` centring in
  `make_multi_label_button`.

The left-alignment is therefore a **symptom of the wrap**, not a separate alignment bug — centring
is already correct for text that does not wrap. `TEAM SCORE` (shipped 2026-08-10) has the same
defect.

## Scope boundary

> **Amended during execution, 2026-08-11.** The scope below is what shipped, which is wider than
> what was approved. Each addition came from the human seeing the result on screen and asking for
> it; every one is the same underlying bug in a slot the original scope had ruled out. The original
> boundary and the reason each line moved are recorded under "Deviations from the approved design".

**In scope** — the button helpers in `refbox/src/app/view_builders/shared_elements.rs`:

| Helper | Call sites | Lines | Starting size |
|---|---|---|---|
| `make_button` | 118 | 1 | app default (`SMALL_PLUS_TEXT`, 29px) |
| `make_smaller_button` | 19 | 1 | app default (29px) |
| `make_multi_label_button` | 31 | 2 | app default (29px) |
| `make_small_button` | 21 | 1 | the `size` argument the caller already passes (66px for `+`/`−`, 38px for `ZERO`) |
| `make_value_button` | ~40 | 1 label + 1 value | `MEDIUM_TEXT` / `SMALL_TEXT` per the caller's `large_text` flags |

**Also in scope** — two hand-built slots in `refbox/src/app/view_builders/main_view.rs` that are not
button helpers but carry the identical defect on the main screen:

| Slot | What it is |
|---|---|
| The alarm face (`main_view.rs:181`) | A label stack inside a coloured panel with a mouse handler — not a `button` at all. Its two lines are deliberately *different* sizes, so each fits independently rather than sharing one. |
| The warnings panel title (`main_view.rs:221`) | A bare title above the live warnings list. |

**Explicitly not doing:**

- Changing any translation text.
- Resizing or restyling the buttons themselves — every button keeps its current dimensions,
  padding, colour and style.
- The keypad title row (`keypad_pages/mod.rs:297`) — still a separate audit. Note that the backlog
  note offers `num-tos-per-half` as its example of single-line overflow risk; that string renders
  there as the keypad **title row**, a plain text label, not a button.
- The game-time banner (`shared_elements.rs:649`) — it has its own hand-rolled `compact` shrink
  rule that drops the clock from 66px to 38px when a timeout competes for width. Same class of bug,
  but the clock is the most safety-critical element on the screen and deserves its own before/after
  rather than being folded into an already-wide change. Follow-up.
- `make_lang_button_with_note` — it deliberately uses *two different* sizes for name and note, and
  already has a hand-tuned `OneLine` / `OneLineSmall` switch. Auto-fit would change a carefully
  tuned screen. Follow-up candidate.

## Decisions taken

| Decision | Choice | Reason |
|---|---|---|
| Coverage | All four label-button helpers | Auto-fit only ever *shrinks*, so every button that fits today renders identically — the blast radius is self-limiting to buttons that are already broken. Covering the 169 single-line call sites at the same time costs almost nothing and removes the same overflow risk across all 15 languages, instead of a second branch that redoes the same five-language rendering check. |
| Size floor | **19px** (`SMALL_TEXT`) | Already used in the app (the notes under language names), so it is known-legible on the refbox screen. Roughly two-thirds of normal button text — enough headroom to absorb German and Spanish. |
| Fit axis | **Width only** | Two lines at 29px are already ~2px taller than the inside of an 89px button. A height check would shrink every two-line button a step even in English, breaking the "already-fitting buttons are untouched" property. |
| Step | 1px, descending from the starting size to 19px | Simple and obviously correct. Cost is irrelevant because the result is cached. |
| Wrapping | **Wrap first, shrink second** (amended — see Deviations) | Two lines at full size read better across a pool deck than one shrunken line. Applies only where the app supplies the label as a single string; a caller-chosen split is never re-wrapped. |
| Break points | Space, tab, and after `/` | The slash stays with the first half, so `1/HALBZEIT` breaks as `1/` above `HALBZEIT`. `:` is deliberately excluded so `15:00` can never split. |
| Alignment | Done by placing each line's box, never by the text engine | Working around an iced repaint defect — see Edge cases. |
| New dependency | None | `iced_core` is already a direct dependency of `refbox`, and `iced_core::widget::text` publishes the layout/draw helpers and the `Paragraph` measurement API. |

## Behaviour

- A label that fits at its starting size renders **pixel-identically to today**. Auto-fit never
  grows text.
- A label supplied as one string that does not fit is **wrapped** at the break point that leaves
  the wider half narrowest, staying at full size. Only if two lines still do not fit does it
  shrink, 1px at a time, to a floor of 19px.
- A label the caller already split into two lines keeps that split, and both lines share one size —
  governed by the wider of the two.
- A translation that carries its own line breaks keeps them: several `.ftl` entries are written
  across two lines deliberately, and those win over automatic wrapping.
- Words are never broken mid-word. `MANNSCHAFT` stays whole.
- Every line is centred **individually**, so a short line and a long one in the same label no
  longer share a left edge. That is what the German warnings page was really showing.
- Scripts without spaces — Thai, Japanese, Chinese — have no break points, so they fall through to
  shrink-only. Verified as legible by rendering.

## Architecture

Four pieces, in a new file `refbox/src/app/view_builders/fit_text.rs`. The first three are pure
functions taking measurement as a callback, so they test with a fake ruler — no window, no fonts,
no renderer, no graphics backend.

### 1. `best_split` — where to break

```rust
/// Splits `line` in two at the break point that leaves the wider half as narrow
/// as possible. `None` when there is nowhere to break.
fn best_split(measure: impl Fn(&str) -> f32, line: &str) -> Option<(String, String)>
```

### 2. `fit_layout` — which arrangement, at what size

```rust
/// `line_sets` ordered fewest-lines-first, `candidates` largest-size-first.
/// Returns the chosen arrangement and size.
fn fit_layout(
    measure: impl Fn(&str, f32) -> f32,
    max_width: f32,
    candidates: &[f32],
    line_sets: &[Vec<String>],
) -> (usize, f32)
```

Size is the **outer** loop. That single detail is what makes a label prefer two lines at full size
over one shrunken line.

### 3. `line_left` — where each line sits

```rust
/// Where a line of `line_width` starts inside a box of `box_width`.
fn line_left(align: Horizontal, box_width: f32, line_width: f32) -> f32
```

### 4. `FitText` — the custom widget

Measurement requires a `Renderer`, which iced only provides inside a widget's `layout()`, never in
the declarative view function. Hence a widget rather than a plain helper. It owns all the lines of a
label so that the "one size for every line" constraint can be enforced in one place.

- `layout()`: takes the available width from the limits, measures each candidate line at each
  candidate size via `Paragraph::with_text(...).min_bounds()` with unbounded width, calls
  `best_split` and `fit_layout`, then places one child box per line — each exactly as wide as its
  own line, positioned by `line_left`.
- `draw()`: reuses `iced_core::widget::text::draw` per line, clipped to the widget's own bounds.
- Caches the chosen arrangement and size in the widget's tree state, keyed on the line contents,
  the available width and the starting size. The app rebuilds its whole screen on every clock tick,
  so re-measuring every frame would be wasteful on the Pi.
- Builders: `size`, `width`, `height`, `align_x`.

This is the **first custom widget in the `refbox` crate**. It reuses iced's own published text
layout and draw helpers rather than reimplementing text rendering, and it inherits the app's
default font and `Shaping::Basic` — the same choices `text()` makes today — so measurement matches
what is drawn.

### 5. The call-site swaps

Each helper replaces its `text(...)` with `fit_text(...)`, passing the starting size it already
uses. No call site changes; no signature changes.

`make_value_button` is the exception: its label and value now take **guaranteed shares** of the
width (3:2) rather than competing for it. iced lays fixed-width children out before flexible ones
and gives them what they ask for, so whichever was measured first used to starve the other — the
value was clipped to `1/`, and giving the value priority instead let `1/HALBZEIT` at the large size
crowd out the label. Fixed shares mean neither can starve the other, and each wraps or shrinks
inside its own share.

## Edge cases

### Two iced 0.13 defects this works around

Both were found by rendering, not by reading. Anyone changing this widget needs to know them.

**1. `Wrapping::None` is silently ignored.** `iced_graphics` defines `to_wrap` and never calls it,
so the text engine always word-wraps regardless of what the widget asks for. Trusting the setting
cost us a real defect: a paragraph given a one-line-tall box wrapped anyway and the second line fell
outside the box and vanished, so `VERWARNUNG HINZUFÜGEN` rendered as `VERWARNUNG`. Every paragraph
here is therefore laid out with **unbounded width**, which is the only reliable way to stop it
wrapping. All wrapping is done by this widget, never by the engine.

**2. Repaint tracking mishandles aligned text.** `iced_graphics::text::visible_bounds` applies a
paragraph's alignment offset *after* clipping, using the clipped width, while drawing applies it
*before* clipping, using the full width:

```rust
Horizontal::Center => bounds.x -= bounds.width / 2.0,
Horizontal::Right  => bounds.x -= bounds.width,
```

A centre-anchored paragraph therefore reports a dirty rectangle half a text-width from where it
draws, leaving **half the text as stale pixels**; a right-anchored one is off by a full width. This
only shows on text that *changes* — a static label paints once and stays correct — which is why it
appeared on the `CANCEL`/`BACK` swap and on `EVENT:` going from `Loading...` to the event name, and
not on any of the four language sweeps.

The rest of the app never hits this because every `text()` in it is left-aligned inside a centring
container, and for `Left` the offset is zero. **Every paragraph here is anchored top-left and the
widget positions each line itself** (`line_left`). Do not "simplify" this by handing the alignment
to the paragraph — that reintroduces the artifact.

### Other edge cases

- **Still too wide at 19px.** Stop at 19px and clip, with drawing clipped to the button's own bounds
  so text can never spill onto a neighbouring button. Both lines stay visible and the failure reads
  as "this label is too long" rather than the old failure, where a whole line silently vanished.
  This is a floor, not a plan — it should not trigger for any current translation.
- **Empty label** measures zero, fits at the starting size, unchanged.
- **Zero available width**, which happens transiently during layout, falls through to the 19px
  floor and corrects itself on the next pass.
## Testing

17 unit tests, driven by a fake ruler — no window, no fonts, no renderer:

- **Sizing:** everything fits → largest candidate; too wide → steps down to the first that fits; a
  line exactly as wide as the space counts as fitting; nothing fits → the floor; an empty label
  keeps full size; zero available width falls to the floor.
- **Arrangement:** one line preferred when it fits at full size; two lines at a bigger size
  preferred over one shrunken line; the wider line governs the shared size.
- **Breaking:** the split balances the two halves; a break after `/` keeps the slash with the first
  half; `15:00` is never split at its colon; a single word cannot be split.
- **Alignment:** lines of different widths are each centred on their own — the property whose
  absence made German `TEAM WARNING` look left-aligned; left and right pin the matching edge.

**Assertion trap avoided:** in each sizing test the expected answer is distinct from both the floor
and the maximum. Otherwise the assertion cannot distinguish "chose correctly" from "fell back to
the default" — the same shape of mistake as asserting a field that two theme styles share.

**What tests cannot cover here:** every defect found in this work — the ignored wrap setting, the
repaint offset, the row starving one side — was invisible to unit tests and only appeared on
screen. The pure functions are worth having, but rendering is the real gate.

`just check` (fmt, clippy `-D warnings`, tests, audit) must be clean before the PR.

## Verification — by rendering, never arithmetic

Staged, because a mis-wired custom widget puts text in the wrong place or fails to draw it:

1. Build `FitText`, use it in `make_multi_label_button` **only**, and render the German warnings
   page. `MANNSCHAFT` / `VERWARNUNG` must both appear, whole, centred.
2. Only then roll out to the other three helpers.
3. Screenshots across the languages that actually stress it:
   - **de-DE** and **es** — long words
   - **th-TH** — exercises the Thai font subset
   - **ja-JP** and **zh-CN** — wide glyphs; character count is a poor proxy for width
4. **TEAM SCORE** in German on the main view — same defect as the warnings page.
5. English regression pass across the busiest pages — main view, warnings, fouls, configuration —
   to confirm nothing shrank that should not have, and that `+` / `−` / `ZERO` are untouched.

An English screenshot alone proves nothing.

**Completed 2026-08-11.** All five languages walked through on the main view, the warnings page and
the game parameters page, plus the English regression pass and the portal/event page. Thai rendered
its font subset with no missing glyphs and no vertical clipping; Japanese and Chinese handled
full-width glyphs at readable sizes. Neither Thai nor CJK has spaces to break at, so both rely
entirely on shrinking — confirmed legible by eye, not by arithmetic.

Every launch must use the portal URL override — launching without it hits production and deletes
the saved portal link:

```
WAYLAND_DISPLAY= UWH_PORTAL_URL_OVERRIDE=http://localhost:8090 \
  ./target/debug/refbox --allow-http --language de-DE
```

Rebuild the binary first: `just check` builds a *test* binary, not the one this command runs.

## Risks

| Risk | Mitigation |
|---|---|
| First custom widget in the crate — layout/draw lifecycle wired wrong | Reuse iced's own `text::layout` / `text::draw`; verify by rendering after one helper, before the other three |
| Measurement disagrees with rendering (font, shaping) | Widget inherits the same default font and `Shaping::Basic` the app already uses |
| Per-frame measurement cost on the Pi | Cache keyed on contents + available width + starting size |
| A currently-fine button shrinks unexpectedly | English regression pass; auto-fit can only shrink, so any change is visible and attributable |

## Deviations from the approved design

Recorded because the branch that shipped is materially wider than the one approved. Every change
came from the human seeing the result on screen; each is noted with what forced it.

| # | Approved | Shipped | Why |
|---|---|---|---|
| 1 | "Labels no longer wrap — shrink instead" | **Wrap first, shrink second** | Correct for caller-split labels, wrong for single strings. `JETZT STARTEN` was being squeezed onto one small line when two full-size lines read far better. Raised by the human on sight. |
| 2 | Four button helpers | Also **`make_value_button`** | Its values were being clipped — `NEIN`→`N`, `4:00`→`4:0`, `1/HALBZEIT`→`1/`. Pre-existing on `master`, but newly conspicuous beside buttons that now behaved. |
| 3 | No `main_view.rs` changes | Also the **alarm face** and the **warnings panel title** | Both carry the identical defect in plain sight on the main screen and neither goes through a button helper. |
| 4 | `fit_size(measure, max_width, candidates)` | `best_split` + `fit_layout` + `line_left` | Wrapping needs to choose an arrangement, not just a size, and alignment had to move into layout (see the repaint defect). |
| 5 | Paragraphs centred | Paragraphs **top-left anchored**, widget positions lines | Forced by the iced repaint defect. Not optional — reverting it reintroduces visible stale pixels. |
| 6 | Break at spaces | Break at spaces **and after `/`** | Requested by the human so `1/HALBZEIT` could render larger on two lines. |
| 7 | — | **Translator line breaks honoured** | Several `.ftl` entries are deliberately two lines; the first implementation overrode them. |

**Two defects were introduced and fixed during execution, both found only by rendering:** a missing
second line on `VERWARNUNG HINZUFÜGEN` (trusting `Wrapping::None`), and stale pixels over changing
labels (the repaint offset). Neither was catchable by the unit tests.

## Follow-ups left open

- **The game-time banner** (`shared_elements.rs:649`) still has its own hand-rolled `compact` shrink
  rule. Same class of bug; deliberately excluded because the clock is the most safety-critical
  element on screen.
- **The keypad title row** (`keypad_pages/mod.rs:297`) — the separate slots audit.
- **`make_lang_button_with_note`** — hand-tuned two-size switch.
- **The alarm-during-timeout rule.** Raised while testing and then withdrawn: during a timeout in an
  active half the alarm drops to hold-to-test rather than staying armed. Working as written
  (`main_view.rs:157`), not a text-fitting matter, and left alone.

## Housekeeping folded into this branch

`docs/backlog/team-coloured-grid-cells/NOTE.md` still says Dark mode "was never viewed". It was
checked and approved after that note merged. One-paragraph correction, folded into this branch's
docs commit rather than getting its own.
