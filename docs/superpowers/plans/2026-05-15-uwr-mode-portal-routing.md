# UWR Mode Portal Routing Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL — use `superpowers:subagent-driven-development` or `superpowers:executing-plans` to execute this plan task-by-task. Each task ends with a commit checkpoint. Steps use checkbox (`- [ ]`) syntax.

**Goal:** Make the refbox call `api.uwrportal.com` when `Mode::Rugby` and `api.uwhportal.com` otherwise; swap every operator-facing portal string to match the active mode; require explicit confirmation + app restart when the operator switches across the portal boundary.

**Architecture:** Derive the portal URL from `Mode` at startup (no persisted URL field). Reuse the existing `portal_name_for_mode(Mode) -> &'static str` helper for all UI/log copy via Fluent `{ $portal }` variables. When the operator commits a Mode change that crosses portals (Hockey ↔ Rugby), raise a new `ConfirmationKind::PortalTenantSwitch` variant; the operator confirms by pressing **Restart to Apply**, which mirrors Unit 8's existing language-restart pattern (clear event id, flush retry queue, persist config, spawn fresh exe, exit).

**Tech stack:** Rust 2024 / MSRV 1.85, iced 0.13 UI, Fluent (i18n-embed-fl) translations, confy for config persistence. No new dependencies.

**Branch:** `feat/refbox/uwr-mode-portal-routing` (worktree at `.worktrees/uwr-mode-portal-routing/`, cut from `audit/refbox/portal-health` tip `e1a5577`). All work stays local until Final Integration of the audit chain.

**Spec source:** `docs/superpowers/specs/2026-05-15-uwr-mode-portal-routing-design.md`. The spec is authoritative for behaviour; this plan is the executable decomposition.

---

## Context

The refbox today calls the UWH portal regardless of Mode and shows UWH-prefixed text in every portal-related UI string. Rugby-mode operators can't link their event because the wrong tenant 404s their requests, and the UI gives no hint that the routing is wrong. The Unit 7 audit made the portal-health *tile* mode-aware (the logo on the time banner picks UWH or UWR) but did not address URL routing or text strings. This plan closes both gaps in a single focused branch.

---

## Process Tier (lean vs heavy)

This work is classified as **lean process** per `.claude/rules/plan-execution.md`:
- The cross-crate touch on `uwh-common` is **log-message text only** (no types, no wire format, no serialization). Blast radius is low.
- No state-machine changes (game clock, tournament manager, penalty tracking are not touched).
- Commits: one commit per logical task below. No per-task deviation commits. Record deviations in a "Deviations" section at the bottom of this plan file before the final verification step.
- Code review: once at the end of the feature via `superpowers:requesting-code-review`, not per-task.
- Verification ceremony: pure-function tasks get unit tests (Task 7); UI/wiring tasks get compile + `just check` + manual walkthrough.

---

## Resolved Open Implementation Choices

The spec left three implementation details to resolve at plan time. Resolutions:

1. **Queue flush mechanics.** Write an empty `QueueFile` via the existing atomic `save(dir, &queue)` helper in `refbox/src/portal_manager/queue.rs:113-123`. Reuses the tested temp-file-and-rename code path; avoids introducing a separate "delete this file" path. The executor will read `queue.rs` to confirm the exact field shape and either use `QueueFile::default()` (if a Default impl exists) or construct one inline.
2. **`ConfirmationOption` variant.** Add a new variant `RestartAndApply` to the `ConfirmationOption` enum in `refbox/src/app/message.rs:617-623`. The existing four variants (`DiscardChanges`, `GoBack`, `EndGameAndApply`, `KeepGameAndApply`) all relate to whether to preserve an in-progress game; overloading any of them for "restart the app" would conflate semantics.
3. **`portal-login-instructions` key name.** Keep the key name unchanged. Only the body is edited to inject `{ $portal }`. Minimum-churn choice; the spec preferred this.

---

## File Map

The full set of files touched by this plan, grouped by responsibility:

### Translations (15 files, mechanical)
- `refbox/translations/{en-US, es, fr, de-DE, id-ID, it-IT, ja-JP, ko-KR, ms-MY, nl-NL, pt-PT, th-TH, tl-PH, tr-TR, zh-CN}/refbox.ftl`
  - Rename `using-uwh-portal` → `using-portal`; body gains `{ $portal }` variable
  - Update body of `portal-login-instructions` to use `{ $portal }` (key name unchanged)
  - Rename `UWHPortal-enabled` → `portal-enabled`; body gains `{ $portal }`
  - New key `mode-switch-portal-tenant` with `$from_mode`, `$to_mode`, `$from_portal`, `$to_portal`

### Existing translation call sites (3 files)
- `refbox/src/app/view_builders/configuration.rs:518` — `using-uwh-portal` → `using-portal` + variable
- `refbox/src/app/view_builders/keypad_pages/portal_login.rs:9` — `portal-login-instructions` gains variable; function gains `mode` parameter
- `refbox/src/app/view_builders/confirmation.rs:30` — `UWHPortal-enabled` → `portal-enabled` + variable; function gains `mode` parameter

### URL routing and config
- `refbox/src/app/mod.rs:984-987,998` — replace `config.uwhportal.url` read with `match config.mode {...}` and add `UWR_PORTAL_URL_OVERRIDE` env var
- `refbox/src/config.rs:63-75` — remove `url` field from `UwhPortal` struct + its default value

### Window title
- `refbox/src/main.rs:461` — derive title from `config.mode`

### Log messages
- `refbox/src/app/mod.rs` — update the portal-related `info!` / `error!` lines (`UWH_PORTAL_URL_OVERRIDE active:`, `Got a response from UWH Portal …`, `Failed to get uwhportal token: …`, etc.) to use `portal_name_for_mode(self.config.mode)`. Executor greps for `uwhportal\|UWH Portal\|UWH_PORTAL` in this file and updates each.
- `uwh-common/src/uwhportal/mod.rs` — any portal-related log lines here lack access to `Mode`. Change literal "UWH" / "uwhportal" in operator-facing log strings to neutral "portal" (lowercase). The portal client doesn't know its own tenant; threading a portal-name field through purely for log strings is over-engineering.

