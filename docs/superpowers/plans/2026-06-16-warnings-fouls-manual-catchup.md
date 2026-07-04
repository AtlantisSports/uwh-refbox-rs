# Warnings-and-Fouls User Manual — Full Catch-Up Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Draft paste-ready Markdown (the manual's terse numbered-callout style) for all 36 entries in the "warnings and fouls" manual, plus a screenshot shot-list, bringing it level with the current shipped app.

**Architecture:** Documentation only — no code. Content is built incrementally into two deliverable files by drafting one section-group per task, each task following the per-screen callout outlines already captured in the spec. A final task stitches the draft into navigation order and compiles the shot-list + renames list.

**Tech Stack:** Markdown. Source of truth = the spec and the `origin/master` @ `7d208fe9` audit it records.

---

## Process notes (lean — documentation)

- **No code, no tests, no commits.** Deliverables are local working docs (same convention as
  `docs/superpowers/specs/`), kept uncommitted. "Verification" per task = a self-check against the
  spec's acceptance criteria (§9), not a test run.
- **Do NOT duplicate final prose into this plan.** The spec (`docs/superpowers/specs/2026-06-16-warnings-fouls-manual-catchup-design.md`)
  already holds the exact labels and callout outline for every screen. Each task turns those
  outlines into the manual's terse paste-ready voice — drafting the prose in the deliverable file,
  not here.
- **Style contract (apply in every task):** mirror the existing manual element-for-element —
  `#` heading per screen, a numbered list where each number = one screenshot callout, indented
  sub-points for "what tapping it does / where it leads", terse imperative voice, exact on-screen
  labels in the callout text, `[Screen Name]`-style cross-references, and a "Prerequisite:" line
  for any conditional screen.
- **Reference, don't re-derive:** all callout outlines and exact labels are in spec §7. If a label
  is unclear, read it from `/tmp/refbox-master-audit/refbox/translations/en-US/refbox.ftl` — do NOT
  read from the current working-directory branch (it is the stale v0.4.1 base).

## Deliverable files

- **Draft (paste-ready manual content):**
  `docs/superpowers/specs/2026-06-16-warnings-fouls-manual-catchup-DRAFT.md`
- **Shot-list:**
  `docs/superpowers/specs/2026-06-16-warnings-fouls-manual-catchup-SHOTLIST.md`

Both are appended to across tasks. Task 1 creates them with a header; Tasks 2–6 append their
sections; Task 7 reorders the draft into the §6 sequence and finalises.

## File map (what each task writes)

| Task | Manual entries drafted | Tag |
|---|---|---|
| 1 | In-Game (added callouts), Score Confirmation, Game Info (added callouts) | ADD/NEW |
| 2 | Settings menu, Game Options (non-portal), Portal Game Options, App Options | REWRITE |
| 3 | Portal Login, Parameter Editor (+2 Halves/1 Period), Parameter Help, User Options, Language | NEW |
| 4 | Display Options, Sound Options (added callout), Manage Remotes | REWRITE/NEW |
| 5 | Updates, Portal Status Detail, Portal Action Required, Confirmation Dialogs | NEW |
| 6 | BeepTest Main, Settings, Sound (cross-ref), Edit Levels, Language (cross-ref) | NEW |
| 7 | Assembly: reorder, KEEP-screen notes, full shot-list, renames-to-confirm, acceptance check | — |

KEEP screens (Pre-Game, Time Edit, Time Edit w/ Timeout, Score, Score Edit, Add Warning, Add Foul,
Warning & Foul Summary, Edit Warnings & Fouls, Penalty Summary, Penalty, Timeout) need no rewrite;
Task 7 records them as "no change" placeholders so the assembled draft maps 1:1 to §6.

---

## Task 1: Game-operation additions

**Files:**
- Create: `docs/superpowers/specs/2026-06-16-warnings-fouls-manual-catchup-DRAFT.md`
- Create: `docs/superpowers/specs/2026-06-16-warnings-fouls-manual-catchup-SHOTLIST.md`
- Read: the spec, §4 (style), §7 entries 2/7/15, §5 (shot-list format)

