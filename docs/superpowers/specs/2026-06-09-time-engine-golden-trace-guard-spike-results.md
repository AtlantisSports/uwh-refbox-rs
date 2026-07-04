# Phase 0 Spike — Results & Two Findings (for your feedback)

**Status:** The gating spike is built, run, and PASSED. It surfaced two findings; one of them **overturns a model decision you and I both signed off on**. I want your read on the revised model before we build the real guard.

Self-contained: relevant facts are restated inline. `file:line` references are my attestations from the repo.

---

## 0. What I'm asking you

1. Finding #1 invalidates the "drive the engine at its own `next_update_time` instants" model we agreed on. I replaced it with a **fixed-step** driver. Attack that: is fixed-step faithful, or did I trade away something important to escape the infinite loop?
2. Finding #2 is a normalization question (how to key trace lines). Tell me if my proposed fix is right or if there's a sharper option.
3. Anything else in the result that looks wrong or too good.

---

## 1. Recap: what the spike was supposed to prove

Earlier we designed a golden-trace differential guard for the refbox time engine (`TournamentManager`), pinned against the last human-authored baseline `46ec0973`. Before building it, we agreed a **Phase 0 spike** had to prove two things, or the whole approach was reconsidered:

1. The engine can be driven through a scenario crossing a period boundary AND expiring a penalty, faithfully mirroring how the real app drives it.
2. The observed time-state is **deterministic** across runs, despite the engine's internal `Instant::now()` calls (`tournament_manager/mod.rs:65`, `:898`).

The agreed driving model (which you verified) was: a discrete-event loop advancing virtual time to the engine's own `next_update_time(now)` instants, calling the real tick block (`could_end_game`/`pause_has_ended`/`update` → `generate_snapshot` with `None`-retry → reschedule).

Scenario used: config half=40s, half-time=10s; set FirstHalf + start clock at t=0; 30-second penalty on Black #7 at t=2; stop clock at t=15, restart at t=18; run to t=55.

---

## 2. Result: PASSED

All three checks pass and the trace is hand-verified correct. Deduplicated trace (one line per observed-state change):

```
t=  0 | FirstHalf  | clock=40s | pens=[]
t=  2 | FirstHalf  | clock=38s | pens=[B#7:30]      <- penalty starts
 ...  (counts down 1/s) ...
t= 14 | FirstHalf  | clock=25s | pens=[B#7:17]
t= 18 | FirstHalf  | clock=24s | pens=[B#7:16]      <- clock+penalty FROZE across the 15-18s stop
 ...
t= 34 | FirstHalf  | clock= 8s | pens=[B#7:0]       <- penalty expires
t= 42 | FirstHalf  | clock= 0s
t= 43 | HalfTime    | clock=10s                      <- period crossing #1
t= 52 | HalfTime    | clock= 0s
t= 53 | SecondHalf | clock=40s                       <- period crossing #2
```

- **Determinism (criterion 2):** two independent runs produced byte-identical traces (`assert_eq!` passed). The constructor's internal `Instant::now()` does NOT leak into observed state, because `set_period_and_game_clock_time` + `start_clock(base)` re-anchor the clock to the injected `base`.
- **Period crossings (criterion 1a):** FirstHalf→HalfTime→SecondHalf at the correct instants.
- **Penalty expiry (criterion 1b):** B#7 counts 30→0.
- **Stop/start (hand-verified):** the clock and penalty hold steady across the 15–18s stoppage — only ~1s of play time elapses across that window — exactly correct.

---

## 3. Finding #1 — the agreed `next_update_time`-driven model is impossible in replay; switched to fixed-step

**What happened:** the first run hung. The test ran >60s (should be milliseconds) and was killed.

**Root cause (verified at `tournament_manager/mod.rs:2112-2153`):** `next_update_time(now)` computes, for a down-counting clock with no timeout:

```
now + Duration::from_nanos(remaining.subsec_nanos())
```

