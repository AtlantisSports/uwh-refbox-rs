# Web Refbox Parity Walkthrough Plan

> **For workers:** This is a **manual side-by-side verification walkthrough**, not a
> code-change plan. Steps use checkbox (`- [ ]`) syntax for tracking. There is no TDD
> loop here — each scenario is "set up identical state, perform the action on the Rust
> refbox (the reference), perform the same action on the web refbox, compare look AND
> function, record the result + a screenshot pair." No Rust or web source code is
> modified by this plan.

**Goal:** Confirm that every behaviour *shared* between the web refbox and the Rust
refbox looks and functions the same, using the Rust refbox as the reference of record;
produce a pass/fail report with matched screenshots and a prioritized punch-list of
web-app fixes.

**Approach:** The completed audit (`docs/audits/web-app-vs-refbox-2026-04-18.md`) already
enumerates ~50 shared behaviours across 7 areas. This plan turns each into a concrete
side-by-side scenario. Both apps run at once on identical game configuration so any
difference observed is a *real* divergence, not a config mismatch. The Rust app is
operated by hand (it is a native desktop app — no browser automation can drive it); the
web app is operated in a browser.

**Tools / inputs:**
- Rust refbox: this workspace, launched per the project's standard launch procedure.
- Web refbox: UWH Portal web app (`/home/estraily/projects/uwh-portal/`).
- Reference audit: `docs/audits/web-app-vs-refbox-2026-04-18.md`.
- Output: two documents under `docs/audits/` (see File Structure).

---

## Precondition (do not start until true)

- [ ] The uwhportal agent's in-progress web-vs-rust audit pass is **finished**, and its
  confirmed shared-vs-new map has been reconciled against
  `docs/audits/web-app-vs-refbox-2026-04-18.md`. Any scenario the finished audit adds,
  removes, or re-classifies is folded into the scenario tables below **before** the
  walkthrough begins.

---

## File Structure

Two deliverables, both created fresh under `docs/audits/`:

- Create: `docs/audits/web-refbox-parity-report-2026-05-30.md`
  - The pass/fail record. One row per scenario with Rust behaviour, web behaviour,
    look-match, function-match, verdict, screenshot-pair filename, and notes.
- Create: `docs/audits/web-refbox-fix-punchlist-2026-05-30.md`
  - The actionable fix list for the web app — every FAIL/divergence, prioritized, with a
    repro and a pointer to the relevant web-side file:line from the audit. This is the
    document handed to the uwhportal agent.
- Create: `docs/audits/screenshots/parity-2026-05-30/` (directory)
  - Matched screenshot pairs, named `<scenario-id>-rust.png` and `<scenario-id>-web.png`.

No existing files are modified.

---

## Deliverable templates

### Report row format (`web-refbox-parity-report-2026-05-30.md`)

```markdown
| ID | Scenario | Rust behaviour (reference) | Web behaviour | Look | Function | Verdict | Screens | Notes |
|----|----------|----------------------------|---------------|------|----------|---------|---------|-------|
| 1.1 | Period chain sequence | … | … | ✅/❌ | ✅/❌ | PASS/FAIL/EXPECTED-DIFF | 1.1-rust.png / 1.1-web.png | … |
```

### Punch-list item format (`web-refbox-fix-punchlist-2026-05-30.md`)

```markdown
| ID | Severity | Area | Diverges from Rust how | Repro steps | Web file:line (from audit) |
|----|----------|------|------------------------|-------------|----------------------------|
| P1 | High/Med/Low | Penalties | … | … | js/@underwater-web/… |
```

Severity guide: **High** = wrong game state or a referee could be misled mid-game;
**Med** = missing edit/recovery path or wrong option set; **Low** = cosmetic/visual only.

---

## Task 1: Establish a matched configuration baseline

This is the single most important correctness step. If the two apps run different game
settings, behaviours will "differ" for reasons that are not real divergences.

**Files:** Create `docs/audits/web-refbox-parity-report-2026-05-30.md` (header + config
table), create the screenshots directory and the punch-list file (headers only).

- [ ] **Step 1: Record the Rust config.** In the Rust refbox, open Game/Tournament
  settings and record every field: half lengths, half-time, OT enabled + OT half length
  + pre-OT break, sudden-death enabled, between-games break, timeout duration, timeouts
  count + per-half flag, sport mode, all sound option toggles. Note the exact values.
