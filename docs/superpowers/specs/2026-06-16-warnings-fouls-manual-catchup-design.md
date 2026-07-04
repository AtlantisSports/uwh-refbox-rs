# Design — "Warnings and Fouls" User Manual: full catch-up against current app

- **Date:** 2026-06-16
- **Status:** Design / spec (awaiting user review before writing-plans)
- **Reference build:** `origin/master` @ `7d208fe9` (the v0.4.2 base). Audited via a read-only
  worktree at `/tmp/refbox-master-audit`.
- **Input manual:** `/mnt/c/Users/Eric/Downloads/Refbox user manual, warnings and fouls.md`
  (exported from the styled Google Doc; ~21 screens, terse numbered-callout style).

> Note: this session opened on branch `feat/refbox/time-golden-trace-spike`, which is built on
> the **v0.4.1** release and is **139 commits behind** master. An initial audit accidentally ran
> against that stale tree and reported new features as "missing." The audit was re-run against
> `origin/master` (commit `7d208fe9`); everything in this spec reflects the **current shipped app**.

---

## 1. Goal

Bring the "warnings and fouls" variant of the refbox User Manual fully up to date with the app's
**complete current functionality** (not limited to v0.4.2's headline features). Produce, in the
manual's existing screen-by-screen numbered-callout style:

1. **Paste-ready Markdown** for every new screen and every changed screen, plus added callouts on
   screens that gained features.
2. A **screenshot shot-list**: for each screen needing a new/updated capture, the state to set the
   app into first and the numbered callouts the screenshot must show.

The user assembles the text into the styled Google Doc and captures the screenshots. Claude drafts
only; Claude does not edit the Doc, the PDFs, or capture screenshots.

This is a **v0.4.2 publish gate**.

## 2. Scope

**In scope (this pass):** the full "warnings and fouls" manual — all game-operation screens, all
settings screens (including the reorganized menu), the new self-update flow, portal-troubleshooting
screens, confirmation dialogs, and **the BeepTest mode screens** (user chose the most exhaustive
coverage on 2026-06-16).

**Explicitly NOT doing:**
- The other manual variants — Getting Started, Spanish, Providing App Credentials, and the
  non-fouls variant — are separate follow-ups.
- Editing the Google Doc, the exported PDFs, or any styling; capturing screenshots.
- Any code change to the refbox app. This is documentation only.
- Re-auditing the stale `time-golden-trace-spike` branch.

## 3. Source of truth

All exact on-screen labels, navigation paths, and conditional-visibility rules come from the
audit of `origin/master` @ `7d208fe9`. Key files (paths under `/tmp/refbox-master-audit`):
`refbox/src/app/view_builders/` (per-screen builders), `configuration.rs` (all settings screens +
Updates page), `main_view.rs` / `shared_elements.rs` (banners, health tile, alarm),
`game_info.rs` (referee names, Game Block info), `beep_test*.rs`,
`refbox/translations/en-US/refbox.ftl` (label text), `refbox/src/config.rs` /
`uwh-common/src/config.rs` (option sets).

For screenshots, the operator must run a **master/v0.4.2 build** — the current working directory's
branch (v0.4.1) will NOT show the new screens. The `/tmp/refbox-master-audit` worktree (or a fresh
master checkout) is the correct source to build and launch.

## 4. Output format & style rules

Mirror the existing manual element-for-element:

- One `#` heading per screen, optionally with a Google-Doc anchor `{#anchor}` (the user manages
  anchors in the Doc; Claude provides plain headings and may suggest anchors).
- A numbered list under each heading. **Each number = one callout on the screenshot.** Sub-points
  (`1.`, `2.` indented) describe what tapping that callout does / where it leads, exactly as the
  current manual does.
- Terse, imperative voice. No programmer terms. Match the density of the existing entries.
- Use the **exact on-screen label** in the callout text (e.g. *"Clicking **Check Version** opens
  the Updates Screen"*).
- Cross-reference other screens by name, mirroring the existing `[Screen Name](#anchor)` links.
- For any screen or control that only appears under a condition, state the prerequisite plainly
  (e.g. *"Only shown when **Track Fouls and Warnings** is on"*).

**Template per screen:**

```
# <Screen Name>

Prerequisite (if any): <state needed to reach/see this screen>

1) <what callout 1 is> <what it does / where it goes>
2) <callout 2> ...
```

## 5. Shot-list format

One block per screen that needs a new or updated capture:

```
## <Screen Name>
- Setup: <how to reach it + any toggles/state to set first>
- Capture so these are visible: <list of the numbered callouts/elements>
- Notes: <anything that must be on-screen, e.g. "with a behind-schedule DELAY showing">
```

## 6. The manual's new structure (ordered, tagged)

Order follows navigation flow, matching how the existing manual reads. Tags:
**KEEP** (no change), **ADD CALLOUTS** (existing screen gained elements), **REWRITE** (structure or
options changed materially), **NEW** (no page exists today).

**Game operation**
1. Pre-Game Screen — KEEP (minor)
2. In-Game Screen — ADD CALLOUTS
3. Time Edit Screen — KEEP
4. Time Edit With Timeout Screen — KEEP
5. Score Screen (add-score keypad) — KEEP
6. Score Edit Screen — KEEP
7. Score Confirmation Screen — **NEW**
8. Add Warning Screen — KEEP
9. Add Foul Screen — KEEP
10. Warning and Foul Summary Screen — KEEP
11. Edit Warnings and Fouls Screens — KEEP
12. Penalty Summary Screen — KEEP
13. Penalty Screen — KEEP
14. Timeout Screens — KEEP

**Game info & settings**
15. Game Info Screen — ADD CALLOUTS
16. Settings Screen (menu) — REWRITE
17. Game Options Screen (was "Tournament Options") — REWRITE
18. UWH/UWR Portal Game Options Screen — REWRITE
19. Portal Login (code entry) Screen — **NEW**
20. Parameter Editor Screen (incl. 2 Halves / 1 Period) — **NEW**
21. Parameter Help Screen ("?") — **NEW**
22. App Options Screen — REWRITE
23. Updates Screen — **NEW**
24. User Options Screen — **NEW**
25. Display Options Screen — REWRITE
26. Sound Options Screen — ADD CALLOUTS
27. Manage Remotes Screen — **NEW**
28. Language Screen — **NEW**

**Portal troubleshooting**
29. Portal Status Detail Screen — **NEW**
30. Portal Action Required Screen — **NEW**
31. Confirmation Dialogs (combined page) — **NEW**

**BeepTest mode**
32. BeepTest Main Screen — **NEW**
33. BeepTest Settings Screen — **NEW**
34. BeepTest Sound Settings Screen — **NEW**
35. BeepTest Edit Levels Screen — **NEW**
36. BeepTest Language Screen — **NEW**

## 7. Per-screen callout outlines

These are the callout outlines to draft from. Exact labels are in **bold**. The executing draft
turns each into the manual's terse paste-ready prose.

### 2. In-Game Screen — ADD CALLOUTS
Keep existing callouts 1–8. Add:
- **DELAY** banner: red "DELAY" label + a negative time on the time banner. Shows only when
  **Show Behind Time/Delay** is on, the game is behind schedule, and no timeout is active.
- Portal health dot (green / yellow / red) on the banner — present when connected to the portal
  with an event linked; tap opens the **Portal Status Detail** screen.
- Manual alarm button: shows **ALARM** during play (tap fires the buzzer) or **HOLD TO TEST**
  during breaks/timeouts (hold to test); subtitle "Or Press Spacebar" / "Or Hold Spacebar".
  Present only when **Alarm Button** is enabled in Sound Options.

### 7. Score Confirmation Screen — NEW
Prerequisite: **Confirm Score at Game End** on (also appears when a goal is added in sudden death).
1) Time banner. 2) Large score with an "is this score correct?" prompt. 3) Score adjust (+/−).
4) Confirm (green). 5) Reject / go back (red).

