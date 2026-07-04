# Timeout-Revive Label Text Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the held-button label during the timeout-revive RED and YELLOW phases so it reads "HOLD TO / RESTORE" (red) and "TIMEOUT / RESTORED" (yellow), instead of the normal team-name + "TIMEOUT".

**Architecture:** Pure presentation change in the timeout ribbon. The revive interaction (state machine, hold timing, red→yellow→bank/start flow) already exists and is unchanged. We add 3 new translation keys and swap which keys the four revive faces (red/yellow × black/white) render.

**Tech Stack:** Rust 2024, iced 0.13 (`refbox` crate), Fluent (`.ftl`) translation files.

## Global Constraints

- **Crate scope:** `refbox` only. No `uwh-common`, no wire format, no state machine changes.
- **MSRV:** Rust 1.85; **Edition:** Rust 2024.
- **Clippy:** `cargo clippy -p refbox -- -D warnings` must be clean (mirrors `just lint`).
- **Literal values (exact, ALL CAPS):** red phase line 1 = `HOLD TO`, line 2 = `RESTORE`; yellow phase line 1 = `TIMEOUT` (reuse existing `timeout` term), line 2 = `RESTORED`.
- **Translations:** all 15 locales (de-DE, en-US, es, fr, id-ID, it-IT, ja-JP, ko-KR, ms-MY, nl-NL, pt-PT, th-TH, tl-PH, tr-TR, zh-CN) get a best-guess translation — **no English placeholders** (en-US fallback notwithstanding).
- **Team name intentionally dropped** during red/yellow — the team is implied by which button is held. Do not add a team-name line.
- **Process:** lean (translation-key + view wiring). Compile + clippy + `just check`. No per-task code review; one review at feature end.

---

### Task 1: Add the three new translation keys to all 15 locales

**Files:**
- Modify: `refbox/translations/en-US/refbox.ftl` (timeout-ribbon section, near `dark-timeout-line-1`)
- Modify the same `refbox.ftl` under each of the other 14 locale directories listed in Global Constraints.

**Interfaces:**
- Produces: three Fluent message keys usable from `fl!()` in `shared_elements.rs`:
  - `revive-hold-line-1` → "HOLD TO" (red phase, line 1)
  - `revive-hold-line-2` → "RESTORE" (red phase, line 2)
  - `revive-deciding-line-2` → "RESTORED" (yellow phase, line 2)
- The yellow phase line 1 reuses the **existing** `timeout` term (`{ timeout }`) — no new key for it.

- [ ] **Step 1: Add the keys to en-US**

In `refbox/translations/en-US/refbox.ftl`, in the `## Timeout ribbon` section (just after the `light-timeout-line-2` entry), add:

```ftl
revive-hold-line-1 = HOLD TO
revive-hold-line-2 = RESTORE
revive-deciding-line-2 = RESTORED
```

- [ ] **Step 2: Add best-guess translations to the other 14 locales**

In each locale's `refbox/translations/<locale>/refbox.ftl`, add the same three keys in the timeout-ribbon section with a best-guess ALL-CAPS translation appropriate to that language. Recommended values (refine per existing wording in each file; match how each file already renders "TIMEOUT" and imperative button copy):

| Locale | revive-hold-line-1 (HOLD TO) | revive-hold-line-2 (RESTORE) | revive-deciding-line-2 (RESTORED) |
|--------|------------------------------|------------------------------|-----------------------------------|
| de-DE | HALTEN ZUM | WIEDERHERST. | WIEDERHERGEST. |
| es | MANTENER PARA | RESTAURAR | RESTAURADO |
| fr | MAINTENIR POUR | RESTAURER | RESTAURÉ |
| id-ID | TAHAN UNTUK | PULIHKAN | DIPULIHKAN |
| it-IT | TIENI PER | RIPRISTINA | RIPRISTINATO |
| ja-JP | 長押しで | 復元 | 復元しました |
| ko-KR | 길게 눌러 | 복원 | 복원됨 |
| ms-MY | TAHAN UNTUK | PULIH | DIPULIHKAN |
| nl-NL | HOUD VAST OM | HERSTELLEN | HERSTELD |
| pt-PT | SEGURE PARA | RESTAURAR | RESTAURADO |
| th-TH | กดค้างเพื่อ | คืนค่า | คืนค่าแล้ว |
| tl-PH | PINDUTIN PARA | IBALIK | NAIBALIK |
| tr-TR | BASILI TUT | GERİ AL | GERİ ALINDI |
| zh-CN | 长按以 | 恢复 | 已恢复 |

Keep each within the same visual length ballpark as the existing `*-timeout-line-*` values so it fits the button face. Where a language's natural phrasing for "HOLD TO" + verb reads awkwardly split across two lines, prefer the clearest two-line reading over a literal word-for-word split.

- [ ] **Step 3: Verify no locale is missing a key**

Run from the worktree root:

```bash
for k in revive-hold-line-1 revive-hold-line-2 revive-deciding-line-2; do
  echo "== $k =="; grep -L "$k" refbox/translations/*/refbox.ftl
done
```

