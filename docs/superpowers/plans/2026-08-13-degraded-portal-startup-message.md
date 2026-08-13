# Degraded Portal Startup Message — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** When the portal subsystem cannot start, the refbox must report a system fault instead of claiming the operator's access token expired.

**Architecture:** Add a `startup_problem` flag to `PortalManager`, set only by `new_degraded()`. It drives the Red indicator but leaves `token_expired` false, so the false re-login row and the REFRESH lockout both disappear. A new non-tappable `DetailRow::StartupFailed` gives the operator honest wording in its place. One guard is completed in `app/mod.rs` so the now-live REFRESH button cannot stick.

**Tech Stack:** Rust 2024, MSRV 1.85, `iced` 0.13, Fluent (`.ftl`) translations.

## Global Constraints

- **Crate scope:** `refbox` only. Do NOT touch `uwh-common`, `overlay`, or `wireless-remote`.
- **Process:** lean, per `.claude/rules/plan-execution.md` — no per-task deviation commits, one code review at the end. Record deviations in the "Deviations" section at the bottom of this file.
- **Clippy:** `cargo clippy --workspace --all-features -- -D warnings` must be clean. No new `#[allow]`.
- **No `unwrap()`/`expect()`** in non-test code without a comment explaining why it cannot panic.
- **Translations:** every new key must exist in **all 15** locales (`de-DE en-US es fr id-ID it-IT ja-JP ko-KR ms-MY nl-NL pt-PT th-TH tl-PH tr-TR zh-CN`). No placeholders, no English left in a non-English file. A missing key is caught by `refbox/build.rs`, not by a test.
- **Literal copy, exactly:** English row text is `Connection unavailable — results will not upload`. Use the spaced em-dash `—`, matching the neighbouring `portal-row-token-expired` key in every locale.
- **Do NOT fix** the two misleading "https-only config" comments at `app/mod.rs:2389` and `:1112`. They are wrong, they are recorded in `docs/backlog/portal-login-silent-when-no-client/NOTE.md`, and they are out of scope for this branch.
- **Branch:** `fix/refbox/degraded-portal-startup-message`. Ask the human before creating it and before every commit.

---

### Task 1: The `startup_problem` flag and the indicator

**Files:**
- Modify: `refbox/src/portal_manager/mod.rs` (struct ~`:390-411`, `new_for_test` ~`:416-434`, `recompute_indicator` ~`:513-529`, `new` ~`:561-587`, `new_degraded` ~`:600-622`, tests ~`:1349-1371`)

**Interfaces:**
- Consumes: nothing from earlier tasks.
- Produces: private field `startup_problem: bool` on `PortalManager`, readable by later tasks in the same module. No public API change — `indicator_state()` keeps its signature and `PortalIndicatorState` gains no field.

- [ ] **Step 1: Write the failing tests**

Add to the `mod tests` block in `refbox/src/portal_manager/mod.rs`, next to the existing `connection_problem_is_red_but_not_token_expired` test (~`:1069`):

```rust
    #[test]
    fn degraded_startup_is_red() {
        let (m, _rx) = PortalManager::new_degraded();
        assert_eq!(
            m.indicator_state().health,
            HealthState::Red,
            "a portal subsystem that never started must show red"
        );
    }

    #[test]
    fn degraded_startup_does_not_report_token_expired() {
        // The triggers are a system fault (the HTTP client failed to build,
        // realistically a TLS/certificate-store problem) or an unreadable
        // queue file. Neither is evidence about the operator's login.
        // Reporting token_expired greys out the schedule REFRESH button and
        // shows a "tap to re-login" row — and with no client a re-login
        // cannot even send a request, so the operator is sent nowhere.
        let (m, _rx) = PortalManager::new_degraded();
        assert!(
            !m.indicator_state().token_expired,
            "degraded startup must not blame the operator's access token"
        );
    }

    #[test]
    fn genuine_token_rejection_still_reports_token_expired() {
        // Guard the path this change must NOT disturb: a real rejection
        // from the portal still greys REFRESH and still offers re-login.
        let mut m = PortalManager::new_for_test(QueueFile::empty(), false, false);
        m.on_token_status(false);
        assert_eq!(m.indicator_state().health, HealthState::Red);
        assert!(m.indicator_state().token_expired);
    }
```