### Helper and detection logic
- `refbox/src/app/view_builders/shared_elements.rs:309-314` — `portal_name_for_mode` already exists; add a sibling `crosses_portal(old: Mode, new: Mode) -> bool` helper

### Enums for the new confirmation
- `refbox/src/app/mod.rs:175-183` — add `ConfirmationKind::PortalTenantSwitch { from_mode: Mode, to_mode: Mode }`
- `refbox/src/app/message.rs:617-623` — add `ConfirmationOption::RestartAndApply`

### Confirmation page view
- `refbox/src/app/view_builders/confirmation.rs` — add the rendering arm for `PortalTenantSwitch`; mirror the Unit 8 restart button shape

### Apply intercept and restart flow
- `refbox/src/app/mod.rs:619-645` (`apply_app_options`) — intercept cross-portal mode change; raise `PortalTenantSwitch` instead of committing immediately
- `refbox/src/app/mod.rs` `apply_game_confirmation` (lines 798–876) — add arm for `PortalTenantSwitch`: on Discard revert; on `RestartAndApply` execute the restart flow (uses pattern from lines 2410–2419)
- `refbox/src/app/mod.rs:593-607` (`set_current_event_id`) — called as part of the restart flow; not modified

### Portal queue
- `refbox/src/portal_manager/queue.rs` — read to confirm `QueueFile` shape; either reuse `save()` with an empty `QueueFile`, or add a thin `flush(dir: &Path)` helper that does the same. Executor's choice based on the file's existing pattern.

---

## Reused Existing Code (DRY)

These already exist — do not re-invent them:

- **`portal_name_for_mode(Mode) -> &'static str`** at `refbox/src/app/view_builders/shared_elements.rs:309`. Returns `"UWR"` for `Mode::Rugby`, `"UWH"` for both Hockey variants. Currently used at 2 call sites (`portal_detail.rs:39`, `portal_attention_action.rs:62`). All new sites in this plan call this helper directly.
- **Atomic queue save** at `refbox/src/portal_manager/queue.rs:113-123` — `save(dir, &queue)` writes via temp-file + fsync + rename. Reuse for queue flush.
- **Language-restart pattern** at `refbox/src/app/mod.rs:2410-2419`:
  ```rust
  if let Some(mut child) = self.sim_child.take() {
      let _ = child.kill();
  }
  if let Ok(exe) = std::env::current_exe() {
      let _ = std::process::Command::new(exe).spawn();
  }
  std::process::exit(0);
  ```
  Mirror exactly in the new restart flow.
- **`set_current_event_id(Option<EventId>)`** at `refbox/src/app/mod.rs:593-607` — already mirrors to the shared `portal_event_id` handle. Call it with `None` during restart.
- **`Mode::Display`** at `refbox/src/config.rs:180-187` — returns localized mode names via `fl!("hockey6v6")` etc. Use `format!("{}", mode)` to produce the `$from_mode` / `$to_mode` variable values for the confirmation message (the spec implied passing Fluent keys, but `Mode::Display` already returns the rendered string, so pre-render via `format!`).

---

## Tasks

### Task 1: Translation files — all 15 locales in one pass

This is mechanical and best done as one unit. Editing en-US alone would leave the codebase in a non-building state with call sites still referencing old keys; doing all locales + call sites in one commit keeps each commit self-consistent.

**Files:**
- Modify: `refbox/translations/en-US/refbox.ftl` lines 87, 154, 167
- Modify: each of the other 14 locale `.ftl` files (same key shape)

- [ ] **Step 1.1: Update `refbox/translations/en-US/refbox.ftl`.**

  Find and replace:
  ```
  using-uwh-portal = USING UWHPORTAL:
  ```
  with:
  ```
  using-portal = USING { $portal }PORTAL:
  ```

  Find and replace:
  ```
  portal-login-instructions = Please go to the UWH Portal >> Event Management >> Referee Management, click on the + button to add a new Refbox, and enter this Refbox ID:
  ```
  with:
  ```
  portal-login-instructions = Please go to the { $portal } Portal >> Event Management >> Referee Management, click on the + button to add a new Refbox, and enter this Refbox ID:
  ```

  Find and replace:
  ```
  UWHPortal-enabled = When UWHPortal is enabled, all fields must be filled out.
  ```
  with:
  ```
  portal-enabled = When { $portal }PORTAL is enabled, all fields must be filled out.
  ```

  Add a new key (place it near the other confirmation/mode keys — search for `mode-` or `confirm-` for a natural neighbour):
  ```
  mode-switch-portal-tenant = Changing mode from { $from_mode } to { $to_mode } will disable the link to { $from_portal }PORTAL and you must re-connect to { $to_portal }PORTAL.
  ```

- [ ] **Step 1.2: Roll the same shape across the other 14 locale files.**

  For each of `es, fr, de-DE, id-ID, it-IT, ja-JP, ko-KR, ms-MY, nl-NL, pt-PT, th-TH, tl-PH, tr-TR, zh-CN`:
  - Read the file, find each of the three existing keys (`using-uwh-portal`, `portal-login-instructions`, `UWHPortal-enabled`).
  - Rename the first key to `using-portal` and the third to `portal-enabled`; in the bodies of all three, replace each "UWH" literal with `{ $portal }` while preserving surrounding spacing and the locale's word order.
  - Add the new `mode-switch-portal-tenant` key by translating the en-US body: substitute the locale's translation for "Changing mode from … to … will disable the link to …PORTAL and you must re-connect to …PORTAL." If the locale's existing portal-related strings give a precedent for sentence structure, follow that; otherwise translate from en-US as faithfully as the existing locale style allows. The brand name does not translate; only the surrounding sentence structure adapts.

