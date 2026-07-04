# Game Info Button ↔ Page Consistency Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the main-screen Game Info button consistent with the Game Information page — hide referee fields when not using the Portal, always show "Stop Clock in Last 2 Minutes", and place it directly above "Team Timeouts" on both surfaces.

**Architecture:** Two parallel string builders render game info: `config_string` (the main-screen button, in `shared_elements.rs`) and `details_strings` (the full page, in `game_info.rs`). The fix changes only the *order* and the *show/hide conditions* of two existing lines in each builder — no new translation keys, no game-logic changes.

**Tech Stack:** Rust 2024, `iced` 0.13, `i18n_embed` fluent localization (`fl!` macro). Crate: `refbox` only.

**Spec:** `docs/superpowers/specs/2026-06-17-game-info-consistency-design.md`

**Worktree / branch:** `.worktrees/game-info-consistency`, branch `fix/refbox/game-info-consistency`, based on master `3ff86b61`. All line numbers below are exact for that commit.

## Global Constraints

- MSRV Rust 1.85; Edition 2024. No newer-than-1.85 APIs.
- Clippy `-D warnings` must pass: `cargo clippy -p refbox -- -D warnings` (mirrors CI/`just lint`; do **not** add `--all-targets`).
- Tests: `cargo test -p refbox` (do **not** add `--lib`).
- No new dependencies. No `.ftl` (translation) changes — `stop-clock-last-2` and `ref-list` already exist in every locale.
- No `unwrap()`/`expect()` in non-test production code. (Tests may use `expect`.)
- Referees are Portal-only on **both** surfaces. "Stop Clock in Last 2 Minutes" is **always** shown on both, directly above "Team Timeouts". Out of Portal Mode it renders "Unknown" — intentional, leave as-is.
- Lean process (refbox UI, display-only): no per-task deviation commits; one code review at the end.

---

### Task 1: Button — gate referees on Portal, move Stop Clock above Team Timeouts (`config_string`)

**Files:**
- Modify: `refbox/src/app/view_builders/shared_elements.rs` (function `config_string`, lines 1020–1139; the edited region is 1075–1138)
- Test: `refbox/src/app/view_builders/shared_elements.rs` (append to the existing `#[cfg(test)] mod tests` at line 1430)

**Interfaces:**
- Consumes: `config_string(snapshot: &GameSnapshot, config: &GameConfig, using_uwhportal: bool, schedule: Option<&Schedule>, teams: Option<&TeamList>) -> String` (signature unchanged). `GameConfig` is the in-module alias for `crate::config::Game` (has `Default`, with `num_team_timeouts_allowed: 1`). `GameSnapshot` derives `Default`. `fl!` is a crate-global macro.
- Produces: same function, same signature — only the produced string's line order and the conditions change.

- [ ] **Step 1: Write the failing tests**

Append these two tests inside the existing `mod tests { ... }` block (which already has `use super::*;`) in `refbox/src/app/view_builders/shared_elements.rs`:

```rust
    #[test]
    fn config_string_hides_referees_outside_portal_mode() {
        let config = GameConfig::default();
        let snapshot = GameSnapshot::default();

        // The referee block exactly as config_string renders it with no assignments.
        let ref_block = fl!(
            "ref-list",
            chief_ref = "-",
            timer = "-",
            water_ref_1 = "-",
            water_ref_2 = "-",
            water_ref_3 = "-"
        );

        let out_no_portal = config_string(&snapshot, &config, false, None, None);
        assert!(
            !out_no_portal.contains(&ref_block),
            "referees must not show when not using the portal"
        );

        let out_portal = config_string(&snapshot, &config, true, None, None);
        assert!(
            out_portal.contains(&ref_block),
            "referees must show when using the portal"
        );
    }

    #[test]
    fn config_string_shows_stop_clock_above_team_timeouts() {
        let config = GameConfig {
            num_team_timeouts_allowed: 0,
            ..Default::default()
        };
        let snapshot = GameSnapshot::default();

        let out = config_string(&snapshot, &config, false, None, None);

        let stop_clock_line = fl!("stop-clock-last-2", stop_clock = fl!("unknown"));
        let team_timeouts_line = fl!("team-timeouts", value = "0");

        let stop_idx = out
            .find(&stop_clock_line)
            .expect("stop-clock line should always be present");
        let to_idx = out
            .find(&team_timeouts_line)
            .expect("team-timeouts line should be present");
        assert!(
            stop_idx < to_idx,
            "stop-clock should appear above team timeouts"
        );
    }
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p refbox config_string_ -- --nocapture`
Expected: both new tests FAIL — `config_string_hides_referees_outside_portal_mode` because referees currently render unconditionally; `config_string_shows_stop_clock_above_team_timeouts` because stop-clock currently renders *after* team-timeouts (`stop_idx < to_idx` is false).