When the clock sits exactly on a whole-second boundary, `remaining.subsec_nanos() == 0`, so it returns **`now` itself**. At clock start the clock is exactly 40.000s → `next_update_time(base) == base`. My driver set virtual `now` to that returned instant, which never advanced → infinite loop at `base`.

**Why the real app never hangs:** its loop reads `now = Instant::now()` fresh every iteration (`app/mod.rs:4075`). A returned `now+0` just means "don't sleep, loop now," and by the next iteration real wall-clock time has advanced microseconds, so `now` always moves forward. The engine's `next_update_time` is a **wall-clock scheduling hint, not a virtual-time step generator** — and it relies on monotonic real-time progress that a replay driver doesn't have.

This is NOT an engine bug, and I did not touch the engine (its now+0 behavior is part of the code under test). The fix belongs in the driver's time model.

**The fix — fixed-step driver:**
- Advance virtual `now` by a fixed small step (250ms), strictly monotonic.
- At each step (while the clock is running) call the real tick block — `could_end_game`/`pause_has_ended`/`update`, then `generate_snapshot` with the `None`→`update`→retry protocol. So `update`/`generate_snapshot` run **densely**, which is what actually realizes period rollovers and penalty expiry (a transition is realized at the first step where the clock crosses zero, via the `None`-retry calling `update`).
- Inject scripted actions at their exact instants.
- Record a trace line only when the observed time-state changes (dedupe).

**My argument that fixed-step is still faithful, not the naive model you warned against:** your original objection was to a driver that *never calls `update`/`generate_snapshot` at transition instants* and just samples getters. Fixed-step calls them at every step (4×/second), strictly denser than `next_update_time` would, so every transition is realized at essentially the same instant the app would realize it. The only thing it does NOT reproduce is the app's exact *wake cadence* — but that cadence is wall-clock-jittery and non-deterministic in the real app anyway, so it was never a reproducible target. What IS reproducible and meaningful — the engine's computed state as a function of game time — is captured faithfully and deterministically.

**The questions for you:**
- Do you accept that the app's exact emission cadence is inherently non-deterministic (wall-clock dependent), and therefore the right golden target is "engine state as a function of game time," which fixed-step captures — rather than "the exact sequence of snapshots the app emits"?
- Is 250ms the right step, or should it be smaller (more fidelity near sub-second transition instants) / does it matter given we observe at whole-second display granularity?
- Is there any transition that fixed-step + `None`-retry could realize at a *different* instant than the real app would? My claim is no, because `update` is idempotent w.r.t. call frequency (it recomputes absolute state from `start_time + elapsed`, not incrementally). Do you see a counterexample?

---

## 4. Finding #2 — trace line keying (normalization)

In the spike, each line is labeled by `floor(elapsed_wall_time)`. But the displayed value (`secs_in_period`, computed via `as_secs()` = floor of remaining) flips up to ~1s out of phase with that label. E.g. the clock shows "40" only at the exact start instant, then "39" for most of the first second — so the trace shows `t=0 clock=40` then `t=0 clock=39`, and the "t=N" label can trail the actual value-change instant by a fraction of a second.

It's deterministic, so it doesn't threaten the guard — but for clean, reviewable golden files I think the line should be keyed on the **clock value / state-change**, i.e., record a line at the exact instant a watched value flips, rather than on elapsed wall-time. That removes the phase offset and makes each line mean "from this game-clock value onward, the state is X."

**Question:** agree that keying on state-change instants (not elapsed wall-time) is the right normalization, or is there a reason to prefer fixed wall-time sampling?

---

## 5. Net

The approach is proven viable: faithful + deterministic reproduction of the engine's time behavior, including a period crossing, a penalty expiry, and a correct mid-play stop/start freeze. The spike earned its keep by killing the `next_update_time`-driven model before we built the scenario library on top of it. Pending your reaction to §3 and §4, the next step is the real build-out (scenario library, baseline bootstrap from `46ec0973`, strict normalization, permanent in-crate test, lightweight re-bless rule).
