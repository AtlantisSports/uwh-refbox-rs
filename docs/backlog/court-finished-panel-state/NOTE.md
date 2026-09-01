# Backlog: tell the LED panel and the overlay that a court's schedule is finished

**Status:** Design idea, not started, not on any branch. Raised by Eric 2026-08-16 during the
walkthrough of `fix/uwh-common/no-next-game-on-court`.
**Type:** Cross-cutting — `uwh-common` (shared types + wire format), `matrix-drawing`,
`led-panel` (FPGA firmware), `overlay`, `refbox`.

## The problem

When the selected court has no further games, the refbox now says so plainly: the clock stops
dead, the banner reads `END` over `--:--`, and the game info table dashes out.

**The LED panel does not, and cannot.** It keeps showing `NEXT GAME IN` with a time. The panel's
message format has no way to express "there is no next game" — it carries only the period, the
seconds, the scores and the penalties (`uwh-common/src/wire_format.md`). The refbox conveys the
finished state with a *blank next-game number*, and there is no game number in the panel message
at all.

So poolside, the scoreboard still promises a game that is not coming. The design spec
(`docs/superpowers/specs/2026-08-05-no-next-game-on-court-design.md`) scoped the panel out on
exactly this ground; this note is that decision coming back.

Eric's proposal: **introduce an explicit game state** (working name `TournamentEnd`) rather than
having each surface infer the situation. Today the refbox infers it from a blank number and the
panel cannot infer it at all. One explicit state would let every surface read the same fact.

## Verified facts (checked in code 2026-08-16 — do not re-derive)

**1. The panel wire value is explicit, not positional.**
`GamePeriod::encode()` (`uwh-common/src/game_snapshot.rs:524`) is a hand-written match:
`BetweenGames → 0` … `SuddenDeath → 9`. A new state would be `→ 10`. **Adding a variant cannot
silently shift the existing wire values.** (This was the failure mode originally feared; it does
not apply.)

**2. An un-updated physical panel degrades gracefully.**
The panel is an FPGA. Its period decoder (`led-panel/rtl/segments.sv:131`) has a `default:` branch
whose five assignments are **identical** to the `4'd0 // Between Games` branch. So a panel that
has not been re-flashed and receives state `10`:

- shows its period indicators exactly as it does for between-games today
- decodes clock, scores and penalties normally — they are separate fields
- does not crash, freeze, garble, or mislabel another period

**Consequence for planning: refbox can ship first and panels can be flashed lazily.** Not
updating a panel costs nothing relative to today's behaviour; it simply does not gain the
improvement. This removes the lockstep-release risk.

**3. The real ceiling is 15 states, not 31 — the doc and the hardware disagree.**
`wire_format.md` says the period occupies bits 4:0 (5 bits, 31 values). The RTL actually reads
`data[1][3:0]` — **4 bits, 15 values** (`led-panel/rtl/segments.sv`, the `case` selector). Harmless
at 10 states, with 6 spare. Past 15, an old panel would silently alias a new state onto an existing
one. **Fix the document or widen the RTL before anyone relies on the larger number.**

**4. Rust surfaces the change at compile time; only the FPGA is silent.**
`refbox`, `matrix-drawing`, `led-panel-sim` and `overlay` all match on `GamePeriod` exhaustively,
so adding a variant is a **compile error** until every Rust surface decides what `END` looks like.
That is the good kind of failure.

**5. TRAP: `GamePeriod::decode()` will NOT produce a compile error.**
`decode()` (`uwh-common/src/game_snapshot.rs:538`) matches on a `u8` and ends in
`_ => Err(DecodingError::InvalidGamePeriod(val))`. Adding a variant compiles fine without adding
`10 => Ok(...)`. And `GameSnapshotNoHeap::decode` propagates that error with `?`
(`game_snapshot.rs:610`), so the **entire 19-byte snapshot is rejected**, not just the period.
The refbox's own panel simulator (`refbox --is-simulator`, via `refbox/src/sim_frame.rs:87`) is a
decode consumer, so forgetting this arm would make the simulator stop updating while the real
panel carried on working. **Add the `decode` arm in the same commit as the `encode` arm.**

## Options to weigh (not yet decided)

| | Approach | Cost | Notes |
|---|---|---|---|
| A | New `GamePeriod` variant (Eric's proposal) | Touches every Rust match + FPGA | Cleanest semantics; one fact, read everywhere. Old panels degrade to between-games. |
| B | Additive flag in the panel message | Wire format change, no enum change | The 19-byte layout has spare bits (period uses 4 of 8 in byte 0; bits 5–6 are free). Less invasive, but the state stays implicit. |
| C | Sentinel time value (e.g. `--:--` encoded as a reserved seconds value) | Smallest change | A magic number; the panel would need to know the sentinel anyway, so it is option B with worse ergonomics. |

Option A is the only one that also cleans up the refbox side, where "finished" is currently
inferred from a blank string in several places.

## Not yet checked

- **The reverse mismatch:** a NEW panel or overlay against an OLD refbox. Fact 5 covers the Rust
  decode path; the FPGA side of that direction has not been traced.
- Whether `GamePeriod`'s derived `PartialOrd`/`Ord` (declaration order) is relied on anywhere.
  Appending the variant last is safest regardless.
- Whether the overlay needs its own treatment, or inherits it from `next_game_number()` already
  answering `None` for a blank number.

## Separate but adjacent

A second panel defect surfaced in the same walkthrough and is **not explained**: with the court
finished and the engine's clock provably stopped at zero, the panel simulator displayed a frozen
`NEXT GAME IN 0:30`. The engine value is definitively `0` (`ClockState::Stopped { ZERO }` →
`clock_time()` → `Some(0)`), and the panel draws nothing but `secs_in_period`
(`matrix-drawing/src/drawing.rs:88,184`). Only one refbox and one simulator process were running,
so it is not a stale instance. **Something between the engine and the panel substitutes a value.**
This needs a probe on the wire, not another theory — two attempts to explain it from code reading
were both wrong. It may well be fixed incidentally by any of the options above, but it should be
understood first, in case it indicates a snapshot-delivery bug that affects more than this state.

## Related

- Branch `fix/uwh-common/no-next-game-on-court` — the refbox-side feature this completes.
- `docs/backlog/beep-test-panel-firmware/NOTE.md` — the other outstanding panel-firmware task.
  NB: that note says the `led-panel` firmware is "external to this repo". That is misleading —
  the FPGA source **is** in this repo at `led-panel/` (Verilog/SystemVerilog under `rtl/`,
  built with fusesoc). It is simply not a Cargo crate.
