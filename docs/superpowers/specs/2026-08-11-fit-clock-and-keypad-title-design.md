# Design: fit the game-time banner and the keypad title row

**Date:** 2026-08-11
**Branch:** `feat/refbox/fit-clock-and-keypad-title` (off `master` @ `d09ff701`)
**Follows:** `2026-08-11-auto-fit-button-text-design.md` (PR #2063, merged), which deliberately left
these two slots alone.

---

## Goal

The game clock stops disappearing from the banner, and the keypad page title row stops crowding its
value.

## The bug, measured

**Baseline captured on `d09ff701`, before any change.** Main page, German unless noted:

| Configuration | Baseline result |
|---|---|
| UWH, timeout running | Period wrapped to two lines, **no game clock at all** |
| UWH, timeout running, Spanish | Clock clipped to `14:5` |
| UWR **without** portal tile, timeout | **No game clock at all** |
| UWR **with** portal tile, **no timeout** | **No game clock at all** |
| UWR with portal tile, between games | **No clock**, `NÄCHSTES SPIEL` wrapped |
| UWR with portal tile, timeout | Works — period small and wrapped, clock present at `14:58` |

### Why the clock vanishes rather than merely clipping

The tall banner is `HEALTH_TILE_SIZE + PADDING + SMALL_PLUS_TEXT` = 126px, and its button adds
`.padding(PADDING)`, leaving **110px** inside. The column stacks the period above the clock: at full
size that is ~37.7px + ~85.8px = ~123.5px of line boxes, which renders correctly only because line
height includes leading the glyphs do not use. **One wrap of the period name adds ~37.7px**, reaching
~161px, and the clock is pushed out and never drawn.

So the trigger for catastrophe is a *wrap*, not a size.

### Two independent causes of crowding, not one

The banner's width depends on how many fixed side tiles squeeze it:

| Side tiles | Configuration | Banner width |
|---|---|---|
| 0 | UWH, no portal | Widest |
| 1 | UWH + portal | ~317px |
| **2** | **UWR + portal tile *and* play/pause button** | **~222px** |

A timeout then halves whichever applies. The existing rule combined these wrongly:

```rust
let compact = portal_indicator.is_some() && mode == Mode::Rugby && snapshot.timeout.is_some();
//                                     ^^ "two side tiles"   ^^^                        ^^^ "a timeout"
```

Two independent causes, `AND`-ed as though both were required — which is why five of the six baseline
configurations were broken and only the sixth, where both happened to hold, worked.

## What shipped

Sizing depends only on **how many readouts share the banner**, which is the thing that actually
determines the space. No mode checks, no tile checks, no `compact`.

| | One readout (game clock alone) | Two readouts (game + timeout) |
|---|---|---|
| Columns | One, full width | Two, equal halves |
| Label | One line, **never wraps**, shrinks to fit | **May wrap to two lines**, shrinks to fit |
| Label starting size | `SMALL_PLUS_TEXT` | `SMALL_TEXT` |
| Label size | Its own | **Shared** — both labels take one size |
| Time starting size | `LARGE_TEXT` | `BANNER_TWO_TIME_TEXT` (46px) |
| Time | Never wraps, shrinks to fit | Never wraps, **shared** with the other time |

Three details carry the design, and each exists for a measured reason:

**The one-readout label never wraps.** A second line there is what deleted the clock. It shrinks
instead, down to a floor of 14px — below the 19px that suits buttons, because in the narrowest
banner a long German period name needs to come down that far to stay on one line, and a small label
is a far better outcome than a missing clock.

**The two-readout label starts small.** A label wrapping to two lines *at full size* plus a time
exceeds the 110px, which is the original failure. Starting at `SMALL_TEXT` means two wrapped lines
(~49px) always fit beside a time capped at 46px (~60px).

**46px is derived, not chosen by eye.** 110px minus a two-line label at `SMALL_TEXT` leaves ~60.6px,
so 60.6 / 1.3 = 46px. Anything larger and a wrapped label pushes the time out.

**Sizes are shared, not fitted independently.** Both labels take one size and both times take
another, each the largest that fits *every* string in its group. Without this a short timeout label
(`SCH AUS`) renders at full size beside a shrunken period name and the two stop looking like a pair.

**Times are fitted, not pinned.** The two modes do not offer the same half-width, so a single fixed
size would render UWH at the size UWR needs.

## CJK line breaking

Found while verifying: the alarm button's hint was clipped at both ends in Japanese. The wording
shown when the clock is stopped is 13 full-width glyphs — wider than its button at `SMALL_TEXT`, and
`SMALL_TEXT` is also the floor, so it had nowhere to go.

The cause was general: `best_split` only broke at spaces, tabs and `/`, and **CJK has no spaces**, so
those scripts had no break opportunities at all and could only ever shrink. `best_split` now also
breaks **between CJK characters**, which is how those scripts wrap, with an abridged form of the
Japanese kinsoku rules so a line never begins with closing punctuation or a long-vowel mark.

This completes the wrap-first-shrink-second rule shipped in PR #2063 rather than changing it: Latin
still breaks only at spaces (pinned by a test), and every CJK label in the app can now wrap instead
of shrinking — so those locales should render *larger*, not differently.

The alternative was a per-call-site floor override for that one hint. Rejected on the human's point
that bespoke text sizes should not accumulate when a general rule will do.

## Scope

**In scope:**

| File | Change |
|---|---|
| `fit_text.rs` | `style` (colour), `no_wrap`, `min_size`, `shared_with`; CJK break opportunities |
| `shared_elements.rs` | Both banner layouts fit their text; `compact` deleted |
| `keypad_pages/mod.rs` | `make_panel_label` uses the label/value share pattern |
| `main_view.rs` | Nothing — the alarm hint fix needed no call-site change once CJK breaking existed |

**Not doing:**

- `make_delay_line` and the behind-schedule layout — **verified untouched**: zero lines of it appear
  in the diff, so it needs no rendered regression check.
- Translation text, the abbreviated period names, what the banner displays, when it shows a timeout
  column, its colours, or any timing behaviour.
- The portal login page, whose CANCEL and APPLY buttons collapse in German. Confirmed pre-existing
  against `master` and filed at `docs/backlog/portal-login-page-overflow/NOTE.md`. Auto-fit cannot
  fix it — those buttons are not too small for their text, they are given almost no height.

## Rejected designs

Recorded because each was rejected on evidence, and the reasoning is most of the value here.

| Design | Why rejected |
|---|---|
| Swap in `FitText` with colour support as step one | The human pointed out the elements and colours were already correct — this is a sizing problem. Sent us to check the cheaper fix first, which was right to do even though it did not survive. |
| Width-only change, no shrinking | The baseline killed it: 2.5px of headroom means a wider column makes a wrap *less likely* but nothing makes it *impossible*, and one wrap deletes the clock. |
| Stacked rows instead of columns | Proposed to match the behind-schedule layout. Rejected: a row makes label and clock compete for the same width, yielding a *smaller* clock (~38px vs ~50px). Columns let the label eat cheap vertical space. |
| Dynamic height fitting with a common scale factor | The height budget is already over-subscribed at full size and works only because of leading, so a strict height rule would "correct" the wide banner that renders fine today. Preventing the wrap is sufficient. |
| Widen the `compact` trigger to `(two tiles) \|\| (timeout)` | Covers everything observed, in one line. Rejected because the condition re-derives banner width from mode and tile presence, and three configurations had already been found by rendering — each one a clause somebody had to think to add. That is how the original bug was written. |
| A lower floor for the alarm hint | Fixed one symptom with a bespoke number. CJK breaking fixes the class. |

## Testing

21 unit tests in `fit_text.rs`, driven by a fake ruler — no window, no fonts, no renderer. The 17
from PR #2063 pass **unchanged** except for `size_ladder` gaining a floor argument; their assertions
are untouched, which is the guard on the merged button behaviour.

New: a lower floor reaches sizes the default cannot; Japanese splits between characters; a line never
begins with closing punctuation; Latin still breaks only at spaces.

**What tests cannot cover:** every defect in this branch was found by rendering, not by CI — the
missing clock, the mismatched label sizes, the clipped Japanese hint. Full-green CI coexisted with a
banner that silently dropped the clock.

## Verification — by rendering, against the captured baseline

Completed 2026-08-11:

- **UWH and UWR, German**, timeout and no timeout: clock present in every case, labels matching,
  times matching.
- **Spanish**: the `14:5` clipping resolved.
- **Thai**: shrinks cleanly with no wrapping available; no missing glyphs from the font subset.
- **Japanese and Chinese**: full-width glyphs handled; the alarm hint now wraps to two lines at full
  size rather than clipping.
- **Keypad title row**: label and value both complete.
- **Short banner without a timeout**, against the shot captured before the change.
- **Short banner with a timeout** — German, black team timeout, the warnings page. Both readouts
  complete: `ERSTE HALBZEIT 14:58` beside `AUSZEIT SCHWARZ 0:40`. This is the widest label pair the
  banner ever shows, because the short banner uses the *full* timeout wording rather than `SCH AUS`
  and does not abbreviate the period name. Labels settled on one shared size and times on another,
  with nothing clipped at either end. No baseline shot was captured for this configuration, so it was
  judged against the rule rather than against `master`.

**Reasoned, not rendered:** an overtime period name (`ZWEITE VERLÄNGERUNGSHÄLFTE`, 26 characters) on
the short banner during a timeout. It cannot reproduce the original failure, which needed the
*stacked* layout where a second label line pushes the clock down and out of the button. The short
banner lays the label *beside* the clock, so a wrap grows into vertical slack instead — about 13px of
the 73px inner height went unused in the rendered case — and the label shrinks, taking the timeout
label with it by the shared-size rule. Reaching it live needs a tied game played into overtime.

**Reproducing the UWR + portal tile configuration:** set `mode = "Rugby"` in
`~/.config/refbox/default-config.toml` **and** make `~/.config/refbox/portal_link.json` name the same
mode with a recent `last_active` — otherwise the link is parked as stale/cross-portal and the tile
never appears. Both were backed up as `*.bak-rugby-baseline` and restored afterwards.

## Risks

| Risk | Mitigation |
|---|---|
| The clock ends up smaller than today somewhere | Every baseline configuration was re-rendered and compared; the one that previously worked came out no smaller. |
| CJK breaking changes labels across the app | It only ever allows wrapping where previously only shrinking was possible, so CJK text should come out larger. Latin behaviour is pinned by a test. |
| A future translation defeats the 14px floor | It would clip rather than lose the clock — a visible, attributable failure rather than a silent one. |
| A new element on the banner reintroduces crowding | Sizing depends on the number of readouts, not on mode or tiles, so there is no condition to forget to update. |
