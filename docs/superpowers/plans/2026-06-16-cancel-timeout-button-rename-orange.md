# Cancel Timeout Button — Rename, Orange Fill, Honest Switch Labels — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Rename the refbox "End Timeout" button to "Cancel Timeout" in all 15 locales, give every Cancel Timeout button an orange fill, and make the other timeout-ribbon buttons show "SWITCH TO …" only when that switch is actually available (otherwise show the timeout's own name, greyed).

**Architecture:** UI-only change in the `refbox` crate. Two Fluent keys renamed across 15 locale files; five `yellow_button` → `orange_button` style swaps on the `Message::EndTimeout` button; and a label split in `build_timeout_ribbon` so each non-active slot picks its label/clickability from the existing `can_switch_to_*` result. No `uwh-common` and no `tournament_manager` (state-machine) changes — clickability and click behaviour are unchanged; only the words on already-greyed buttons change.

**Tech Stack:** Rust 2024, iced 0.13, Fluent (`fl!`) translations. Reference design: `docs/superpowers/specs/2026-06-16-cancel-timeout-button-rename-orange-design.md`.

**Notes for the worker:**
- `refbox` is a bin-only crate. There is **no unit-test harness for iced widget labels**, and this is mechanical UI / translation work, so per the repo's lean process (`.claude/rules/plan-execution.md`) verification is **compilation + `just check` + the manual walkthrough** in the final task — not new unit tests.
- `cargo clippy -p refbox -- -D warnings` (no `--all-targets`) mirrors CI/`just lint` for this crate; `cargo test -p refbox` (no `--lib`).
- A button built with `make_multi_label_button(...)` / `make_button(...)` and **no** `.on_press(...)` renders greyed/disabled in iced 0.13 — that is exactly how we get the disabled state in Task 3.

---

## Task 0: Create the worktree

**Branch:** `feat/refbox/cancel-timeout-button` (type `feat`, scope `refbox`).

- [ ] **Step 1: Create a fresh worktree off `origin/master`**

Use the `superpowers:using-git-worktrees` skill. Base the worktree on `origin/master` (fetch first — local `master` is routinely stale in this repo). If the auto-generated branch name is not in `type/scope/desc` form, rename it: `git branch -m feat/refbox/cancel-timeout-button` (the pre-commit hook rejects other shapes).

- [ ] **Step 2: Confirm starting point**

Run: `git -C <worktree> log --oneline -1` and `git -C <worktree> rev-parse --abbrev-ref HEAD`
Expected: HEAD matches `origin/master`; branch is `feat/refbox/cancel-timeout-button`.

> Do NOT stage the `docs/superpowers/` spec or plan files into this branch (repo convention: they stay local).

---

## Task 1: Rename in en-US

**Files:**
- Modify: `refbox/translations/en-US/refbox.ftl:232` and `:250`

- [ ] **Step 1: Change the single-line key**

`refbox/translations/en-US/refbox.ftl` line 232:
```
end-timeout = CANCEL TIMEOUT
```

- [ ] **Step 2: Change the two-line key's first line (leave line 2 as the shared `{ timeout }`)**

`refbox/translations/en-US/refbox.ftl` line 250:
```
end-timeout-line-1 = CANCEL
```
Leave line 251 (`end-timeout-line-2 = { timeout }`) unchanged.

- [ ] **Step 3: Build to confirm the keys still resolve**

Run: `cd <worktree> && cargo build -p refbox`
Expected: builds clean.

- [ ] **Step 4: Commit**

```bash
git add refbox/translations/en-US/refbox.ftl
git commit -m "feat(refbox): rename End Timeout button to Cancel Timeout (en-US)"
```

---

## Task 2: Rename in the other 14 locales

**Files:** Modify `end-timeout` and `end-timeout-line-1` in each of:
`de-DE, es, fr, id-ID, it-IT, ja-JP, ko-KR, ms-MY, nl-NL, pt-PT, th-TH, tl-PH, tr-TR, zh-CN`.

Each value reuses that locale's existing `cancel` verb and `timeout` noun (gathered from the files), so there are **no English placeholders**. `end-timeout-line-2` stays `{ timeout }` in every locale.

