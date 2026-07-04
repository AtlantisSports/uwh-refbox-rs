# Phase 0 Spike: Observation-Loop Pseudocode (for fidelity verification)

**Purpose:** This is the draft you offered to write; per the swap, I drafted it from the real code and am asking you to verify it faithfully mirrors how the running app drives the engine. All claims carry `file:line` references so you can check them even without repo access (treat the references as my attestation; flag any that look internally inconsistent).

**What the spike must prove (success criteria, restated):**
1. Driving the engine through a scenario that crosses a period boundary *and* expires a penalty produces a trace matching a hand-walked expectation.
2. The *observed* time state (period, game clock, timeout clock, penalty remaining times) is bit-identical across repeated runs given the same injected `now` sequence — despite the engine's internal `Instant::now()` calls (`tournament_manager/mod.rs:65`, `:898`, and `status_string(Instant::now())` log paths).

---

## 1. The real app drives the engine from TWO sources, not one

My earlier "observation loop" framing was incomplete. In the running app, `TournamentManager` state is mutated by two independent producers sharing one wall clock:

**Source A — the background clock-tick task** (`fn time_updater`, `app/mod.rs:4025-4118`).
A loop that sleeps until the next scheduled instant, then ticks the engine:
- It keeps `next_time` (init `Some(now)`, `:4036`).
- Each iteration it waits until `next_time` **or** until a clock-running change arrives, via `timeout_at(next_time, clock_running_receiver.changed())` (`:4040-4071`). The `clock_running_receiver` is the engine's start/stop watch channel (`:4035`, from `get_start_stop_rx()`).
- When it fires, it runs the **tick block** (`:4073-4114`, detailed in §3) and recomputes `next_time = clock_running ? Some(next_update_time(now)) : None` (`:4107-4111`).

**Source B — user-action message handlers** (`App::update(message)`, `app/mod.rs:1368+`).
Each operator action is an iced `Message` whose handler calls the corresponding engine method — `add_score`, `start_clock`, `start_penalty`, `start_team_timeout`, etc. — and frequently follows with `tm.update(now)` and/or `tm.generate_snapshot(now)`. Examples: `:1444`, `:1529`, `:2923` call `tm.update(now)`; and the game-ending path calls `tm.update(now + Duration::from_millis(2))` (`:2823`, the "+2ms" quirk). Toggling the clock also flips the `clock_running` watch channel, which is what wakes Source A.

A faithful driver must therefore be a **discrete-event simulation that merges both event streams on one virtual clock** — not a fixed-cadence sampler, and not "apply all actions then tick."

---

## 2. Driver as discrete-event simulation (top-level)

```
INPUTS:
  scenario = { config_overrides, actions: [(t_offset, Action)], end_offset }
  base : Instant            // one fixed base; virtual now = base + offset
  obs  : Trace = []         // accumulated normalized observation lines

tm = TournamentManager::new(config_from(scenario.config_overrides))
clock_running = false       // mirrors the start/stop watch; toggled by actions
next_tick = None            // mirrors Source A's next_time

action_queue = scenario.actions sorted by t_offset      // Source B events
now = base + 0
record_observation(tm, now, obs)                        // t=0 baseline line

loop:
    # the next event is the earliest of: next scripted action, next scheduled tick
    t_action = action_queue.peek()?.t_offset            // or +inf if empty
    t_tick   = next_tick.map(|i| i - base)              // or +inf if None
    t_next   = min(t_action, t_tick)
    if t_next > scenario.end_offset: break
    now = base + t_next

    if t_tick <= t_action AND next_tick.is_some():
        run_tick(tm, now, &mut clock_running, &mut next_tick, obs)   # Source A (§3)
    else:
        act = action_queue.pop()
        apply_action(tm, act.Action, now, &mut clock_running, &mut next_tick, obs)  # Source B (§4)

    record_observation(tm, now, obs)
```

Note: ticks and actions that land on the *same* instant must be ordered deterministically. Proposed rule: **tick before action** when timestamps tie, matching that the background task and the message handler are independent and the tick represents "time reaching this instant" which logically precedes the operator's action at that instant. **(Verify: is this the right tie-break, or does it matter? This is a candidate source of subtle divergence.)**

---

## 3. `run_tick` — faithful copy of the tick block (`app/mod.rs:4073-4114`)

```
run_tick(tm, now, clock_running, next_tick, obs):
    # branch order is significant and copied verbatim from :4077-4086
    if tm.could_end_game(now)? :
        tm.pause_for_confirm(now)?           # emits ConfirmScores in app
    elif tm.pause_has_ended(now) :
        tm.end_confirm_pause(now)?           # emits AutoConfirmScores in app
    else :
        tm.update(now)?                      # emits NewSnapshot in app

    # snapshot with None/retry protocol, copied from :4088-4105
    i = 0
    loop:
        if i > 4: FAIL("no snapshot after 5 attempts")   # app panics here
        match tm.generate_snapshot(now):
            Some(s): snapshot = s; break
            None:    tm.update(now)?; i += 1

    # reschedule, copied from :4107-4111
    *next_tick = clock_running ? Some(tm.next_update_time(now)?) : None
```