### 15. Game Info Screen — ADD CALLOUTS
Keep "complete list of game details", BACK, SETTINGS. Add:
- **REFRESH** button (portal mode; shows "REFRESHING…" while loading the schedule).
- Referee names panel (read-only, portal mode): Chief Ref / Timer / Water Ref 1 / Water Ref 2 /
  Water Ref 3; unassigned slots show "-".
- Game Block line: **"Game Block: <time>"** (non-portal mode only).

### 16. Settings Screen (menu) — REWRITE
1) Time banner. 2) **GAME OPTIONS**. 3) **APP OPTIONS**. 4) **USER OPTIONS**. 5) **LANGUAGE**.
6) **BACK**. Note for the reader: the game-number entry moved into **Game Options**; Display and
Sound options moved under **User Options**.

### 17. Game Options Screen (non-portal) — REWRITE
Keep the existing terse treatment ("all the game timing options are set here", switch-to-portal,
save). Add/clarify:
- **GAME BLOCK** — time from one game's start to the next (the renamed "Nominal Break"). Opens the
  Parameter Editor; warns when too short or tight.
- Tapping **Half Length** / **Game Length** opens the Parameter Editor, which carries the
  **2 Halves / 1 Period** choice.
- **USING UWH PORTAL** toggle switches to the portal version of this screen.

