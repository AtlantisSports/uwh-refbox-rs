# Game Block button colon + APPLY disable on red — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the Game Parameters page's Game Block button read `GAME BLOCK:` (with a colon, matching its neighbours) and disable the green APPLY button while the Game Block is RED (too short).

**Architecture:** Pure `refbox` UI change in one view-builder file plus the 15 locale files. Part 1 adds a button-only label string (mirroring the existing `half-length` / `half-length-full` split) and points the button at it. Part 2 folds a "Game Block too short" condition into the existing inline APPLY-enabled rule in `make_event_config_page`, gated on portal mode being OFF (the only mode that renders the Game Block button).

**Tech Stack:** Rust 2024, iced 0.13 GUI, Fluent (`fl!`) translations.

## Global Constraints

- MSRV Rust 1.85; Edition 2024. Do not use newer APIs.
- Clippy `-D warnings` must pass on all platforms; do not add `#[allow(...)]`.
- No new dependencies.
- New translation keys must be translated in **all 15 locales** — never leave an English placeholder (en-US fallback exists but we do not rely on it).
- Branch cut from current `origin/master` (which contains the Game Block feature). The session's starting branch (`feat/refbox/time-golden-trace-spike`) predates it and must NOT be used.
- Branch name: `fix/refbox/game-block-colon-and-apply-disable`.
- Lean process (refbox UI, low blast radius): no per-task deviation commits; one code review at the end before PR. Approval required from the human before creating the branch, committing, or pushing.
- Only file touched in Rust: `refbox/src/app/view_builders/configuration.rs`. Only other files: the 15 `refbox/translations/<locale>/refbox.ftl`.

---

## Task 0: Set up isolated branch (pre-task)

**Files:** none (git only)

- [ ] **Step 1:** With the human's approval, create an isolated worktree/branch from current `origin/master`:

```bash
git fetch origin master
# via superpowers:using-git-worktrees, or fallback:
git worktree add .worktrees/game-block-colon -b fix/refbox/game-block-colon-and-apply-disable origin/master
cd .worktrees/game-block-colon
```

- [ ] **Step 2:** Confirm the Game Block feature is present (sanity check that the base is correct):

Run: `grep -n 'fl!("game-block")' refbox/src/app/view_builders/configuration.rs`
Expected: matches at the button (in `make_event_config_page`), the editor title, and the help-page label (3 hits).

---

## Task 1: Add `GAME BLOCK:` button label (15 locales + button wiring)

**Files:**
- Modify: `refbox/translations/<locale>/refbox.ftl` (all 15 locales — add one new key)
- Modify: `refbox/src/app/view_builders/configuration.rs` (Game Block button in `make_event_config_page`)

**Interfaces:**
- Consumes: nothing from other tasks.
- Produces: a new Fluent key `game-block-full` (button-only label). No Rust signatures change.

- [ ] **Step 1: Add the `game-block-full` key to every locale.** In each file, insert the new line immediately **after** the existing `game-block = ...` line (so it sits beside `game-block-help`). Use these exact values (note: zh-CN uses a fullwidth colon `：` to match its existing `half-length-full`; ja-JP/ko-KR/th-TH use an ASCII colon `:` to match theirs):

| File | New line to add |
|------|-----------------|
| `refbox/translations/de-DE/refbox.ftl` | `game-block-full = SPIELBLOCK:` |
| `refbox/translations/en-US/refbox.ftl` | `game-block-full = GAME BLOCK:` |
| `refbox/translations/es/refbox.ftl` | `game-block-full = BLOQUE DE JUEGO:` |
| `refbox/translations/fr/refbox.ftl` | `game-block-full = BLOC DE JEU:` |
| `refbox/translations/id-ID/refbox.ftl` | `game-block-full = BLOK PERTANDINGAN:` |
| `refbox/translations/it-IT/refbox.ftl` | `game-block-full = BLOCCO PARTITA:` |
| `refbox/translations/ja-JP/refbox.ftl` | `game-block-full = ゲームブロック:` |
| `refbox/translations/ko-KR/refbox.ftl` | `game-block-full = 게임 블록:` |
| `refbox/translations/ms-MY/refbox.ftl` | `game-block-full = BLOK PERLAWANAN:` |
| `refbox/translations/nl-NL/refbox.ftl` | `game-block-full = SPELBLOK:` |
| `refbox/translations/pt-PT/refbox.ftl` | `game-block-full = BLOCO DE JOGO:` |
| `refbox/translations/th-TH/refbox.ftl` | `game-block-full = บล็อกเกม:` |
| `refbox/translations/tl-PH/refbox.ftl` | `game-block-full = BLOKE NG LARO:` |
| `refbox/translations/tr-TR/refbox.ftl` | `game-block-full = OYUN BLOĞU:` |
| `refbox/translations/zh-CN/refbox.ftl` | `game-block-full = 赛程块：` |

- [ ] **Step 2: Point the Game Block button at the new key.** In `refbox/src/app/view_builders/configuration.rs`, inside `make_event_config_page`, the Game Block `make_value_button` currently starts with `fl!("game-block")`. Change ONLY that one call (the one immediately followed by `time_string(config.game_block)` and the `.style(match game_block_validity(config) {...})`):

