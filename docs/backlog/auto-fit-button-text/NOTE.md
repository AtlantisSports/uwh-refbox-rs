# Backlog: auto-fit button text (shrink to fit, keep the line split)

**Status:** Approved in principle, not started. Next branch after
`2026-08-10-team-score-button-and-dash-readout`.
**Surfaced:** 2026-08-10, after rendering the warnings page in German.
**Raised by:** the user — *"would it be a major project to center justify all button text and allow
the text to resize to fit? I still want to keep the separate lines like we have, but the text size
should adjust (equally for both lines if applicable) if needed."*

**Supersedes** `../translation-fit-audit/NOTE.md` for buttons: this fixes the class of bug rather
than auditing instances of it, and would have prevented the German defect entirely.

## The bug it fixes (measured, not inferred)

Rendered with `./target/debug/refbox --language de-DE`, warnings page:

- `team-warning-line-1` = `MANNSCHAFT` is **wider than the button**, so iced word-wraps it: line 1
  shows `MANNSCHAF`, the orphaned `T` drops to line 2.
- `team-warning-line-2` = `VERWARNUNG` is pushed out of the 89px button and **never renders**.
- The wrapped text renders left-aligned, because `Length::Shrink` expands to the full width once it
  wraps, defeating the `container(t).center_x(Length::Fill)` centring in
  `make_multi_label_button` (`refbox/src/app/view_builders/shared_elements.rs:1093`).

Note the centring is therefore **already correct** for non-wrapping text — `make_button` and
`make_small_button` both centre. The visible left-alignment in German is a *symptom of the wrap*.
Fixing alignment alone fixes nothing.

## Why this is tractable — and cheaper than the old prototype assumed

**No new dependency.** iced 0.13 already exposes text measurement:
`iced::advanced::text::Paragraph::with_text(...)` plus `.min_bounds() -> Size`
(`iced_core-0.13.2/src/text/paragraph.rs:12,34`). The stalled prototype at
`docs/backlog/dynamic-font-sizing/` used `fontdue` and was blocked on approving that dependency
(project rule: no new deps without discussion). That blocker is gone — use iced's own measurement.

**The prototype has the right algorithm**: measure the string, step the size down, stop at a floor
(it used `MIN_FONT_SIZE = 12.0`). Reuse the approach, not the code — its `GameInfoCell` enum
targets a screen that has since been rebuilt as `game_info_table.rs`.

**One helper covers 31 call sites.** `make_multi_label_button` alone has 31 callers, so fixing the
helper fixes every current button and every future locale.

## Design sketch

Measurement needs a `Renderer`, which is **not** available in the declarative view function — only
inside a widget's `layout()`. So this needs a small custom widget rather than a plain helper.

Split it so the interesting part is testable without a renderer:

```rust
/// Largest size from `candidates` (descending) whose measured bounds fit `max`.
/// Falls back to the smallest candidate rather than overflowing.
fn fit_size(measure: impl Fn(f32) -> Size, max: Size, candidates: &[f32]) -> f32
```

`fit_size` is pure and unit-testable with a fake `measure` closure — no renderer, no font. The
custom widget supplies the real measurement in `layout()` and caches the chosen size for `draw()`.
The old prototype had no tests; this design is the reason to rewrite rather than port.

Apply the **same** chosen size to both lines, per the user: *"equally for both lines if
applicable."*

## Scope when picked up

Own branch, own Scope Card and plan.

- Fold in the centring tweak (`align_x(Horizontal::Left)` → centre in `make_multi_label_button`)
  since wrapped-text alignment and sizing belong together.
- Decide whether `make_button` / `make_small_button` also get auto-fit, or only the two-line
  variant. The single-line buttons have their own overflow risk (`num-tos-per-half` is near the
  edge **in English**) but a wider blast radius.
- Watch the interaction with iced's layout/draw lifecycle — the size picked in `layout()` must
  reach `draw()`.
- **Verify by rendering**, not arithmetic, across the worst cases: de-DE and es (long words), th-TH
  (needs the Thai font subset — see the CJK/font-subset memory), and ja-JP / zh-CN (wide glyphs, so
  character count is a poor proxy for width). Do not claim a fit from an English screenshot.
