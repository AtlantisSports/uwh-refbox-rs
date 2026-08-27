# Bridge operator page: simplify, and drop side of pool

**Status:** approved in conversation 2026-08-27, not yet implemented.
**Scope:** `overlay-bridge` only. The refbox is not touched.

---

## 1. Why this exists

The bridge's status page is the only user interface the program has. Walking it on a live bridge
showed it carrying material that either serves nobody or actively misleads:

- an **Operator settings** section for two settings, one of which feeds nothing at all;
- a **connection line** longer than it needs to be;
- a **Current game** section naming only the event, game and period — none of the things an
  operator actually checks when confirming the bridge is on the right game.

This redesign removes the first, tightens the second, and expands the third.

It also resolves something the walkthrough exposed: the **side of pool** setting is the only place
the bridge transforms what the refbox sent, and it does so invisibly. That is being removed
outright, which makes this a change to the served contract and not merely a page redesign.

## 2. Scope boundary

**In scope:** `overlay-bridge`'s status page, its config and CLI flags, and the `/scorebug`
column set. Two bridge documents that describe the removed columns.

**Explicitly out of scope:**

- The refbox's own `white_on_right`. It is a different setting on a different program, governing
  the LED panel, and shares only a name. Several refbox documents mention it; none are touched.
- The `court` column in `/nextgame`. It comes from the portal's schedule, not from the operator
  setting of the same name, and it stays exactly as it is.
- Logos on the page. Wanted eventually; not part of this.
- Hover tooltips. Considered and dropped — see §7.
- The bridge's bind address. It binds every interface; whether it should bind only loopback is a
  reachability question and belongs on its own branch.

## 3. What is being removed, and what that costs

### 3.1 Court — free to remove

The operator's `court` setting reaches nothing. It flows CLI → `config::Resolved` → the status
page's own display, and stops. The `court` that vMix polls in `/nextgame` is resolved from the
portal's schedule (`portal.rs`), never from this setting.

So removing the display, the `court` field and the `--court` flag changes no served value. It
deletes a setting that has been stored, remembered and documented while doing nothing.

### 3.2 Side of pool — removing it changes what vMix receives

`white_on_right` decides whether `leftTeam`/`leftScore` carry the white or the black team
(`tables.rs:213`). It is the single place the bridge alters the meaning of what the refbox sent,
on the reasoning that the refbox cannot know which side of the pool the camera sees.

Removing it is a deliberate decision taken with that consequence understood. The side-of-pool
arrangement becomes the vMix title's business, which is where the camera angle is actually known.

**Nothing has shipped, so no venue depends on the current behaviour.**

### 3.3 The left/right columns go with it

With the setting gone, `leftTeam`/`leftScore`/`rightTeam`/`rightScore` would be permanent
duplicates of `whiteTeam`/`whiteScore`/`blackTeam`/`blackScore`.

They are removed rather than kept as aliases. A column named `leftTeam` that is really just the
white team is worse than no column: vMix matches Data Source columns **by name**, so the name is
the contract, and this one would promise a physical side the bridge no longer knows anything
about. The day a camera sees black on the left it would be silently wrong, with a name asserting
otherwise.

`/scorebug` therefore serves `whiteTeam`, `whiteScore`, `blackTeam`, `blackScore` and no
left/right pair. Every other column is unchanged.

### 3.4 Existing settings files still load

`config::Settings` does not use `serde(deny_unknown_fields)` — verified, not assumed. A settings
file written by an older bridge will still contain `court` and `white_on_right`; serde ignores
both. No migration, and no risk of resetting a saved refbox address.

## 4. The page

### 4.1 Heading

`Atlantis Sports Overlay Bridge`, in the `<h1>` and the browser tab title. The crate name, binary
name and `--help` text keep saying `overlay-bridge` — renaming those is packaging, not display.

Logos are wanted later. The heading should be built so one can sit beside it without rework.

### 4.2 Connection

The coloured dot stays. One label changes:

| Connection state | Now | Becomes |
|---|---|---|
| Connected | `Connected` | unchanged |
| Disconnected | `Disconnected` | unchanged |
| Never connected | `Never connected to a refbox yet` | `Never connected` |

The `Down for 2m 15s` line and the keepalive warning are unchanged. No test asserts the old
string.

### 4.3 Current game

Event name sits above the box; everything else inside it.

```
Event Name: Kings Cup 2026
┌─────────────────────────────────────┐
│  Game: 12                           │
│  Period: Second Half                │
│  Time: 4:23                         │
│  White Team: Sharks | 3             │
│  Black Team: Barracudas | 2         │
└─────────────────────────────────────┘
```

