# Backlog: the portal login page overflows and eats its own buttons

**Status:** Diagnosed, not fixed. Pre-existing — **not** introduced by the auto-fit work.
**Surfaced:** 2026-08-11, while render-checking `feat/refbox/fit-clock-and-keypad-title`.
**Confirmed against:** `master` @ `d09ff701`, built and rendered side by side with the branch. The
defect is identical in both, so it predates PR #2063 as well.

## What the operator sees

UWR or UWH, **German**, game parameters → `UWHPORTAL VERWENDEN` → the login/token page:

- `ABBRECHEN` (cancel) and `FERTIG` (done) collapse to **thin red and green stripes with no text**.
  They are still there and still pressable, but unreadable and only a few pixels tall.
- The instruction text is also **cut off mid-sentence** — it ends at "*Drücken Sie Fertig, sobald
  Sie den Code*" with the rest missing.

Both symptoms have the same cause, and the truncated text is the more important clue: the page
simply has more content than height.

## Mechanism

`refbox/src/app/view_builders/keypad_pages/portal_login.rs:13-38`:

```rust
column![
    text(fl!("portal-login-instructions", ...)).width(Length::Fill),
    vertical_space(),
    row![ make_button(cancel), make_button(done) ].spacing(SPACING),
]
```

There is **no scrolling anywhere on this page**. iced's flex lays out non-`Fill` children first,
giving each what it asks for, and hands the remainder to the `Fill` ones. The instruction text is
laid out first at its full natural height; in German that consumes the column. The button row's
children have a fixed 89px height, but `Limits::resolve` clamps to the space that is left — which by
then is nearly zero — so they render as stripes.

English fits, which is why this has survived.

## Why auto-fit does not fix it

Worth stating plainly so nobody assumes the fitting work covers it: shrinking the **button labels**
would change nothing. The buttons are not too small for their text; they have been given almost no
height. The text is what needs to yield.

## Sketch of the options

Not decided — needs its own design pass:

- Make the **instructions** the flexible part: put them in a `scrollable`, so the page can never
  push its own controls out. Probably the most robust, and matches "the buttons must always be
  reachable".
- Or reserve the button row's height **before** the text is laid out, so the text gets the remainder
  and is clipped or scrolled instead.
- Or shrink the instruction text to fit, which keeps everything visible but may end up unreadably
  small in the longest locales — the instructions are a paragraph, not a label.
- Shortening the German string is a workaround, not a fix: the next long translation reintroduces it.

## Repro

```
mode = "Rugby"   # or "Hockey6V6"; both affected
WAYLAND_DISPLAY= UWH_PORTAL_URL_OVERRIDE=http://localhost:8090 \
  ./target/debug/refbox --allow-http --language de-DE
```

Game parameters → `UWHPORTAL VERWENDEN` → the token page.

Related: `../auto-fit-button-text/` (merged, PR #2063) fixed label fitting inside buttons; this is
the complementary case where a *container* denies its children the space they were promised.
