# ADR 018 Event/Court Picker Sort Order — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Sort the event picker by event start date (nearest-upcoming first) and eliminate scroll-position bugs in both the event and court pickers by always opening the pickers at the top of the list.

**Architecture:** Two small refbox-only changes, no `uwh-common` touch, no wire-format change. (1) Extract a pure sort helper that orders events by `(date_range.start, date_range.end, id)`, unit-test it, and use it in the event picker view. (2) Replace the initial-scroll-index lookups for the event and court arms of `Message::SelectParameter` with a constant `Some(0)`. The game picker is untouched.

**Tech Stack:** Rust 2024 edition, MSRV 1.85, iced 0.13, `time` 0.3 (already a workspace dependency).

**Source spec:** [docs/superpowers/specs/2026-05-15-event-picker-sort-design.md](../specs/2026-05-15-event-picker-sort-design.md)
**ADR:** [docs/decisions/018-event-picker-sort-order.md](../../decisions/018-event-picker-sort-order.md)

**Process tier:** Lean (per [.claude/rules/plan-execution.md](../../../.claude/rules/plan-execution.md)). This is refbox UI work — no state machine, no wire format, no `uwh-common`. One code review at the end, no per-task deviation commits, mechanical tasks skip verification ceremony.

---

## Pre-flight — Operator Approval Required

**Do not proceed past this gate without explicit operator approval.** Per the project's communication rule, branch and worktree creation needs go-ahead.

Suggested setup:

- Branch name: `fix/refbox/event-picker-sort`
- Worktree path: `.worktrees/event-picker-sort/`
- Base: `master` (ADR-016 is in flight on a separate branch; per the spec, ADR-016 lands first and ADR-018 rebases onto whatever's current at integration time)

Steps:

- [ ] Confirm operator approval to create the branch.
- [ ] Invoke `superpowers:using-git-worktrees` to create the worktree.
- [ ] `cd` into the worktree directory before running any `cargo` or `just` commands. (Memory rule: the Bash tool doesn't preserve `cwd`; not cd-ing produces silent fallthrough to the master binary.)
- [ ] Baseline check: run `just check` from the worktree root and confirm a clean pass before any edits. Record the result.

---

## Task 1: Extract a pure sort helper for the event picker, with unit tests

**Files:**
- Modify: `refbox/src/app/view_builders/list_selector.rs`

**Why this is a separate task:** The current sort is incidental (`events.values().rev()`); the new sort has real ordering semantics that deserve unit tests. Extracting the logic to a pure function makes it testable without rendering iced widgets.

- [ ] **Step 1: Add a failing test for ascending sort by start date**

Inside `refbox/src/app/view_builders/list_selector.rs`, after the existing `build_list_selector_page` function (after the closing `}` of the function around current line 147), add the following test module:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::datetime;
    use uwh_common::uwhportal::schedule::DateRange;

    fn test_event(partial_id: &str, start: time::OffsetDateTime, end: time::OffsetDateTime) -> Event {
        Event {
            id: EventId::from_partial(partial_id),
            name: format!("Event {partial_id}"),
            slug: partial_id.to_string(),
            date_range: DateRange { start, end },
            teams: None,
            schedule: None,
            courts: None,
        }
    }

    #[test]
    fn sorts_events_by_start_date_ascending() {
        let mut events = BTreeMap::new();
        let later = test_event(
            "later",
            datetime!(2026-06-01 0:00 UTC),
            datetime!(2026-06-03 0:00 UTC),
        );
        let sooner = test_event(
            "sooner",
            datetime!(2026-05-15 0:00 UTC),
            datetime!(2026-05-17 0:00 UTC),
        );
        events.insert(later.id.clone(), later);
        events.insert(sooner.id.clone(), sooner);

        let sorted = sorted_events_for_picker(&events);
        assert_eq!(sorted[0].slug, "sooner");
        assert_eq!(sorted[1].slug, "later");
    }
}
```

- [ ] **Step 2: Run the test to confirm it fails**

Run from the worktree root:

```bash
cargo test -p refbox --lib --no-fail-fast view_builders::list_selector::tests
```

Expected: compilation error — `sorted_events_for_picker` is not defined. That's the expected failure mode for a TDD red phase.

- [ ] **Step 3: Add the sort helper function**

Inside `refbox/src/app/view_builders/list_selector.rs`, *above* the existing `build_list_selector_page` function (so somewhere between the `use` block at the top and the existing `pub(in super::super) fn build_list_selector_page` declaration around line 9), insert:

```rust
fn sorted_events_for_picker(events: &BTreeMap<EventId, Event>) -> Vec<&Event> {
    let mut sorted: Vec<&Event> = events.values().collect();
    sorted.sort_by(|a, b| {
        a.date_range
            .start
            .cmp(&b.date_range.start)
            .then(a.date_range.end.cmp(&b.date_range.end))
            .then(a.id.cmp(&b.id))
    });
    sorted
}
```

- [ ] **Step 4: Run the test to confirm it passes**

```bash
cargo test -p refbox --lib --no-fail-fast view_builders::list_selector::tests
```

Expected: `test view_builders::list_selector::tests::sorts_events_by_start_date_ascending ... ok`

- [ ] **Step 5: Add a tiebreaker test (same start date, different end dates)**

Append to the existing `tests` module:

```rust
    #[test]
    fn breaks_ties_on_end_date_ascending() {
        let mut events = BTreeMap::new();
        let longer = test_event(
            "longer",
            datetime!(2026-05-15 0:00 UTC),
            datetime!(2026-05-20 0:00 UTC),
        );
        let shorter = test_event(
            "shorter",
            datetime!(2026-05-15 0:00 UTC),
            datetime!(2026-05-16 0:00 UTC),
        );
        events.insert(longer.id.clone(), longer);
        events.insert(shorter.id.clone(), shorter);

        let sorted = sorted_events_for_picker(&events);
        assert_eq!(sorted[0].slug, "shorter");
        assert_eq!(sorted[1].slug, "longer");
    }
```

- [ ] **Step 6: Run the tests to confirm both pass**

```bash
cargo test -p refbox --lib --no-fail-fast view_builders::list_selector::tests
```

Expected: 2 passed.

- [ ] **Step 7: Add a stability tiebreaker test (same start AND end, different EventIds)**

Append to the `tests` module:

```rust
    #[test]
    fn breaks_ties_on_event_id() {
        let mut events = BTreeMap::new();
        let aaa = test_event(
            "aaa",
            datetime!(2026-05-15 0:00 UTC),
            datetime!(2026-05-17 0:00 UTC),
        );
        let bbb = test_event(
            "bbb",
            datetime!(2026-05-15 0:00 UTC),
            datetime!(2026-05-17 0:00 UTC),
        );
        events.insert(aaa.id.clone(), aaa);
        events.insert(bbb.id.clone(), bbb);

        let sorted = sorted_events_for_picker(&events);
        // EventId Ord is lexicographic on the full id ("events/aaa" < "events/bbb")
        assert_eq!(sorted[0].slug, "aaa");
        assert_eq!(sorted[1].slug, "bbb");
    }
```

- [ ] **Step 8: Run the tests to confirm all three pass**

```bash
cargo test -p refbox --lib --no-fail-fast view_builders::list_selector::tests
```

Expected: 3 passed.

- [ ] **Step 9: Format and lint, then commit**

```bash
just fmt
cargo clippy -p refbox --all-targets --all-features -- -D warnings
```

Expected: both clean. If clippy reports an issue, fix it before continuing.

Commit (using the project's HEREDOC convention so the message formats correctly):

```bash
git add refbox/src/app/view_builders/list_selector.rs
git commit -m "$(cat <<'EOF'
fix(refbox): extract event picker sort helper with unit tests

Pulls the picker's event ordering into a pure function sorted by
(date_range.start, date_range.end, id) and adds unit tests for the
primary key and tiebreakers. The helper is not yet wired into the
view; that follows in the next commit.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 2: Wire the sort helper into the event picker view

**Files:**
- Modify: `refbox/src/app/view_builders/list_selector.rs` (the `ListableParameter::Event` arm, currently lines 75–81)

- [ ] **Step 1: Replace the unsorted iterator with the sorted helper**

In `refbox/src/app/view_builders/list_selector.rs`, find this block (currently around lines 75–81):

```rust
        ListableParameter::Event => {
            let list = events.as_ref().unwrap();
            let num_items = list.len();
            let iter = list.values().rev();
            let transform = |e: &Event| (e.name.clone(), e.id.full().to_string());
            (num_items, make_buttons!(iter, transform))
        }
```

Replace it with:

```rust
        ListableParameter::Event => {
            let list = events.as_ref().unwrap();
            let num_items = list.len();
            let sorted = sorted_events_for_picker(list);
            let iter = sorted.into_iter();
            let transform = |e: &Event| (e.name.clone(), e.id.full().to_string());
            (num_items, make_buttons!(iter, transform))
        }
```

- [ ] **Step 2: Verify the crate still builds and tests still pass**

```bash
cargo check -p refbox --all-targets
cargo test -p refbox --lib --no-fail-fast view_builders::list_selector::tests
```

Expected: both clean.

- [ ] **Step 3: Format and lint**

```bash
just fmt
cargo clippy -p refbox --all-targets --all-features -- -D warnings
```

Expected: clean.

- [ ] **Step 4: Commit**

```bash
git add refbox/src/app/view_builders/list_selector.rs
git commit -m "$(cat <<'EOF'
fix(refbox): sort event picker by start date

Events in the picker now appear in chronological order, with the
nearest-upcoming event at the top, instead of in EventId order.
Operators key off date proximity, not the portal's internal id.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 3: Always scroll the event picker to the top on open

**Files:**
- Modify: `refbox/src/app/mod.rs` (the `ListableParameter::Event` arm inside `Message::SelectParameter`, currently around lines 1480–1489)

**Why this is mechanical:** No behaviour test fits — the change is a constant. Verification is by `cargo check` and the manual walkthrough in Task 5.

- [ ] **Step 1: Replace the lookup block with `Some(0)`**

In `refbox/src/app/mod.rs`, find this block inside `Message::SelectParameter(param)`'s `let index = match param { ... }` (currently around lines 1480–1489):

```rust
                    ListableParameter::Event => {
                        self.current_event_id.as_ref().and_then(|cur_event_id| {
                            self.events
                                .as_ref()?
                                .iter()
                                .enumerate()
                                .find(|(_, (event_id, _))| **event_id == *cur_event_id)
                                .map(|(i, _)| i)
                        })
                    }
```

Replace it with:

```rust
                    ListableParameter::Event => Some(0),
```

The surrounding `.unwrap_or(0)` is still fine; explicit `Some(0)` documents intent.

- [ ] **Step 2: Verify the crate still builds**

```bash
cargo check -p refbox --all-targets
```

Expected: clean.

- [ ] **Step 3: Format and lint**

```bash
just fmt
cargo clippy -p refbox --all-targets --all-features -- -D warnings
```

Expected: clean. Clippy may warn about the unused field `current_event_id` if no other code uses it — confirm by reading the warning, but `self.current_event_id` is read elsewhere in the file (the court picker arm at the next index lookup, the schedule-receive handler, and others). Do not silence warnings.

- [ ] **Step 4: Commit**

```bash
git add refbox/src/app/mod.rs
git commit -m "$(cat <<'EOF'
fix(refbox): event picker always opens scrolled to top

Replaces the index lookup against the non-reversed BTreeMap with a
constant 0. This fixes the disappearing-row bug (where the picker
hid the first rows because the view iterated reversed but the index
was computed unreversed) by removing the mismatched lookup entirely.
Combined with the new date sort, the top of the list is always the
next-upcoming event.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 4: Always scroll the court picker to the top on open

**Files:**
- Modify: `refbox/src/app/mod.rs` (the `ListableParameter::Court` arm inside `Message::SelectParameter`, currently around lines 1490–1500)

**Why this is mechanical:** Same reasoning as Task 3. The bug here is the applied-vs-edited mismatch identified during brainstorming; setting the index to a constant 0 removes the read of `self.current_court` entirely, so the mismatch cannot occur.

- [ ] **Step 1: Replace the lookup block with `Some(0)`**

In `refbox/src/app/mod.rs`, find this block (currently around lines 1490–1500):

```rust
                    ListableParameter::Court => self.current_court.as_ref().and_then(|cur_court| {
                        self.events
                            .as_ref()?
                            .get(self.current_event_id.as_ref()?)?
                            .courts
                            .as_ref()?
                            .iter()
                            .enumerate()
                            .find(|(_, court)| **court == *cur_court)
                            .map(|(i, _)| i)
                    }),
```

Replace it with:

```rust
                    ListableParameter::Court => Some(0),
```

Leave the `ListableParameter::Game` arm immediately below unchanged.

- [ ] **Step 2: Verify the crate still builds**

```bash
cargo check -p refbox --all-targets
```

Expected: clean.

- [ ] **Step 3: Format and lint**

```bash
just fmt
cargo clippy -p refbox --all-targets --all-features -- -D warnings
```

Expected: clean. As with Task 3, `self.current_court` is used elsewhere in the file, so removing this one read should not trigger an unused-field warning. If clippy *does* warn, read the warning carefully — the right response is to investigate, not to add `#[allow]`.

- [ ] **Step 4: Commit**

```bash
git add refbox/src/app/mod.rs
git commit -m "$(cat <<'EOF'
fix(refbox): court picker always opens scrolled to top

Replaces the index lookup against self.current_court (applied state)
with a constant 0. This removes the applied-vs-edited mismatch where
re-opening the picker after picking a court would land on the
*previously applied* court's position, sometimes hiding the
just-picked court from view.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 5: Full validation and manual acceptance walkthrough

**Why this is required:** The four changes above are individually small, but the operator-visible behavior is what matters. Per the spec's acceptance criteria, each criterion has a concrete observable check that needs to be run before claiming completion.

- [ ] **Step 1: Run `just check`**

```bash
just check
```

Expected: all green (fmt, clippy on all targets across the workspace, tests, audit).

If anything fails: stop, diagnose root cause, fix, re-run. Do not skip or bypass.

- [ ] **Step 2: Launch the refbox**

Per memory rules:

```bash
WAYLAND_DISPLAY= cargo run -p refbox
```

Run with `run_in_background: true` so the operator can interact with it while the agent stays available. Use `dangerouslyDisableSandbox: true` because the refbox needs Wayland/X11 socket access (per saved feedback).

- [ ] **Step 3: Manually walk the six acceptance criteria from the spec**

The operator drives the UI. For each criterion in [the spec](../specs/2026-05-15-event-picker-sort-design.md#acceptance-criteria), confirm the observable outcome. Capture results in chat — pass / fail / notes per criterion.

Summary of what to check:

1. Event picker shows next-upcoming first.
2. Event picker always opens at the top (pick a non-top event, return, reopen).
3. Event picker no longer hides rows (the original disappearing-scroll bug).
4. Court picker always opens at the top (needs an event with 5+ courts).
5. Court picker no longer hides the just-picked row (needs an event with 5+ courts).
6. Game picker is unchanged (still centered on current game).

For criteria 4 and 5: if no event in the loaded portal data has 5+ courts, note that explicitly and either (a) test against the dev portal (`UWH_PORTAL_URL_OVERRIDE=https://api.dev.uwhportal.com`) if a suitable event exists there, or (b) record that the criterion is unverified-against-real-data and recommend visual companion / synthetic test before merge.

- [ ] **Step 4: Stop the refbox**

Close the window from the UI (operator) or send a termination signal if needed.

- [ ] **Step 5: Summarise results to the operator**

Report: each criterion's outcome, any deviations from the plan, and a recommendation on whether to proceed to PR/integration or to address open items first.

---

## Post-execution: Code review (single pass at the end)

Per lean process, run code review once at the end, not per-task.

- [ ] Invoke `superpowers:requesting-code-review` to review the cumulative diff of Tasks 1–4 against `master` (or whatever base the worktree was cut from).
- [ ] Address any review feedback that affects correctness or violates project rules. Cosmetic feedback can be deferred to a follow-up.

---

## Out of scope (do NOT do)

The following are deliberately excluded from this plan. If you encounter them, stop and ask the operator before acting:

- Any change to the game picker (`ListableParameter::Game` arm in either file).
- Any change to `uwh-common` (including `Event`, `EventId`, `DateRange`).
- Any change to the `--all-events` CLI flag or its filter behavior.
- Any change to court ordering inside an event (the BTreeSet-derived lexicographic order).
- Any change to picker row layout (no date in rows; per operator decision in Q3).
- Any change addressing ADR-017 (portal data lifecycle / loading state in pickers).
- Touching ADR-016's branch (`feat/refbox/uwr-mode-portal-routing`) or any audit-unit worktree.
- Pushing the branch or opening a PR — per the operator's standing rule, all merges wait for Final Integration.
