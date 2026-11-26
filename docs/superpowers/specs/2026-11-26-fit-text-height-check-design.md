# Fitted text checks height as well as width — design

Status: approved by the PO 2026-11-26 (option 1 of 3). Last item v0.5.5 waits on.

## Why

On the Settings page (portal mode off) the TEAM TIMEOUTS value "1/HALF" is drawn as "1/"
over "HALF" at the large value size, and both lines are cut off top and bottom (PO screenshot
2026-11-16, `docs/backlog/team-timeouts-value-clipped/`). The cause is in `fit_text`, the
widget every button uses to fit its text: when a line is too wide it may split it in two
(after a space, a `/`, or between CJK characters), and it accepts the split when each line is
narrow enough. It never asks whether two lines fit the button's **height**.

## Goal

A split onto two lines is only accepted when both lines' letters fit inside the box. If they
do not, the size steps down, exactly as it already does for width, so the text ends on one
line (usually) or on two smaller lines that do fit.

## Non-goals

- No wording change ("1/HALF" stays), no new translation strings.
- No layout change to any page, no change to the text sizes themselves.
- A single line is never rejected for height. That is today's behaviour and stays, so this
  change can only alter text drawn on two or more lines: split by `fit_text`, or broken by
  its caller (`fit_text_lines`) or by a translation carrying its own line break. A caller's
  or translator's lines cannot be re-joined, so when they are too tall even at 19 they
  stay as they are.

## Anti-goals

- Two-line labels that read fine today (BLACK / TIMEOUT, OVERTIME / ALLOWED:) must not
  change. The code already notes that two lines at the default size are "a couple of pixels
  taller than the inside of a button". That overflow is blank line spacing, not letters, so
  the check must measure letters and not line spacing.

## The rule

Two or more lines need `(lines - 1) × line height + text size` of height. Line height is
iced's default (1.3 × size). The final `+ text size` is the em box of the last line, which
holds Roboto Medium's capitals and the descent of `/`. Measured offline with fontTools:
capital top 0.281 × size below the top of its line box, `/` bottom 1.053 × size. So this
amount, centred as `fit_text` centres it, keeps those marks inside the box.

Worked against the PO's screenshot (Windows, 125% scaling): the tile's inside is about 64
high. "1/" over "HALF" at 38 needs 87, so it is rejected, and at 38 the letters would need
about 80 of the 64. BLACK / TIMEOUT at 29 needs 67 of about 73, so it is unchanged.

When nothing fits even at the smallest size (19), the most-lines arrangement is kept if it
fits the height there; otherwise the text is drawn on **one** line at 19, trimmed at its
edges, never two lines spilling out top and bottom (PO-approved).

## Approaches

1. **Height check in the shared fitting rule (chosen).** Fixes this tile and any other split
   that does not fit, without having to find them first. It touches every button, which the
   anti-goal and the walk below guard.
2. Turn off splitting for the TEAM TIMEOUTS value only. Smaller, but it leaves the rule
   broken everywhere else, and German "1/HALBZEIT" could never use two lines even when they
   fit.
3. "PER HALF" small over a large "1", like SUDDEN DEATH MAX. New strings in 16 languages, a
   different style from the Main and Game Info pages, and the slowest path to v0.5.5.

## Verification

- Unit tests on the pure decision functions, including one that fails on today's code.
- "1/<half>" and "1/<game>" in all 16 languages: replay the ladder offline with fontTools
  against the tile's real box size.
- A debug log line wherever the height check changes what is drawn. The PO walks with it
  on, so any other place this touches shows up by name.

## Left to the builder

The exact names of the helpers, and how the decision is threaded through the shared-string
paths (`shared_start`, `shared_width`), so that related readouts and reserved widths agree
with what is drawn.
