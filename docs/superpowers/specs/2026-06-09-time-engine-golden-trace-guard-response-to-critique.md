# Response to your critique of the Golden-Trace Regression Guard design

**Context:** You critiqued the design in the companion document (Golden-Trace Regression Guard for the refbox time engine). Before accepting or rejecting your points, I verified the load-bearing ones against the actual source. This is my response, with the evidence. A second pass from you is welcome — especially on the two points where I diverge from your recommendation.

No need to re-read the original design to follow this; the relevant facts are restated inline.

---

## 1. Your "single weakest point" is confirmed — and it's worse than either of us framed it

You argued the driver's fidelity to the real application's driving loop is the make-or-break risk, and that state transitions are likely realized inside `generate_snapshot` (called at `next_update_time` instants) rather than as lazy pure functions sampled through getters. I checked. Here's the actual mechanism, with line references:

- **Period rollovers are realized inside a `&mut self` method `update(now)`, not in any getter.** Every transition assignment — `FirstHalf → HalfTime → SecondHalf → PreOvertime → OvertimeFirstHalf → … → SuddenDeath` — sits inside `pub(super) fn update(&mut self, now: Instant)` at `tournament_manager/mod.rs:1133` (the assignments span lines 1191–1392). The read-only getters (`game_clock_time`, `current_period`, `timeout_clock_time`) never advance the period.

- **The real app loop calls `update(now)` *first*, then `generate_snapshot(now)`** (`app/mod.rs:4084` then `:4097`).

- **`generate_snapshot(now)` can return `None`, and the correct response is to call `update(now)` again and retry — up to 5 times, then panic.** This is the actual loop at `app/mod.rs:4090-4106`:
  ```
  loop {
      if i > 4 { error!(...); panic!("No snapshot"); }
      match tm_.generate_snapshot(now) {
          Some(val) => break val,
          None => { warn!(...); tm_.update(now).unwrap(); i += 1; }
      }
  }
  ```

- **`next_update_time(now)` schedules the next wake — but only while the clock is running** (`app/mod.rs:4108-4112`).

- **There is a deliberate off-by-2ms quirk**: `tm.update(now + Duration::from_millis(2))` with the comment "Need to update after game ends" (`app/mod.rs:2823`).

Implication: a naive "apply scripted actions, then sample getters" driver would **never advance the period at all** and would **choke on the `None` return**. This isn't a subtle fidelity gap — a naive sampler is simply non-functional here. So your recommendation to emulate the real loop is not just the safer choice; it's the only one that works. The existing in-module tests already follow this shape (`tournament_manager/mod.rs:2530-2545` call `tm.update(now)` then assert), which gives us a proven pattern to mirror.

**Design change accepted:** the spec will specify the observation loop explicitly — `update(now)` → `generate_snapshot(now)` with the `None`/retry protocol → `next_update_time(now)` (gated on clock-running) → advance virtual time to `min(next_action, next_update)` → repeat — plus the +2ms-after-game-end quirk, all sourced from the real app loop, with a note that the driver is coupled to that loop and must track changes to it.

## 2. Your Assumption-1 concern (deterministic / synchronous drivability) — partially de-risked, partially confirmed

- **Synchronous: confirmed safe.** There is no `async fn` and no `.await` anywhere in the 7,016-line engine file. It can be driven from a plain `#[cfg(test)]` unit test without spawning the app's async runtime. This removes the scariest version of your concern.

- **Internal wall-clock reads: confirmed present.** The engine *does* call `Instant::now()` in production paths — notably the constructor (`mod.rs:65`, sets a default `game_start_time`) and a scheduled-start computation (`mod.rs:898`, `Instant::now() + dur`), plus several `status_string(Instant::now())` calls that appear to be log-only. So "perfectly deterministic from injected instants" is not free. Whether the non-log calls actually influence the *observed time state* (vs. being overwritten by the next injected `now`) is the open question.

**Design change accepted:** resolving this becomes an explicit success criterion of the spike (below), not an assumption carried into the build.

## 3. The spike is now Phase 0 and gates everything (accepted)

Before any scenario library or golden file is written:

- Build the smallest driver that runs one scenario crossing a period boundary **and** expiring a penalty, using the faithful `update/generate_snapshot/next_update_time` loop from §1.
- Confirm the produced trace matches a hand-walked expectation.
- Run it twice (and, if cheap, on a second platform) to confirm bit-identical output — settling the §2 determinism question concretely.
- If the engine cannot be driven faithfully and deterministically, the approach is reconsidered *here*, cheaply, rather than after investment.

## 4. Normalization spec written before the first golden file (accepted)

Agreed that loose normalization is how these guards die. The spec will fix, up front and version-independently:
- penalty ordering by a canonical key (remaining time desc, then color, then player number);
- a single `Duration` rendering (e.g. fixed `M:SS` or whole-second form, decided once);
- explicit distinct states for running / stopped / present-but-zero;
- explicit timeout **type** (team / ref / penalty-shot / rugby), not merely "timeout active";
- no raw `Instant`/debug formatting leaking into traces.

## 5. Accepted without changes

- **Don't extract `tournament_manager` into a library crate** (your Question 4). Agreed — it violates the hard "stay within `refbox`" scope constraint and is exactly the kind of structural refactor that could introduce the timing bugs we're hunting. The in-crate unit-test placement stands.
- **Test machinery goes in a `golden_time` submodule** with data under a dedicated directory, not dumped into the 7k-line `mod.rs`. Accepted.
- **Config-default divergence is a legitimate finding, not harness breakage** — accepted, with the caveat you raised (the permanent guard then also pins time-relevant config defaults; documented as part of what "blessing" means).
- **Coverage illusion is an accepted limitation** — the guard's contract is "detect unintended changes on the paths we chose to defend," not exhaustiveness.

## 6. Two points where I diverge — your reaction wanted

Both are places you yourself flagged as possibly over-engineering; I'm choosing the lighter option for v1 and want to know if you still think that's wrong.

**(a) No heavyweight re-bless gate in v1.** You noted the visible-PR-diff safeguard is weak because the same non-programmer who directed the 370 commits is the reviewer, and subtle time diffs may get blessed carelessly. Real risk. But my read is that the *human-readable trace itself* is the mitigation — a blessed change shows up as concrete before/after lines like "penalty expires at 1:58 instead of 2:00," which is precisely the kind of thing the domain expert *can* adjudicate. I'd defer the machine-readable "what changed and by how much" summary until triage fatigue actually appears, rather than build it speculatively. **Do you still consider an upfront stronger gate worth the cost, given the reviewer profile?**

**(b) Seeded-random differential scenarios stay out of the durable guard.** I'd use them, if at all, only as a one-off breadth probe during the initial investigation — not as part of the permanent suite — because random traces undercut the "these are the scenarios we have consciously decided to defend forever" property, and are hard for the domain expert to triage. You seemed to agree random is a poor *primary* guard; I'm going further and excluding it from the permanent suite entirely. **Is there a version of seeded-random that earns its place in the durable guard, or do you agree it belongs only in the investigation phase?**

## 7. Net

Your central thesis held up under verification and was, if anything, understated — the naive observation model isn't merely lower-fidelity, it's non-functional against this engine. The design now leads with a gating spike, specifies the real driving loop and a strict normalization contract, and otherwise stays within the original constraints. The only open disagreements are the two deliberate scope-reductions in §6.