Expected: no file paths printed under any key (every locale contains all three keys). Any path printed = that locale is missing that key; add it.

- [ ] **Step 4: Commit is deferred**

Do not commit yet — Task 2 changes the view and the whole feature lands as one commit (see Task 2, Step 5).

---

### Task 2: Wire the new keys into the four revive faces

**Files:**
- Modify: `refbox/src/app/view_builders/shared_elements.rs` — the `build_timeout_ribbon` function, the black Deciding/Reviving branches (~lines 198–229) and the white Deciding/Reviving branches (~lines 268–293).

**Interfaces:**
- Consumes: `revive-hold-line-1`, `revive-hold-line-2`, `revive-deciding-line-2` (Task 1), plus the existing `timeout` term.
- The existing `make_multi_label_button((line1, line2))` helper and the `red_button_armed` / `yellow_button_armed` styles are unchanged.

- [ ] **Step 1: Update the BLACK Deciding (yellow) branch**

In `build_timeout_ribbon`, the black branch where `black_phase == Some(RevivePhase::Deciding)` currently builds:

```rust
make_multi_label_button((
    fl!("dark-timeout-line-1"),
    fl!("dark-timeout-line-2"),
))
.style(yellow_button_armed),
```

Change the two label lines to:

```rust
make_multi_label_button((fl!("timeout"), fl!("revive-deciding-line-2")))
.style(yellow_button_armed),
```

(Keep the surrounding `mouse_area(...)`, `.on_press`/`.on_release`/`.on_exit` wiring exactly as-is.)

- [ ] **Step 2: Update the BLACK Reviving (red) face**

In the same black branch, where `black_phase == Some(RevivePhase::Reviving)` builds the red face:

```rust
make_multi_label_button((
    fl!("dark-timeout-line-1"),
    fl!("dark-timeout-line-2"),
))
.style(red_button_armed)
```

Change the two label lines to:

```rust
make_multi_label_button((fl!("revive-hold-line-1"), fl!("revive-hold-line-2")))
.style(red_button_armed)
```

Leave the `else` (greyed, not-yet-holding) face using `dark-timeout-line-1/2` unchanged — that is still the normal team-name + TIMEOUT label.

- [ ] **Step 3: Update the WHITE Deciding (yellow) branch**

In the white branch where `white_phase == Some(RevivePhase::Deciding)`, change:

```rust
make_multi_label_button((
    fl!("light-timeout-line-1"),
    fl!("light-timeout-line-2"),
))
.style(yellow_button_armed),
```

to:

```rust
make_multi_label_button((fl!("timeout"), fl!("revive-deciding-line-2")))
.style(yellow_button_armed),
```

- [ ] **Step 4: Update the WHITE Reviving (red) face**

In the white branch where `white_phase == Some(RevivePhase::Reviving)`, change:

```rust
make_multi_label_button((
    fl!("light-timeout-line-1"),
    fl!("light-timeout-line-2"),
))
.style(red_button_armed)
```

to:

```rust
make_multi_label_button((fl!("revive-hold-line-1"), fl!("revive-hold-line-2")))
.style(red_button_armed)
```

Leave the white `else` (greyed) face using `light-timeout-line-1/2` unchanged.

- [ ] **Step 5: Verify, then commit the whole feature change**

Run from the worktree root (`.claude/worktrees/feat+refbox+timeout-revive-long-press`):

```bash
just check
```

Expected: PASS (fmt, clippy `-D warnings`, tests, audit all clean).

Then commit Task 1 + Task 2 together:

```bash
git add refbox/translations refbox/src/app/view_builders/shared_elements.rs
git commit -m "feat(refbox): label timeout-revive phases HOLD TO/RESTORE and TIMEOUT/RESTORED"
```

---

## Manual Walkthrough (after commit)

Build and launch the binary, then confirm by eye:

```bash
WAYLAND_DISPLAY= ./target/debug/refbox   # background, sandbox disabled
```

Drive the UI to where a team has used all its team timeouts (button greyed), then:
1. Press and hold the greyed team-timeout button → the button turns **RED** and reads **HOLD TO / RESTORE** (no team name).
2. Continue holding past the revive → the button turns **YELLOW** and reads **TIMEOUT / RESTORED**.
3. Release in the yellow window → the timeout is banked back (button returns to its normal greyed/active TIMEOUT label).
4. Repeat for the other team's button to confirm both colours show the same phase labels.

## Self-Review Notes

- **Spec coverage:** 3 new keys (Task 1) + four faces rewired (Task 2) + team-name drop (no team-name line added). Covered.
- **Type consistency:** key names `revive-hold-line-1`, `revive-hold-line-2`, `revive-deciding-line-2` and the reused `timeout` term are used identically in Task 1 (defined) and Task 2 (consumed).
- **Out of scope (unchanged):** the greyed not-holding faces (`dark-/light-timeout-line-1/2`), the running-timeout / switch-to / ref branches, the revive state machine, button styles.
