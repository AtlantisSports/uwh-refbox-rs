# ADR 005 — v0.4.0 Feature Audit

**Status:** Phase 2 complete — scope confirmed by human review 2026-04-17
**Date:** 2026-04-17
**Branch:** `feat/workspace/desktop-build`

---

## Summary Table

| # | Feature | Include in v0.4.0? | Concern Level | Notes |
|---|---|---|---|---|
| 1 | Confirm-score timing fix | **YES** | LOW | Binary sent to Henk (Apr 10) did not include this fix — he needs the proper release |
| 2 | RPi CI cross-compile fix | **YES** | LOW | — |
| 3 | ListOfPlacements data type | **YES** | LOW | Includes a single-half bug fix bundled in |
| 4 | Referee info display | **YES** | LOW | Role strings confirmed current — Eric controls the portal API |
| 5 | Scoresheet generation | **OUT OF SCOPE** | — | Not part of the refbox software; supporting tool only |
| 6 | Manual alarm button | **YES** | LOW | — |
| 7 | UI text clipping fixes | **YES** | LOW | — |
| 8 | Language support (10 languages + CJK/Thai fonts) | **YES** | MEDIUM | Translation accuracy not reviewed by native speakers — backlog item |
| 9 | CI release pipeline fixes | **YES** | LOW | — |
| 10 | Security update (rustls-webpki) | **YES** | LOW | One advisory (time crate) deferred — needs Rust 1.88 |
| 11 | Sound artifacts fix | **YES** | LOW | — |
| 12 | Version bump to 0.4.0 | **YES** | LOW | Must go last |

---

## Detailed Audit Entries

---

### 1. Confirm-Score Timing Fix

**Commit:** `f042e96`
**Files:** `refbox/src/tournament_manager/mod.rs`

**What the bug was:**
When the refbox is configured with "confirm scores" turned off, the game could end up in a
half-finished state internally. After the second half ended, the game's internal "waiting for
score confirmation" flag was never properly cleared. About 90 seconds later, an internal timer
fired and discovered the game was in an unexpected state — and rather than handling it gracefully,
it crashed the application. The refbox then refused to work until it was restarted.

**What the operator would have seen:**
The game would appear to end normally. But roughly 90 seconds later, the refbox would become
unresponsive — buttons would stop working. The only recovery was to close and reopen the app.
This was observed 6 times at real tournaments (January 13, January 19, and February 24, 2026).

**What the fix does:**
Two changes were made:
1. The primary fix ensures the "waiting for score confirmation" flag is always cleared before
   the game clock is released to start the next period.
2. A defensive fix replaces the crash with a warning message. If the same unexpected state is
   ever reached again (for any reason), the app will log it and continue rather than crash.

**Does it have a test?**
The existing test suite covers the score confirmation pause mechanism, but no test specifically
exercises the `confirm_score = false` path that caused this crash. The crash path was reached
by a timer firing in an unexpected game state — this is very difficult to unit-test, and none
was added with the fix.

**Concern level: LOW**
The fix is well-targeted and has been verified against the exact crash sequence that was
observed in production. The defensive fallback provides additional protection going forward.

---

### 2. RPi CI Cross-Compile Fix

**Commit:** `527953d`
**Files:** `Cross.toml`, `Dockerfile.aarch64-unknown-linux-gnu`

**What broke:**
The automated process that builds the Raspberry Pi version of the software was failing. The
problem was that the build environment (a pre-built Docker image) was running a very old version
of Ubuntu (16.04, from 2016). Modern Rust build tools require a newer version of a core system
library (`glibc 2.34+`), and the old image only had version 2.23.

**How it was fixed:**
The old pre-built image was replaced with a custom build recipe using Ubuntu 22.04. This newer
environment includes the required system libraries and the correct cross-compilation tools for
the Raspberry Pi's ARM processor. The new Dockerfile also correctly sets up the audio and TLS
libraries needed by the refbox.

