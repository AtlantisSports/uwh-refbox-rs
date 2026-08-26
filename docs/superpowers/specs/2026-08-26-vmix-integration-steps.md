# vMix integration — mock walkthrough

**Date:** 2026-08-26
**Companion to:** `2026-08-26-vmix-overlay-bridge-design.md`
**Status:** Written from vMix's own documentation and the installed files, **before vMix has been
opened**. Every step is labelled with how it is known. The live run at the end of phase 1 either
confirms this document or corrects it; it then becomes the setup guide shipped to operators and
third parties.

**Version this was written against:** vMix 28.0.0.42 (the installation on this machine).

---

## Legend

- **[verified]** — confirmed from vMix's own documentation or the installed program files.
- **[assumed]** — a reasonable inference not yet confirmed. Flagged so the live run can check it.

---

## What the bridge serves

The bridge exposes tables at addresses like `http://localhost:8099/scorebug`. Each is
**a JSON array of objects, one object per row** — the structure vMix requires. **[verified:
vMix "supports JSON data that is stored as an object array"; each element becomes a row.]**

`/scorebug` — always exactly one row:

```json
[
  {
    "blackTeam": "AUSTRALIA",
    "blackScore": "3",
    "whiteTeam": "CANADA",
    "whiteScore": "2",
    "clock": "3:47",
    "clockSeconds": "227",
    "period": "Second Half",
    "timeout": "",
    "timeoutClock": ""
  }
]
```

`/penalties` — **always exactly six rows**, blank-padded (see "Gotchas", G2):

```json
[
  {"team": "BLACK", "number": "7",  "player": "SMITH",  "time": "1:42", "infraction": "Hold"},
  {"team": "WHITE", "number": "3",  "player": "NGUYEN", "time": "TD",   "infraction": "Strike"},
  {"team": "",      "number": "",   "player": "",       "time": "",     "infraction": ""},
  {"team": "",      "number": "",   "player": "",       "time": "",     "infraction": ""},
  {"team": "",      "number": "",   "player": "",       "time": "",     "infraction": ""},
  {"team": "",      "number": "",   "player": "",       "time": "",     "infraction": ""}
]
```

`/fouls`, `/warnings` and `/nextgame` follow the same pattern.

**Every value is a string,** including numbers. **[assumed]** — vMix maps columns into text
fields, so strings avoid any question of how it renders a number. The live run should confirm
whether native numbers work too; if they do, this is a preference, not a requirement.

---

## Setting it up in vMix

### 1. Add the data source

**[verified]** Open the **Data Sources Manager** from the menu in the bottom right of the main
vMix window, click **Add**, and type the web address:

```
http://localhost:8099/scorebug
```

**[verified]** The setup screen offers exactly three settings:

| Setting | What to do | Why |
|---|---|---|
| **Name** | `Scorebug` | How it appears when mapping titles |
| **Use first row as column names** | **Leave unticked** | Our JSON already names its columns; this option is for spreadsheet-style sources |
| **Convert rows to columns** | **Leave unticked** | vMix's own documentation advises against it for performance; we do not need it because our tables have fixed rows |

Repeat for `/penalties`, `/fouls`, `/warnings` and `/nextgame`, naming each one.

### 2. Build the title

**[assumed]** Create or open a Title input — the actual design work is phase 2 of the plan, and
this walkthrough only covers wiring it up.

### 3. Map each title field

**[verified]** Right-click the Title input, choose **Title Editor**. For each field:

1. Select the field in the left-hand column.
2. Click **Data Source** in the top menu.
3. Set **Table** — usually the only one available for a JSON source.
4. Set **Column** — see G1 below.
5. Set **Row** — see G2 below. **Do not leave this on "Selected".**
6. Check the preview, then **OK**.

**[verified]** There is also a **Format** setting that can wrap the value, e.g. `Cap {0}`.

Suggested mapping for a basic scorebug:

| Title field | Data source | Column | Row |
|---|---|---|---|
| Black team name | Scorebug | `blackTeam` | 1 |
| Black score | Scorebug | `blackScore` | 1 |
| White team name | Scorebug | `whiteTeam` | 1 |
| White score | Scorebug | `whiteScore` | 1 |
| Clock | Scorebug | `clock` | 1 |
| Period | Scorebug | `period` | 1 |
| Penalty 1 player | Penalties | `player` | 1 |
| Penalty 1 time | Penalties | `time` | 1 |
| Penalty 2 player | Penalties | `player` | 2 |
| Penalty 2 time | Penalties | `time` | 2 |

---

## Gotchas found while writing this

### G1 — "Column: Auto" matches by name, then by position

**[verified]** With Column set to **Auto**, vMix first looks for a column whose name matches the
title field's name exactly; failing that, it falls back to matching by position — field 1 to
column 1, and so on.

**Consequence for us:** column names must be stable and predictable, because operators will bind
to them by name, and third parties will too. Once published they are part of the contract and
renaming one silently breaks every title built against it.

**Consequence for the operator:** naming title fields to match our column names exactly makes
mapping automatic. Otherwise set Column explicitly rather than trusting positional fallback.

### G2 — "Row: Selected" is a trap, and it forces fixed-size tables

**[verified]** If Row is left on **Selected**, the field follows whichever row is currently
highlighted in the Data Sources Manager.

**Consequence for the operator:** a scorebug that changes depending on what was clicked last.
Always bind to an explicit row number.

**Consequence for the bridge:** if titles bind to fixed row numbers, then a table whose length
varies leaves a title bound to row 3 reading whatever was there before, or nothing at all, when
only two penalties are active. **So the penalty, foul and warning tables are always served at a
fixed length, padded with blank rows.** Six penalty rows is proposed; the real ceiling should be
checked against what the game can produce before it is fixed.

### G3 — The refresh interval is not publicly documented

**[verified]** vMix's data-source layer exposes an interval in **milliseconds**
(`get_TimerMilliseconds` / `set_TimerMilliseconds` in `DataSourceAPI.dll`), so sub-second
refreshes are supported by design. But none of the published Data Sources pages state where that
interval is set or what the minimum is.

**This is the one open question in the vMix integration.** It matters only for the clock — a
scorebug clock refreshing once per second is acceptable, once every five seconds is not.

Two fallbacks exist if the interval turns out too slow, neither of which changes the bridge's
core:

- **Push instead of poll.** vMix has its own web API for setting title text directly, so the
  bridge could push the clock as it changes. This would tie that path to vMix specifically,
  so it would be an addition to the generic tables, never a replacement.
- **Let vMix run the clock.** vMix titles can count down on their own. The bridge would supply
  the target rather than the current value. More moving parts, and it would need care around
  pauses and manual time edits, so this is the second choice.

Resolved at the live run, end of phase 1.

---

## What the live run must confirm

1. A JSON array of objects served over `http://` is accepted, and each object becomes a row.
2. Column names appear as columns, and Auto matching finds them by name.
3. A title bound to an explicit row updates on its own as the served data changes.
4. The refresh interval, where it is set, and its practical minimum (G3).
5. Whether values must be strings or whether native JSON numbers also work.
6. That blank-padded rows render as empty rather than showing an error or stale text.
