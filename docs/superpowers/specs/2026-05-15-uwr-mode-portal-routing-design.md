# 2026-05-15 — UWR Mode Portal Routing Design

**Status:** proposed (brainstormed 2026-05-15; awaiting implementation plan)
**Companion ADR:** [ADR 016 — UWR Mode Portal Routing](../../decisions/016-uwr-mode-portal-routing.md)
**Branches off:** the integrated audit-chain tip (`audit/refbox/portal-health` after the Unit 7 rebase, commit `e1a5577`).
**Operator who decided:** Eric (2026-05-15 brainstorming session)

---

## Context

The refbox supports three game-mode values via the `Mode` enum: `Mode::Hockey6V6`, `Mode::Hockey3V3`, and `Mode::Rugby`. Underwater hockey and underwater rugby are hosted on separate portal tenants (`uwhportal.com` / `api.uwhportal.com` for hockey, `uwrportal.com` / `api.uwrportal.com` for rugby). Today the refbox talks to the UWH portal regardless of Mode and shows "UWH"-prefixed text everywhere, even in Rugby mode. Rugby-mode operators can't successfully link an event because the wrong tenant 404s their requests, and the UI gives no hint that the routing is wrong.

The Unit 7 audit (just integrated) made the portal health *tile* mode-aware — the logo on the time banner picks UWH or UWR based on Mode — but did not address the URL routing or the text strings. The post-smoke-test finding from 2026-05-15 made the gap concrete: in Rugby mode the operator still sees "Using UWH Portal" and the refbox still calls `api.uwhportal.com`.

This spec captures the operator-decided shape for fixing both the routing and the text consistency in a single focused branch.

---

## Goals

1. The refbox calls `api.uwrportal.com` when `Mode::Rugby`, `api.uwhportal.com` otherwise.
2. Every operator-facing string that references a portal swaps explicitly between "UWH" and "UWR" based on Mode — no generic "Portal" fallback.
3. Switching Mode across the portal boundary (Hockey ↔ Rugby) requires an explicit operator confirmation and an app restart; switching within Hockey variants (6v6 ↔ 3v3) does not.
4. Log messages reflect the active portal name (UWH or UWR) so tournament-side debugging is consistent.
5. Existing operator workflows for Hockey-only tournaments behave identically to today (no regressions).

---

## Non-Goals

- **Internal Rust identifier renaming.** `UwhPortalClient`, `using_uwhportal`, `uwh_common::uwhportal` module path, enum variants like `ConfirmationKind::UwhPortalLinkFailed`, and function names like `request_uwhportal_token` all stay as-is. The operator never sees them; renaming is a 25+ file refactor with regression risk and zero operator-visible benefit.
- **Portal subsystem dormancy follow-up.** The smoke-test finding that the indicator should re-hide when "Using UWH Portal" is toggled back to No is a separate concern. Tracked in memory entry `project_portal_subsystem_dormancy_followup` and reserved for a future focused branch.
- **Queue tenant tagging.** Sidestepped by flushing the queue on Mode-switch restart (see Architecture below). No schema bump for the persisted queue file.
- **Config migration for the existing `url` field.** Confy silently ignores unknown fields on load; operators who hand-edited the field will fall back to the default. Acceptable because the field was never exposed in the UI.

---

## User Experience

### Mode-switch trigger

The Mode tile lives on the App Options page. Tapping it cycles through Hockey 6v6 → Hockey 3v3 → Rugby → back to Hockey 6v6.

The new confirmation-and-restart behaviour fires *only* when the new Mode talks to a different portal than the old Mode:

| From → To | Cross-portal? | Confirmation + restart? |
|-----------|---------------|-------------------------|
| Hockey 6v6 → Hockey 3v3 | No | No (normal Mode change) |
| Hockey 3v3 → Hockey 6v6 | No | No |
| Hockey 6v6 → Rugby | Yes | Yes |
| Hockey 3v3 → Rugby | Yes | Yes |
| Rugby → Hockey 6v6 | Yes | Yes |
| Rugby → Hockey 3v3 | Yes | Yes |