**Concern level: LOW**
Infrastructure-only change. No application code changed. The fix is standard cross-compilation
setup for modern Ubuntu.

---

### 3. ListOfPlacements Data Type

**Commit:** `c8478e7`
**Files:** `uwh-common/src/uwhportal/schedule.rs`, `uwh-common/src/uwhportal/mod.rs`

**What this adds:**
The portal API can describe how final tournament standings are determined in different ways.
Previously, the software only understood standings determined by game results (who won which
game). This adds support for a second format: standings determined by a ranked list of placed
positions, where each position can come from either a game result or a seeded position within
a group.

In plain terms: some tournaments use a format where the final 3rd-place team is "whoever
finished 1st in Group B," rather than "the winner of Game 42." This new data type lets the
software parse and store that information correctly.

**Bundled bug fix:**
The same commit also corrects a mistake in how "single half" games are detected: the code was
checking whether the playing time was zero (which never happens), when it should have been
checking whether the halftime break duration is zero (which defines a single-half game). This
was a silent bug that would have caused single-half game timing to be set up incorrectly.

**What uses it:**
The `schedule-processor` crate references `ListOfPlacements` in its schedule validation logic.
No other crates currently use it.

**Concern level: LOW**
Small, well-contained addition. The bundled timing bug fix is a positive bonus. No downstream
crates are broken.

---

### 4. Referee Info Display

**Commits:** `2ff03d9`, `58a29f4`, `6285e5b`, `6074eea`
**Files:** `refbox/src/app/view_builders/game_info.rs`, `refbox/src/app/view_builders/shared_elements.rs`, `uwh-common/src/uwhportal/schedule.rs`, `refbox/src/app/mod.rs`

**What this adds:**
The game info screen now shows real referee names instead of blank fields. When the refbox loads
a schedule from the portal, it also fetches the event's referee list and matches names to the
role assignments for each game. Roles shown are: Chief, Water 1, Water 2, Water 3, and
Timekeeper.

The display falls back gracefully:
- If the portal is unreachable: no names are shown (same as before this feature)
- If a name can't be resolved for an assignment: the assignment identifier is shown instead
- If no individual refs are assigned (only team refs): team referee info is shown instead

**Correctness issue discovered and fixed:**
The initial implementation guessed at the portal's role name strings ("Chief Ref", "Water Ref 1",
etc.). These were all wrong — the actual values from the portal are "Chief", "Water1", "Water2",
"Water3", "TimeOrScoreKeeper". This caused every role to show as "Unknown" regardless of what
the portal assigned. The second commit (`58a29f4`) corrected all the strings.

Additionally, the first version of the referee name lookup parsed the `/referees` API response
incorrectly (expecting a flat array, but the actual response is a nested object). This was fixed
in `6074eea`, verified against a live Women's Tournament event where 14 referee names resolved
correctly.

**Concern level: MEDIUM**
The feature works correctly in its final state, but it took four commits to get there including
two correctness fixes. The final state has been verified against live portal data.

> ⚠️ **FLAG:** Before merging, confirm that the role strings "Chief", "Water1", "Water2",
> "Water3", "TimeOrScoreKeeper" are still current and match what the portal sends in 2026.
> These are hard-coded strings matching portal API values — if the portal changes them, the
> display silently falls back to "Unknown" for all roles.

---

### 5. Scoresheet Generation

**Commits:** `7182349` through `1f3ed9b` (9 commits in `schedule-processor/`)
**Files:** `schedule-processor/src/scoresheets.rs`, `schedule-processor/src/main.rs`, ADR 003, ADR 004

**What this adds:**
The `schedule-processor` command-line tool can now generate scoresheets for an entire
tournament from a schedule fetched from the portal. Output formats are:
- PDF scoresheets (requires Chrome/Chromium installed on the machine running the tool)
- XLSX files for custom user-designed templates (token substitution into Excel files)