- [ ] **Step 2: Rewrite the test that asserts the removed behaviour**

The existing test at ~`:1349` is named for, and documents, the exact behaviour being removed. Replace it wholesale (name included) — do not leave the old name in place:

```rust
    #[test]
    fn new_degraded_is_red_with_no_spawned_task() {
        let (manager, mut rx) = PortalManager::new_degraded();

        // Red so the operator sees the problem — via `startup_problem`,
        // not by claiming the access token expired.
        let state = manager.indicator_state();
        assert_eq!(
            state.health,
            HealthState::Red,
            "degraded mode must surface the failure to the operator via a red dot"
        );

        // The queue should be empty (no persistence attempted).
        assert_eq!(manager.queue.items.len(), 0);

        // The returned receiver should not receive any events (no spawned task,
        // and the sender half was dropped at construction).
        assert!(
            rx.try_recv().is_err(),
            "degraded mode must not produce portal events"
        );
    }
```

- [ ] **Step 3: Run the tests and verify they fail**

Run: `cargo test -p refbox portal_manager -- --nocapture`
Expected: `degraded_startup_does_not_report_token_expired` FAILS (`token_expired` is currently true in degraded mode). The other three should already pass — they are regression guards, and a guard that never fails is still worth keeping here.

- [ ] **Step 4: Add the field to the struct**

In the `pub struct PortalManager` block (~`:390`), directly after the `connection_problem` field and its doc comment:

```rust
    /// Set true only by `new_degraded()`: the portal subsystem could not
    /// start at all. Either the HTTP client failed to build (realistically
    /// a TLS/certificate-store fault) or the retry queue was unreadable in
    /// both the config dir and the system temp dir.
    ///
    /// Drives the Red indicator like the other two problem flags, but
    /// deliberately leaves `token_expired` false: neither trigger is
    /// evidence about the operator's login, and claiming otherwise sends
    /// them into a re-login that — with no client — cannot even send a
    /// request.
    ///
    /// There is deliberately no setter. Neither trigger can heal without a
    /// restart (no client can appear mid-session — see `repoint_client` in
    /// `app/mod.rs`), so this stays true for the life of the process.
    startup_problem: bool,
```

- [ ] **Step 5: Initialise it in the three other constructors**

All three must compile. Add `startup_problem: false,` next to `connection_problem: false,` in:
- `new_for_test` (~`:426`)
- `new` (~`:573`)

And in `new_degraded` (~`:613`), replace the `token_known_problem: true` line and its comment with:

```rust
            // The portal subsystem never started. Red so the operator sees
            // the problem — but NOT `token_known_problem`: nothing here is
            // evidence the login expired.
            token_known_problem: false,
            connection_problem: false,
            startup_problem: true,
```

(Delete the now-duplicated `connection_problem: false,` line that followed it.)

- [ ] **Step 6: Make the indicator treat it as a red cause**

In `recompute_indicator` (~`:514`), extend the Red condition only:

```rust
        let health = if self.needs_attention() || self.connection_problem || self.startup_problem {
            HealthState::Red
```

Leave the `token_expired: self.token_known_problem` line at `:524` untouched — that is what makes the fix work.

- [ ] **Step 7: Run the tests and verify they pass**

Run: `cargo test -p refbox portal_manager`
Expected: PASS, including all four tests above.

- [ ] **Step 8: Mutation-test the new guard**

This is required, not optional. The original defect survived because a test passed both ways.

1. Temporarily revert Step 6 (drop `|| self.startup_problem`). Run `cargo test -p refbox portal_manager`. Expected: `degraded_startup_is_red` and `new_degraded_is_red_with_no_spawned_task` FAIL. Restore.
2. Temporarily set `token_known_problem: true` in `new_degraded` again. Run the tests. Expected: `degraded_startup_does_not_report_token_expired` FAILS. Restore.