```rust
                    make_value_button(
                        fl!("game-block-full"),
                        time_string(config.game_block),
                        (false, true),
                        Some(Message::EditParameter(LengthParameter::GameBlock)),
                    )
                    .style(match game_block_validity(config) {
                        GameBlockValidity::TooShort => red_button,
                        GameBlockValidity::Tight => yellow_button,
                        GameBlockValidity::Ok => light_gray_button,
                    })
```

Leave the OTHER two `fl!("game-block")` uses (the editor-screen title in the parameter-edit builder, and the help-page `(fl!("game-block"), fl!("game-block-help"))` pair) unchanged.

- [ ] **Step 3: Verify nothing references a missing key and it builds.**

Run: `cargo build -p refbox`
Expected: builds cleanly.

- [ ] **Step 4: Confirm exactly one button uses the new key, two titles keep the old key.**

Run: `grep -n 'fl!("game-block-full")' refbox/src/app/view_builders/configuration.rs && grep -c 'fl!("game-block")' refbox/src/app/view_builders/configuration.rs`
Expected: one hit for `game-block-full`; count of `2` for the remaining `game-block` uses.

- [ ] **Step 5: Commit.**

```bash
git add refbox/translations refbox/src/app/view_builders/configuration.rs
git commit -m "fix(refbox): add colon to Game Block button label"
```

---

## Task 2: Disable APPLY when Game Block is red (too short)

**Files:**
- Modify: `refbox/src/app/view_builders/configuration.rs` (APPLY gating in `make_event_config_page`)

**Interfaces:**
- Consumes: `game_block_validity(&GameConfig) -> GameBlockValidity` (existing, same file); `GameBlockValidity::TooShort` (existing); the in-scope locals `using_uwhportal: bool` and `config: &GameConfig` (both already used in this function).
- Produces: no new public signatures.

**Note on testing:** the core red-threshold logic is already covered by the existing unit test `test_game_block_validity_thresholds` in this file (it asserts `game_block_validity` returns `TooShort` below the minimum). This task only wires that already-tested function into the APPLY-enabled boolean inside an iced view builder, which the codebase does not unit-test (view builders return `Element` and are verified by the manual walkthrough). So this task is verified by `just check` plus the manual UI walkthrough in Task 3 — no new unit test.

- [ ] **Step 1: Add the gate.** In `make_event_config_page`, find the existing APPLY-enabled block:

```rust
    let apply_blocked = settings.uwhportal_incomplete();
    let apply_enabled =
        page_has_changes(ConfigPage::Game, settings, page_entry_snapshot) && !apply_blocked;
```

Replace it with (adds `game_block_too_short`, gated on portal mode OFF so APPLY behaviour is unchanged in portal-ON mode where no Game Block button is shown):

```rust
    let apply_blocked = settings.uwhportal_incomplete();
    // A red (too-short) Game Block is invalid, so APPLY must be disabled until it
    // is widened. Only gate this in portal-OFF mode — that is the only mode that
    // renders the Game Block button, so the disabled APPLY always has a visible
    // red button explaining it. Yellow ("tight") is a caution, not invalid, and
    // does not block.
    let game_block_too_short =
        !using_uwhportal && matches!(game_block_validity(config), GameBlockValidity::TooShort);
    let apply_enabled = page_has_changes(ConfigPage::Game, settings, page_entry_snapshot)
        && !apply_blocked
        && !game_block_too_short;
```

- [ ] **Step 2: Run the full check suite.**

Run: `just check`
Expected: fmt clean, clippy `-D warnings` clean, all tests pass (including `test_game_block_validity_thresholds`).

- [ ] **Step 3: Commit.**

```bash
git add refbox/src/app/view_builders/configuration.rs
git commit -m "fix(refbox): disable Apply when Game Block is too short"
```

---

## Task 3: Manual walkthrough verification

**Files:** none (runtime verification)

- [ ] **Step 1: Build the binary** (clippy/just build a test binary, not `target/debug/refbox`):

Run: `cargo build -p refbox`

- [ ] **Step 2: Launch refbox** (WSLg: force X11, run sandboxed-off in background):

Run: `WAYLAND_DISPLAY= ` then launch the built `target/debug/refbox` in the background with sandbox disabled.

- [ ] **Step 3: Verify the label.** Open Game Parameters with USING UWHPORTAL = NO. Confirm the Game Block button reads `GAME BLOCK:` (trailing colon, matching HALF LENGTH: etc.).

- [ ] **Step 4: Verify red disables APPLY.** Set the Game Block too short (below "game + minimum break") so the button turns RED. Confirm the green APPLY button is greyed/unpressable.

- [ ] **Step 5: Verify re-enable.** Widen the Game Block until it is no longer red. Confirm APPLY becomes pressable again.

- [ ] **Step 6: Verify yellow still applies.** Set the Game Block to a YELLOW (tight) value. Confirm APPLY stays pressable.

---

## Self-Review (completed by plan author)

- **Spec coverage:** Part 1 (colon) → Task 1. Part 2 (disable on red, yellow allowed, portal-OFF gating) → Task 2. Acceptance criteria → Task 3 walkthrough + `just check` in Task 2. All spec sections mapped.
- **Placeholder scan:** No TBD/TODO; all label values and code blocks are concrete.
- **Type consistency:** `game_block_validity`, `GameBlockValidity::TooShort`, `using_uwhportal`, `config` all match existing names in `configuration.rs`. New Fluent key `game-block-full` is consistent between the ftl files (Task 1 Step 1) and the button call site (Task 1 Step 2).