The architecture is documented in ADRs 003 and 004. Key design decisions:
- All scoresheet data is collected into a `GameRenderContext` struct before rendering
- Each built-in style is a pure function: receives context, returns HTML
- Page size is standardised on A4
- XLSX templates use `{token_name}` placeholders in cell values

**External dependencies:**
- Requires an authenticated portal API token to fetch the schedule
- Referee names are fetched from a second portal endpoint (`/referees`)
- PDF generation requires Chrome or Chromium installed locally
- If any network calls fail, the tool falls back gracefully (blank referee fields, team IDs
  instead of team names)

**Test coverage:**
Integration tests exist in `schedule-processor/tests/scoresheets_integration.rs`. They use a
local mock JSON schedule and a dead portal client, so they test the full generation pipeline
without any network dependency. PDF generation is skipped if Chrome is not installed but the
test still passes.

**Open items (by design):**
PDF generation from XLSX templates is deliberately deferred until the portal integration
target is better defined (per ADR 004).

**Concern level: MEDIUM**
Feature is large (9 commits, ~2500+ line file) and complex, but the ADRs document the
architecture clearly and integration tests cover the core scenarios. The XLSX PDF path is
an explicitly deferred future item, not a gap.

---

### 6. Manual Alarm Button

**Commits:** `26e6f09` through `abdc159` (approximately 12 commits in `refbox/`)
**Files:** `refbox/src/app/mod.rs`, `refbox/src/app/view_builders/main_view.rs`, `refbox/src/sound_controller/mod.rs`, `refbox/src/app/message.rs`

**What this adds:**
An on-screen "Alarm" button appears on the main game screen when the feature is enabled in
Sound settings. The button fires the buzzer sound immediately during active play. During the
Between Games period, the button is blue and labelled "HOLD TO TEST" — the operator must hold
it for 1 second before the sound plays. The spacebar provides the same function as the button.

The button label reads "Alarm" with "Or Press Spacebar" below it, so the keyboard shortcut is
visible to the operator without needing to read any documentation.

**Feature is opt-in:**
The setting `manual_alarm_enabled` defaults to `false`. The button only appears when the
operator has turned it on in Sound settings. This is the correct default — operators who don't
need the feature won't see any change.

**Migration:**
Settings files that don't contain `manual_alarm_enabled` (i.e., all existing installations)
will default the field to `false` on load. A migration test was added (`b9cc613`) to confirm this.

**Spacebar conflict check:**
The spacebar is wired via iced's keyboard subscription. During normal game operation, the main
view has no text input fields, so no other widget should consume the spacebar. The risk of
conflict is low but has not been formally tested.

**Concern level: LOW**
Feature is well-scoped, opt-in, migration-tested, and the keyboard shortcut is self-documenting
in the UI.

---

### 7. UI Text Clipping Fixes

**Commits:** `cd577b2`, `2749104`
**Files:** `refbox/src/app/view_builders/shared_elements.rs`, `refbox/src/app/view_builders/keypad_pages/mod.rs`

**Bug 1 — Keypad player number (`cd577b2`):**
On the keypad page (used when entering a player number for a foul or penalty), the player's
current number wasn't rendering correctly. The cause was an iced 0.13 rendering limitation:
short text strings don't render when `align_x(Right)` is combined with `width(Fill)`. The fix
replaces that combination with a `Space` widget to push the number to the right, which works
reliably.

**Bug 2 — Multi-label button text (`2749104`):**
Buttons that show two lines of text (used on several state-transition screens) were clipping
the text after a game state change. The cause was iced 0.13's paragraph position caching: when
the widget was re-rendered after a state change, the cached text position was stale, placing
the text outside its visible area. The fix uses a container to handle centering structurally,
which keeps each widget's paragraph anchor within its own bounds.

**Concern level: LOW**
Both are targeted workarounds for known iced 0.13 rendering limitations. No logic changed.

---

### 8. Language Support (10 Languages + CJK and Thai Fonts)