If either mutation leaves the suite green, the test is not discriminating — fix the test before continuing.

- [ ] **Step 9: Commit** (ask the human first)

```bash
git add refbox/src/portal_manager/mod.rs
git commit -m "fix(refbox): stop degraded startup blaming the access token"
```

---

### Task 2: The honest detail row

**Files:**
- Modify: `refbox/src/portal_manager/mod.rs` (`DetailRow` enum ~`:321-354`, `detail_rows` ~`:930-935`, tests)

**Interfaces:**
- Consumes: `startup_problem: bool` from Task 1.
- Produces: new variant `DetailRow::StartupFailed` (no fields). Task 3 renders it. It is emitted first in the `Vec<DetailRow>` returned by `detail_rows()`.

- [ ] **Step 1: Write the failing test**

Add to the tests module:

```rust
    #[test]
    fn degraded_startup_shows_startup_failed_row_not_token_expired() {
        let (m, _rx) = PortalManager::new_degraded();
        let rows = m.detail_rows();
        assert!(
            matches!(rows.first(), Some(DetailRow::StartupFailed)),
            "the degraded detail page must lead with the startup-failure row, got {rows:?}"
        );
        assert!(
            !rows.iter().any(|r| matches!(r, DetailRow::TokenExpired)),
            "degraded startup must never offer a re-login that cannot send a request"
        );
    }
```

- [ ] **Step 2: Run it and verify it fails**

Run: `cargo test -p refbox degraded_startup_shows_startup_failed_row -- --nocapture`
Expected: FAIL to compile — `no variant named StartupFailed found for enum DetailRow`. A compile failure is a legitimate red here; do not "fix" it by weakening the test.

- [ ] **Step 3: Add the enum variant**

In `pub enum DetailRow` (~`:321`), as the first variant, above `TokenExpired`:

```rust
    /// Shown at the top when the portal subsystem could not start at all
    /// (`startup_problem`). Deliberately NOT tappable: there is nothing
    /// the operator can do from the refbox, and the fault cannot heal
    /// without a restart.
    StartupFailed,
```

- [ ] **Step 4: Emit it from `detail_rows`**

In `detail_rows` (~`:931`), before the existing `token_known_problem` check:

```rust
        if self.startup_problem {
            out.push(DetailRow::StartupFailed);
        }

        if self.token_known_problem {
            out.push(DetailRow::TokenExpired);
        }
```

Then update the ordering doc comment above the function (~`:923-929`) so item 1 reads:

```rust
    /// 1. `StartupFailed`, if the portal subsystem never started; then the
    ///    `TokenExpired` banner, if a token problem is flagged. In practice
    ///    only one can occur — degraded startup no longer sets the token
    ///    flag — but the order is defined so the page is deterministic.
```

- [ ] **Step 5: Run the tests and verify they pass**

Run: `cargo test -p refbox portal_manager`
Expected: PASS. `render_row` in `portal_detail.rs` will now fail to compile with a non-exhaustive match — that is Task 3's job. If the build breaks here, that is expected; proceed to Task 3 before judging the crate broken.

`render_row` (`portal_detail.rs:134`) is the **only** exhaustive match on `DetailRow` in the codebase — verified. The other site (`portal_detail.rs:54-59`) is a `matches!` listing only `Stuck`/`Pending`/`StatsPending`, and the existing test assertions all use `matches!` too, so none of them need touching.

- [ ] **Step 6: Mutation-test the emission**

Temporarily delete the `if self.startup_problem { out.push(DetailRow::StartupFailed); }` block. Run `cargo test -p refbox portal_manager`. Expected: `degraded_startup_shows_startup_failed_row_not_token_expired` FAILS. Restore it.

- [ ] **Step 7: Commit** (ask the human first)

```bash
git add refbox/src/portal_manager/mod.rs
git commit -m "fix(refbox): add an honest startup-failure row to portal detail"
```