- [ ] **Step 2: Set the web refbox to the identical config.** Enter the same values in
  the web app's settings. Where the web app lacks a field the Rust app has (e.g.
  `penalty_shot_duration`, `post_game_duration` per audit 1.2), note it in the report's
  config table as a known structural gap, not a per-scenario failure.
- [ ] **Step 3: Set sport mode to Hockey 6v6 on both.** The audit (TL;DR 5, 3.1) shows
  the web app's sport mode is cosmetic, so all shared scenarios are run in **Hockey 6v6**,
  where the option sets are expected to match. The 3v3/Rugby gaps are carried straight
  from the audit into the punch-list (Task 9), not re-derived by hand.
- [ ] **Step 4: Write the config table** into the report under a "Baseline configuration"
  heading, listing each setting and its value on both sides, flagging any field that only
  one app has.

## Task 2: Launch both apps side by side

**Files:** none.

- [ ] **Step 1: Launch the Rust refbox** from this workspace using the project's standard
  launch procedure for this machine (background launch, sandbox disabled for Wayland/audio
  access; force X11 if launching natively on WSLg). Confirm the window opens and stays up.
- [ ] **Step 2: Open the web refbox** in a browser, pointed at the intended portal
  environment, and load to the refbox game view.
- [ ] **Step 3: Confirm audio is audible on both** (a sound scenario set follows). On the
  web app, click once inside the page first so the browser's audio is unlocked (audit 6.8).
- [ ] **Step 4: Place the two windows side by side** so a single screenshot can capture
  both, or capture them as a matched pair per scenario.

---

## Task 3: Area 1 — Timing & game-state-machine transitions

For each scenario: drive the state on the Rust app, observe; drive the same on the web
app, observe; compare look + function; record a row + screenshot pair. Audit reference:
section 1.

- [ ] **1.1 Period chain** — step a game through every period. Reference: Rust goes
  game-end → BetweenGames directly; web inserts an extra `PostGame` state. Expected
  **DIFF** (web-only state). Confirm web's PostGame is benign / does not strand the ref.
- [ ] **1.2 Period durations** — confirm each configured duration counts correctly on
  both. Note the web's missing `penalty_shot_duration`/`post_game_duration` fields.
- [ ] **1.3 Skip-OT/SD when both disabled** — disable OT and SD; confirm a tied game ends
  cleanly (no OT/SD) on both. Expected **PASS**.
- [ ] **1.4 Single-half mode** — enable single-half; confirm both skip Half Time and
  Second Half. Expected **PASS**.
- [ ] **1.5 Clock direction** — confirm both count DOWN in normal play and UP in Sudden
  Death. Expected **PASS**.
- [ ] **1.6 BetweenGames auto-start** — let the between-games countdown reach 0; confirm
  both auto-transition to First Half. Expected **PASS**.
- [ ] **1.7 SD goal ends game** — score in Sudden Death; confirm both stop the clock →
  confirmation screen → on confirm, end game to BetweenGames. Expected **PASS**.

## Task 4: Area 2 — Timeout handling

Audit reference: section 2.

- [ ] **2.1 Team-timeout limit** — exhaust the configured team timeouts; confirm both
  block further team timeouts. Expected **PASS**.
- [ ] **2.2 Per-half vs per-game counting** — toggle `timeouts_counted_per_half`; confirm
  both reset the count at the right boundary. Expected **PASS**.
- [ ] **2.3 Team timeout auto-ends at duration** — start a team timeout; confirm both
  auto-end at the configured length. Expected **PASS**.
- [ ] **2.4 Game clock pauses during timeout** — confirm both freeze the game clock for
  the timeout. Expected **PASS**.
- [ ] **2.5 Ref timeout open-ended** — start a ref timeout; confirm both count up from 0
  with no auto-end. Expected **PASS**.
- [ ] **2.6 Penalty-shot duration (Hockey)** — start a penalty shot in 6v6; confirm both
  count UP, open-ended. Expected **PASS** in Hockey. (Rugby count-down variant → punch-list
  via Task 9, not tested here.)
- [ ] **2.7 Penalty-shot clock behaviour** — consequence of 2.6; confirm display matches.
- [ ] **2.8 Period gating on starting a timeout** — attempt to start a team timeout during
  Half Time. Reference: Rust *rejects* at the handler; web relies on button-disabling
  only. Confirm the web button is correctly disabled; flag the weaker gating. Likely
  **DIFF** (defense-in-depth), record severity.
- [ ] **2.9 Resume at paused time** — confirm the game clock resumes from the exact paused
  value on both. Expected **PASS**.

