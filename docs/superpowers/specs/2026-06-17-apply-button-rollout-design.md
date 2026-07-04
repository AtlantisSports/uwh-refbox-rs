# Apply-button rollout (Done → Apply) — Design

**Date:** 2026-06-17
**Branch (to be created off fresh `origin/master`):** `feat/refbox/apply-button-rollout`
**Follows:** PR #1218 (time-edit Done→Apply). Independent of it — no file overlap.

---

## Goal

On six commit-style refbox pages, rename the green **"Done"** button to **"Apply"** (reusing the
existing `apply` Fluent key — already in all 15 locales, no new key) **and** gray the button out
until the operator has actually made a change, so "Apply" only lights up when there is something
to apply.

This brings these pages in line with the house style already used by the time editor (PR #1218)
and the configuration sub-pages, both of which already say "Apply" and gray when unchanged.

## Scope

**Pages that change (green button: rename + gray-when-unchanged):**

| # | Page | View builder |
|---|------|--------------|
| 1 | Parameter editor | `refbox/src/app/view_builders/configuration.rs` — `build_game_parameter_editor` |
| 2 | Game-number editor | `refbox/src/app/view_builders/keypad_pages/game_number_edit.rs` — `make_game_number_edit_page` |
| 3 | Score EDIT | `refbox/src/app/view_builders/score_edit.rs` — `build_score_edit_view` |
| 4 | Penalties overview | `refbox/src/app/view_builders/penalties.rs` — `build_penalty_overview_page` |
| 5 | Warnings overview | `refbox/src/app/view_builders/warnings.rs` — `build_warning_overview_page` |
| 6 | Fouls overview | `refbox/src/app/view_builders/fouls.rs` — `build_foul_overview_page` |

Plus the matching dispatch lines in `refbox/src/app/mod.rs` `view()` that pass the new
"original value" argument into pages 1–3, and the `build_keypad_page` signature in
`keypad_pages/mod.rs` (threads one new arg through to page 2).

**Explicitly out of scope (unchanged):**

- The pages that keep **"Done"**: `score_add`, `foul_add`, `warning_add`, `penalty_edit`,
  `portal_login`.
- The configuration sub-pages (Game / App / Display / Sound / Remotes / Language) — already
  "Apply" + gated via `make_cancel_apply_footer` / `page_has_changes`.
- PR #1218's `time_edit.rs` (separate branch).
- The existing parameter-editor quirk where it *opens* from `config.game.*` but *applies* to
  `edited_settings.config.*`. Left exactly as-is. Change detection compares against what is
  *shown* (i.e. `config.game.*`), so it stays correct regardless of that quirk.
- No new Fluent key, no English placeholders, no new dependency, no `RefBoxApp` struct fields,
  no `AppState` enum changes.

## Key design decisions (settled in brainstorming)

1. **Rename + gray everywhere** (not rename-only). Matches the time editor and config pages.
2. **Score EDIT: edit-mode only.** The same view (`build_score_edit_view`) is reused for the
   end-of-game final-score *confirmation* (`is_confirmation == true`), where Cancel is already
   disabled and the operator *must* be able to commit the score as-is. So the rename + gray apply
   **only when `is_confirmation == false`**. In confirmation mode the button keeps `fl!("done")`
   and stays always-clickable. (Prevents trapping the operator on the confirmation screen.)
3. **No new state — re-derive the "original" at render time.** PR #1218 added a `time_edit_old`
   field because it had just *stopped a running clock*: the pre-edit time was live and would be
   lost. None of our three editors edit a *live* value — each pre-edit value sits untouched in a
   stable place until Apply — so we re-derive it where it already lives. Zero struct/enum changes.

## How "changed" is determined per page

### Value editors (pages 1–3): current edit buffer vs. the value shown when the screen opened

A small `*_has_changes` helper lives in each view file with unit tests, mirroring #1218's
`time_edit_has_changes`. The "original" is re-derived at the `view()` dispatch site and passed in.

| Page | Original (re-derived, no new state) | "changed" test |
|------|-------------------------------------|----------------|
| Parameter editor | `config.game.*` matched on `param` — the *same expression* `Message::EditParameter` uses to open the editor. `config.game.*` is not mutated until Apply. | `dur.as_secs() != old.as_secs()` — whole-second / display precision, exactly like #1218 (the editor shows mm:ss). |
| Game-number editor | `edited_settings.game_number`, passed in as `Option<GameNumber>` (`Some` whenever this keypad page is live; other keypad pages pass `None`). | `player_num.to_string() != old` — the exact string `ParameterEditComplete` would write on Apply. |
| Score EDIT | `self.snapshot.scores` (a `BlackWhiteBundle<u8>`; the tournament-manager scores, and thus the snapshot, are not changed during an edit — only the `AppState::ScoreEdit { scores }` buffer is). | `scores != old` — bundle equality. Only evaluated when `is_confirmation == false`. |

### List pages (pages 4–6): any row is not `NoChange`

No new state at all. Every row already carries a `FormatHint`
(`NoChange` / `Edited` / `Deleted` / `New`), computed by the existing `ListEditor` diff. "Has
changes" = any row across all lists on the page is not `NoChange`. Computed inside the existing
`build_*_overview_page`, before the row vectors are consumed by the per-color list builders:

```rust
let has_changes = <all rows on the page>
    .iter()
    .any(|row| !matches!(row.format_hint, FormatHint::NoChange));
```

- Penalties: scan `penalties.black` + `penalties.white`.
- Warnings: scan `warnings.black` + `warnings.white`.
- Fouls: scan `warnings.black` + `warnings.equal` + `warnings.white` (3 lists).

## Common rendering pattern (all six)

- Green button text: `fl!("done")` → `fl!("apply")`.
- Keep `.style(green_button)`. Gate the press:
  `.on_press_maybe(has_changes.then_some(Message::…Complete { canceled: false }))`.
  A `green_button` with no `on_press` renders grayed — the identical mechanism used by both
  PR #1218 (`time_edit.rs`) and the config footer (`make_cancel_apply_footer`).
- Cancel button and the blue "New" button on list pages: untouched.

## Acceptance criteria (operator-observable)

For each of the six pages:

1. The green button reads **"Apply"** (localized) instead of "Done".
2. On first opening the page with nothing changed, the Apply button is **grayed and not
   pressable**.
3. After making any real change (edit a length, type a different game number, +/- a score, or
   add/edit/delete a list row), the Apply button becomes **active** and commits as before.
4. Reverting back to the original state (e.g. +1 then −1 on a score; retype the original game
   number) grays the button again.
5. **Score confirmation screen** (end-of-game final-score confirmation) still reads **"Done"**
   and is always pressable.
6. The keep-"Done" pages (`score_add`, `foul_add`, `warning_add`, `penalty_edit`,
   `portal_login`) are unchanged.

## Testing

- Unit test each value editor's `*_has_changes` helper: no-change → false; a change → true;
  round-trip back to original → false (mirrors #1218's helper tests). Parameter editor test also
  covers the whole-second/display-precision case.
- Unit test a list-page change-detection helper (or inline expression): empty / all-`NoChange`
  → false; any `Edited`/`New`/`Deleted` → true.
- `just check` green (fmt, clippy `-D warnings`, tests, audit).
- Manual walkthrough of each of the six screens against the acceptance criteria above, plus a
  check that the score *confirmation* screen still says "Done" and commits.

## Process notes

- Lean process (per `.claude/rules/plan-execution.md`): `refbox` UI feature, no `uwh-common` /
  wire-format / state-machine / embedded changes. One code review at the end, deviations tracked
  in the plan file.
- This spec and the implementation plan stay **local** (project convention: Superpowers
  spec/plan docs are not committed to feature branches/PRs).
- Do **not** auto-merge PR #1218 — the user merges it via the GitHub button when CI is green.