Two fidelity notes:
- `could_end_game` / `pause_has_ended` are checked *before* `update` on every tick, and they mutate state (`pause_for_confirm`, `end_confirm_pause`). A driver that only ever calls `update` would diverge on game-end and auto-confirm paths.
- The `None`-retry calling `update(now)` again is the documented mechanism by which a tick that crosses a transition boundary actually advances state. This is the heart of why naive sampling fails.

---

## 4. `apply_action` — mirror each action's real handler

The driver does **not** reimplement the giant `update(message)` match. Instead, for each scripted `Action` it replicates the *specific* engine calls that action's handler makes, including post-call `update`/`generate_snapshot` and any quirks.

```
apply_action(tm, action, now, clock_running, next_tick, obs):
    match action:
        StartClock:        tm.start_clock(now);  *clock_running = true
        StopClock:         tm.stop_clock(now)?;  *clock_running = false
        AddScore(c, n):    tm.add_score(c, n, now)
        StartPenalty(...): tm.start_penalty(...)?
        StartTeamTimeout:  tm.start_team_timeout(c, now)?; *clock_running = ...
        EndTimeout:        tm.end_timeout(now)?
        SetGameClock(d):   tm.set_game_clock_time(d)?
        ... etc ...

    # after a clock start/stop the real app's watch channel flips, which both
    # wakes Source A and changes whether next_tick is scheduled. Mirror that:
    if action changed clock_running:
        *next_tick = clock_running ? Some(tm.next_update_time(now)?) : None
```

**Open coupling point for the verifier:** the per-action post-call sequence. Some handlers do extra `tm.update(now)`/`generate_snapshot(now)` after the primary call (e.g. `:1444`, `:1529`, `:2923`) and the game-ending handler uses `tm.update(now + 2ms)` (`:2823`). The driver must enumerate, per action type, the exact handler call sequence. The narrow spike scenario will only exercise a few action types; the **general rule** is "for each Action, the driver's calls equal that action's handler's tm-calls." Is enumerating these per-action sequences the right approach, or is there a lower-level seam (e.g. driving the actual `App::update` path) that would be more faithful and less drift-prone — at the cost of dragging in the iced layer?

---

## 5. `record_observation` — what each trace line captures (time-only v1)

```
record_observation(tm, now, obs):
    period  = tm.current_period()
    clock   = tm.game_clock_time(now)          # Option<Duration>
    to      = tm.timeout_clock_time(now)       # Option<Duration>; plus timeout TYPE
    pens    = active penalties with remaining time, sorted by canonical key
    obs.push( normalize(now-base, period, clock, to_type, to, pens) )
```

Normalization (strict, version-independent — defined before any golden file):
- time rendered one fixed way (decided once: e.g. whole seconds `M:SS`);
- explicit distinct tokens for running / stopped / present-at-zero;
- timeout **type** explicit (team/ref/penalty-shot/rugby), not just "active";
- penalties ordered by (remaining desc, color, player#);
- no raw `Instant`/debug formatting in the line.

**Open question:** should observations be recorded only at event instants (as in §2), or also one tick *after* each transition to capture the post-transition steady state? The app emits a snapshot every tick; recording at every event instant approximates that, but a transition realized via the `None`-retry inside a single tick might warrant capturing the settled state explicitly. **(Verify.)**

---

## 6. Determinism handling (success criterion 2)

- Virtual `now = base + offset`; `base` captured once. All comparisons in the trace are relative, so the absolute `base` is irrelevant to output — *provided* the engine's internal `Instant::now()` calls don't leak into observed state.
- The spike asserts identical traces across ≥2 runs (and ideally a second OS). If they differ, we localize which getter/field varies and whether an internal `Instant::now()` (`mod.rs:65` constructor default `game_start_time`, `:898` scheduled-start) influences observed time state vs. being overwritten by the next injected `now`. That localization is a required spike output, not an afterthought.

---

## 7. The specific things I want you to attack

1. The **two-source merge** model (§2) — is a discrete-event sim merging scripted actions with `next_update_time`-scheduled ticks the correct abstraction of `time_updater` + message handlers, or am I missing a third mutation path?
2. The **tie-break** (tick-before-action on equal timestamps, §2).
3. The **per-action handler replication** vs. driving the real `App::update` seam (§4) — which is more faithful *and* less prone to silent drift as the app evolves?
4. The **observation timing** question (§5): event-instant only, or also post-transition settle?
5. Anything in `run_tick` (§3) that does not match the real tick block's order/semantics.