## Task 5: Area 3 — Penalty clock

Audit reference: section 3.

- [ ] **3.1 Penalty kinds (Hockey 6v6)** — open the penalty keypad; confirm both show
  1m / 2m / 5m / TD. Expected **PASS** in 6v6. (3v3 missing-30s and Rugby missing-4m →
  punch-list via Task 9.)
- [ ] **3.2 Penalty start durations** — add one of each kind; confirm 60s / 120s / 300s /
  non-counting on both. Expected **PASS**.
- [ ] **3.3 Penalty runs only during play** — confirm penalties tick in First/Second Half
  and OT/SD halves, and pause in Half Time / pre-OT / BetweenGames, on both. Expected **PASS**.
- [ ] **3.4 Concurrent penalties** — add two; confirm both count down together (not
  sequential) on both apps. Expected **PASS**.
- [ ] **3.5 Total dismissal never counts down** — add a TD; confirm it stays frozen until
  manually removed on both. Expected **PASS**.
- [ ] **3.6 Completed penalties stay visible** — let a penalty expire; confirm it shows as
  served/visible on both. Expected **PASS** (practically).
- [ ] **3.7 Pending-penalty flow (web-only risk)** — on the web app: add a penalty, then
  press CANCEL on the overview page; confirm the penalty is saved but does NOT count down
  (the `pending` risk). Confirm the Rust app has no such state (penalties are active the
  moment the keypad closes). Expected **DIFF** → High-severity punch-list item.
