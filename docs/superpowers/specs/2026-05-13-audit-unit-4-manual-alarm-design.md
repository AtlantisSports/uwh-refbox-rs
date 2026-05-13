# Audit Unit 4 — Manual Alarm Button — Audit Design

**Date:** 2026-05-13
**Audit unit:** 4 — Manual alarm button
**Branch (to be cut):** `audit/refbox/manual-alarm-button`
**Authoritative reference:** `docs/superpowers/specs/2026-04-14-manual-alarm-button-design.md`
**Related ADR:** `docs/decisions/006-multi-remote-alarm-buttons.md` (proposed successor; out of audit scope)
**Playbook:** `AUDIT-PLAN.md` (gitignored working document)

---

## Purpose

This document captures the brainstormed shape of Unit 4 of the AI Code Audit. Unit 4 is unusual because the manual-alarm-button feature already has a pre-implementation spec and two plan documents on disk, plus a successor ADR (006) in `proposed` state. The audit's job is to confirm that the shipped code matches the spec, flag any divergences for explicit operator decision, and record the surviving feature as a retroactive ADR.

This spec is the input to the per-unit plan that will be written next via `superpowers:writing-plans`. The per-unit plan decomposes the playbook's 9-step workflow into Unit-4-specific tasks.

---

## Scope

### In scope

The 16 manual-alarm commits between `bc66e1e` (2026-04-14) and `ff6018b` (2026-04-17), inclusive (`git rev-list bc66e1e^..ff6018b --count` = 16). Files touched:

- `refbox/src/sound_controller/mod.rs` — `manual_alarm_enabled: bool` field, migration support
- `refbox/src/app/message.rs` — `AlarmPressed`, `AlarmReleased`, `AlarmDelayElapsed`, `SpacebarPressed`, `SpacebarReleased`; `BoolGameParameter::ManualAlarmEnabled`
- `refbox/src/app/mod.rs` — state flags (`mouse_alarm_held`, `spacebar_held`), token counter (`alarm_delay_token`), press/release handlers, keyboard subscription, hold-duration helper
- `refbox/src/app/view_builders/main_view.rs` — layout switch when enabled, `mouse_area`-wrapped alarm face, red/blue colour and label switching on `is_active_play`
- `refbox/src/app/view_builders/configuration.rs` — sound-settings row, greyed when Sound Enabled is Off
- `refbox/translations/*.ftl` — 5 new keys (`alarm-button`, `alarm`, `or-press-spacebar`, `hold-to-test`, `or-hold-spacebar`, `game-info`) across ~13 languages
- `refbox/tests/features/manual_alarm.feature` — pre-existing scenario file from doc-revision commit `9cef2c4`

### Out of scope

- Anything ADR 006 proposes (per-remote tiles, digit-keyboard triggers, packet-flash visual confirmation). The audit catalogs what exists; ADR 006 is a separately-decided successor and stays in the post-audit ADR backlog.
- The original heavy plan `docs/superpowers/plans/2026-04-14-manual-alarm-button.md` (815 lines, retconned mid-flight) and the companion delta plan `docs/superpowers/plans/2026-04-17-manual-alarm-uniform-hold-delta.md` are historical work products. They are referenced as evidence during the audit but are not authoritative — divergences are flagged against the spec, not against either plan.

---

## Divergence policy

The spec at `docs/superpowers/specs/2026-04-14-manual-alarm-button-design.md` is ground truth for the audit. Code that diverges from the spec is flagged as a B-entry the operator decides on (three options per divergence: code is right and spec should update, spec is right and code should fix, both are tolerable as-is). Code behaviour the spec is silent about is flagged via the slop-catching checklist.

---

## Audit approach

**Diff-led catalog with spec as oracle (Unit 3 pattern).** This matches how all prior units operated. The catalog is built from the 16 commits' diff; per-operator-observable-behaviour decomposition; the spec is the oracle for each entry.

1. Walk the diff. Write one B-entry per distinct operator-observable behaviour or backend touchpoint.
2. For each B-entry, mark spec-status: **matches spec**, **diverges from spec**, or **not in spec**.
3. **Matches spec** → recommendation: keep. Operator confirms in Step 4.
4. **Diverges from spec** → the B-entry's Recommendation line states the divergence explicitly (e.g., "code says 150ms; companion plan says 250ms; spec says 150ms — code/spec agree; companion plan is stale"). Operator decides what truth wins.
5. **Not in spec** → flag via the playbook's slop-catching checklist. Operator decides keep or delete.
6. Scenarios derived from operator-observable catalog entries per the playbook's Step 3 extension.
7. Pre-existing `refbox/tests/features/manual_alarm.feature` aligned during Step 6, like Unit 1's pre-existing file.

### Expected catalog size