- [ ] **Step 1.3: Update the 3 existing `fl!` call sites.**

  Site 1 — `refbox/src/app/view_builders/configuration.rs:518`:
  ```rust
  // Before:
  fl!("using-uwh-portal")
  // After:
  fl!("using-portal", portal = portal_name_for_mode(mode))
  ```
  Read the surrounding function to confirm `mode` is in scope; if not, thread it through from the caller. Add `use super::shared_elements::portal_name_for_mode;` at the top of the file if not already imported.

  Site 2 — `refbox/src/app/view_builders/keypad_pages/portal_login.rs:9`:
  ```rust
  // Before:
  fl!("portal-login-instructions", id = id)
  // After:
  fl!("portal-login-instructions", id = id, portal = portal_name_for_mode(mode))
  ```
  Thread `mode: Mode` as a new parameter to the surrounding function and update its callers. Add the import.

  Site 3 — `refbox/src/app/view_builders/confirmation.rs:30`:
  ```rust
  // Before:
  fl!("UWHPortal-enabled")
  // After:
  fl!("portal-enabled", portal = portal_name_for_mode(mode))
  ```
  Thread `mode: Mode` as a new parameter; update callers; add the import.

- [ ] **Step 1.4: Verify build is clean.**

  Run: `just check`
  Expected: zero warnings, zero errors. If a translation key reference is missed anywhere, the i18n-embed-fl macro will fail at compile time — fix any reports.

- [ ] **Step 1.5: Commit.**

  ```bash
  git add refbox/translations refbox/src/app/view_builders/configuration.rs refbox/src/app/view_builders/keypad_pages/portal_login.rs refbox/src/app/view_builders/confirmation.rs
  git commit -m "feat(refbox): mode-aware translations for portal strings"
  ```

---

### Task 2: URL routing — derive from Mode + symmetric env-var

**Files:**
- Modify: `refbox/src/app/mod.rs:984-998`
- Modify: `refbox/src/config.rs:63-75`

- [ ] **Step 2.1: Remove the `url` field from `UwhPortal`.**

  In `refbox/src/config.rs:63-75`, change the struct from:
  ```rust
  pub struct UwhPortal {
      pub url: String,
      pub token: String,
  }

  impl Default for UwhPortal {
      fn default() -> Self {
          Self {
              url: "https://api.uwhportal.com".to_string(),
              token: String::new(),
          }
      }
  }
  ```
  to:
  ```rust
  pub struct UwhPortal {
      pub token: String,
  }

  impl Default for UwhPortal {
      fn default() -> Self {
          Self {
              token: String::new(),
          }
      }
  }
  ```

  Confy silently ignores unknown fields on load, so existing on-disk config files with `url = "..."` will load fine; the field is just dropped.

- [ ] **Step 2.2: Replace the URL read in `app/mod.rs`.**

  At `refbox/src/app/mod.rs:984-987`, change:
  ```rust
  let url_override = std::env::var("UWH_PORTAL_URL_OVERRIDE").ok();
  let portal_url: &str = url_override.as_deref().unwrap_or(&config.uwhportal.url);
  if url_override.is_some() {
      info!("UWH_PORTAL_URL_OVERRIDE active: using {portal_url}");
  }
  ```
  to:
  ```rust
  let (default_url, override_var) = match config.mode {
      Mode::Rugby => ("https://api.uwrportal.com", "UWR_PORTAL_URL_OVERRIDE"),
      Mode::Hockey6V6 | Mode::Hockey3V3 => ("https://api.uwhportal.com", "UWH_PORTAL_URL_OVERRIDE"),
  };
  let url_override = std::env::var(override_var).ok();
  let portal_url: String = url_override.clone().unwrap_or_else(|| default_url.to_string());
  if url_override.is_some() {
      info!(
          "{override_var} active for {} Portal: using {portal_url}",
          portal_name_for_mode(config.mode)
      );
  }
  ```

  Note: existing code passes `portal_url` (a `&str`) into `UwhPortalClient::new(portal_url, ...)` at line 998. Adjust to pass `&portal_url` if needed for the new owned-String version. Confirm the call still compiles.

  Add imports at the top of `app/mod.rs` if not already present:
  ```rust
  use crate::config::Mode;
  use crate::app::view_builders::shared_elements::portal_name_for_mode;
  ```
  (Path may differ — adjust to match existing imports in the file.)

- [ ] **Step 2.3: Verify build is clean.**

  Run: `just check`
  Expected: zero warnings, zero errors.

- [ ] **Step 2.4: Manual smoke (run app once).**

  Launch refbox: `cd .worktrees/uwr-mode-portal-routing/ && WAYLAND_DISPLAY= cargo run -p refbox`
  Expected: app launches, window title still reads "UWH Ref Box" (title work is in Task 3), no startup panic. If startup fails, the URL-read change has a regression — diagnose before proceeding.
  Kill the app once verified.

- [ ] **Step 2.5: Commit.**

  ```bash
  git add refbox/src/config.rs refbox/src/app/mod.rs
  git commit -m "feat(refbox): derive portal URL from Mode with symmetric env override"
  ```

---

### Task 3: Window title

**Files:**
- Modify: `refbox/src/main.rs:461`

- [ ] **Step 3.1: Derive title from Mode.**

  Change:
  ```rust
  iced::application("UWH Ref Box", app::RefBoxApp::update, app::RefBoxApp::view)
  ```
  to:
  ```rust
  let title = match config.mode {
      Mode::Rugby => "UWR Ref Box",
      Mode::Hockey6V6 | Mode::Hockey3V3 => "UWH Ref Box",
  };
  iced::application(title, app::RefBoxApp::update, app::RefBoxApp::view)
  ```

  Add a `use crate::config::Mode;` import at the top of `main.rs` if not present.

- [ ] **Step 3.2: Verify build is clean.**

  Run: `just check`
  Expected: zero warnings, zero errors.

- [ ] **Step 3.3: Commit.**

  ```bash
  git add refbox/src/main.rs
  git commit -m "feat(refbox): mode-aware window title"
  ```