- [ ] **3.8 Edit / delete penalty** — edit and delete a penalty; confirm both support
  both. Expected **PASS** (penalties are the web's exception).
- [ ] **3.9 Penalty pause across period transitions** — confirm a penalty pauses at
  half-time and resumes next play period on both. Expected **PASS**.

## Task 6: Area 4 — Score confirmation & goal flow

Audit reference: section 4.

- [ ] **4.1 No confirm-pause on regular-play goals** — score in First Half; confirm
  neither app pauses the clock during player-number entry. Expected **PASS**.
- [ ] **4.2 SD confirm ordering** — N/A to web (Rust `ConfirmPause` fires only in SD).
  Record as **EXPECTED-DIFF**, no test.
- [ ] **4.3 Cancel reverts the goal** — cancel from the player-number keypad (nothing
  saved); cancel "NO" from the SD confirmation (routes to ScoreEdit, manual decrement).
  Confirm both behave identically. Expected **PASS**.
- [ ] **4.4 Goal undo after resume** — confirm neither app offers a user-facing one-tap
  undo; reverting means opening ScoreEdit and decrementing. Expected **PASS** (Rust's
  15s `recent_goal` only drives the stream overlay).
- [ ] **4.5 Goal snapshot captures player + time** — score a goal; confirm both record the
  player number and the clock time. Expected **PASS**.
- [ ] **4.6 Multiple quick goals** — regular play: score two in a row; confirm both allow
  it with no pause. SD: confirm both block a second SD goal until the first resolves.
  Expected **PASS**.
- [ ] **4.7 Goal at exactly 0:00** — score with the keypad open as the period rolls over;
  confirm both timestamp at DONE and record under the new period predictably. Expected
  **PASS**.
- [ ] **4.8 SD confirmed goal ends game** — duplicate of 1.7 from the goal-flow side;
  confirm consistent. Expected **PASS**.

## Task 7: Area 5 — Warnings & fouls

Audit reference: section 5.

- [ ] **5.1 Infraction type set** — open the warning and foul keypads; confirm all 12
  infraction types appear with matching IDs on both. Expected **PASS**.
- [ ] **5.2 Display short names** — confirm the visible strings are identical
  ("Stick Foul", "Illegal Advance", "Out Of Bounds", etc.). Expected **PASS** (look check).
- [ ] **5.3 Warning lifecycle** — add, then attempt edit and delete. Reference: Rust
  supports all three; web is add-only. Expected **DIFF** → Med-severity punch-list item.
- [ ] **5.4 Foul lifecycle** — same as 5.3; confirm both support the "equal / neither
  team" category. Add-only on web → punch-list.
- [ ] **5.5 Team-level (no player number)** — add a team-level warning/foul; confirm both
  accept a null player number. Expected **PASS**.
- [ ] **5.6 Persistence across period transitions** — confirm warnings/fouls survive a
  half change on both. Expected **PASS**.
- [ ] **5.7 Persistence across game end** — confirm timing: Rust clears at NEXT game start
  (still visible during BetweenGames); web clears at previous game end. Expected **DIFF**
  (minor) → Low-severity punch-list item.
- [ ] **5.8 Summary page** — confirm both have a page listing all warnings/fouls.
  Expected **PASS**.
- [ ] **5.9 Additions during BetweenGames** — confirm both allow adds when role permits.
  Expected **PASS**.

## Task 8: Area 6 — Sound cues

Audit reference: section 6. (Unlock web audio first — see Task 2 Step 3.)

- [ ] **6.1 Sounds play in live game** — confirm cues fire during a running game on both.
  Expected **PASS**.
- [ ] **6.2 Period-end buzzer** — let a period reach 0:00; confirm the buzzer fires on
  both (gated by auto-sound-stop, default on). Expected **PASS**.
- [ ] **6.3 30-second warning whistle** — in a break/pre-game period, confirm a whistle at
  30s remaining on both. Expected **PASS**.
- [ ] **6.4 Timeout warnings** — during a team timeout, confirm whistle at 15s and buzzer
  at 0s on both. Expected **PASS**.
- [ ] **6.5 Manual alarm button** — confirm Rust has the hold-to-fire manual alarm
  (spacebar); confirm web has none. Expected **DIFF** → punch-list (web feature gap).
- [ ] **6.6 Sound-option settings** — compare the settings screens; confirm the 8 shared
  fields match by name/meaning; note web is missing `manual_alarm_enabled` and `remotes`.
  Record as DIFF tied to 6.5 / hardware.
- [ ] **6.7 Sound asset set** — confirm the same six sounds exist on both. Expected **PASS**.
- [ ] **6.8 Browser audio-context** — note web suspends/resumes an AudioContext (may need
  a user gesture); not a Rust-parity defect. Record as web-specific note.

## Task 9: Area 7 — Hardware & web-only features (inventory, not pass/fail)

Audit reference: section 7 + TL;DR 5.

- [ ] **Step 1: Record expected hardware differences** — LED panel, stream overlay, and
  wireless remote are Rust-only by design. List them in the report as **EXPECTED-DIFF**.
- [ ] **Step 2: Record web-only features** — multi-court (`selectedCourt`) and any extras
  the finished audit confirms. List them in the report under "Web-only (out of scope)".
- [ ] **Step 3: Carry the sport-mode gaps to the punch-list directly from the audit** —
  3v3 missing 30s penalty; Rugby missing 4m penalty and the count-down (45s auto-end)
  penalty shot; sport mode being cosmetic. These are not hand-tested in 6v6; cite the
  audit's file:line evidence. High severity (real for non-6v6 tournament play).

## Task 10: Compile the report and the punch-list

**Files:** finalize `web-refbox-parity-report-2026-05-30.md` and
`web-refbox-fix-punchlist-2026-05-30.md`.

- [ ] **Step 1:** Confirm every scenario ID from Tasks 3–9 has a filled report row with a
  verdict and a screenshot-pair reference. List any scenario left unrun and why.
- [ ] **Step 2:** Pull every FAIL and every DIFF (excluding EXPECTED-DIFF and web-only) into
  the punch-list, each with severity, repro, and the web-side file:line from the audit.
- [ ] **Step 3:** Sort the punch-list by severity (High → Low). Put a one-line summary at
  the top (counts: PASS / DIFF / FAIL / expected).
- [ ] **Step 4:** Write a plain-English summary paragraph at the top of the report: how
  many shared behaviours matched, where the web app diverges, and which divergences could
  affect a referee mid-game.
- [ ] **Step 5:** Hand the punch-list to the uwhportal agent as the web-app fix backlog.

---

## Self-review notes

- **Coverage:** Tasks 3–9 cover all 7 audit areas and all ~50 sub-questions (1.1–1.7,
  2.1–2.9, 3.1–3.9, 4.1–4.8, 5.1–5.9, 6.1–6.8, 7.x). Reconcile against the finished audit
  before starting (Precondition).
- **Fairness:** Task 1 (matched config, Hockey 6v6) prevents config-driven false
  divergences — the most likely source of bad findings in a side-by-side.
- **Reference direction:** Rust is the reference everywhere; "FAIL" always means the web
  app diverges from Rust, never the reverse.
- **Open coordination point:** who operates the web app during the walkthrough (you, or
  the uwhportal agent) is decided at execution start; it does not change the scenario list.
