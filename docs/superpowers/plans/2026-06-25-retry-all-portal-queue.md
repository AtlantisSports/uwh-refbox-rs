# RETRY ALL (portal queue) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a blue "RETRY ALL" button to the portal status screen that re-sends every unsent game at once (a safe resend, never a force-overwrite).

**Architecture:** A new `PortalManager::retry_all()` resets every queued item's retry timers — including `queued_at`, which returns stuck items to the auto-retry pool — and leaves `force` untouched. The existing background retry task then sends them on its next tick. A new `Message::PortalRetryAll` is raised by a blue footer button on the portal detail page, grayed out when there are no unsent games.

**Tech Stack:** Rust 2024, iced 0.13 (Elm-style message/update/view), Fluent (`fl!`) translations, `time` crate.

## Global Constraints

- MSRV Rust 1.85; edition 2024. No APIs newer than 1.85.
- Clippy must pass `-D warnings` (run `just lint`); no new `#[allow(...)]`.
- No new dependencies. No `unwrap()`/`expect()` in non-test code without justification.
- Crate scope: **`refbox` only**. Do NOT touch `uwh-common`, the wire format, or `tournament_manager`.
- Behavior is **safe resend**: `retry_all` must NEVER set `force = true`.
- Label literal: **"RETRY ALL"** (en-US). All 15 locales get a translation; no English placeholders.
- **Approval gates (project rule):** branch creation and every commit require the human's explicit OK. The executor must pause at each commit step for approval; do not push or open a PR without it. Suggested branch: `feat/refbox/portal-retry-all`.
- Build the refbox binary before any on-screen walkthrough: `cargo build -p refbox` (clippy/test build a different binary).

---

### Task 1: Add the `Message::PortalRetryAll` variant

**Files:**
- Modify: `refbox/src/app/message.rs` (enum variant; `is_repeatable`; `PartialEq`)

**Interfaces:**
- Produces: `Message::PortalRetryAll` — a no-payload message, mirroring `Message::PortalGoToLogin`. Consumed by Task 3 (handler) and Task 5 (button).

- [ ] **Step 1: Add the enum variant**

In `refbox/src/app/message.rs`, immediately after the `RequestPortalRefresh,` variant (currently line 109), add:

```rust
    /// Emitted when the operator taps RETRY ALL on the portal detail
    /// page. `update()` calls `PortalManager::retry_all`, which resets
    /// every queued game (including stuck ones) so the background task
    /// re-attempts them all on its next tick. A plain resend — never a
    /// force-overwrite.
    PortalRetryAll,
```

- [ ] **Step 2: Classify it in `is_repeatable`**

Find the `false`-returning group in `Message::is_repeatable` and the line `| Self::RequestPortalRefresh` (currently line 327). Add the new variant right after it:

```rust
            | Self::RequestPortalRefresh
            | Self::PortalRetryAll
```

- [ ] **Step 3: Add the `PartialEq` arm**

In `impl PartialEq for Message`, after `| (Self::RequestPortalRefresh, Self::RequestPortalRefresh)` (currently line 423), add:

```rust
            | (Self::RequestPortalRefresh, Self::RequestPortalRefresh)
            | (Self::PortalRetryAll, Self::PortalRetryAll)
```

- [ ] **Step 4: Verify it compiles**

Run: `cargo build -p refbox`
Expected: builds (the match in `is_repeatable` is exhaustive; an omission would error with "non-exhaustive patterns").

- [ ] **Step 5: Commit** (pause for approval first)

```bash
git add refbox/src/app/message.rs
git commit -m "feat(refbox): add PortalRetryAll message variant"
```

---

### Task 2: `PortalManager::retry_all()` (core logic, TDD)

**Files:**
- Modify: `refbox/src/portal_manager/mod.rs` (new method after `force_submit`, ~line 536; test in the `#[cfg(test)] mod tests` block near the `force_submit` test ~line 815)

