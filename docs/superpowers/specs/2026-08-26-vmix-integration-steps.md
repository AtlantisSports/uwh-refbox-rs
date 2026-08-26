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

**Status update (end of phase 1, Task 5):** the shapes below are what was actually built and
tested (`overlay-bridge/src/tables.rs`), not the sketch this document started as. Three
differences from the original sketch, made deliberately during implementation and recorded here:

- `/scorebug` gained `leftTeam`/`leftScore`/`rightTeam`/`rightScore` alongside the `black*`/
  `white*` columns, and `blackFouls`/`whiteFouls`/`blackWarnings`/`whiteWarnings`. See its example
  below for why.
- `/penalties`, `/fouls` and `/warnings` gained a `timeSeconds` column next to `time` (penalties
  only — fouls and warnings have no duration to report) — every duration-shaped value is served
  both display-ready and as plain seconds, and the original example only showed the display-ready
  half.
- `/fouls` and `/warnings` are no longer fixed at six (or any fixed number) — see the rewritten
  G2 below. This is the one shape change that isn't purely additive, and it matters to anyone
  binding a title to a row past 10.

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
    "timeoutClock": "",
    "timeoutClockSeconds": "",
    "leftTeam": "CANADA",
    "leftScore": "2",
    "rightTeam": "AUSTRALIA",
    "rightScore": "3",
    "blackFouls": "4",
    "whiteFouls": "2",
    "blackWarnings": "1",
    "whiteWarnings": "0"
  }
]
```

`blackTeam`/`whiteTeam`/`blackScore`/`whiteScore` never change meaning — a team's kit colour is
fixed for the whole game. `leftTeam`/`leftScore`/`rightTeam`/`rightScore` carry the *same* two
teams, reordered by the operator's side-of-pool setting (whichever physical side the camera has
white on) — the example above has white on the left, i.e. `white_on_right = false`. Bind to
black/white if a title only ever needs to say "the black team"; bind to left/right if the title's
physical position on screen needs to match the physical side of the pool, however the venue is
set up. `blackFouls`/`whiteFouls`/`blackWarnings`/`whiteWarnings` are the *true* total recorded
for that team — independent of how many rows `/fouls` and `/warnings` themselves carry (see those
tables' entries below), so a title can show a running count even when the row-carrying table has
been truncated.

`/penalties` — **always exactly ten rows**, blank-padded (see "Gotchas", G2):

```json
[
  {"team": "BLACK", "number": "7",  "player": "SMITH",  "time": "1:42", "timeSeconds": "102", "infraction": "Stick Foul"},
  {"team": "WHITE", "number": "3",  "player": "NGUYEN", "time": "TD",   "timeSeconds": "",    "infraction": "Free Arm"},
  {"team": "",      "number": "",   "player": "",       "time": "",     "timeSeconds": "",    "infraction": ""},
  {"team": "",      "number": "",   "player": "",       "time": "",     "timeSeconds": "",    "infraction": ""},
  {"team": "",      "number": "",   "player": "",       "time": "",     "timeSeconds": "",    "infraction": ""},
  {"team": "",      "number": "",   "player": "",       "time": "",     "timeSeconds": "",    "infraction": ""},
  {"team": "",      "number": "",   "player": "",       "time": "",     "timeSeconds": "",    "infraction": ""},
  {"team": "",      "number": "",   "player": "",       "time": "",     "timeSeconds": "",    "infraction": ""},
  {"team": "",      "number": "",   "player": "",       "time": "",     "timeSeconds": "",    "infraction": ""},
  {"team": "",      "number": "",   "player": "",       "time": "",     "timeSeconds": "",    "infraction": ""}
]
```

Ordered total dismissals first, then by longest remaining time — the same order the overlay's own
penalty flags already use. A total dismissal's `timeSeconds` is empty, not `"0"`: it has no
countdown, and `"0"` would read as "about to expire" instead. Penalties are culled by the refbox
the moment they finish being served, so ten is generous headroom, not a realistic ceiling, and the
table never grows past it.

`/fouls` — **at least ten rows, growing as needed, see G2**:

```json
[
  {"team": "WHITE", "number": "3", "player": "NGUYEN", "infraction": "Obstruction"},
  {"team": "BLACK", "number": "7", "player": "SMITH",  "infraction": "Out Of Bounds"},
  {"team": "EQUAL", "number": "",  "player": "",       "infraction": "Delay Of Game"},
  {"team": "",      "number": "",  "player": "",       "infraction": ""}
]
```

(shown here at 4 rows for brevity; the real table is never fewer than 10). Carries all three of
the refbox's foul buckets — `BLACK`, `WHITE`, and `EQUAL` (the "both teams at fault" case, which
the bridge does not drop). Ordered most-recent-first: row 1 is (an approximation of, see below)
the most recently committed foul, and the newest are what survive if the table has grown past 100
and needs to shed the oldest.

`/warnings` — same shape as `/fouls` minus the `EQUAL` case (warnings have no both-at-fault
bucket): `team`, `number`, `player`, `infraction`, same row-count rule.

`/nextgame` — always exactly one row:

```json
[
  {
    "blackTeam": "SYDNEY KINGS A",
    "whiteTeam": "BRISBANE A",
    "court": "2",
    "startTime": "09:30"
  }
]
```

`court` and `startTime` are bare values — never `"COURT: 2"` or `"START: 09:30"` — so an
operator's own Format setting (e.g. `Court {0}`) can add a label without also seeing the bridge's.
`startTime` is rendered from the portal's raw ISO 8601 timestamp using the offset the timestamp
itself carries. If nothing is known yet about the next game — or a game genuinely has no court or
start time assigned — every column in this row is an empty string, never `"None"` or a
placeholder.

**Every value is a string,** including numbers. **[assumed, but no longer just assumed for us]** —
the bridge always serves strings; whether vMix would also accept native JSON numbers is still an
open question for the live run, but is a preference at most, not something the bridge needs to
revisit.

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
only two penalties are active. **So `/penalties` is always served at a fixed length (ten rows),
padded with blank rows.**

`/fouls` and `/warnings` turned out to need a different rule, decided once the real ceiling was
checked against what a game can produce (as this gotcha originally asked for): penalties are
culled by the refbox the moment they finish being served, so their active set is naturally bounded
and ten is generous headroom, not a realistic ceiling. Fouls and warnings are **never** culled —
they accumulate for the whole game — and more than ten in a single half is not rare. A fixed table
sized to always hold every possible entry would mean sending mostly-empty rows on every poll of
every quiet game, since a title realistically binds within the first few rows and never to, say,
row 87.

**So `/fouls` and `/warnings` have a variable row count, and this is part of the published
contract, not an implementation detail:**

- Fewer than 10 entries → padded with blank rows up to exactly 10.
- 10 to 100 entries → exactly one row per entry, no padding.
- More than 100 entries → exactly 100 rows: the 100 newest are kept, the oldest are dropped.

**Consequence for the operator:** bind a title within the first ten rows and it will always find a
row there, blank or not, no matter how the game has gone. Binding past row 10 is only safe if the
title is meant to disappear once a game is quiet enough not to need it — a row past 10 exists only
if there is an entry for it.

Both tables are also ordered most-recent-first (row 1 is the latest foul or warning, not the
first one of the game), so that truncation at 100 always drops the oldest entries, never the
newest. Within one team's own list this is exact. **Across the two (or, for fouls, three) teams'
lists it is only ever an approximation** — nothing in the refbox's feed timestamps an individual
foul or warning, so there is no way to know for certain whether a given black foul happened before
or after a given white one. The bridge resolves this with a round-robin merge (newest of team A,
newest of team B, then each team's next-newest, and so on) rather than concatenating one team's
list before the other's, specifically so that both teams' recent activity surfaces within the
first few rows instead of one team's larger tally crowding out the other team's more recent
entries. It is a deliberate best effort, not a claim of exact chronological order.

Per-team totals are still available even where the row-carrying table has been truncated: see
`/scorebug`'s `blackFouls`/`whiteFouls`/`blackWarnings`/`whiteWarnings` above, which always report
the true count.

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