25–40 entries. Unit 4 has multiple bundled-feature commits, which per Unit 3's Process refinement #2 yields catalog sizes in the 25–50 band, not the playbook's original 15–25 default. Anticipated B-entry families (working sketch; actual decomposition happens in execution):

- Settings: `manual_alarm_enabled` field, default Off, migration test, sound-page row placement, greyed-when-sound-off
- Messages: 5 message variants + `BoolGameParameter::ManualAlarmEnabled`
- State: `mouse_alarm_held`, `spacebar_held`, `alarm_delay_token` (spec naming divergence: `alarm_hold_generation`)
- Helper: `manual_alarm_hold_duration()` extraction
- Active-play vs other-state: 150ms vs 1s bands; the 150ms tuning commit `ff6018b`
- Subscription: spacebar key-down / key-up, gated on `manual_alarm_enabled && sound_enabled`
- Main view layout: GAME INFO compact button, alarm container, fouls-on vs fouls-off variants
- Button appearance: `red`/`red_pressed`/`blue`/`blue_pressed` containers, `mouse_area` always-wrapped
- Translations: 5 keys × ~13 languages
- Doc-revision commit `9cef2c4`: spec/scenarios/plan revised mid-flight to uniform hold — meta-behaviour worth one B-entry

### Review pattern

Page-batched review for Step 4 if the catalog exceeds ~25 entries, per Unit 3 Process refinement #3. Ambiguity carve-outs reserved for entries the implementer flags during the catalog draft.

---

## Scenarios

**Pre-existing file:** `refbox/tests/features/manual_alarm.feature` already exists from commit `9cef2c4`. Mirror Unit 1's settled convention (Step 6.1 Option A): keep the file at its existing path; **align** the scenarios to the audit's wording rather than creating a parallel file.

**Scenario shape:** one scenario per operator-observable B-entry. Expected scenario count: 10–18. Anticipated scenarios:

- Active play, no timeout — mouse hold past 150ms fires; tap under 150ms does not fire
- Active play, no timeout — spacebar parity with mouse
- Active play, timeout active — 1-second hold required; short press does not fire
- Break period (Between Games / Half Time / Pre-OT / OT Half Time / Pre-Sudden Death) — 1-second hold
- Settings toggle: enables layout switch; greyed when Sound Enabled is Off; defaults to Off
- Layout: fouls-on splits vertically (alarm left, warnings panel right); fouls-off is full-width
- Button appearance: red + "Alarm / Or press Spacebar" in active-play-no-timeout; blue + "Hold to Test / Or hold Spacebar" elsewhere; pressed-state container while held
- Release: currently-playing tone finishes natural cycle; no further tones queued
- Spacebar has no effect on screens other than the main game screen

**Concrete-phrasing rule** per the playbook's scenario slop-catching: every scenario names the actual game-state period (e.g., "First Half clock running, no timeout active"), the actual button label, and the actual hold-duration threshold. No abstract phrasing.

**Backend-only behaviours** (migration default, `BoolGameParameter` toggle wiring, helper-method extraction, token cancellation logic) have no scenario. Their test status is captured in the retroactive ADR's prose during Step 7.

---

## Test pass

### Test sessions