**Commits:** `9ea795b` through `e33ccfa` (7+ commits including 2 reverts)
**Files:** `refbox/src/app/languages.rs`, `refbox/src/app/view_builders/configuration.rs`, `refbox/resources/NotoSansCJK-Subset.otf`, `refbox/resources/NotoSansThai-Subset.ttf`, translation files

**What this adds:**
The refbox now supports 14 languages (up from 3):

| Existing | Added |
|---|---|
| English | Mandarin (Simplified Chinese) |
| Spanish | Korean |
| French | Italian |
| | German |
| | Tagalog |
| | Indonesian |
| | Dutch |
| | Japanese |
| | Malay |
| | Portuguese |
| | Thai |

The language selection UI was changed from a cycle button (tap to rotate through languages) to
a grid page that shows all languages at once.

Languages requiring non-Latin script (Mandarin, Korean, Japanese) use a bundled CJK font subset.
Thai uses a merged font (Noto Sans Thai + Roboto Latin glyphs). Both font subsets are generated
by scripts (`scripts/regen-cjk-font.py`, `scripts/regen-thai-font.py`) that collect only the
characters actually used in the translation files.

**The messy commit history:**
The Thai language support arrived via a sequence of reverts and re-adds:
1. `10e59a6` — Thai added (initial attempt)
2. `8573225` — Explicit font setting for non-Latin buttons (attempted workaround)
3. `794eead` — Revert of font fix (caused other issues)
4. `c42300e` — Revert of initial Thai (back to no Thai)
5. `e33ccfa` — Thai added again with the merged font approach

The two reverts indicate that the initial approach of relying on font fallback from the
system's text renderer (`cosmic-text`) was unreliable. The merged font (Thai glyphs + Latin
digits in one font file) bypasses font fallback entirely. The final commit (`e33ccfa`) is the
correct approach and resolves the font rendering issues.

**CJK font coverage:**
The `regen-cjk-font.py` script reads all three CJK translation files and generates a subset
containing exactly the characters used. This is the correct approach, but if translations
are updated without re-running the script, characters may be missing from the bundled font.

**Thai font coverage:**
The `regen-thai-font.py` script generates a merged font. The merged approach was specifically
chosen because `cosmic-text` (the text renderer used by iced on Linux) does not reliably fall
back from a Thai font to Roboto for Latin digits — confirmed by testing.

**CJK languages and restart:**
Switching to a CJK language requires restarting the app because the font must be loaded at
startup. The UI shows a "Restart to apply" message. Other language switches take effect
immediately.

**Concern level: MEDIUM**

> ⚠️ **FLAG:** Font subsets are generated from translation files at the time the scripts are
> run. If translation files are updated in the future, the subset scripts must be re-run or
> characters will render as boxes. There is no automated check for this.

> ⚠️ **FLAG:** Thai, Japanese, Korean, and Chinese translations were contributed in this
> development session. The correctness of the translated text has not been reviewed by a
> native speaker.

---

### 9. CI Release Pipeline Fixes

**Commits:** `94a58a1`, `b7ff06e`, `5f23a48`
**Files:** `.github/workflows/release.yml`

**What was broken (Mac builds):**
The Mac release build had been failing since at least v0.3.0. Two problems:
1. `cargo bundle` was being called without specifying which package in the workspace to bundle.
   It can't auto-detect this in a multi-crate workspace — it needs `-p refbox`.
2. When building for x86 Mac, `macos-latest` GitHub runner is now ARM hardware, so the x86
   target must be explicitly installed before building. Also, when `--target` is specified,
   build artifacts go to a target-specific path, not the default `target/release/` path —
   the upload step was looking in the wrong place.

**What was replaced (Google Drive):**
The release pipeline was downloading assets from Google Drive via a service account. That
service account stopped working. The step was replaced with a direct `curl` download, which is
simpler and doesn't require credential management.

**Concern level: LOW**
All three fixes are CI infrastructure only. No application code changed.

---

### 10. Security Update — rustls-webpki

**Commit:** `0d1a5ee`
**Files:** `Cargo.lock`