**Interfaces:**
- Produces: `pub fn retry_all(&mut self) -> std::io::Result<()>` — resets every queued item (`attempts = 0`, `last_attempt_at = None`, `queued_at = now`), leaves `force` untouched, persists once, recomputes the indicator, pushes one snapshot. No-op-safe on an empty queue. Consumed by Task 3.
- Consumes: existing `self.queue`, `self.config_dir`, `queue::save`, `self.recompute_indicator()`, `self.push_queue_snapshot()`, and the free fn `is_item_stuck` (same module). All already present.

- [ ] **Step 1: Write the failing test**

In the `mod tests` block of `refbox/src/portal_manager/mod.rs` (it already has `use super::*;`, `mk_young_item()`, and `mk_stuck_item()`), add:

```rust
    #[tokio::test]
    async fn retry_all_unsticks_and_resets_every_item_without_forcing() {
        let tmp = tempfile::TempDir::new().unwrap();
        let (mut m, _rx) = PortalManager::new(tmp.path(), NullIo).unwrap();

        // One stuck game (queued 31 min ago) and one young pending game.
        m.enqueue_game_end("event".into(), "G_STUCK".into(), 0, 0, "{}".into())
            .unwrap();
        m.enqueue_game_end("event".into(), "G_YOUNG".into(), 1, 0, "{}".into())
            .unwrap();

        // Age the first past the 30-min stuck threshold and give both
        // some prior attempt history.
        let old = OffsetDateTime::now_utc() - TimeDuration::minutes(31);
        m.queue.items[0].queued_at = old;
        m.queue.items[0].attempts = 5;
        m.queue.items[0].last_attempt_at = Some(old);
        m.queue.items[1].attempts = 2;
        m.queue.items[1].last_attempt_at = Some(OffsetDateTime::now_utc());

        // Precondition: the first item is currently stuck.
        assert!(is_item_stuck(&m.queue.items[0], OffsetDateTime::now_utc()));

        m.retry_all().unwrap();

        let now = OffsetDateTime::now_utc();
        for item in &m.queue.items {
            // Reset to a fresh, immediately-retriable state...
            assert_eq!(item.attempts, 0);
            assert!(item.last_attempt_at.is_none());
            // ...and no longer stuck (queued_at moved to ~now), which
            // together with last_attempt_at == None means the background
            // task will pick it up on its next tick.
            assert!(!is_item_stuck(item, now));
            // Safe resend: force must never be set by retry_all.
            assert!(!item.force);
        }
    }
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p refbox retry_all_unsticks_and_resets_every_item_without_forcing`
Expected: FAIL — `no method named retry_all found for ... PortalManager`.

- [ ] **Step 3: Implement `retry_all`**

In `refbox/src/portal_manager/mod.rs`, immediately after the `force_submit` method (ends ~line 536), add:

```rust
    /// Operator tapped RETRY ALL on the portal detail page. Resets every
    /// queued game so the background task re-attempts all of them on its
    /// next tick: clears each item's attempt counter and last-attempt
    /// timestamp, and resets `queued_at` to now so any item past the
    /// 30-minute stuck threshold returns to the auto-retry pool.
    ///
    /// `force` is deliberately left untouched: RETRY ALL is a plain
    /// resend, never an overwrite. A game the portal genuinely rejects
    /// (a conflict) simply re-fails and re-surfaces as stuck for the
    /// operator to Force or Discard individually. See the design doc
    /// (2026-06-25-retry-all-portal-queue) and ADR 011.
    ///
    /// No-op-safe on an empty queue. Persistence is best-effort, matching
    /// `token_refreshed`/`force_submit`: the in-memory reset stands even
    /// if the disk write fails (the error propagates for logging).
    pub fn retry_all(&mut self) -> std::io::Result<()> {
        let now = OffsetDateTime::now_utc();
        for item in &mut self.queue.items {
            item.attempts = 0;
            item.last_attempt_at = None;
            item.queued_at = now;
        }
        queue::save(&self.config_dir, &self.queue)?;
        self.recompute_indicator();
        self.push_queue_snapshot();
        Ok(())
    }
```

- [ ] **Step 4: Run the test to verify it passes**

Run: `cargo test -p refbox retry_all_unsticks_and_resets_every_item_without_forcing`
Expected: PASS.

- [ ] **Step 5: Lint**

Run: `cargo clippy -p refbox -- -D warnings`
Expected: no warnings.

