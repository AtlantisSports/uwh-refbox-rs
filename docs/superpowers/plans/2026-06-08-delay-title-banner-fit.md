# DELAY Title + Banner-Fit Check Implementation Plan

> Lean process (refbox UI + translations). Spec: docs/superpowers/specs/2026-06-08-delay-title-banner-fit-design.md

**Goal:** Add a stacked "DELAY" title above the behind-schedule figure in the main-view banner (translated in all 15 locales), and render the densest banner once (throwaway override) to confirm fit before PR.

**Architecture:** View-layer + Fluent only. New `delay` key; `make_game_time_button` renders title-over-figure when `behind_label` is `Some`. A temporary, uncommitted override in `build_main_view` forces the dense case for one screenshot, then is removed.

---

### Task 1: Add the `delay` translation key to all 15 locales

**Files:** every `refbox/translations/<locale>/refbox.ftl`

- [ ] **Step 1:** Insert `delay = <VALUE>` after the `show-behind-schedule-time` line in each locale. Uppercase to match the existing label convention (`next-game = NEXT GAME`). Values (best-guess; flag for native review):

| locale | value | | locale | value |
|--------|-------|-|--------|-------|
| en-US | DELAY | | nl-NL | VERTRAGING |
| de-DE | VERZÖGERUNG | | pt-PT | ATRASO |
| es | RETRASO | | th-TH | ล่าช้า |
| fr | RETARD | | tl-PH | ANTALA |
| id-ID | TERLAMBAT | | tr-TR | GECİKME |
| it-IT | RITARDO | | zh-CN | 延误 |
| ja-JP | 遅延 | | ko-KR | 지연 |
| ms-MY | LEWAT | | | |

- [ ] **Step 2:** Verify each file got exactly one `delay = ` line: `grep -c "^delay = " refbox/translations/*/refbox.ftl` → all `1`.

---

### Task 2: Render the stacked DELAY title

**Files:** Modify `refbox/src/app/view_builders/shared_elements.rs` (the `overrun_label` block, ~556-563)

- [ ] **Step 1:** Replace the single delay `text` with a centered title-over-figure column:

```rust
    if let Some(label) = overrun_label {
        content = content.push(
            column![
                text(fl!("delay")).style(red_text).size(SMALL_TEXT),
                text(label).style(red_text).size(MEDIUM_TEXT),
            ]
            .align_x(Alignment::Center),
        );
    }
```

(`column`, `Alignment`, `text`, `red_text`, `SMALL_TEXT`, `MEDIUM_TEXT` are already in scope in this file.)

- [ ] **Step 2:** `cargo build -p refbox` — expect clean compile.

---

### Task 3: Temporary dense-banner override (throwaway)

**Files:** Modify `refbox/src/app/view_builders/main_view.rs` — top of `build_main_view`, after the `ViewData` destructure

- [ ] **Step 1:** Insert a clearly-marked demo block that shadows the locals (forces Rugby + portal tile + penalty-shot timeout + 61s delay). Uses `Mode`, `PortalIndicatorState`, `TimeoutSnapshot` (import-qualify as needed):

```rust
    // ===== TEMP DEMO — REVERT BEFORE PR (banner-fit check) =====
    let demo_snapshot = {
        let mut s = snapshot.clone();
        s.timeout = Some(uwh_common::game_snapshot::TimeoutSnapshot::PenaltyShot(23));
        s
    };
    let snapshot = &demo_snapshot;
    let mode = crate::app::Mode::Rugby;
    let portal_indicator = Some(crate::portal_manager::PortalIndicatorState::default());
    let behind_schedule = std::time::Duration::from_secs(61);
    // ===== END TEMP DEMO =====
```

(Exact import paths for `Mode`/`PortalIndicatorState` to be confirmed against this file's existing `use super::*;` re-exports during execution; adjust to whatever resolves.)

- [ ] **Step 2:** `cargo build -p refbox` — clean compile.

- [ ] **Step 3:** Launch refbox for the screenshot (WSL X11 workaround, run in worktree):
`WAYLAND_DISPLAY= cargo run -p refbox` (background, sandbox disabled).

- [ ] **Step 4:** Screenshot the dense banner. **PAUSE for operator verdict on fit.**

---

### Task 4: Remove the demo and finalize

- [ ] **Step 1:** Delete the entire `TEMP DEMO` block from `main_view.rs`.
- [ ] **Step 2:** `cargo test -p refbox` → 226 pass.
- [ ] **Step 3:** `cargo clippy -p refbox -- -D warnings` → clean.
- [ ] **Step 4:** Confirm `git diff` shows only: the `delay` keys (15 files) + the stacked-title block in `shared_elements.rs`. No `main_view.rs` change remains.
- [ ] **Step 5:** Present the diff for commit approval (do not commit unprompted).

---

## Deviations

The fit check (Task 3) found the banner DID overflow in UWR + portal mode, so —
per operator direction — the fit was fixed inline rather than deferred to a
follow-up (the spec had said a fit problem would be recorded, not fixed here):

1. **Delay yields to an active timeout.** The delay column is hidden whenever a
   timeout/penalty shot is on screen (it keeps accruing, reappears after). Keeps
   the timeout case on the known-good shipped layout.
2. **Compact sizing in UWR + portal.** When both side tiles (portal health tile +
   UWR pause button) are present AND a second middle column competes (a timeout
   or the delay), the period/timeout label drops to `SMALL_TEXT` and the clock to
   `MEDIUM_TEXT` so nothing wraps or clips. Every other banner keeps the full-size
   `LARGE_TEXT` clock for poolside readability.
3. **DELAY block reuses `make_time_view_col`.** Instead of a bespoke column, the
   delay title+figure are built with the same helper as the period/clock, so their
   font size and vertical alignment match exactly (the title matches the game-state
   title; the figure matches the clock and tracks `compact` the same way).

Net production change is confined to `refbox/src/app/view_builders/shared_elements.rs`
plus the `delay` key in 15 locales. `main_view.rs` is unchanged from master-of-branch.