- [ ] **Step 1: Scaffold the two deliverable files.** Draft header = title, a one-line note that
  this is the catch-up draft against the current app (v0.4.2), and a "How to use" line (paste each
  section under the matching Google-Doc heading). Shot-list header = title + the §5 format reminder.
- [ ] **Step 2: Draft "In-Game Screen — additions".** Following spec §7 entry 2, write the new
  callouts to splice into the existing In-Game entry: the **DELAY** banner (with its prerequisite),
  the portal health dot, and the **ALARM / HOLD TO TEST** manual-alarm button (with prerequisite
  and the spacebar subtitle). Make clear these are *additions* to the existing numbered list.
- [ ] **Step 3: Draft "Score Confirmation Screen" (NEW).** Spec §7 entry 7. Heading + Prerequisite
  line + numbered callouts (banner, score + "is this score correct?" prompt, adjust, confirm, reject).
- [ ] **Step 4: Draft "Game Info Screen — additions".** Spec §7 entry 15. New callouts: **REFRESH**
  (with "REFRESHING…" note, portal mode), the read-only referee-names panel, and the
  **"Game Block: <time>"** line (non-portal).
- [ ] **Step 5: Add shot-list entries** for In-Game (with a behind-schedule DELAY + health dot +
  alarm button visible), Score Confirmation, and Game Info (portal mode so refs + refresh show;
  and a non-portal capture if needed for the Game Block line).
- [ ] **Step 6: Verify** against spec §9: every entry has heading + numbered callouts, exact labels
  used, prerequisites stated, shot-list entries present, voice matches the existing manual.

## Task 2: Core settings rewrite

**Files:**
- Modify: the DRAFT and SHOTLIST files (append)
- Read: spec §7 entries 16/17/18/22, §8 (renames)

- [ ] **Step 1: Draft "Settings Screen" (REWRITE).** Spec §7 entry 16. Callouts: GAME OPTIONS,
  APP OPTIONS, USER OPTIONS, LANGUAGE, BACK + the reader note that game-number moved into Game
  Options and Display/Sound moved under User Options.
- [ ] **Step 2: Draft "Game Options Screen" (REWRITE, renamed from "Tournament Options").** Spec §7
  entry 17. Keep the terse treatment; add the **GAME BLOCK** callout (renamed Nominal Break;
  too-short/tight warnings), the note that Half Length / Game Length opens the Parameter Editor
  carrying the **2 Halves / 1 Period** choice, and the **USING UWH PORTAL** switch. Add a one-line
  "(was Tournament Options)" note for the user.
- [ ] **Step 3: Draft "UWH/UWR Portal Game Options Screen" (REWRITE).** Spec §7 entry 18. Keep
  switch-to-not-portal + event/court/game selection + apply; add the **token status**
  (OK / FAILED / CHECKING…) callout that taps to Portal Login.
- [ ] **Step 4: Draft "App Options Screen" (REWRITE).** Spec §7 entry 22. All eight callouts:
  **APP MODE** (HOCKEY6V6 / HOCKEY3V3 / RUGBY / BEEP TEST), TRACK CAP NUMBER OF SCORER, TRACK FOULS
  AND WARNINGS, CONFIRM SCORE AT GAME END, SHOW BEHIND TIME/DELAY, CANCEL, **Check Version** (opens
  Updates; disabled during a game), APPLY.
- [ ] **Step 5: Add shot-list entries** for the Settings menu, Game Options (non-portal, with Game
  Block visible), Portal Game Options (portal mode, token status visible), App Options.
- [ ] **Step 6: Verify** against spec §9.

## Task 3: Settings sub-screens & editors

**Files:**
- Modify: the DRAFT and SHOTLIST files (append)
- Read: spec §7 entries 19/20/21/24/28