- [ ] **Step 6: Commit** (pause for approval first)

```bash
git add refbox/src/portal_manager/mod.rs
git commit -m "feat(refbox): add PortalManager::retry_all for bulk safe resend"
```

---

### Task 3: Handle `PortalRetryAll` in `update()`

**Files:**
- Modify: `refbox/src/app/mod.rs` (new match arm in `update()`, near the `PortalForceSubmit` arm ~line 2656)

**Interfaces:**
- Consumes: `Message::PortalRetryAll` (Task 1), `PortalManager::retry_all` (Task 2).

- [ ] **Step 1: Add the match arm**

In `refbox/src/app/mod.rs`, immediately after the `Message::PortalForceSubmit(id) => { ... }` arm (ends ~line 2663), add:

```rust
            Message::PortalRetryAll => {
                if let Err(e) = self.portal_manager.retry_all() {
                    error!("retry_all failed: {e}");
                }
                Task::none()
            }
```

(The operator is already on the detail page, so we stay there; the list updates on the next portal UI tick.)

- [ ] **Step 2: Verify it compiles**

Run: `cargo build -p refbox`
Expected: builds (the `update()` match now handles the new variant).

- [ ] **Step 3: Commit** (pause for approval first)

```bash
git add refbox/src/app/mod.rs
git commit -m "feat(refbox): handle PortalRetryAll in update()"
```

---

### Task 4: Add the `portal-retry-all` translation to all 15 locales

**Files:**
- Modify (one line each): `refbox/translations/<locale>/refbox.ftl` for all 15 locales.

**Interfaces:**
- Produces: Fluent key `portal-retry-all`, consumed by `fl!("portal-retry-all")` in Task 5.

- [ ] **Step 1: Add the key to every locale**

In each file below, add the line right after the existing `portal-summary-title = ...` entry. Use the value shown (best-guess; native review to follow per project convention):

```text
refbox/translations/en-US/refbox.ftl : portal-retry-all = RETRY ALL
refbox/translations/es/refbox.ftl    : portal-retry-all = REINTENTAR TODO
refbox/translations/fr/refbox.ftl    : portal-retry-all = TOUT RÉESSAYER
refbox/translations/de-DE/refbox.ftl : portal-retry-all = ALLE WIEDERHOLEN
refbox/translations/it-IT/refbox.ftl : portal-retry-all = RIPROVA TUTTO
refbox/translations/pt-PT/refbox.ftl : portal-retry-all = REPETIR TUDO
refbox/translations/nl-NL/refbox.ftl : portal-retry-all = ALLES OPNIEUW
refbox/translations/id-ID/refbox.ftl : portal-retry-all = ULANGI SEMUA
refbox/translations/ms-MY/refbox.ftl : portal-retry-all = CUBA SEMULA SEMUA
refbox/translations/tl-PH/refbox.ftl : portal-retry-all = ULITIN LAHAT
refbox/translations/tr-TR/refbox.ftl : portal-retry-all = TÜMÜNÜ TEKRAR DENE
refbox/translations/th-TH/refbox.ftl : portal-retry-all = ลองใหม่ทั้งหมด
refbox/translations/ja-JP/refbox.ftl : portal-retry-all = すべて再試行
refbox/translations/ko-KR/refbox.ftl : portal-retry-all = 모두 재시도
refbox/translations/zh-CN/refbox.ftl : portal-retry-all = 全部重试
```

- [ ] **Step 2: Verify each locale has the key**

Run: `grep -rL "portal-retry-all" refbox/translations/*/refbox.ftl`
Expected: no output (every file contains the key; `-L` lists files *missing* the match).

- [ ] **Step 3: Verify it compiles** (Fluent keys are loaded at build time)

Run: `cargo build -p refbox`
Expected: builds.

- [ ] **Step 4: Commit** (pause for approval first)

```bash
git add refbox/translations
git commit -m "feat(refbox): add portal-retry-all label in all locales"
```

---

### Task 5: Add the blue RETRY ALL button to the portal detail footer

**Files:**
- Modify: `refbox/src/app/view_builders/portal_detail.rs` (`build_portal_detail_page`)