Within-Hockey changes commit immediately as today. Cross-portal changes raise the confirmation page described below.

### Confirmation page

A new `ConfirmationKind::PortalTenantSwitch { from_mode: Mode, to_mode: Mode }` variant displays a single localized message that varies based on the four passed Fluent variables:

- Key: `mode-switch-portal-tenant`
- Template: *"Changing mode from { $from_mode } to { $to_mode } will disable the link to { $from_portal }PORTAL and you must re-connect to { $to_portal }PORTAL."*
- Rendered in Hockey-6v6 → Rugby case: *"Changing mode from Hockey 6v6 to Rugby will disable the link to UWHPORTAL and you must re-connect to UWRPORTAL."*
- Rendered in Rugby → Hockey-3v3 case: *"Changing mode from Rugby to Hockey 3v3 will disable the link to UWRPORTAL and you must re-connect to UWHPORTAL."*

Variables:
- `$from_mode`, `$to_mode`: mode-name translation keys (already exist, e.g., `mode-hockey-6v6`, `mode-rugby`), resolved at format time.
- `$from_portal`, `$to_portal`: result of `portal_name_for_mode(from_mode)` and `portal_name_for_mode(to_mode)`, i.e., `"UWH"` or `"UWR"`.

Footer buttons:
- **Cancel** (red, left): reverts the in-flight Mode change. Returns to App Options with Mode unchanged. No persistence, no restart.
- **Restart to Apply** (red, right): the same widget shape Unit 8 introduced for cross-font-family language switching. Commits the new Mode and triggers the restart sequence.

### Restart flow

On "Restart to Apply":

1. The new Mode is committed to `EditableSettings.mode` and to `self.config.mode` (the live state).
2. `self.current_event_id` is cleared, routed through `set_current_event_id(None)` so the shared `portal_event_id` handle (consumed by the background portal-health task) is mirrored to `None` before the task winds down.
3. The persisted retry queue file at `portal_manager/queue.rs`'s configured path is flushed (file deleted or replaced with an empty queue, whichever is mechanically cleaner).
4. The config is persisted to disk via `confy::store(APP_NAME, None, &self.config)`.
5. The app spawns a fresh exe via the same `std::process::Command::spawn + std::process::exit` pattern Unit 8 uses for cross-font-family language restarts (`make_language_select_page`'s restart button handler).

On the next launch:
- The portal client is constructed against the new Mode's URL (see Architecture below).
- The operator sees a fresh refbox in the new Mode, with no event linked, and is prompted to link an event against the new portal via the existing portal-link flow.

### What the operator does NOT see

- Internal Rust identifiers (`UwhPortalClient`, `using_uwhportal`, etc.) remain. The operator never touches these — they're code-internal.
- The retry queue's flushed items are silently discarded. Operators who had pending submissions to the old portal would have been blocked by the wrong-tenant 404s anyway; there's no value in preserving them across a Mode switch.

---

## Architecture

### URL routing

The portal client is constructed at startup in `refbox/src/app/mod.rs` near line 984 (current line; reference to be re-verified at implementation time). The construction picks the base URL based on Mode and respects symmetric env-var overrides:

```rust
let base_url = match self.config.mode {
    Mode::Rugby => std::env::var("UWR_PORTAL_URL_OVERRIDE")
        .unwrap_or_else(|_| "https://api.uwrportal.com".to_string()),
    Mode::Hockey6V6 | Mode::Hockey3V3 => std::env::var("UWH_PORTAL_URL_OVERRIDE")
        .unwrap_or_else(|_| "https://api.uwhportal.com".to_string()),
};
```

The existing `UwhPortal::url` field on the persisted config struct is removed. Confy ignores unknown fields on load; operators with hand-edited URLs in their config file silently fall back to the production default for their Mode.

### Mode-switch decision logic

In the Mode-tile handler (in `apply_app_options` or the Mode cycle path — exact location depends on the current per-page Cancel/Apply flow established in Unit 3):

1. When the operator commits the App Options page (presses Apply or the equivalent per-Unit-3 path), the handler checks whether `edited.mode` crosses the portal boundary relative to `self.config.mode`.
2. Helper function: `fn crosses_portal(old: Mode, new: Mode) -> bool` returns `true` iff `(old, new)` is in the cross-portal table above.
3. If `crosses_portal` returns `true`, raise `ConfirmationKind::PortalTenantSwitch { from_mode: self.config.mode, to_mode: edited.mode }` instead of committing the Mode change directly.
4. If `crosses_portal` returns `false`, commit the Mode change as today (no confirmation, no restart).

### Confirmation handler

A new arm in the existing `apply_confirmation` (or equivalent) dispatcher handles `ConfirmationKind::PortalTenantSwitch`:

- On `ConfirmationOption::DiscardChanges` (Cancel): drop the in-flight Mode edit; return to `AppState::EditGameConfig(ConfigPage::App)`.
- On `ConfirmationOption::Confirm` (Restart to Apply): execute the restart flow above.

A new `ConfirmationOption` variant may be needed if the existing semantics don't fit ("Confirm" vs "Discard" need to map onto "Restart" vs "Cancel" cleanly).

### Queue flushing

`refbox/src/portal_manager/queue.rs` persists queued retry items keyed on `(event_id, game_number)`. At Mode-switch restart time, the queue is flushed before persistence:

- The simplest mechanical approach: delete the queue file at the persisted path. The next run will start with an empty queue.
- Alternative: write an empty queue snapshot atomically. Either is fine; choose whichever fits the existing atomic-save helpers in `queue.rs` (commit `f7bfe5c` introduced `atomic_save`).

### Helper function: `portal_name_for_mode(mode)` — already exists, reused

The existing helper `portal_name_for_mode(Mode) -> &'static str` (introduced in commit `4154262`) is reused — it already returns `"UWR"` for `Mode::Rugby` and `"UWH"` for both Hockey variants. **No new helper is invented.** The 7 log call sites and all new translation-key-variable sites call this helper directly.

---

## Data and Translation Changes

### Translation keys

Per the operator's "no generic Portal" preference and the **`{ $portal }` Fluent variable pattern (option A, consistent with commit 4154262's existing approach)**:

| Key shape | Action | Notes |
|-----------|--------|-------|
| `using-uwh-portal` | **Renamed** to `using-portal`; body gains `{ $portal }` variable. | New template: `using-portal = USING { $portal }PORTAL`. Renders to "USING UWHPORTAL" in Hockey modes, "USING UWRPORTAL" in Rugby. 15 locales — each existing locale's literal "UWH" substring becomes `{ $portal }`. |
| `portal-login-instructions` | **Body updated** (key name stays) to use `{ $portal }` variable wherever "UWH" appears in the body (twice in the English version). | 15 locales — each updates the body in place. |
| `UWHPortal-enabled` | **Renamed** to `portal-enabled`; body gains `{ $portal }` variable. | New template: `portal-enabled = When { $portal }PORTAL is enabled, all fields must be filled out`. 15 locales. |
| `mode-switch-portal-tenant` | **New.** | Single key with 4 variables: `$from_mode`, `$to_mode`, `$from_portal`, `$to_portal`. Template: `mode-switch-portal-tenant = Changing mode from { $from_mode } to { $to_mode } will disable the link to { $from_portal }PORTAL and you must re-connect to { $to_portal }PORTAL.` Renders correctly for both directional cases. 15 locales. |

Net: **1 new key + 3 keys updated in place (2 renamed, 1 body-only).** 1 × 15 = 15 new translation rows; the in-place body updates preserve existing translations (only the "UWH" literal is replaced with the variable, the surrounding sentence structure stays). Compare to option B which would have been 5 new keys + 2 renamed = ~75 new translation rows.

**Helper reuse:** the existing `portal_name_for_mode(Mode) -> &'static str` helper (introduced in commit `4154262`, currently used at three call sites in shared_elements.rs / detail-page / action-page view builders) is reused for the new call sites. No new helper invented.