### 18. UWH/UWR Portal Game Options Screen — REWRITE
Keep: switch-to-not-portal, select event/tournament, select court, select game, save. Add:
- Portal **token status** indicator (OK / FAILED / CHECKING…); tapping it (when an event is
  selected) opens the Portal Login code-entry screen.

### 19. Portal Login (code entry) Screen — NEW
Prerequisite: portal mode, event selected, token status tapped.
1) Refbox ID (read-only). 2) Login-code keypad entry. 3) Cancel. 4) Done/submit.

### 20. Parameter Editor Screen — NEW
Prerequisite: Game Options → tap a time field.
1) Time banner. 2) **For Half Length only:** the **2 HALVES** / **1 PERIOD** selector (highlighted
   button = active choice; **1 PERIOD** = one continuous period, no half-time; **2 HALVES** =
   two halves with a half-time break). 3) Parameter title. 4) **?** help button → Parameter Help.
5) Time keypad with +/−. 6) Cancel. 7) Done. Note: when editing **Game Block**, a too-short value
   shows red and disables Done; a tight value shows yellow.

### 21. Parameter Help Screen — NEW
1) Time banner. 2) Help text for the parameter being edited. 3) Back.

### 22. App Options Screen — REWRITE
1) **APP MODE** — cycles **HOCKEY6V6 / HOCKEY3V3 / RUGBY / BEEP TEST**. 2) **TRACK CAP NUMBER OF
   SCORER**. 3) **TRACK FOULS AND WARNINGS**. 4) **CONFIRM SCORE AT GAME END**. 5) **SHOW BEHIND
   TIME/DELAY**. 6) **CANCEL**. 7) **Check Version** (blue; opens Updates Screen; disabled while a
   game is in progress). 8) **APPLY**.

### 23. Updates Screen — NEW
Prerequisite: App Options → Check Version.
1) Time banner. 2) Current version + **Check for Updates** button. 3) Status line (Checking… /
   Up to date. / Update available: <ver> / Downloading… / Checking the download… / Installing… /
   Restarting… / error messages). 4) Note line (e.g. "Clicking install will download, install, and
   restart the refbox"). 5) **Revert to Previous Version (<ver>)** button — shown when a backup
   exists. 6) **BACK** / **CANCEL** (red; disabled during Installing/Restarting). 7) Action button:
   **Install** (when an update is available) or **Revert** (when confirming a revert). Note: if an
   update fails to start, the app auto-reverts and reopens this screen on next boot with "Reverted
   to the previous version because the update didn't start correctly, please try again."

### 24. User Options Screen — NEW
1) **DISPLAY OPTIONS**. 2) **SOUND OPTIONS**. 3) **VIEW MODE** — cycles **LIGHT / DARK / HIGH
   CONTRAST** (applies immediately). 4) Back.

### 25. Display Options Screen — REWRITE
1) **STARTING SIDES** (swap White/Black sides). 2) **HIDE TIME FOR LAST 15 SECONDS**.
3) **DISPLAY LAYOUT** — cycles **DEFAULT / CLASSIC / BIG TIME / CORNERS / SCORES ONLY** (disabled
   when a physical LED panel is connected). 4) **OPEN NEW DISPLAY** (opens a preview window;
   disabled when an LED panel is connected). 5) **PLAYER DISPLAY BRIGHTNESS** — cycles **LOW /
   MEDIUM / HIGH / OUTDOOR** (only active when an LED panel is connected). 6) Live layout preview
   image. 7) Cancel. 8) Apply.

### 26. Sound Options Screen — ADD CALLOUTS
Keep existing callouts. Add:
- **ALARM BUTTON** toggle (enables the manual alarm button on the In-Game screen).
- **MANAGE REMOTES** now opens a dedicated screen (was described inline).

### 27. Manage Remotes Screen — NEW
Prerequisite: Sound Options → Manage Remotes.
1) Per-remote row: ID (read-only). 2) Per-remote **SOUND** (cycle buzzer sound). 3) **DELETE**.
4) **ADD** / **WAITING** (pair a new remote). 5) Cancel. 6) Apply.