---

### Task 3: Render the row, and translate it

**Files:**
- Modify: `refbox/src/app/view_builders/portal_detail.rs` (`render_row` ~`:134-201`)
- Modify: all 15 of `refbox/translations/<locale>/refbox.ftl`

**Interfaces:**
- Consumes: `DetailRow::StartupFailed` from Task 2.
- Produces: translation key `portal-row-startup-failed` (no variables).

- [ ] **Step 1: Render the variant**

In `render_row`, add an arm. Model it on the existing `RecentSuccess` arm (~`:187`), which is the codebase's established shape for a **non-tappable** informational strip — a `container`, not a `button`. Do not give it `.on_press`: a button with no press handler looks disabled, and this row is informational, not broken.

```rust
        DetailRow::StartupFailed => container(row_text_centered(fl!("portal-row-startup-failed")))
            .style(red_container)
            .padding(PADDING)
            .width(Length::Fill)
            .height(Length::Fixed(MIN_BUTTON_SIZE))
            .into(),
```

`red_container` is exported from `refbox/src/app/theme/mod.rs:361`. `portal_detail.rs` begins with `use super::*`, so it is very likely already in scope; if the compiler disagrees, add it to that glob's module rather than importing `theme` directly here.

**Do not touch `has_unsent` at `portal_detail.rs:54-59`.** That `matches!` decides whether RETRY ALL is actionable, and it deliberately lists only `Stuck`/`Pending`/`StatsPending`. Leaving `StartupFailed` out of it is correct: in degraded mode there is nothing queued and nothing to retry, so RETRY ALL must stay inactive.

- [ ] **Step 2: Add the English string**

In `refbox/translations/en-US/refbox.ftl`, in the `# Portal Health Indicator` section, directly after the `portal-row-token-expired` line:

```
portal-row-startup-failed = Connection unavailable — results will not upload
```

- [ ] **Step 3: Add the other 14 translations**

Same position in each file — immediately after that locale's `portal-row-token-expired` line, so the section order stays identical across locales. Each reuses its own locale's established word for "connection", taken from `portal-advisory-at-game-end`, so the health block reads as one vocabulary. Use exactly these:

```
de-DE   portal-row-startup-failed = Keine Verbindung — Ergebnisse werden nicht hochgeladen
es      portal-row-startup-failed = Conexión no disponible — los resultados no se subirán
fr      portal-row-startup-failed = Connexion indisponible — les résultats ne seront pas envoyés
id-ID   portal-row-startup-failed = Koneksi tidak tersedia — hasil tidak akan diunggah
it-IT   portal-row-startup-failed = Connessione non disponibile — i risultati non verranno caricati
ja-JP   portal-row-startup-failed = 接続できません — 結果はアップロードされません
ko-KR   portal-row-startup-failed = 연결할 수 없습니다 — 결과가 업로드되지 않습니다
ms-MY   portal-row-startup-failed = Sambungan tidak tersedia — keputusan tidak akan dimuat naik
nl-NL   portal-row-startup-failed = Geen verbinding — resultaten worden niet geüpload
pt-PT   portal-row-startup-failed = Ligação indisponível — os resultados não serão enviados
th-TH   portal-row-startup-failed = ไม่สามารถเชื่อมต่อได้ — ผลการแข่งขันจะไม่ถูกอัปโหลด
tl-PH   portal-row-startup-failed = Walang koneksyon — hindi mai-a-upload ang mga resulta
tr-TR   portal-row-startup-failed = Bağlantı yok — sonuçlar yüklenmeyecek
zh-CN   portal-row-startup-failed = 无法连接 — 成绩将不会上传
```

(The locale name is a label showing which file each line belongs in — write only the `portal-row-startup-failed = …` line into each `.ftl`.)

- [ ] **Step 4: Verify the key exists in all 15 locales**

Run: `grep -rc "portal-row-startup-failed" refbox/translations/*/refbox.ftl`
Expected: every one of the 15 files reports `1`. A `0` anywhere means a missing translation, which `refbox/build.rs` will also complain about.