### Locale rollout

All 15 supported locales (en-US, es, fr, de-DE, id-ID, it-IT, ja-JP, ko-KR, ms-MY, nl-NL, pt-PT, th-TH, tl-PH, tr-TR, zh-CN) gain the same 5 new keys. The UWR variant text is added by symmetrically translating the UWH variant (the only difference between the variants is "UWH" ↔ "UWR" in the brand name). For non-English locales, the implementer copies the structure of the existing UWH key and substitutes the brand name; this is mechanical and does not require native-speaker review since the brand name itself doesn't translate.

Call-site changes: every `fl!("using-uwh-portal")`, `fl!("portal-login-instructions")`, `fl!("UWHPortal-enabled")` becomes a `fl!` call with the renamed key plus the `portal` variable. Example:

```rust
// Before:
fl!("using-uwh-portal")

// After:
fl!("using-portal", portal = portal_name_for_mode(self.config.mode))
```

Same pattern at all six (rename × 3 + new × 1 + body-update × 1) sites that touch the affected keys, plus the new confirmation copy site that passes all four variables (`from_mode`, `to_mode`, `from_portal`, `to_portal`).

### Window title

`refbox/src/main.rs` near line 461 currently hardcodes `"UWH Ref Box"` as the window title. This becomes:

```rust
let title = match config.mode {
    Mode::Rugby => "UWR Ref Box",
    Mode::Hockey6V6 | Mode::Hockey3V3 => "UWH Ref Box",
};
```

The title is **not** translated — app brand names are not customarily translated, and the existing title is English-only today.

### Log messages

The 7 hardcoded log strings in `refbox/src/app/mod.rs` and `uwh-common/src/uwhportal/mod.rs` are updated to inject the active portal name via the `portal_name(mode)` helper. Examples:

| Current | New |
|---------|-----|
| `info!("Got a response from UWH Portal token request")` | `info!("Got a response from {} Portal token request", portal_name_for_mode(self.config.mode))` |
| `error!("Failed to get uwhportal token: {e}")` | `error!("Failed to get {} portal token: {e}", portal_name_for_mode(self.config.mode))` |
| `info!("UWH_PORTAL_URL_OVERRIDE active: using {portal_url}")` | Keep the env-var name explicit but add the active portal: `info!("URL override active for {} Portal: using {portal_url}", portal_name_for_mode(self.config.mode))` |
| Similar shape for the remaining 4 messages. | |

Log messages are English-only and not translated.

### Unchanged in this work

- **Internal Rust identifiers** (`UwhPortalClient`, `using_uwhportal`, module paths, enum variants, function names, fields): stay.
- **Doc comments** mentioning UWH: stay (developer-facing).
- **Test fixture function names** mentioning `uwhportal`: stay (developer-facing).
- **Env var names**: `UWH_PORTAL_URL_OVERRIDE` (existing) keeps its name; `UWH_PORTAL_SCRAMBLE_TOKEN` (existing, Unit 7 audit work) keeps its name; new symmetric `UWR_PORTAL_URL_OVERRIDE` is added.

---

## Decisions Made During Brainstorming (Reference)

These map to the operator's Q&A choices on 2026-05-15:

- **Q1 (Mode-switch behaviour):** Restart on cross-portal Mode change, confirmation page first. Within-hockey changes don't trigger restart.
- **Q2 (Config shape):** Derive URL from Mode (pure derivation, no stored URL field). Symmetric env var overrides.
- **Q3 (Translation pattern):** **Revised 2026-05-15 after parallel-work check.** Original choice was separate keys per portal (option B). The pre-commit `git log -S` parallel-work check (per audit refinement #5) surfaced commit `4154262 feat(refbox): mode-aware portal logo and sport-prefix in portal strings` which already established a `{ $portal }` Fluent variable pattern for three portal-related keys, with a helper `portal_name_for_mode(Mode) -> &'static str` already in place. To keep the codebase pattern consistent (one shape, not a mixed 3+3 split), the spec was revised to the variable pattern (effectively option A). Rendered operator-facing text is identical to what option B would have produced; the change is purely in how strings are stored in the .ftl files. The "no generic Portal" preference is preserved at the rendered-text level; only the underlying key name (e.g., `using-portal`) becomes generic.
- **Q4 (Scope):** Operator-facing strings + URL routing + log messages. Internal Rust identifiers explicitly out of scope.

---

## Verification Plan

### Manual walkthrough (end-to-end)

In Hockey 6v6 mode with the refbox launched:
1. App Options → confirm Mode tile reads "Hockey 6v6", "Using UWH Portal" text appears on the toggle button.
2. Toggle "Using UWH Portal" → Yes. Link an event against UWHPORTAL.
3. Verify the portal health tile shows the UWH logo and the indicator behaves per ADR 011.
4. Cycle Mode to Hockey 3v3 → no confirmation page, no restart. UWH text remains.
5. Cycle Mode again to Rugby. Apply.
6. Confirmation page appears: *"Changing mode from Hockey 3v3 to Rugby will disable the link to UWHPORTAL and you must re-connect to UWRPORTAL."*
7. Press Cancel → returns to App Options, Mode still Hockey 3v3.
8. Cycle to Rugby again, Apply, press Restart to Apply.
9. Verify the app restarts, the window title now reads "UWR Ref Box", and the toggle button reads "Using UWR Portal".
10. Toggle "Using UWR Portal" → Yes. Link an event against UWRPORTAL (verify the network call goes to `api.uwrportal.com`, e.g. via a network trace or log inspection).
11. Cycle Mode back through to Hockey 6v6 → symmetric confirmation page in reverse direction → Restart to Apply → window title becomes "UWH Ref Box", toggle reads "Using UWH Portal" again.

### Regression checks

- Existing portal-manager tests (39 tests; see Unit 7 audit) continue to pass.
- `just check` is clean.
- Manual: Hockey-only tournament workflow (the today-behaviour) is unchanged — no extra confirmations, no surprise restarts.

### Operator-side checks

- Log file inspection: after each Mode change, confirm log messages mention the correct portal name.
- The `UWR_PORTAL_URL_OVERRIDE` env var works: set it to a staging URL, launch refbox in Rugby mode, verify network calls go to the staging URL instead of production.

---

## Open Implementation Choices (Deferred to the Plan Step)

These are details the implementer may resolve when writing the plan, and the operator does not need to weigh in on before that step:

- Atomic queue flush mechanics: delete vs. empty-snapshot. Should match `queue.rs`'s existing patterns.
- Whether the `ConfirmationOption::Confirm` semantic needs a new variant or can reuse an existing one.
- Whether `portal-login-instructions` keeps its key name (preferred — minimal churn) or also gets renamed for consistency with the other two rename-and-vary keys.

---

## Out-of-Scope Follow-ups (Future Branches)

1. **Portal subsystem dormancy follow-up** — toggle the indicator off when "Using UWH Portal" goes back to No. Smoke-test finding from 2026-05-15.
2. **Internal identifier renaming** — if ever pursued, would be its own focused branch with its own ADR.
3. **Queue tenant tagging** — only relevant if we ever want to preserve queue items across Mode switches. Not needed under the flush-on-restart design.

---

## References

- [ADR 016 — UWR Mode Portal Routing](../../decisions/016-uwr-mode-portal-routing.md) — the original ADR this spec implements.
- [ADR 011 — Portal Health Indicator](../../decisions/011-portal-health-indicator.md) — the audit-integrated work this spec layers on top of.
- [Unit 8 spec — Language UI Chrome](2026-05-15-audit-unit-8-language-ui-chrome-design.md) — source of the restart-on-cross-family pattern reused here.
- Audit chain tip: `e1a5577 chore(refbox): cargo fmt after Unit 7 rebase integration` on branch `audit/refbox/portal-health`.