**What the vulnerabilities were:**
Two security advisories (RUSTSEC-2026-0098 and RUSTSEC-2026-0099) were reported for the
`rustls-webpki` library, which handles TLS certificate validation. Both relate to name
constraint validation bugs — specific certificate formats could bypass validation checks.
The refbox uses TLS when connecting to the portal API.

**How it was fixed:**
The `rustls-webpki` dependency was updated from version 0.103.10 to 0.103.12, which contains
the fixes for both advisories.

**One remaining advisory:**
RUSTSEC-2026-0009 (a different vulnerability in the `time` crate) was not fixed here. Fixing
it requires updating the `time` crate to version 0.3.47, which in turn requires Rust 1.88.
The project's current minimum toolchain is Rust 1.85. This fix is deferred pending a toolchain
upgrade discussion. See ADR 002 for the MSRV policy.

> ⚠️ **FLAG:** The `time` crate advisory (RUSTSEC-2026-0009) remains unresolved. It will
> reappear in `just audit` output. This is a known, tracked deferral — not an oversight.

**Concern level: LOW**
The two addressed advisories are resolved. The remaining advisory is a tracked deferral with
a documented reason.

---

### 11. Sound Artifacts Fix

**Commit:** `701d12d`
**Files:** `refbox/src/sound_controller/mod.rs`, `refbox/resources/crazy.raw`

**What the bugs were:**
Two audio glitches in the timed buzzer:
1. **Fade-out click:** The buzzer sound was configured to fade out at exactly 2.0 seconds.
   However, the Buzz and Whoop sounds have a natural fade-in in their loop cycle at 0.5-second
   intervals. When the 2.0-second fade-out landed inside one of those fade-in regions, it
   produced an audible click or artifact. Fixed by extending the sound duration to 2.15 seconds,
   placing the fade-out in a neutral part of the waveform.
2. **Stop-click on auto-buzzer end:** When the timed buzzer finished naturally, it called
   `stop()` on a sound that was already fading out. Calling `stop()` on a silent channel
   produced a brief tap sound. Fixed by checking whether audio is already silent before
   calling stop.
3. **Crazy sound clipping:** The `crazy.raw` audio file had a peak amplitude of 2.03 (over
   the maximum of 1.0), causing distortion. Replaced with a clean version with peak 1.0 and
   correct loop points.

**Concern level: LOW**
All three issues were well-understood, verified manually, and the fixes are straightforward.

---

## Open Items Across All Features

1. **No test for the `confirm_score=false` panic path** (Group 1) — the specific timer-firing
   scenario that caused production crashes is not covered by unit tests. The defensive fix
   prevents crashes, but no regression test was added.

2. **Portal role strings are hard-coded** (Group 4) — **RESOLVED:** Eric confirmed the portal
   API is internally controlled. Role strings will be updated in both systems if they ever
   change. Not a concern for v0.4.0.

3. **Scoresheet generation out of scope** (Group 5) — confirmed out of v0.4.0. It is a
   supporting CLI tool, not part of the refbox software itself.

4. **Font subsets not auto-checked** (Group 8) — translation updates require manual re-run of
   `scripts/regen-cjk-font.py` and `scripts/regen-thai-font.py`. A `just regen-fonts` recipe
   will be added to make this easy. Backlog item for CI enforcement.

5. **Translation correctness unreviewed** (Group 8) — Thai, Japanese, Korean, Chinese, and the
   other 6 new languages have not been reviewed by native speakers. Acknowledged as a known gap;
   a translation review matrix will be offered to the community as a backlog task.

6. **One remaining security advisory** (Group 10) — RUSTSEC-2026-0009 (`time` crate) is
   deferred pending a Rust toolchain upgrade. See ADR 002.

7. **Henk's binary is out of date** — the build sent to Henk on ~Apr 10 predates the
   confirm-score fix and several other fixes. He will receive the proper v0.4.0 release once
   the cleanup is complete.