Three sessions cover the surface area without re-navigation fatigue (per the playbook's Step 6 grouping rule):

- **Session 1 — Sound Options page.** Verify the settings row appears, defaults to Off, is greyed when Sound Enabled is Off, toggles On cleanly. Verify the relevant translations render in the active language without overflow.
- **Session 2 — Main game screen with feature enabled.** From one running game, walk: First Half clock running → 150ms hold fires red button; tap under 150ms does not fire; start a timeout → button becomes blue, 1-second hold required; end timeout → button returns to red; advance to Half Time → 1-second hold; advance to Between Games → 1-second hold. Verify in both fouls-on and fouls-off layouts.
- **Session 3 — Spacebar parity and inactive screens.** Repeat Session 2's checks using only the spacebar to confirm parity. Open a non-main screen (Game Options, Penalties Page, Score Edit) — confirm spacebar has no effect.

### Sanity backstop

After the three sessions, cross-check against the companion plan's 11-item manual verification checklist (in `2026-04-17-manual-alarm-uniform-hold-delta.md` Task 3 Step 2). Items not covered by the sessions go to the Findings backlog as gaps.

### Backend regression tests

Migration default and `BoolGameParameter` toggle wiring are covered by existing tests (the migration test from commit `38799da` and the configuration-page tests). Pre-flight estimate for new Rust tests added during the audit: 1–3 (or zero), focused on token cancellation and mouse/spacebar independence if either reveals concerns during catalog review.

---

## Retroactive ADR

**Fresh ADR**, not an amendment. ADR 006 describes a future successor, not what shipped; in-place finalization (the Unit 3 pattern for proposed-ADR audits) does not apply. Pattern matches Unit 1 / Unit 2.

**ADR number:** **021**, with an explicit "numbered against expected post-merge state" note. Unit 1 holds 019; Unit 2 holds 020 (both on their respective audit branches, not merged); Unit 4 takes 021 unless a prior unit (3, 5, 6, 7, 8) lands first. Mirrors Unit 1 refinement #8's gap-handling.

**Title:** ADR 021 — Manual alarm button (retroactive)

**Status:** Accepted (retroactive)

**Decision section** embeds every `@user_verified @tested_pass` scenario from `refbox/tests/features/manual_alarm.feature` as Gherkin code blocks with one sentence of plain-English framing per scenario. Backend behaviours (migration default, helper-method extraction, token cancellation, mouse/spacebar independence) appear as plain-English bullets separately.

**References section** lists:

- The spec at `docs/superpowers/specs/2026-04-14-manual-alarm-button-design.md`
- The companion delta plan at `docs/superpowers/plans/2026-04-17-manual-alarm-uniform-hold-delta.md`
- The original (retconned) plan at `docs/superpowers/plans/2026-04-14-manual-alarm-button.md`
- ADR 006 (proposed successor — explicit note that ADR 006 is gated on the findings recorded here)
- The 16 original commits by SHA

**"What was not verified"** section if any catalog entry ends `@tested_inconclusive` or is testable-only-with-hardware. Likely empty for this unit.

---

## Risk areas

Specific things to watch for during catalog and test pass:

1. **Hold-duration constant.** Spec says 150ms; companion plan says 250ms (3 places); final tuning commit `ff6018b` lands at 150ms. Verify the spec line was updated to 150 in doc-revision commit `9cef2c4`. One B-entry records the final landing point and the divergence chain.

2. **State-naming divergence.** Spec uses `alarm_hold_generation` (line 117); code uses `alarm_delay_token`. Same concept, different name. B-entry to confirm code naming wins over spec naming.

3. **Independent-input-flag design vs spec's generation counter.** Spec describes a single generation counter. Code uses two independent flags (`mouse_alarm_held`, `spacebar_held`) plus a token counter. The two-flag design handles the spacebar-and-mouse-simultaneously-held case explicitly; the spec did not describe that case. B-entry asks whether the added complexity is wanted.

4. **`manual_alarm_hold_duration()` helper extraction.** Companion plan Task 1 Step 4 flagged it as "Optional but recommended." B-entry confirms whether it was extracted; if not, two near-identical match blocks live in `mod.rs`.

5. **`disabled_container` import cleanup** (companion plan Task 2 Step 2). Quick grep confirms whether the obsolete import remains.

6. **`mouse_area` always-wrapped guarantee.** If the code still has a conditional wrap from the original mixed-model design, the button can become non-interactive in some state — operator-observable regression.

7. **Translation coverage.** 5 new keys × ~13 languages. Plain-text grep confirms all keys exist in every locale's `.ftl` file.

8. **Doc-revision commit `9cef2c4` as a meta-behaviour.** Commit revised spec + scenarios + plan in one shot (bundled docs commit per Unit 2 Process refinement #2). Less risky than bundled code commits but worth a B-entry that records the design-correction event itself.

9. **Spacebar gating on non-main screens.** Spec line 75 says spacebar has no effect on config / penalties / score-edit screens. Verify whether the gating is at subscription time, handler time, or both — and whether typing spacebar in a text field has any unintended effect.

10. **Sound-while-held continuous-mode dispatch.** Spec line 127 says "Start the buzzer in a continuous mode so it keeps sounding while the button is held." Confirm the dispatch path here matches the regular automatic-buzzer continuous path; if it's a separate code path, the slop-checklist flags re-implementation.

---

## Definition of audit-unit-done

Per the playbook's existing definition: every behaviour in the catalog has a final Decision (no `@proposed` remaining), all `@deleted` code is removed and committed, `just check` passes, every UI `@user_verified` scenario has a test tag in `manual_alarm.feature`, every backend `@user_verified` behaviour has its test status captured in the retroactive ADR, any `@tested_fail` finding is resolved or recorded as an explicit known-defect, the retroactive ADR is written and committed, and the operator has reviewed and signalled approval.

Branch stays local until Final Integration.

---

## Transition to writing-plans

This spec is the input to `superpowers:writing-plans`. The per-unit plan will live at `docs/superpowers/plans/2026-05-13-audit-unit-4-manual-alarm-button.md` and will decompose the 9-step playbook workflow into Unit-4-specific tasks: branch creation, diff generation, catalog draft (with the anticipated B-entry families above as the starting frame), Step 4 review session prep, scenario alignment, three test sessions, retroactive ADR draft, and closeout.