- [ ] **Step 5: Run the enforcing translation tests**

Run: `cargo test -p refbox translation_consistency`
Expected: PASS. `refbox/src/translation_consistency.rs` landed on master on 2026-08-13 and enforces three rules that matter here:
1. every `en-US` key exists in every locale — so all 15 must be added;
2. every key uses the same `{ $variables }` as English — trivially satisfied, this key has none;
3. **every `en-US` key is referenced in source** — so the `fl!("portal-row-startup-failed")` call from Step 1 must land in the same commit as the key, or this test fails.

This replaces the older warning-only `build.rs` check as the real gate.

- [ ] **Step 6: Commit** (ask the human first)

```bash
git add refbox/src/app/view_builders/portal_detail.rs refbox/translations/
git commit -m "fix(refbox): show an honest message when the portal cannot start"
```

---

### Task 4: Stop REFRESH sticking when there is no client

**Files:**
- Modify: `refbox/src/app/mod.rs` (`Message::RequestPortalRefresh` arm, ~`:3555-3572`)

**Interfaces:**
- Consumes: nothing. Independent of Tasks 1–3, but required *because* of Task 1 — without it this branch introduces a new bug.
- Produces: no new API.

**Why this task exists.** Today REFRESH is greyed out in degraded mode because `token_expired` is true. Task 1 makes it live again. In a no-client session with a restored portal link, pressing it sets the spinner and calls `request_schedule`, which returns `Task::none()` when there is no client — so nothing ever arrives to clear the flag and the button reads "Refreshing…" indefinitely.

- [ ] **Step 1: Replace the guard**

The existing arm already refuses to spin when no event is linked. Extend the same reasoning to the client:

```rust
            Message::RequestPortalRefresh => {
                // Only spin the REFRESH button when there is actually an event
                // to refresh AND a client to fetch it with; otherwise nothing
                // would arrive to clear the flag. The no-client case is real:
                // a degraded startup (see PortalManager::new_degraded) can
                // still have an event linked from a restored link note, and
                // `request_schedule` returns Task::none() with no client, so
                // without this the button sticks on "Refreshing..." forever.
                match (
                    self.current_event_id.clone(),
                    self.uwhportal_client.is_some(),
                ) {
                    (Some(event_id), true) => {
                        if let AppState::GameDetailsPage(ref mut is_refreshing) = self.app_state {
                            *is_refreshing = true;
                        }
                        // request_schedule yields NoAction when the fetch fails;
                        // translate that into a refresh-finished signal so the
                        // "Refreshing..." button cannot stick on a network error.
                        self.request_schedule(event_id).map(|msg| match msg {
                            Message::NoAction => Message::PortalRefreshFinished,
                            other => other,
                        })
                    }
                    _ => Task::none(),
                }
            }
```

- [ ] **Step 2: Build**

Run: `cargo build -p refbox`
Expected: compiles clean.

- [ ] **Step 3: Record the test gap honestly**

This guard lives inside `update()`, which this codebase does not exercise directly in unit tests. **Do not invent a test that does not really cover it, and do not claim coverage in the commit message.** Note it in the Deviations section below, and verify it in Task 5's walkthrough instead.

- [ ] **Step 4: Commit** (ask the human first)

```bash
git add refbox/src/app/mod.rs
git commit -m "fix(refbox): don't spin REFRESH when there is no portal client"
```

---

### Task 5: Full validation and the before/after walkthrough

**Files:** none modified.

- [ ] **Step 1: Run the full gate**

Run: `just check`
Expected: fmt, lint, tests and audit all clean. Do NOT pipe this to `tail` — that masks the exit code.

- [ ] **Step 2: Build a real binary**

Run: `cargo build -p refbox`
Expected: success. `just check` builds a *test* binary; a walkthrough needs this one.

- [ ] **Step 3: Force degraded mode for the walkthrough**