- [ ] **Step 3: Implement — reorder and gate**

In `refbox/src/app/view_builders/shared_elements.rs`, replace lines **1075–1138** (from `let team_timeouts_value = ...` down to and including the final `result`) with the block below. This moves the stop-clock computation above the team-timeouts line and wraps the referee block in `if using_uwhportal { ... }`:

```rust
    // Stop Clock in Last 2 Minutes — a normal game rule (not Portal-specific), shown directly
    // above Team Timeouts. Reads "Unknown" when no schedule timing rule is available
    // (e.g. when not using the Portal).
    let stop_clock = if let Some(sched) = schedule {
        if let Some(timing_rule) = sched.get_game_timing(&game_number) {
            bool_string(timing_rule.last_2_min_stop_time)
        } else {
            fl!("unknown")
        }
    } else {
        fl!("unknown")
    };
    result += "\n";
    result += &fl!("stop-clock-last-2", stop_clock = stop_clock);

    let team_timeouts_value = if config.num_team_timeouts_allowed == 0 {
        "0".to_string()
    } else if config.timeouts_counted_per_half {
        format!("{}/{}", config.num_team_timeouts_allowed, fl!("half"))
    } else {
        format!("{}/{}", config.num_team_timeouts_allowed, fl!("game"))
    };
    result += "\n";
    result += &fl!("team-timeouts", value = team_timeouts_value);

    // Referees only exist when using the Portal — hide the whole block otherwise.
    if using_uwhportal {
        let mut chief_ref = "-".to_string();
        let mut timer = "-".to_string();
        let mut water_ref_1 = "-".to_string();
        let mut water_ref_2 = "-".to_string();
        let mut water_ref_3 = "-".to_string();

        if let Some(games) = games {
            if let Some(game) = games.get(&game_number) {
                if let Some(refs) = &game.referee_assignments {
                    for ref_assignment in refs {
                        if ref_assignment.user_id.is_some() {
                            // Fall back to '-' for unassigned slots — language-neutral,
                            // visually distinct from real names.
                            let display = ref_assignment
                                .display_name
                                .clone()
                                .unwrap_or_else(|| "-".to_string());
                            match ref_assignment.role.as_str() {
                                "Chief" => chief_ref = display,
                                "TimeOrScoreKeeper" => timer = display,
                                "Water1" => water_ref_1 = display,
                                "Water2" => water_ref_2 = display,
                                "Water3" => water_ref_3 = display,
                                _ => {}
                            }
                        }
                    }
                }
            }
        }

        result += "\n";
        result += &fl!(
            "ref-list",
            chief_ref = chief_ref,
            timer = timer,
            water_ref_1 = water_ref_1,
            water_ref_2 = water_ref_2,
            water_ref_3 = water_ref_3
        );
    }

    result
```

Note: the `game-block-info` and `game-config`/`game-config-single-half` lines immediately above (lines 1049–1073) are unchanged. The line just before this block (1073) ends without a trailing newline, so the first `result += "\n"` here puts Stop Clock on its own line.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p refbox config_string_ -- --nocapture`
Expected: both tests PASS.

- [ ] **Step 5: Lint and format**

Run: `cargo fmt -p refbox && cargo clippy -p refbox -- -D warnings`
Expected: no formatting changes left unstaged, zero clippy warnings.

- [ ] **Step 6: Commit**

```bash
git add refbox/src/app/view_builders/shared_elements.rs
git commit -m "fix(refbox): hide referees off-portal and move stop-clock above team timeouts on game-info button"
```

---

### Task 2: Page — always show Stop Clock above Team Timeouts (`details_strings`)

**Files:**
- Modify: `refbox/src/app/view_builders/game_info.rs` (function `details_strings`, lines 92–319; the edited region is 241–318)
- Test: `refbox/src/app/view_builders/game_info.rs` (add a new `#[cfg(test)] mod tests` at the end of the file)

