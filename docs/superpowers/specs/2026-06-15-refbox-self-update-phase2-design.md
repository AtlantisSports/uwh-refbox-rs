# Refbox Self-Update — Phase 2 (Updater Backend) Design Spec

**Date:** 2026-06-15
**Status:** Design approved in brainstorming (2026-06-15). Implementation plan next, gated through
native plan mode (ExitPlanMode) + subagent-driven execution.
**Crate:** `refbox` (new `updater/` module, CLI, UI wiring, relaunch refactor) + additive
`.github/workflows/release.yml` change.
**Builds on:** Phase 1 (restart resilience, MERGED PR #1083) and the approved UI preview
(`890a6562`, worktree `feat+refbox+self-update-ui`).
**Supersedes for Phase 2 detail:** the Phase-2 sections of
`2026-06-15-refbox-self-update-phased-design.md` §3 (this records the resolved open decisions).
Related: `2026-06-10-refbox-self-update-adversarial-pass-2.md` (findings A–F),
`2026-06-10-refbox-self-update-hardware-test-script.md` (Tests B–E),
[[reference_serialport_ebusy_unknown]], [[reference_pi_deployment_launch]].

**Local working doc** — not committed to a branch/PR ([[reference_plan_docs_not_committed]]).

---

## 1. Goal & scope

Give the operator an in-app "check for updates / install / revert" feature on the Raspberry Pi,
wired behind the already-approved Updates-page UI, safe against tournament failure modes. Operator-
initiated, confirmed, draft-then-validated.

**In scope (`refbox`):** new `updater/` module; hidden `--self-check` CLI flag; relaunch refactor
to take an explicit exe path; Updates-page backend wiring (replacing the preview's fake driver);
game-state gate; best-effort auto-revert; one-time post-rollback message. **CI:** additive — publish
the standalone aarch64 binary + a checksum file alongside the existing zip.

**Out of scope (deferred/decided):** the fully-robust "won't-launch-at-all" boot-script-integrated
auto-revert (best-effort only now; robust layer follows once the Pi boot script is known —
[[reference_pi_deployment_launch]]); Mac/Windows self-update (manual download); cryptographic
signatures (checksum only for v1); multi-version backup (one-deep); automatic background checks
(operator-initiated; status starts "Unknown" each session); publishing v0.4.2 (separate gated step).

## 2. Resolved decisions (this session)

- **Dependencies:** add **no genuinely-new crates**. HTTP reuses `reqwest` (already a direct refbox
  dep). Version compare is **hand-rolled** (major/minor/patch tuple) — no `semver`. File swap is
  **hand-rolled** hard-link+rename — no `self_replace`. Checksum uses **`sha2`**, which is already
  compiled into the build (transitive in `Cargo.lock`); exposing it as a direct dep adds no new crate.
- **Update-step model (approach A):** each step is its own async iced task that does the real work
  and advances the existing `UpdateUiState`; heavy lifting in pure, unit-testable functions in
  `updater/`. Mirrors the approved preview (swap fake `sleep`s for real calls); cancellable between
  steps via the existing Back/Cancel button.
- **Auto-revert UX:** on a post-rollback launch, **open straight to the Updates page** showing a
  plain message ("An update was installed but didn't start correctly, so the previous version was
  restored. Try again later or update manually."), then **clear the one-time marker** so it's shown
  exactly once and never sticks across reboots.
- **Testing the update path:** use a **throwaway published `v0.4.3` test release** (low repo
  traffic), validate, then delete it — no test-override mechanism needed.

## 3. Update flow (operator-confirmed; nothing on disk changes before the swap)

1. **Check** — GET GitHub's latest published release for `AtlantisSports/uwh-refbox-rs` (with a
   `User-Agent` header); drafts/pre-releases are excluded by that API (finding F). Compare to the
   running version → `UpToDate` / `UpdateAvailable(X)` / `Error`.
2. **Pre-flight** — enough free disk space **and** the install dir is writable (finding F);
   re-check the game gate.
3. **Download** the standalone aarch64 binary to a temp file in the **install dir** (same
   filesystem); follow the asset redirect over HTTPS without leaking auth headers.
4. **Verify** the published SHA-256 checksum; mismatch → abort, change nothing.
5. **Smoke-test** — run the temp binary as `--self-check <the real restart argv>` (finding C):
   proves it runs on this Pi and accepts the argv it'll be restarted with. v1 self-check =
   initialise-and-exit without opening a window, spawning a sim child, or grabbing hardware.
6. **Swap (finding B)** — capture the canonicalized install path **first**; `link(current →
   refbox-v{prev}.bak)` then `rename(new → install path)`. No instant with no program present.
7. **Restart (finding A)** — the single relaunch routine takes the exe path as a parameter; the
   updater passes the captured install path (a rename makes `current_exe()` point at the backup);
   carries the original restart argv (Phase 1's `build_restart_argv`).
8. **Auto-revert** — §5.

## 4. Safety model (layered, outermost first)

- **Game gate** — feature disabled while a game is in progress, incl. half-time/breaks, timeouts,
  score-review; one predicate over tournament-manager state; re-checked at install.
- **Verify-before-swap** — checksum + smoke-test pass before any disk change.
- **One-deep backup** — `refbox-v{prev}.bak` (same dir, version-named), the manual-revert target.
- **Manual revert** — "Revert to Previous Version (X)".
- **Best-effort auto-revert** — §5.
- **Manual SD-card / SSH fallback** — unchanged ultimate floor.

Net worst case: "the button didn't work; update manually" — recoverable, though not always from
inside the app (an update that won't launch at all needs power-cycle/manual).

## 5. Best-effort auto-revert (no boot script)

- On the post-swap launch, write a **trial marker** next to the config (records "trying vX, backup
  vY").
- When the new version reaches a **healthy state** — reached the main screen and ran ~N seconds
  without crashing — clear the marker (`fsync`, finding F: minimise false revert on power-cut).
- On any launch, an **uncleared** trial marker from a prior launch ⇒ the new version didn't get
  healthy ⇒ restore the backup (swap back), write a one-time **rolled-back marker**, clear the trial
  marker, relaunch the previous binary.
- The post-rollback launch sees the rolled-back marker → starts on the Updates page with the message
  (§2) → clears that marker. Strictly once.
- **Honest limit:** an in-app marker catches a crash *after* the checkpoint; an update that won't run
  at all is caught on the next launch (marker stays uncleared), with power-cycle (boot auto-start) /
  manual as the floor. The robust boot-integrated layer is the deferred follow-up.

## 6. Architecture

- **`refbox/src/updater/`** — pure functions: `compare_version`, parse latest-release JSON, select
  assets (binary + checksum), `verify_sha256`, `swap_in_place` (hard-link+rename), backup/revert,
  marker read/write. Thin async wrappers (reqwest/tokio) call these.
- **Relaunch** — extend Phase 1's routine in `main.rs` to take `exe_path: PathBuf`; plain restart
  passes `current_exe()`, updater passes the captured path. One audited routine.
- **CLI** — add hidden `--self-check` to `Cli` (`main.rs`); handled early in `main()` (init + exit).
- **Version** — `env!("CARGO_PKG_VERSION")`; hand-rolled compare; shown on the Updates page + logged
  at startup.
- **UI** — keep `ConfigPage::Updates`, the `UpdateUiState` machine, the view, and the user-action
  messages from the preview; **remove** `UpdateScenario`, `UpdatesSetScenario`, the simulated
  `sleep` driver, and the scenario picker; real tasks drive the states. `AppState::UpdatesPreview`
  → a real `AppState::Updates { state, current, backup: Option<Version> }`. Mirror the preview's
  view element-for-element ([[feedback_mirror_existing_code_patterns]]).
- **Game gate** — single predicate; "Check Version" button on App Options disabled when true.
- **CI** — `release.yml`: also upload `refbox` (aarch64) + `refbox.sha256` as release assets; zip
  unchanged.
- **Errors** — specific, translated: "Couldn't reach the update server, please check your internet
  connection" (network/captive-portal non-JSON), "The downloaded update wasn't valid and was not
  installed" (checksum), "Update server is busy, please try again later" (rate limit), "Not enough
  free space to update", "Can't update — the program folder isn't writable".

## 7. Testing

- **Unit:** version compare; latest-release parse; asset selection; `verify_sha256` (good +
  tampered); error mapping (incl. non-JSON 200); `swap_in_place` + revert on temp dirs; marker
  read/write/clear + auto-revert trigger logic.
- **On-Pi (hardware-test-script Tests B–E):** against a throwaway published `v0.4.3` release
  (deleted after) — successful update, revert, failure handling (no internet, mid-download cancel),
  power-cut recovery. Plus re-confirm Phase 1 Test A (5× restart) and that a held serial port really
  surfaces as `Unknown` ([[reference_serialport_ebusy_unknown]]).

## 8. Translations

All operator-facing strings via Fluent in **every** locale, no English placeholders
([[feedback_translate_all_locales_no_placeholders]]).

## 9. Acceptance criteria (operator-observable)

Check finds the newer version; Install downloads/verifies/restarts onto it (full-screen, scoreboard,
buzzer intact); the version display confirms it; Revert returns to the previous; no internet → plain
message, app unchanged; Check disabled during game/break/timeout; Cancel mid-download → back to
*Update available*, nothing changed; a deliberately-broken update auto-reverts and the post-rollback
launch shows the message once; power loss during install → the Pi always boots a working refbox.