- [ ] **Step 1: Apply these values** (line-1 = the cancel verb; single-line = cancel verb + the locale's timeout noun, in natural order)

| Locale | `end-timeout-line-1` | `end-timeout` |
|--------|----------------------|---------------|
| de-DE  | ABBRECHEN   | AUSZEIT ABBRECHEN |
| es     | CANCELAR    | CANCELAR TIEMPO MUERTO |
| fr     | ANNULER     | ANNULER LE TEMPS MORT |
| id-ID  | BATALKAN    | BATALKAN TIME-OUT |
| it-IT  | ANNULLA     | ANNULLA TIME-OUT |
| ja-JP  | キャンセル    | タイムアウトをキャンセル |
| ko-KR  | 취소         | 타임아웃 취소 |
| ms-MY  | BATALKAN    | BATALKAN MASA REHAT |
| nl-NL  | ANNULEREN   | TIME-OUT ANNULEREN |
| pt-PT  | CANCELAR    | CANCELAR TEMPO DE EQUIPA |
| th-TH  | ยกเลิก       | ยกเลิกพักทีม |
| tl-PH  | KANSELAHIN  | KANSELAHIN ANG TIMEOUT |
| tr-TR  | İPTAL       | MOLAYI İPTAL ET |
| zh-CN  | 取消         | 取消暂停 |

(When editing, open each file and confirm the locale's actual `cancel`/`timeout` wording matches before substituting; adjust word order to whatever reads naturally in that file's existing style.)

- [ ] **Step 2: Verify no locale still says the old "END" wording for these two keys, and none was left in English**

Run: `cd <worktree> && grep -rn "^end-timeout = \|^end-timeout-line-1 = " refbox/translations/`
Expected: 15 lines each for the two keys; none contains the old verb ("END"/"AKHIRI"/"TERMINAR"/"FIN"/"FINE"/"TAMAT"/"TAPUSIN"/"BİTİR"/"结束"/"종료"/"終了"/"สิ้นสุด"/"BEENDIGEN"/"BEENDEN"); en-US is the only one whose value is English.

- [ ] **Step 3: Build**

Run: `cd <worktree> && cargo build -p refbox`
Expected: builds clean.

- [ ] **Step 4: Commit**

```bash
git add refbox/translations
git commit -m "feat(refbox): rename End Timeout to Cancel Timeout (all locales)"
```

---

## Task 3: Orange fill on every Cancel Timeout button

**Files:**
- Modify: `refbox/src/app/view_builders/main_view.rs:72`
- Modify: `refbox/src/app/view_builders/shared_elements.rs:197,223,249,275`

- [ ] **Step 1: Swap the centre cancel button's style**

In `main_view.rs`, the `else` branch of the `snapshot.timeout.is_some()` block:
```rust
center_col = center_col.push(
    make_button(fl!("end-timeout"))
        .style(orange_button)
        .on_press(Message::EndTimeout),
);
```
(Only `yellow_button` → `orange_button` changes.)

- [ ] **Step 2: Swap the four ribbon cancel buttons' style**

In `shared_elements.rs`, in each of the four `Some(TimeoutSnapshot::X(_)) => { make_multi_label_button((fl!("end-timeout-line-1"), fl!("end-timeout-line-2"))).on_press(Message::EndTimeout).style(yellow_button) }` arms (Black, White, Ref, PenaltyShot), change `.style(yellow_button)` to `.style(orange_button)`.

- [ ] **Step 3: Confirm `orange_button` is in scope**

`orange_button` is already imported in `shared_elements.rs` and `main_view.rs` (used by the foul button). Run: `cd <worktree> && cargo build -p refbox`
Expected: builds clean (no missing-import error).

- [ ] **Step 4: Commit**

```bash
git add refbox/src/app/view_builders/main_view.rs refbox/src/app/view_builders/shared_elements.rs
git commit -m "feat(refbox): give Cancel Timeout button an orange fill for all timeout types"
```

---

## Task 4: Honest "Switch to …" labels in the ribbon

**Files:**
- Modify: `refbox/src/app/view_builders/shared_elements.rs` — the four "other timeout active" arms in `build_timeout_ribbon` (currently lines ~199-209, ~225-235, ~251-261, ~277-288).

For each non-active slot: branch on the existing `can_switch_to_*` result. `Ok` → "SWITCH TO …", clickable (now an unconditional `.on_press`, since we are inside the `Ok` arm). `Err` → that timeout's own start label, no `.on_press` (greyed). Style is unchanged per slot.

- [ ] **Step 1: Black slot — replace the `Some(White|Ref|PenaltyShot)` arm**

```rust
        Some(TimeoutSnapshot::White(_))
        | Some(TimeoutSnapshot::Ref(_))
        | Some(TimeoutSnapshot::PenaltyShot(_)) => {
            match tm.can_switch_to_team_timeout(GameColor::Black) {
                Ok(()) => make_multi_label_button((fl!("switch-to"), fl!("dark-team-name-caps")))
                    .on_press(Message::TeamTimeout(GameColor::Black, true))
                    .style(black_button),
                Err(_) => make_multi_label_button((
                    fl!("dark-timeout-line-1"),
                    fl!("dark-timeout-line-2"),
                ))
                .style(black_button),
            }
        }
```

- [ ] **Step 2: White slot — replace the `Some(Black|Ref|PenaltyShot)` arm**

```rust
        Some(TimeoutSnapshot::Black(_))
        | Some(TimeoutSnapshot::Ref(_))
        | Some(TimeoutSnapshot::PenaltyShot(_)) => {
            match tm.can_switch_to_team_timeout(GameColor::White) {
                Ok(()) => make_multi_label_button((fl!("switch-to"), fl!("light-team-name-caps")))
                    .on_press(Message::TeamTimeout(GameColor::White, true))
                    .style(white_button),
                Err(_) => make_multi_label_button((
                    fl!("light-timeout-line-1"),
                    fl!("light-timeout-line-2"),
                ))
                .style(white_button),
            }
        }
```

- [ ] **Step 3: Ref slot — replace the `Some(Black|White|PenaltyShot)` arm**

```rust
        Some(TimeoutSnapshot::Black(_))
        | Some(TimeoutSnapshot::White(_))
        | Some(TimeoutSnapshot::PenaltyShot(_)) => {
            match tm.can_switch_to_ref_timeout() {
                Ok(()) => make_multi_label_button((fl!("switch-to"), fl!("ref")))
                    .on_press(Message::RefTimeout(true))
                    .style(yellow_button),
                Err(_) => make_multi_label_button((
                    fl!("ref-timeout-line-1"),
                    fl!("ref-timeout-line-2"),
                ))
                .style(yellow_button),
            }
        }
```

- [ ] **Step 4: Penalty slot — replace the `Some(Black|White|Ref)` arm**

```rust
        Some(TimeoutSnapshot::Black(_))
        | Some(TimeoutSnapshot::White(_))
        | Some(TimeoutSnapshot::Ref(_)) => {
            let can_switch = if mode == Mode::Rugby {
                tm.can_switch_to_rugby_penalty_shot()
            } else {
                tm.can_switch_to_penalty_shot()
            };
            match can_switch {
                Ok(()) => make_multi_label_button((fl!("switch-to"), fl!("pen-shot")))
                    .on_press(Message::PenaltyShot(true))
                    .style(red_button),
                Err(_) => make_multi_label_button((
                    fl!("penalty-shot-line-1"),
                    fl!("penalty-shot-line-2"),
                ))
                .style(red_button),
            }
        }
```

- [ ] **Step 5: Build and lint**

Run: `cd <worktree> && cargo build -p refbox && cargo clippy -p refbox -- -D warnings`
Expected: builds clean, zero clippy warnings.

- [ ] **Step 6: Commit**

```bash
git add refbox/src/app/view_builders/shared_elements.rs
git commit -m "feat(refbox): show switch-to label only when the switch is available"
```

---

## Task 5: Full validation + manual walkthrough

- [ ] **Step 1: Run the full gate**

Run: `cd <worktree> && just check`
Expected: fmt, clippy (`-D warnings`), tests, and audit all clean.

- [ ] **Step 2: Launch the app and walk through the four timeout states**

Launch the built binary in the background with `WAYLAND_DISPLAY=` (force X11 on WSLg) and `dangerouslyDisableSandbox: true`. The human drives the UI; confirm against the design's "How to verify" list:
  1. Team timeout → team slot "CANCEL"/"TIMEOUT" orange; Ref + Penalty slots show "Ref Timeout" / "Penalty Shot" greyed (not "SWITCH TO …").
  2. Ref timeout → Ref slot "CANCEL"/"TIMEOUT" orange; team slots show "Dark/Light Timeout" greyed; "Switch to Pen Shot" clickable.
  3. Penalty shot → Penalty slot "CANCEL"/"TIMEOUT" orange; "Switch to Ref" clickable; team slots greyed with their own names.
  4. "Track fouls & warnings" off + any timeout → centre button reads "CANCEL TIMEOUT" orange.

- [ ] **Step 3: Code review + PR**

Run `superpowers:requesting-code-review`, then prepare the PR per `.claude/rules/pr-review.md` (plain-language What/Why/Scope/How-to-verify body). Get the human's approval before pushing/opening the PR.