**Interfaces:**
- Consumes: `details_strings(snapshot: &GameSnapshot, config: &GameConfig, using_uwhportal: bool, schedule: Option<&Schedule>, teams: Option<&TeamList>) -> (String, String)` — returns `(left_column, right_column)`. Referees render into the right column; everything else into the left. Signature unchanged.
- Produces: same function — only the left column's line order and the stop-clock condition change. Referees stay in the right column, Portal-only.

- [ ] **Step 1: Write the failing tests**

Add this module at the very end of `refbox/src/app/view_builders/game_info.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn details_page_shows_stop_clock_even_outside_portal_mode() {
        let config = GameConfig {
            num_team_timeouts_allowed: 0,
            ..Default::default()
        };
        let snapshot = GameSnapshot::default();

        let (left, _right) = details_strings(&snapshot, &config, false, None, None);

        let stop_clock_line = fl!("stop-clock-last-2", stop_clock = fl!("unknown"));
        let team_timeouts_line = fl!("team-timeouts", value = "0");

        let stop_idx = left
            .find(&stop_clock_line)
            .expect("page must show stop-clock even when not using the portal");
        let to_idx = left
            .find(&team_timeouts_line)
            .expect("team-timeouts line should be present");
        assert!(
            stop_idx < to_idx,
            "stop-clock should appear above team timeouts"
        );
    }

    #[test]
    fn details_page_keeps_referees_portal_only() {
        let config = GameConfig::default();
        let snapshot = GameSnapshot::default();

        let ref_block = fl!(
            "ref-list",
            chief_ref = "-",
            timer = "-",
            water_ref_1 = "-",
            water_ref_2 = "-",
            water_ref_3 = "-"
        );

        let (_l, right_no_portal) = details_strings(&snapshot, &config, false, None, None);
        assert!(
            !right_no_portal.contains(&ref_block),
            "referees must not show when not using the portal"
        );

        let (_l2, right_portal) = details_strings(&snapshot, &config, true, None, None);
        assert!(
            right_portal.contains(&ref_block),
            "referees must show when using the portal"
        );
    }
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p refbox details_page_ -- --nocapture`
Expected: `details_page_shows_stop_clock_even_outside_portal_mode` FAILS — stop-clock is currently inside the `if using_uwhportal` block, so the left column omits it when not using the portal (the `.expect` panics). `details_page_keeps_referees_portal_only` should already PASS (referees are already Portal-gated) — it is a regression guard for this task.

- [ ] **Step 3: Implement — pull Stop Clock out of the Portal block and reorder it above Team Timeouts**

In `refbox/src/app/view_builders/game_info.rs`, replace lines **241–318** (from `let team_timeouts_value = ...` down to and including `    (left_string, right_string)`) with:

```rust
    // Stop Clock in Last 2 Minutes — a normal game rule (not Portal-specific), shown directly
    // above Team Timeouts. Reads "Unknown" when no schedule timing rule is available
    // (e.g. when not using the Portal).
    let stop_clock = if let Some(sched) = schedule {
        if let Some(timing_rule) = sched.get_game_timing(game_number) {
            bool_string(timing_rule.last_2_min_stop_time)
        } else {
            fl!("unknown")
        }
    } else {
        fl!("unknown")
    };
    left_string += &fl!("stop-clock-last-2", stop_clock = stop_clock);
    left_string += "\n";

    let team_timeouts_value = if config.num_team_timeouts_allowed == 0 {
        "0".to_string()
    } else if config.timeouts_counted_per_half {
        format!("{}/{}", config.num_team_timeouts_allowed, fl!("half"))
    } else {
        format!("{}/{}", config.num_team_timeouts_allowed, fl!("game"))
    };
    left_string += &fl!("team-timeouts", value = team_timeouts_value);
    left_string += "\n";

    if config.num_team_timeouts_allowed != 0 {
        left_string += &fl!(
            "team-to-len",
            to_len = time_string(config.team_timeout_duration)
        );
        left_string += "\n";
    };
    left_string += &fl!(
        "min-brk-btwn-games",
        min_brk_time = time_string(config.minimum_break)
    );
    left_string += "\n";

    // Referees only exist when using the Portal — they render into the right column.
    if using_uwhportal {
        let mut chief_ref = "-".to_string();
        let mut timer = "-".to_string();
        let mut water_ref_1 = "-".to_string();
        let mut water_ref_2 = "-".to_string();
        let mut water_ref_3 = "-".to_string();

        if let Some(games) = games {
            if let Some(game) = games.get(game_number) {
                if let Some(refs) = &game.referee_assignments {
                    for ref_assignment in refs {
                        if ref_assignment.user_id.is_some() {
                            // Fall back to '-' for unassigned slots — language-neutral,
                            // visually distinct from real names.
                            let display = ref_assignment
                                .display_name
                                .clone()
                                .unwrap_or_else(|| "-".to_string());
                            match ref_assignment.role.as_str() {
                                "Chief" => chief_ref = display,
                                "TimeOrScoreKeeper" => timer = display,
                                "Water1" => water_ref_1 = display,
                                "Water2" => water_ref_2 = display,
                                "Water3" => water_ref_3 = display,
                                _ => {}
                            }
                        }
                    }
                }
            }
        }

        right_string += &fl!(
            "ref-list",
            chief_ref = chief_ref,
            timer = timer,
            water_ref_1 = water_ref_1,
            water_ref_2 = water_ref_2,
            water_ref_3 = water_ref_3
        );
    }

    (left_string, right_string)
```