**Interfaces:**
- Consumes: `Message::PortalRetryAll` (Task 1), `fl!("portal-retry-all")` (Task 4), `DetailRow` (already imported), `blue_button`/`make_button`/`horizontal_space`/`SPACING` (in scope via `use super::*;` and the existing `iced::widget` import).

- [ ] **Step 1: Compute whether there are unsent games**

In `build_portal_detail_page`, immediately after the line `let num_items = rows.len();` (currently line 47) and BEFORE `rows` is consumed by the `row_buttons` map, add:

```rust
    // RETRY ALL is actionable only when there is at least one unsent
    // game (a stuck/red or pending/yellow row). Recent-success and
    // token-expired rows don't count.
    let has_unsent = rows
        .iter()
        .any(|r| matches!(r, DetailRow::Stuck { .. } | DetailRow::Pending { .. }));
```

- [ ] **Step 2: Build the button and place it in the footer**

Replace the existing `back` binding and the footer `row!` (currently lines 76–93):

```rust
    let back = make_button(fl!("back"))
        .on_press(Message::ClosePortalDetailPage)
        .style(red_button);

    column![
        make_game_time_button(
            snapshot,
            false,
            false,
            mode,
            clock_running,
            portal_indicator,
            None,
        ),
        list,
        row![back, horizontal_space(), horizontal_space(),]
            .spacing(SPACING)
            .width(Length::Fill),
    ]
    .spacing(SPACING)
    .height(Length::Fill)
    .into()
```

with:

```rust
    let back = make_button(fl!("back"))
        .on_press(Message::ClosePortalDetailPage)
        .style(red_button);

    // Blue safe-action button, anchored bottom-right opposite BACK.
    // Grayed (no on_press) when there is nothing unsent to retry.
    let retry_all = make_button(fl!("portal-retry-all"))
        .on_press_maybe(has_unsent.then_some(Message::PortalRetryAll))
        .style(blue_button);

    column![
        make_game_time_button(
            snapshot,
            false,
            false,
            mode,
            clock_running,
            portal_indicator,
            None,
        ),
        list,
        row![back, horizontal_space(), retry_all,]
            .spacing(SPACING)
            .width(Length::Fill),
    ]
    .spacing(SPACING)
    .height(Length::Fill)
    .into()
```

(`back`, blank middle third, `retry_all` — all `make_button`s are `Length::Fill`, so the footer splits into equal thirds: Option A.)

- [ ] **Step 3: Verify it compiles and lints**

Run: `cargo build -p refbox && cargo clippy -p refbox -- -D warnings`
Expected: builds, no warnings.

- [ ] **Step 4: On-screen walkthrough**

Run: `cargo build -p refbox` then launch (per project run convention; native WSL needs `WAYLAND_DISPLAY=` and `dangerouslyDisableSandbox`). Open the portal status screen and confirm:
- A blue **RETRY ALL** button sits in the bottom-right, opposite BACK.
- With at least one red/yellow game row, the button is active; tapping it makes the waiting games send over the next few seconds (rows go green / drop off).
- With only green (recent-success) rows or an empty list, the button is grayed and inert.

- [ ] **Step 5: Commit** (pause for approval first)

```bash
git add refbox/src/app/view_builders/portal_detail.rs
git commit -m "feat(refbox): add blue RETRY ALL button to portal detail page"
```

---

## Final verification (before PR)

- [ ] `just check` (fmt, clippy `-D warnings`, tests, audit) is green.
- [ ] Walkthrough in Task 5 Step 4 confirmed.
- [ ] Open PR only after the human approves (project rule). PR body: What changed / Why / Scope (`refbox` only) / How to verify (the Task 5 walkthrough).

## Notes / deviations

- The design doc mentioned a possible `has_unsent_items()` helper on `PortalManager`; we derive `has_unsent` from the `rows` the view already receives instead, so no new public method and no visibility change. (`has_any_queue_items()` already exists if a future caller needs it.)
- `retry_all` also zeroes `attempts` (not just the timers) to mirror `token_refreshed`/`force_submit` and give a clean "attempt" display after a bulk retry. This is within the spec's "reset retry timers" intent.
- Record any execution deviations here (lean process — no standalone deviation commits).