Labels are exactly as written above.

`Time` is the game clock in its display form — the same value `/scorebug` serves as `clock`
(`4:23`), not the raw `clockSeconds`. A running timeout is not shown here; the page reports the
game clock only, as the box's other rows all describe the game rather than an interruption to it.

**Every value must be renderable blank.** A disconnected bridge blanks all of them, and the box
must still look deliberate rather than broken — the same rule the rest of the page already
follows.

**The values must come from the same snapshot the served tables are built from.** The page must
never be able to show a score that vMix is not receiving. This is the one correctness constraint
in the redesign, and it is what the tests in §6 exist to hold.

### 4.4 Removed section

**Operator settings** goes entirely. The sentence beneath it listing command-line flags is
rewritten so it stops advertising `--white-on-right` and `--court`, both of which will no longer
exist. The settings-file path line stays — it is still where a mistyped port is fixed.

## 5. Files

| File | Change |
|---|---|
| `status.rs` | The page: heading, connection label, new game section, section removed. `PageData` gains team/score/clock fields, loses `white_on_right` and `court`. |
| `server.rs` | Populates the new `PageData` fields from the live snapshot; drops `white_on_right` from `AppState`. |
| `tables.rs` | `/scorebug` loses the four left/right columns and the `white_on_right` parameter. |
| `config.rs` | `court` and `white_on_right` removed from `Overrides`, `Settings`, `Resolved`. |
| `main.rs` | `--court` and `--white-on-right` flags removed. |

Plus tests in each.

## 6. Testing

- **The page cannot disagree with the tables.** A test that drives a snapshot through both and
  asserts the page's team names and scores match `/scorebug`'s. This is the constraint from §4.3
  and the one worth a dedicated test.
- **Blank rendering.** The game box renders completely with a default/disconnected snapshot.
- **`/scorebug` column set.** Asserts the four left/right columns are absent and the white/black
  ones present — a column set is a contract, so it is asserted explicitly rather than implied.
- **Flags are gone.** `--court` and `--white-on-right` are rejected by the CLI parser.
- **Old settings files still load.** A settings file containing `court` and `white_on_right`
  deserializes without error and preserves the refbox address.
- Existing status-page and table tests updated for the removals.

## 7. Rejected

~~**Hover tooltips.**~~ **REVERSED 2026-08-27, after seeing the rebuilt page.** They were
dropped on the argument that hiding help behind hover costs the first-time volunteer, who does not
know there is anything to hover over. Eric asked for them anyway once the page was in front of
him, which is the right way round: the recommendation was made from a description, the decision
from the thing itself.

Both long explanations now hang off a `(?)` marker:

- beside the **Search the network** button — what searching does, that it takes a few seconds,
  and the Windows firewall prompt;
- beside the **Refbox connection** heading — the command-line flags, that settings are remembered,
  and the settings file's path.

Implemented as `title` attributes on a marked-up `(?)`: no JavaScript, and the page stays a single
self-contained HTML document. The settings path stays interpolated per machine.

**The known cost stands:** the Windows firewall warning is now behind hover, and that prompt is
the most likely thing to derail a first-time volunteer. If venue reports point at it, the fix is
to bring that one sentence back onto the page rather than to remove the tooltips.

**Keeping left/right as aliases.** See §3.3.

**A scoreboard-style block above the detail rows.** Considered and dropped once the event name
moved outside the box: the box already carries both teams and both scores, so a scoreboard two
lines above would repeat all of it.

## 8. Acceptance

Observable on a running bridge:

1. The page is headed `Atlantis Sports Overlay Bridge`.
2. With no refbox, the dot is red and reads `Never connected`.
3. The game box shows the six labelled rows, blank, and looks deliberate.
4. Connected to a refbox, the box fills in and its teams and scores match what `/scorebug`
   serves at that moment.
5. There is no Operator settings section, and the flag sentence names only flags that exist.
6. `/scorebug` has no `leftTeam`, `leftScore`, `rightTeam` or `rightScore`.
7. `--court` and `--white-on-right` are rejected.
8. A bridge that had saved settings before still starts on its remembered refbox.

## 9. Documents to update

- `2026-08-26-vmix-integration-steps.md` — six references to the left/right columns and to
  `white_on_right`, including a worked JSON example.
- `2026-08-26-vmix-overlay-bridge-design.md` — §5.2's "two settings the feed cannot supply"
  premise no longer holds; both settings are gone.

Refbox documents mentioning `white_on_right` are a different setting and are not touched.