Note: the `sd-allowed` / `pre-sd` lines above (lines 230–239) are unchanged. The only differences from the original are: (1) the stop-clock computation+emission now lives in the left column *before* team-timeouts and is **not** gated by `using_uwhportal`; (2) the `if using_uwhportal` block now contains only the referee logic. `game_number` here is already a `&GameNumber`, so `get_game_timing(game_number)` and `games.get(game_number)` take it by reference (no `&`).

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p refbox details_page_ -- --nocapture`
Expected: both tests PASS.

- [ ] **Step 5: Lint and format**

Run: `cargo fmt -p refbox && cargo clippy -p refbox -- -D warnings`
Expected: no leftover formatting changes, zero clippy warnings.

- [ ] **Step 6: Commit**

```bash
git add refbox/src/app/view_builders/game_info.rs
git commit -m "fix(refbox): always show stop-clock above team timeouts on game info page"
```

---

### Task 3: Full verification + manual walkthrough

**Files:** none (verification only).

- [ ] **Step 1: Run the full local check**

Run: `just check`
Expected: fmt, clippy (`-D warnings`), tests, and audit all pass.

- [ ] **Step 2: Build and run the app for a visual check**

Run (from the worktree root):
```bash
cargo build -p refbox && pkill -x refbox 2>/dev/null; true
WAYLAND_DISPLAY= ./target/debug/refbox
```
(Launch in the background; on WSL the `WAYLAND_DISPLAY=` prefix forces X11. Run with the sandbox disabled.)

Confirm, **not in Portal Mode**, on the **main-screen Game Info panel**:
- No referee lines (no "Chief Ref", "Timer", "Water Ref").
- "Stop Clock in Last 2 Minutes: Unknown" appears **directly above** "Team Timeouts".

Tap into the **Game Information page** and confirm:
- "Stop Clock in Last 2 Minutes: Unknown" is now shown, **directly above** "Team Timeouts".
- Still no referee lines.

- [ ] **Step 3: Hand off for PR**

Do **not** open the PR automatically — opening a PR requires the human's approval (see `.claude/rules/communication.md`). Summarize what changed in plain English and ask the human before creating the branch's PR.

---

## Self-Review

**Spec coverage:**
- "Referees only in Portal Mode (both surfaces)" → Task 1 (button gating) + Task 2 regression guard (page already gated). ✓
- "Stop Clock always shown (both surfaces), directly above Team Timeouts" → Task 1 (button reorder, already unconditional) + Task 2 (page un-gate + reorder). ✓
- "Out of Portal Mode reads 'Unknown'" → preserved verbatim in both builders (`fl!("unknown")` fallback). ✓
- "No translation changes" → no `.ftl` edits in any task. ✓
- "Button stays compact / page stays complete" → button does not gain Team Timeout Duration / Minimum Time Between Games; page keeps them. ✓

**Placeholder scan:** No TBD/TODO; every code step shows complete code. ✓

**Type consistency:** `config_string` and `details_strings` signatures unchanged; `GameConfig`/`GameSnapshot`/`Schedule`/`TeamList` used consistently; `game_number` is owned in `config_string` (`&game_number`) and borrowed in `details_strings` (`game_number`), matching each function's existing usage. ✓