### 28. Language Screen — NEW
1) 15 language buttons (currently-selected highlighted; some show an "(unverified)" note in their
   own script). 2) Cancel. 3) **DONE** — or **RESTART TO APPLY** when the change needs a different
   font family.

### 29. Portal Status Detail Screen — NEW
Prerequisite: tap the portal health dot.
1) Health dot in banner. 2) Scrollable list of queued game results, colour-coded: red =
   token-expired or stuck, yellow = pending, green = recently submitted. 3) Scroll controls.
4) Tapping a red row opens **Portal Action Required**. 5) Back.

### 30. Portal Action Required Screen — NEW
1) Game number + score + problem description. 2) **Force This Game Result** (green). 3) **Discard
   This Submission** (red; two taps to confirm). 4) (If token expired) go-to-login. 5) Back.

### 31. Confirmation Dialogs — NEW (one combined page)
Document the safety prompts as a single page with the variants:
- Game config changed mid-game: **Go Back to Editor / Discard Changes / End Current Game and Apply
  Changes**.
- Game number changed mid-game: adds **Keep Current Game and Apply Change**.
- Switching Hockey ⇄ Rugby (restarts app): **Cancel / Restart to Apply**.
- Portal setup incomplete; invalid login code; generic error: **OK**.

### 32. BeepTest Main Screen — NEW
Prerequisite: App Mode = **BEEP TEST**.
1) Info row: **TIME / LEVEL / LAP**. 2) Levels table (active cell highlighted). 3) **RESET**
   (enabled after first start). 4) **SETTINGS**. 5) **START** → **PAUSE** / **RESUME**.

### 33. BeepTest Settings Screen — NEW
1) **SOUND SETTINGS**. 2) **EDIT LEVELS** (disabled while running). 3) **APP MODE** (switch back to
   a game mode). 4) **LANGUAGE**. 5) **BACK**. 6) **RESTART TO APPLY** (appears when the mode was
   changed).

### 34. BeepTest Sound Settings Screen — NEW
Same controls as the main Sound Options screen (Sound Enabled, Whistle Enabled, Buzzer Sound,
volumes). May cross-reference Sound Options rather than re-list every control. Cancel / Save.

### 35. BeepTest Edit Levels Screen — NEW
1) Levels list (count and duration per level). 2) +/− for count and duration. 3) **NEW** level.
4) **DELETE** level. 5) Cancel. 6) Save. (Disabled while a test is running.)

### 36. BeepTest Language Screen — NEW
Same as the Language screen. Cancel / Done (or Restart to Apply).

## 8. Heading renames to confirm at review

The reader sees the app's labels, so the manual should match. Proposed alignments (each can be
vetoed):
- "Tournament Options Screen" → **"Game Options Screen"** (button is now **GAME OPTIONS**).
- Add a new **"User Options Screen"** section (Display/Sound moved under it).
- Keep "UWH Portal Screen" but present it as the portal mode of Game Options.
- Keep "Settings Screen", "Score Screen", "Time Edit Screen" names (still recognisable), rewriting
  contents where noted.

## 9. Acceptance criteria

- Every screen in §6 has a heading + numbered callouts in the draft (NEW + REWRITE fully written;
  ADD CALLOUTS lists the additions to splice into the existing entry).
- All callouts use the exact on-screen label text from the reference build.
- Every conditional screen/control states its prerequisite.
- A shot-list entry exists for every NEW / REWRITE / ADD CALLOUTS screen, with setup state.
- Voice and density match the existing manual.
- A short "renames to confirm" list accompanies the draft.

## 10. Task breakdown sketch (for writing-plans)

Lean process (documentation). Suggested chunks, each producing paste-ready text + its shot-list
entries:
1. Game-operation additions — In-Game callouts, Score Confirmation, Game Info callouts.
2. Settings restructure — Settings menu, Game Options, App Options, User Options, Display Options,
   Sound Options.
3. New settings sub-screens — Updates, Parameter Editor + Help, Manage Remotes, Language,
   Portal Login.
4. Portal troubleshooting — Status Detail, Action Required, Confirmation Dialogs.
5. BeepTest — five screens.
6. Assembly — stitch into a single paste-ready document in §6 order, compile the full shot-list,
   and the renames-to-confirm list.

## 11. Open questions / deviations

- Heading renames (§8) — confirm at spec review.
- BeepTest Sound/Language screens (§34, §36) — cross-reference the main screens vs. fully re-list.
  Recommendation: cross-reference with a one-line note, to keep the manual lean.
- (Record any execution deviations here rather than in standalone commits.)