- [ ] **Step 1: Draft "Portal Login Screen" (NEW).** Spec §7 entry 19. Prerequisite + callouts
  (Refbox ID read-only, login-code keypad, cancel, done).
- [ ] **Step 2: Draft "Parameter Editor Screen" (NEW).** Spec §7 entry 20. Include the
  **2 HALVES / 1 PERIOD** selector callout (what each option means), the title, the **?** help
  button, the time keypad, cancel/done, and the Game Block red/yellow validation note.
- [ ] **Step 3: Draft "Parameter Help Screen" (NEW).** Spec §7 entry 21. Banner, help text, back.
- [ ] **Step 4: Draft "User Options Screen" (NEW).** Spec §7 entry 24. DISPLAY OPTIONS,
  SOUND OPTIONS, **VIEW MODE** (LIGHT / DARK / HIGH CONTRAST), back.
- [ ] **Step 5: Draft "Language Screen" (NEW).** Spec §7 entry 28. 15 language buttons, the
  "(unverified)" note, cancel, DONE / **RESTART TO APPLY**.
- [ ] **Step 6: Add shot-list entries** for Portal Login, Parameter Editor (capture the Half Length
  variant so the 2 Halves / 1 Period selector shows), Parameter Help, User Options, Language.
- [ ] **Step 7: Verify** against spec §9.

## Task 4: Display, Sound, Remotes

**Files:**
- Modify: the DRAFT and SHOTLIST files (append)
- Read: spec §7 entries 25/26/27

- [ ] **Step 1: Draft "Display Options Screen" (REWRITE).** Spec §7 entry 25. All callouts:
  STARTING SIDES, HIDE TIME FOR LAST 15 SECONDS, **DISPLAY LAYOUT** (DEFAULT / CLASSIC / BIG TIME /
  CORNERS / SCORES ONLY; disabled with LED panel), **OPEN NEW DISPLAY** (disabled with LED panel),
  **PLAYER DISPLAY BRIGHTNESS** (LOW / MEDIUM / HIGH / OUTDOOR; only active with LED panel), the
  live layout preview image, cancel, apply.
- [ ] **Step 2: Draft "Sound Options Screen — additions".** Spec §7 entry 26. Add the **ALARM
  BUTTON** toggle callout and note that **MANAGE REMOTES** now opens its own screen. Mark as
  additions to the existing entry.
- [ ] **Step 3: Draft "Manage Remotes Screen" (NEW).** Spec §7 entry 27. Per-remote ID, **SOUND**
  cycle, **DELETE**, the **ADD / WAITING** pairing button, cancel, apply.
- [ ] **Step 4: Add shot-list entries** for Display Options (capture without an LED panel so the
  layout/brightness controls' enabled/disabled states can be noted), Sound Options (alarm-button
  row visible), Manage Remotes (with at least one paired remote).
- [ ] **Step 5: Verify** against spec §9.

## Task 5: Self-update + portal troubleshooting

**Files:**
- Modify: the DRAFT and SHOTLIST files (append)
- Read: spec §7 entries 23/29/30/31

- [ ] **Step 1: Draft "Updates Screen" (NEW).** Spec §7 entry 23. All seven callouts (banner,
  current version + Check for Updates, status line, note line, Revert-to-Previous-Version, BACK/
  CANCEL behaviour, Install/Revert action) plus the auto-revert-on-next-boot note.
- [ ] **Step 2: Draft "Portal Status Detail Screen" (NEW).** Spec §7 entry 29. Health dot, the
  colour-coded queue list (red/yellow/green meanings), scroll, tap-red-row → Action Required, back.
- [ ] **Step 3: Draft "Portal Action Required Screen" (NEW).** Spec §7 entry 30. Game/score/problem,
  **Force This Game Result**, **Discard This Submission** (two-tap), go-to-login, back.
- [ ] **Step 4: Draft "Confirmation Dialogs" (NEW, combined page).** Spec §7 entry 31. Document each
  variant and its buttons as a single page.