---

### Task 4: Log messages — inject portal name

**Files:**
- Modify: `refbox/src/app/mod.rs` (~5 sites)
- Modify: `uwh-common/src/uwhportal/mod.rs` (~2 sites)

- [ ] **Step 4.1: Update portal-related log strings in `refbox/src/app/mod.rs`.**

  Grep for portal-related log lines:
  ```bash
  grep -n 'info!\|error!\|warn!' refbox/src/app/mod.rs | grep -iE 'uwhportal|uwh portal|portal token|portal url|portal request|portal response'
  ```

  For each match, rewrite the literal "UWH" / "uwhportal" portion to use `portal_name_for_mode(self.config.mode)`. Examples:

  ```rust
  // Before:
  info!("Got a response from UWH Portal token request");
  // After:
  info!(
      "Got a response from {} Portal token request",
      portal_name_for_mode(self.config.mode)
  );

  // Before:
  error!("Failed to get uwhportal token: {e}");
  // After:
  error!(
      "Failed to get {} portal token: {e}",
      portal_name_for_mode(self.config.mode)
  );
  ```

  Keep the `UWH_PORTAL_URL_OVERRIDE` env-var name in any log that mentions the env var (the env var keeps its name — see spec). Only the brand portion changes.

- [ ] **Step 4.2: Update portal-related log strings in `uwh-common/src/uwhportal/mod.rs`.**

  Grep for the same pattern in that file. These log sites do not have `Mode` in scope (the `UwhPortalClient` doesn't know its tenant). For each such site, change literal "UWH" / "uwhportal" in operator-facing log strings to neutral "portal" (lowercase):

  ```rust
  // Before:
  error!("Failed to parse uwhportal response: {e}");
  // After:
  error!("Failed to parse portal response: {e}");
  ```

  Rationale: threading a portal-name field through `UwhPortalClient` purely for log strings is over-engineering for this branch. Internal Rust identifiers (the type name `UwhPortalClient`, module name `uwhportal`) explicitly stay per spec non-goals.

- [ ] **Step 4.3: Verify build is clean.**

  Run: `just check`
  Expected: zero warnings, zero errors.

- [ ] **Step 4.4: Commit.**

  ```bash
  git add refbox/src/app/mod.rs uwh-common/src/uwhportal/mod.rs
  git commit -m "feat(refbox): mode-aware portal name in log messages"
  ```

---

### Task 5: Add `crosses_portal` helper with unit tests

**Files:**
- Modify: `refbox/src/app/view_builders/shared_elements.rs:309-314` (add after `portal_name_for_mode`)

- [ ] **Step 5.1: Write the failing test first.**

  Add to `refbox/src/app/view_builders/shared_elements.rs` (at the bottom, inside a `#[cfg(test)] mod tests { ... }` block, creating the block if it doesn't exist):

  ```rust
  #[cfg(test)]
  mod tests {
      use super::*;
      use crate::config::Mode;

      #[test]
      fn crosses_portal_within_hockey_is_false() {
          assert!(!crosses_portal(Mode::Hockey6V6, Mode::Hockey3V3));
          assert!(!crosses_portal(Mode::Hockey3V3, Mode::Hockey6V6));
          assert!(!crosses_portal(Mode::Hockey6V6, Mode::Hockey6V6));
          assert!(!crosses_portal(Mode::Hockey3V3, Mode::Hockey3V3));
          assert!(!crosses_portal(Mode::Rugby, Mode::Rugby));
      }

      #[test]
      fn crosses_portal_hockey_to_rugby_is_true() {
          assert!(crosses_portal(Mode::Hockey6V6, Mode::Rugby));
          assert!(crosses_portal(Mode::Hockey3V3, Mode::Rugby));
      }

      #[test]
      fn crosses_portal_rugby_to_hockey_is_true() {
          assert!(crosses_portal(Mode::Rugby, Mode::Hockey6V6));
          assert!(crosses_portal(Mode::Rugby, Mode::Hockey3V3));
      }
  }
  ```

- [ ] **Step 5.2: Run the test to verify it fails.**

  Run: `cargo test --package refbox crosses_portal`
  Expected: compilation error — `crosses_portal` is not defined.

- [ ] **Step 5.3: Implement `crosses_portal`.**

  Add immediately after `portal_name_for_mode` (line 314) in the same file:

  ```rust
  pub(super) fn crosses_portal(old: Mode, new: Mode) -> bool {
      portal_name_for_mode(old) != portal_name_for_mode(new)
  }
  ```

- [ ] **Step 5.4: Run the test to verify it passes.**

  Run: `cargo test --package refbox crosses_portal`
  Expected: 3 tests pass.

- [ ] **Step 5.5: Verify the wider build.**

  Run: `just check`
  Expected: zero warnings, zero errors. (May fail with "unused function" if not referenced anywhere yet — silence with `#[allow(dead_code)]` if needed, removed in Task 7.)

- [ ] **Step 5.6: Commit.**

  ```bash
  git add refbox/src/app/view_builders/shared_elements.rs
  git commit -m "feat(refbox): add crosses_portal helper with tests"
  ```

---

### Task 6: Add new enum variants

**Files:**
- Modify: `refbox/src/app/mod.rs:175-183` (`ConfirmationKind`)
- Modify: `refbox/src/app/message.rs:617-623` (`ConfirmationOption`)

This task only adds the variants and writes the minimum match arms needed for the project to build. The real behaviour for the new variants is wired in Tasks 7–8.

- [ ] **Step 6.1: Add `PortalTenantSwitch` to `ConfirmationKind`.**

  In `refbox/src/app/mod.rs:175-183`, change:
  ```rust
  enum ConfirmationKind {
      Error(String),
      UwhPortalLinkFailed(PortalTokenResponse),
      GameNumberChangedFromApply,
      GameConfigChangedFromApply(GameConfig),
      UwhPortalIncompleteFromApply,
  }
  ```
  to:
  ```rust
  enum ConfirmationKind {
      Error(String),
      UwhPortalLinkFailed(PortalTokenResponse),
      GameNumberChangedFromApply,
      GameConfigChangedFromApply(GameConfig),
      UwhPortalIncompleteFromApply,
      PortalTenantSwitch { from_mode: Mode, to_mode: Mode },
  }
  ```

  Ensure `Mode` is in scope (it is — `use crate::config::Mode;` should be present from Task 2, otherwise add).

- [ ] **Step 6.2: Add `RestartAndApply` to `ConfirmationOption`.**

  In `refbox/src/app/message.rs:617-623`, change:
  ```rust
  pub enum ConfirmationOption {
      DiscardChanges,
      GoBack,
      EndGameAndApply,
      KeepGameAndApply,
  }
  ```
  to:
  ```rust
  pub enum ConfirmationOption {
      DiscardChanges,
      GoBack,
      EndGameAndApply,
      KeepGameAndApply,
      RestartAndApply,
  }
  ```

- [ ] **Step 6.3: Add minimum match arms to silence non-exhaustive errors.**

  `just check` will report non-exhaustive matches at every site that matches on these enums. For each such site:
  - If the existing code uses a wildcard `_ =>`, the wildcard already covers it — no change needed.
  - Otherwise, add an arm that produces an `unreachable!()` with a TODO comment pointing to the next task:
    ```rust
    ConfirmationKind::PortalTenantSwitch { .. } => unreachable!("wired in Task 7"),
    ConfirmationOption::RestartAndApply => unreachable!("wired in Task 7"),
    ```

  These get replaced by real handling in Tasks 7 and 8. (`unreachable!` is acceptable here per project rules: it would indicate a programming error if hit, and is documented inline.)

- [ ] **Step 6.4: Verify build is clean.**

  Run: `just check`
  Expected: zero warnings, zero errors.

- [ ] **Step 6.5: Commit.**

  ```bash
  git add refbox/src/app/mod.rs refbox/src/app/message.rs
  git commit -m "feat(refbox): add PortalTenantSwitch and RestartAndApply variants"
  ```

---

### Task 7: Confirmation page view for `PortalTenantSwitch`

**Files:**
- Modify: `refbox/src/app/view_builders/confirmation.rs`
- Read for reference only: `refbox/src/app/mod.rs:2410-2419` (Unit 8 restart button shape)

- [ ] **Step 7.1: Locate the confirmation-view dispatcher.**

  Open `refbox/src/app/view_builders/confirmation.rs`. Find the function (likely `make_confirmation_page` or similar) that matches on `ConfirmationKind` and renders the appropriate page. Find the existing arm for `UwhPortalIncompleteFromApply` — that arm uses the `portal-enabled` key (renamed in Task 1). The new `PortalTenantSwitch` arm goes alongside it.

- [ ] **Step 7.2: Find the Unit 8 restart-button widget.**

  Open `refbox/src/app/view_builders/configuration.rs` (or wherever `make_language_select_page` lives — grep if uncertain). Locate the "Restart to Apply" button it builds; copy its colour/shape conventions.

- [ ] **Step 7.3: Add the rendering arm.**

  Add a new match arm for `ConfirmationKind::PortalTenantSwitch { from_mode, to_mode }`. The body of the page:

  ```rust
  ConfirmationKind::PortalTenantSwitch { from_mode, to_mode } => {
      let message = fl!(
          "mode-switch-portal-tenant",
          from_mode = format!("{from_mode}"),
          to_mode = format!("{to_mode}"),
          from_portal = portal_name_for_mode(*from_mode),
          to_portal = portal_name_for_mode(*to_mode)
      );
      // ... render as a centered text panel with two buttons:
      //   Cancel (red, left)  →  Message::ConfirmationSelected(ConfirmationOption::DiscardChanges)
      //   Restart to Apply (red, right) → Message::ConfirmationSelected(ConfirmationOption::RestartAndApply)
      // Use the same button widget + styling pattern as the language-restart page.
  }
  ```

  Match the exact widget composition used by other arms in the file (probably a `column!` of text + a `row!` of buttons). Use the project's existing red/cancel and red/confirm button styles.

  Mode passed by reference (`*from_mode` to dereference) — adjust based on whether the match binds by ref or value.

- [ ] **Step 7.4: Verify build is clean.**

  Run: `just check`
  Expected: zero warnings, zero errors.

- [ ] **Step 7.5: Commit.**

  ```bash
  git add refbox/src/app/view_builders/confirmation.rs
  git commit -m "feat(refbox): render PortalTenantSwitch confirmation page"
  ```

---

### Task 8: Restart-flow handler in `apply_game_confirmation`

**Files:**
- Modify: `refbox/src/app/mod.rs:798-876` (`apply_game_confirmation` dispatcher)
- Read for reference: `refbox/src/app/mod.rs:593-607` (`set_current_event_id`), `refbox/src/app/mod.rs:2410-2419` (Unit 8 restart pattern), `refbox/src/portal_manager/queue.rs:62-123` (queue file path and save helper)

- [ ] **Step 8.1: Read `apply_game_confirmation` and identify the existing dispatch shape.**

  Note how each existing `ConfirmationKind` arm is laid out: typically matches on `ConfirmationOption` and routes to either commit or revert. The `GameConfigChangedFromApply(config)` arm is the closest cousin — it stashes proposed state in the variant and commits on confirm.

- [ ] **Step 8.2: Replace the placeholder arm from Task 6 with the real handler.**

  ```rust
  ConfirmationKind::PortalTenantSwitch { from_mode: _, to_mode } => match selection {
      ConfirmationOption::DiscardChanges => {
          // Operator pressed Cancel. Discard the in-flight mode edit;
          // return to App Options with the existing mode unchanged.
          // The edit lives in self.edited_settings — clearing it or
          // letting the caller drop it is enough; we did not commit
          // self.config.mode yet.
          self.app_state = AppState::EditGameConfig(ConfigPage::App);
      }
      ConfirmationOption::RestartAndApply => {
          // Commit the new mode.
          self.config.mode = to_mode;

          // Clear the current event id; this also mirrors None to the
          // shared portal_event_id handle that the background portal-health
          // task watches.
          self.set_current_event_id(None);

          // Flush the portal retry queue: write an empty QueueFile via the
          // existing atomic save helper. Queued items would have been bound
          // for the old portal tenant and cannot be delivered post-switch.
          if let Some(dir) = self.portal_queue_dir.as_ref() {
              // Substitute the actual accessor used by RefBoxApp for the
              // queue directory; if no field exists, read from config or
              // the same place portal_manager loads from at startup.
              let empty = crate::portal_manager::queue::QueueFile::default();
              if let Err(e) = crate::portal_manager::queue::save(dir, &empty) {
                  error!("Failed to flush portal queue before restart: {e}");
              }
          }

          // Persist the new mode to disk so the restarted exe picks it up.
          if let Err(e) = confy::store(APP_NAME, None, &self.config) {
              error!("Failed to persist config before restart: {e}");
              // Continue with restart anyway — operator pressed Restart.
          }

          // Restart pattern mirrored from the Unit 8 language-switch path
          // at lines 2410-2419.
          if let Some(mut child) = self.sim_child.take() {
              let _ = child.kill();
          }
          if let Ok(exe) = std::env::current_exe() {
              let _ = std::process::Command::new(exe).spawn();
          }
          std::process::exit(0);
      }
      _ => {}
  },
  ```

  Notes for the executor:
  - The exact field names (`self.portal_queue_dir`, `APP_NAME`, etc.) must be looked up in the surrounding file. Use the same constant and accessor names that the existing portal-manager initialisation in `mod.rs` uses.
  - If `QueueFile::default()` does not exist, read `queue.rs` and construct an empty `QueueFile` inline (`QueueFile { <field>: Vec::new() }`).
  - Removing the `unreachable!` placeholders added in Task 6 is part of this step.

- [ ] **Step 8.3: Verify build is clean.**

  Run: `just check`
  Expected: zero warnings, zero errors.

- [ ] **Step 8.4: Commit.**

  ```bash
  git add refbox/src/app/mod.rs
  git commit -m "feat(refbox): restart-flow handler for portal tenant switch"
  ```

---

### Task 9: Intercept cross-portal mode commit in `apply_app_options`

**Files:**
- Modify: `refbox/src/app/mod.rs:619-645` (`apply_app_options`) and its caller path

This is the trigger that raises the new confirmation. Up to this point all the plumbing is in place but inert; this step wires it up.

- [ ] **Step 9.1: Inspect the current commit path.**

  Re-read `apply_app_options` (lines 619–645) and find its caller (line 2027 in the `ConfirmationSelected` handling, per Explore report). Confirm whether the function is called *before* committing the mode (giving us a chance to intercept) or *as part of* committing (in which case the intercept needs to happen further up the chain).

- [ ] **Step 9.2: Insert the cross-portal check.**

  Inside `apply_app_options`, after binding `let mode = edited.mode;` (line 630) but before `self.config.mode = mode;` (line 641):

  ```rust
  if crosses_portal(self.config.mode, mode) {
      // Cross-portal Mode change requires explicit confirmation and an app
      // restart. Stash the proposed mode in the confirmation variant; the
      // commit happens in the RestartAndApply branch of apply_game_confirmation.
      self.confirmation_kind = Some(ConfirmationKind::PortalTenantSwitch {
          from_mode: self.config.mode,
          to_mode: mode,
      });
      self.app_state = AppState::ConfirmationPage; // or whatever the existing
                                                    // route to the confirmation
                                                    // page is — match the pattern
                                                    // used by GameConfigChangedFromApply.
      return;
  }
  ```

  Important: do NOT commit any of the other editable settings either if returning early. If the existing `apply_app_options` commits multiple fields in one batch, the cross-portal early-return must happen *before* any commits, so cancelling reverts the entire Apply.

  Alternative if the existing flow commits other fields first: split into "commit non-mode fields, then handle mode separately." Use whichever matches the existing pattern for `GameConfigChangedFromApply`.

  Add `use crate::app::view_builders::shared_elements::crosses_portal;` at the top of the file if not already imported.

- [ ] **Step 9.3: Verify build is clean.**

  Run: `just check`
  Expected: zero warnings, zero errors.

- [ ] **Step 9.4: Commit.**

  ```bash
  git add refbox/src/app/mod.rs
  git commit -m "feat(refbox): intercept cross-portal mode change with confirmation"
  ```

---

### Task 10: Manual walkthrough verification

Per the spec's Verification Plan section. Document any deviations from the spec at the bottom of this plan file before committing the walkthrough log.

**Files (read-only):**
- The plan file itself for the Deviations section

- [ ] **Step 10.1: Pre-walkthrough sanity.**

  Run: `just check`
  Expected: clean. Run: `cargo test --package refbox` — all tests pass (Unit 7's 39 portal-manager tests included).

- [ ] **Step 10.2: Launch refbox in Hockey 6v6 mode.**

  ```bash
  cd /home/estraily/projects/uwh-refbox-rs/.worktrees/uwr-mode-portal-routing/
  WAYLAND_DISPLAY= cargo run -p refbox
  ```
  Use `UWH_PORTAL_URL_OVERRIDE=https://api.dev.uwhportal.com` if the operator wants to verify against the dev portal (see memory `reference_dev_portal_url`).

- [ ] **Step 10.3: Walk the 11-step verification script from the spec.**

  Execute steps 1–11 of the spec's "Manual walkthrough (end-to-end)" section (file: `docs/superpowers/specs/2026-05-15-uwr-mode-portal-routing-design.md`, lines 226–238). Confirm each observable outcome. Critical checks:
  - The "Using UWH Portal" / "Using UWR Portal" text matches the active mode.
  - The window title changes after the cross-portal restart.
  - The confirmation page renders the correct localized message with both directional cases (Hockey → Rugby and Rugby → Hockey).
  - Cancel returns to App Options with Mode unchanged.
  - Restart to Apply restarts the app and persists the new Mode.
  - In Rugby mode, network calls go to `api.uwrportal.com` (verify via log inspection or trace).
  - Within-Hockey Mode change (6v6 ↔ 3v3) does NOT trigger the confirmation page or restart.

- [ ] **Step 10.4: Regression check — Hockey-only workflow unchanged.**

  In Hockey 6v6 mode, complete a normal "link an event → start a game → confirm a score" sequence. Verify no spurious confirmations, no surprise restarts, no visible behaviour change vs. pre-branch.

- [ ] **Step 10.5: Record deviations in the plan file.**

  Add a `## Deviations` section at the bottom of `docs/superpowers/plans/2026-05-15-uwr-mode-portal-routing.md` (the final saved location). Note anything that diverged from spec (especially around exact match-arm shapes, queue-file accessor names, or anywhere the placeholder names in this plan didn't match the real codebase).

- [ ] **Step 10.6: Final code-review pass.**

  After all tasks pass: invoke `superpowers:requesting-code-review` once for the whole branch (per lean process — code review once at end, not per task). Report findings back to operator.

- [ ] **Step 10.7: Commit any walkthrough/deviation notes.**

  ```bash
  git add docs/superpowers/plans/2026-05-15-uwr-mode-portal-routing.md
  git commit -m "docs(refbox): record UWR mode portal routing walkthrough"
  ```

---

## Verification Summary

After all 10 tasks:

| Check | Command | Expected |
|-------|---------|----------|
| Compile + lint + tests + audit | `just check` | clean, zero warnings |
| Unit tests for `crosses_portal` | `cargo test --package refbox crosses_portal` | 3 tests pass |
| Manual walkthrough | Spec lines 226–238 | All 11 steps observable behaviours match |
| Hockey-only regression | Normal game flow | Unchanged from pre-branch |
| Log inspection | Tail logs during Rugby mode | Portal-related messages mention "UWR" |
| Network destination | Trace or log during Rugby mode | Calls go to `api.uwrportal.com` |

---

## Self-Review

Spec coverage check (each spec goal mapped to a task):

| Spec goal | Task(s) |
|-----------|---------|
| 1. Call `api.uwrportal.com` for Rugby, `api.uwhportal.com` otherwise | Task 2 |
| 2. Every operator-facing string explicitly UWH or UWR | Tasks 1, 3, 7 |
| 3. Cross-portal Mode switch requires confirmation + restart; within-Hockey does not | Tasks 5, 6, 7, 8, 9 |
| 4. Log messages reflect active portal name | Task 4 |
| 5. No regressions in Hockey-only workflows | Task 10.4 |

Implementation choices resolved: queue flush via empty-snapshot atomic save; new `ConfirmationOption::RestartAndApply` variant; `portal-login-instructions` key name unchanged.

No placeholders or "TODO: handle X" left in the plan body. The `unreachable!` placeholders in Task 6 are explicitly replaced in Tasks 7/8 and the plan calls this out.

---

## Execution Handoff

Plan complete. Two execution options after approval:

1. **Subagent-Driven (recommended for this work).** Dispatch a fresh subagent per task with the relevant file paths and step bodies pasted in. Operator reviews each task's commit before the next subagent fires. Higher safety, slower wall-clock.

2. **Inline Execution.** Execute tasks 1–10 in this session using `superpowers:executing-plans`, with checkpoints between each task for operator review.

The work is lean-process tier, but it touches `apply_app_options` and `apply_game_confirmation` (two dispatchers in `app/mod.rs`) plus a new restart flow. Operator preference governs.

---

## Deviations (recorded during execution)

The plan was executed via subagent-driven-development on 2026-05-15. These are the points where execution diverged from the plan or from the spec, with the reason and the resolution. None alter operator-visible behaviour from the spec; one was a spec-compliance fix that was caught by the final code review before the walkthrough.

### Sound structural deviations (cleaner than the plan)

1. **Task 2 — `portal_name_for_mode` visibility.** Plan asked Task 2 to call `portal_name_for_mode(config.mode)` from `app/mod.rs`. The helper was `pub(super)` and inaccessible at that path. Task 2's implementer inlined a two-arm match instead (functionally identical). Task 4 then relaxed the helper's visibility to `pub(crate)` as a pre-task setup so subsequent tasks (4, 5, 9) could call it directly. Net: one inlined match in the URL-derivation block remained for Task 2; everywhere else uses the helper.

2. **Task 4 — async-borrow lifetime.** Plan suggested `portal_name_for_mode(self.config.mode)` directly inside an `async move` block in `request_uwhportal_token`. That triggers E0521 (`self` doesn't outlive `'static`). The implementer bound `let portal_name = portal_name_for_mode(self.config.mode);` to a `&'static str` *before* the async block and captured the local; identical log output, lifetime-correct.

3. **Task 8 — restart-flow lives under `RestartAndApply`, not `PortalTenantSwitch`.** Plan asked for a `ConfirmationKind::PortalTenantSwitch { from_mode, to_mode } => match selection { ... }` arm in `apply_game_confirmation`. The implementer instead put the restart flow under the `ConfirmationOption::RestartAndApply` arm and extracts `to_mode` from `self.app_state` via `if let AppState::ConfirmationPage(ConfirmationKind::PortalTenantSwitch { .. })`. The `else` branch is `unreachable!()` because `RestartAndApply` is only ever offered by `PortalTenantSwitch` pages (per Task 7's button construction). Structurally sound and arguably cleaner — there is exactly one `RestartAndApply` arm rather than a duplicated handler under every `ConfirmationKind` that ever offered it.

4. **Task 8 — `queue_dir()` accessor on `PortalManager`.** Plan referenced `self.portal_queue_dir` on `RefBoxApp`. That field doesn't exist; `RefBoxApp` discards `config_dir` after init. The implementer added `pub fn queue_dir(&self) -> &Path` on `PortalManager` (which does keep `config_dir`) and called `self.portal_manager.queue_dir()` from the restart handler. Minimal surface-area addition.

5. **Task 8 — `QueueFile::empty()` rather than `QueueFile::default()`.** The `empty()` associated function already existed in `queue.rs`. The implementer reused it instead of adding a `Default` impl.

6. **Task 9 — `apply_app_options` returns `Option<ConfirmationKind>`.** Plan asked for direct mutation of `self.app_state` inside `apply_app_options` with an early `return`. That approach would have been overwritten by the caller's `persist_config + navigate_to_parent` after the function returned. The implementer changed the function signature to `fn apply_app_options(&mut self) -> Option<ConfirmationKind>` (mirroring the existing `apply_game_options` pattern), routing `Some(kind)` to `AppState::ConfirmationPage(kind)` at the call site and falling through to the normal commit path on `None`. Keeps the function pure (no `app_state` side-effects) and mirrors a pattern the codebase already uses.

### Required clippy-driven adjustments

7. **Task 2 — `UwhPortal::Default` derived.** Clippy's `derivable_impls` fired once the `url` field was removed, since the remaining `Default` impl was now trivial. Switched to `#[derive(Default)]` on the struct.

8. **Task 2 — two `url`-field test assertions removed.** Direct consequence of dropping the field; the legacy `url` entry was left in the test-fixture table to demonstrate confy's silent-ignore behaviour on load.

### Spec-compliance fix surfaced by the final code review

9. **Post-Task-9 fix (commit `587ba4f`) — Cancel returns to App Options, not Settings Main.** The final code review caught that Cancel on the `PortalTenantSwitch` page was routing through the generic `DiscardChanges` arm (calling `revert_from_snapshot()` and landing on `AppState::EditGameConfig(ConfigPage::Main)`), violating the spec's explicit "Returns to App Options with Mode unchanged" line. Fix: a `matches!` guard inside the existing `DiscardChanges` arm picks `ConfigPage::App` for `PortalTenantSwitch` and keeps `ConfigPage::Main` for the other variants. ~12 lines, no other variants affected.

### Walkthrough-time observations (2026-05-15)

Operator drove a Rugby → Hockey-6v6 walkthrough (the reverse direction of the spec's example, exercising the symmetric case). The 11-step script was not followed verbatim, but the four critical observable behaviours were exercised end-to-end:

- ✅ **Confirmation page renders.** Operator-supplied screenshot showed the page correctly localized: *"Changing mode from RUGBY to HOCKEY6V6 will disable the link to UWRPORTAL and you must re-connect to UWHPORTAL."* (Aside: the rendered mode names came out as `RUGBY` / `HOCKEY6V6` rather than the human-readable forms the spec example used. That's because the Fluent `mode-*` keys are uppercase-styled — pre-existing behaviour, out of scope for this branch.)
- ✅ **Restart to Apply triggers a real restart.** Log shows two `Starting RefBox App` events on the same refbox launch — the second one is the spawned-fresh-exe from the user's button press at 22:45:16, with no controller-initiated relaunch in between.
- ✅ **URL routing follows the new Mode after restart.** Post-restart portal calls go to `https://api.uwrportal.com/api/events/.../access-keys/verify` and `.../schedule/privileged` (logged at 22:45:27). The 401 responses are the expected post-switch behaviour — the previous UWH token no longer authenticates against the UWR tenant, which is exactly what the confirmation page warned the operator about.
- ✅ **Neutralized uwh-common log strings.** "portal token validation successful" / "portal token validation failed" appear in the log (rather than "uwhportal token validation..."). Confirms Task 4's neutralization landed correctly.

Two walkthrough-time corrections were applied:

- **Restart to Apply button colour** was originally red (matching Cancel). Operator preferred blue for affirmative actions. Fixed in commit `846ef02` mid-walkthrough.
- **Cancel landing page** had already been fixed pre-walkthrough by commit `587ba4f` (caught by the final code review). Operator flagged it during walkthrough as a thing to verify; the fix was in place in the binary they were testing.

Not exercised in this walkthrough (deferred to a later session if needed):

- Cancel-on-confirmation actually being pressed and returning to App Options (operator either skipped this step or it happened silently). The fix is in place per code inspection; future smoke-tests will exercise it.
- Within-Hockey mode cycle (6v6 ↔ 3v3) without restart-confirmation. Hockey-only flow is the project's default code path and was implicitly exercised at the start of the session (refbox started in Hockey 6v6 and the first `portal token validation successful` was against UWH).
- The `UWR_PORTAL_URL_OVERRIDE` env-var override path (not exercised; would require a staging UWR portal URL).

---

## Commit Log

| Commit | Task | Description |
|--------|------|-------------|
| `27f6f2d` | (pre-plan) | docs(refbox): add design spec for UWR mode portal routing |
| `ed93a7a` | Plan commit | docs(refbox): add UWR mode portal routing implementation plan |
| `8dec766` | Task 1 | feat(refbox): mode-aware translations for portal strings |
| `8001a75` | Task 2 | feat(refbox): derive portal URL from Mode with symmetric env override |
| `e9aa7fd` | Task 3 | feat(refbox): mode-aware window title |
| `4580c7c` | Task 4 | feat(refbox): mode-aware portal name in log messages |
| `b3d84da` | Task 5 | feat(refbox): add crosses_portal helper with tests |
| `a10d71a` | Task 6 | feat(refbox): add PortalTenantSwitch and RestartAndApply variants |
| `354c4a6` | Task 7 | feat(refbox): render PortalTenantSwitch confirmation page |
| `162d668` | Task 8 | feat(refbox): restart-flow handler for portal tenant switch |
| `fa59b8a` | Task 9 | feat(refbox): intercept cross-portal mode change with confirmation |
| `587ba4f` | post-review fix | fix(refbox): return to App Options when cancelling portal switch |
| `846ef02` | walkthrough fix | fix(refbox): blue restart button on portal switch confirmation |
