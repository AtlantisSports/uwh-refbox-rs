# Portal follow-up: game-info table fields

The refbox Game Information page (and the Main UI page's compact panel) now render game
information as a table. Two rows are wired in the refbox but stay **dormant** until the UWH
Portal sends the data for them. No further refbox change is needed for item (1); item (2) is an
optional future enhancement.

## 1. Time/Score Helper (a second time/score keeper)

**Status:** the refbox already handles this — the row appears automatically as soon as the Portal
sends the assignment. Nothing else on the refbox side is required.

The refbox reads referee assignments from each game's `refereeAssignments` array and matches by
`role` string. It already recognises these roles:

| `role` value             | Row shown on the table        |
|--------------------------|-------------------------------|
| `Chief`                  | Chief Referee                 |
| `TimeOrScoreKeeper`      | Time/Score Keeper             |
| `TimeOrScoreKeeperHelper`| **Time/Score Helper** (new)   |
| `Water1`                 | Water Referee 1               |
| `Water2`                 | Water Referee 2               |
| `Water3`                 | Water Referee 3               |

To populate the Time/Score Helper row, include an assignment with `role: "TimeOrScoreKeeperHelper"`
in the game's `refereeAssignments`, in the same shape as the existing roles, e.g.:

```json
{ "role": "TimeOrScoreKeeperHelper", "userId": "<user-id>" }
```

The Time/Score Helper row is shown **only when this assignment is present** — not every event has
this role, so when it is absent the row is simply omitted (it is not shown blank). The other five
referee rows always show, using `-` for an unassigned slot.

## 2. Game Type (optional future enhancement)

**Status:** not implemented in the refbox, and not currently shown anywhere. This is a note for a
possible future addition.

There is no "game type" field in the data the refbox receives today. The game record carries only
team names, start time, court, timing rule, referee assignments, and an optional free-text
description; the only grouping concept is Division vs. Pod, which is not the same thing.

If we later want the table to show a game type, the Portal would need to send a game-level value
with one of these five categories:

- Round Robin
- Crossover
- Playoff
- Final
- Medal Game

Once the Portal sends it (and the refbox adds the corresponding field + row), the table would show
the value, falling back to "Unknown" when it is absent — mirroring how "Stop Clock in Last 2 Min"
already behaves. This is out of scope for the current work.