- [ ] **Step 5: Add shot-list entries** for Updates (ideally one "update available" capture showing
  Install + status), Portal Status Detail (with a stuck red row), Portal Action Required, and the
  Confirmation Dialogs (note which variant each capture shows).
- [ ] **Step 6: Verify** against spec §9.

## Task 6: BeepTest mode

**Files:**
- Modify: the DRAFT and SHOTLIST files (append)
- Read: spec §7 entries 32/33/34/35/36, §11 (cross-reference recommendation)

- [ ] **Step 1: Draft "BeepTest Main Screen" (NEW).** Spec §7 entry 32. Prerequisite (App Mode =
  BEEP TEST) + callouts: TIME/LEVEL/LAP row, levels table, RESET, SETTINGS, START→PAUSE/RESUME.
- [ ] **Step 2: Draft "BeepTest Settings Screen" (NEW).** Spec §7 entry 33. SOUND SETTINGS,
  EDIT LEVELS, APP MODE, LANGUAGE, BACK, RESTART TO APPLY.
- [ ] **Step 3: Draft "BeepTest Sound Settings Screen" (NEW, cross-ref).** Spec §7 entry 34 + §11.
  One-line entry that points to the main Sound Options screen for the control descriptions; note
  cancel/save.
- [ ] **Step 4: Draft "BeepTest Edit Levels Screen" (NEW).** Spec §7 entry 35. Levels list, +/− for
  count & duration, NEW, DELETE, cancel, save; note disabled-while-running.
- [ ] **Step 5: Draft "BeepTest Language Screen" (NEW, cross-ref).** Spec §7 entry 36 + §11. One-line
  entry pointing to the Language screen; cancel/done(or restart).
- [ ] **Step 6: Add shot-list entries** for BeepTest Main, Settings, Edit Levels (Sound/Language can
  reuse the main captures — note this).
- [ ] **Step 7: Verify** against spec §9.

## Task 7: Assembly + final pass

**Files:**
- Modify: the DRAFT and SHOTLIST files
- Read: spec §6 (order), §8 (renames), §9 (acceptance)

- [ ] **Step 1: Reorder the draft** so its sections run in the exact §6 sequence (entries 1–36).
- [ ] **Step 2: Insert KEEP-screen markers** for the 12 unchanged screens, each a one-line "No
  change — existing entry stands" note, so the assembled draft maps 1:1 to the §6 list and the user
  can see nothing was dropped.
- [ ] **Step 3: Compile the renames-to-confirm list** (spec §8) at the top of the draft: Tournament
  Options → Game Options; new User Options section; UWH Portal as portal-mode of Game Options.
- [ ] **Step 4: Finalise the shot-list** — confirm every NEW / REWRITE / ADD-CALLOUTS screen has an
  entry with setup state, and group it in §6 order.
- [ ] **Step 5: Run the acceptance checklist (spec §9) over the whole draft** and fix any gaps
  inline. Confirm: all 36 entries present, exact labels throughout, prerequisites stated, voice
  consistent.
- [ ] **Step 6: Hand off** — tell the user the draft + shot-list paths, that they're uncommitted
  working docs, and that screenshots require a current master/v0.4.2 build (offer to build + launch
  it). Record any deviations in the spec §11, not in standalone commits.

---

## Self-review (done at plan-write time)

- **Spec coverage:** every §6 entry (1–36) maps to a task in the file map; KEEP screens handled in
  Task 7. Shot-list (§5) and renames (§8) covered in Tasks 1–7. Acceptance (§9) checked per task +
  in Task 7.
- **Placeholders:** none — each task names exact files, exact spec sections, exact screens and
  labels to draft. Prose is intentionally produced in the deliverable (not duplicated here) per the
  lean-process note; this is a deliberate doc-task adaptation, not a placeholder gap.
- **Consistency:** deliverable file paths, entry numbers, and screen names match the spec
  throughout.