There is no setting or environment switch for degraded mode. Reaching it needs the portal client to be absent, so use a scratch, uncommitted edit in `build_site_client` (`app/mod.rs:501`) returning `None` unconditionally. **This is walkthrough scaffolding and must never be committed** — no env-gated fakes in production source.

- [ ] **Step 4: Run both builds against a throwaway config**

Isolate completely so the real portal link is never touched:

```bash
XDG_CONFIG_HOME=/tmp/refbox-walkthrough WAYLAND_DISPLAY= ./target/debug/refbox
```

The config file created there is `default-config.toml`. A linked event is required for the portal dot to appear at all, so seed the throwaway config accordingly.

Compare `master` against the branch and capture, for the human:
1. The detail page row — was *"Access token expired — tap to re-login"*, now *"Connection unavailable — results will not upload"*.
2. The game-info REFRESH button — was greyed out, now live; pressing it must NOT leave it stuck on "Refreshing…".
3. The new row rendered in **German and Japanese**, checking the text fits the row without clipping. Translation fit has bitten this repo before and cannot be settled by reading code.

- [ ] **Step 5: Remove the scaffolding**

Run: `git status --porcelain refbox/src/app/mod.rs`
Expected: no uncommitted change. Confirm the forced-`None` edit from Step 3 is gone.

- [ ] **Step 6: Code review**

Use `superpowers:requesting-code-review` once, now that the feature is complete (lean process — not after every task).

---

## Acceptance criteria

Restated from the spec so this plan stands alone:

1. A degraded manager reports the indicator as Red.
2. A degraded manager reports `token_expired` as false.
3. A degraded manager's detail rows contain `StartupFailed` and not `TokenExpired`.
4. A genuine token rejection still reports `token_expired` true and still shows the `TokenExpired` row.
5. In a forced-degraded walkthrough: the honest row appears, no re-login row appears, REFRESH is live and does not stick.
6. Normal operation with a working portal is unchanged.

## Deviations

Record here rather than in standalone commits (lean process).

- **Branched from `origin/master` (`adef2f6d`), not local master (`ba1d2c7d`, 6 behind).** The plan's
  line numbers were derived against the older commit. Re-checked before editing: the three Rust
  files this plan touches were **not** modified by those 6 commits (`token_known_problem: true` is
  still at `mod.rs:613`). The `.ftl` files each lost one line (the `using-portal` key removal ×15),
  so `portal-row-token-expired` moved from `en-US:429` to `:428`. All translation steps are
  positional, so no plan step needed changing.
- **Task 3 Step 5 rewritten.** The plan assumed translation coverage was only a warning-only
  `build.rs` check. `refbox/src/translation_consistency.rs` landed in those 6 commits and is a real
  enforcing test. Its third assertion — every English key must be *referenced in source* — means the
  key and its `fl!` call must land together, which Task 3 already does.
- **Row text changed after the walkthrough, and the spec was amended.** The approved string was
  *"Portal unavailable — results will not upload"*. Eric asked during the walkthrough what happens
  on a **custom site**, which exposed that the wording names the wrong system and would have
  reintroduced what PR #2219 removed the day before. Final text, all 15 locales:
  **"Connection unavailable — results will not upload"**. See the new "wording must be
  source-neutral" section in the spec. This was caught by review, not by this plan.
- **Tasks 1 and 2 committed together.** The plan gave them separate commits, but both edit
  `portal_manager/mod.rs`, and interactive hunk staging is unavailable in this environment. Splitting
  them would have required a commit that does not compile (the new enum variant breaks the renderer
  until Task 3). Final structure is two commits: manager + row + rendering + translations, then the
  REFRESH guard.
- **Walkthrough exceeded the plan's step 4.** Verified in three configurations, not one: English
  portal, German portal (fit confirmed — wraps to two lines at full size, no clipping or shrink), and
  a third-party custom site (globe emblem, neutral wording). REFRESH was press-tested by Eric and
  does not stick. Degraded mode was forced with a temporary env-var switch in `build_site_client`,
  now removed and verified absent by grep.
