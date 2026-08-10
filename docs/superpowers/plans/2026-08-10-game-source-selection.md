# Game Source Selection Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development
> (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use
> checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the `USING UWHPORTAL: YES/NO` button with `MANUAL GAMES: YES/NO` plus a choice
between the built-in portal and a custom site, so a third-party server becomes an operator-visible
option instead of an environment variable and a launch flag.

**Architecture:** One persisted three-way `GameSource` replaces the in-memory `using_uwhportal`
boolean. Most existing call sites only ask "remote or manual?", so they route through a single
derived accessor; only the few that must distinguish portal from custom match on the enum. The
custom site is one typed URL carrying the event ID, parsed by a pure, unit-tested function. Apply
swaps the portal client inside its existing lock, guarded so it cannot happen mid-game or with
results queued.

**Tech Stack:** Rust 2024, MSRV 1.85, iced 0.13, Fluent (15 locales), `just check`
(fmt + clippy `-D warnings` + tests + audit).

**Design spec:** `docs/superpowers/specs/2026-08-10-game-source-selection-design.md` — read it
before starting. It carries the rationale and a verified source-reference table.

## Global Constraints

- Branch: `feat/refbox/game-source-selection`. **Do not push and do not open a PR** — the human's
  approval is required for both.
- Crate: `refbox` only. **No changes to `uwh-common`**, `overlay`, or any other crate. If a task
  appears to require one, stop and raise it.
- Every new Fluent key must be added to **all 15 locales** with a real best-guess translation.
  Placeholders and English fallbacks are not acceptable. Locales:
  `de-DE, en-US, es, fr, id-ID, it-IT, ja-JP, ko-KR, ms-MY, nl-NL, pt-PT, th-TH, tl-PH, tr-TR, zh-CN`.
- Exact English copy to use, verbatim: `MANUAL GAMES:`, `CUSTOM`, `SITE:`, `SITE URL`.
  The portal button reuses the existing tenant variable so it reads `UWH PORTAL` or `UWR PORTAL`.
- Exact identifiers to use: enum `GameSource` with variants `Manual`, `Portal`, `Custom`;
  enum `RemoteSource` with variants `Portal`, `Custom`; config keys `source` and
  `remembered_remote`; config section `[custom_site]` with keys `url` and `token`.
- `GameSource` is the authoritative current source. `remembered_remote` is *only* read when the
  operator turns MANUAL GAMES off, to decide which source to return to. It is not a second source
  of truth and nothing else may branch on it.
- `refbox` is a binary crate: run `cargo test -p refbox` with **no** `--lib`, and clippy without
  `--all-targets`. Do not add `unwrap()`/`expect()` to non-test code without a comment saying why
  it cannot panic — match the existing "why this cannot panic" comment style.
- Lean process (`.claude/rules/plan-execution.md`): no per-task deviation commits, one code review
  at the end of the feature, no verification ceremony on translation-only or comment-only steps.
- Verification reality: refbox view/update code has no unit-test harness. Tasks 1 and 2 are
  genuinely test-first; the rest are verified by `just check` plus the GUI walkthrough in Task 9.
  Do not invent test scaffolding in production source for the untestable parts.

---

### Task 1: Persist the source and the custom site

**Files:**
- Modify: `refbox/src/config.rs`
- Test: `refbox/src/config.rs` (existing `#[cfg(test)]` module)

**Interfaces:**
- Produces: `GameSource` (`Manual` | `Portal` | `Custom`, default `Manual`), `RemoteSource` (`Portal` | `Custom`, default `Portal`), `CustomSite { url: String, token: String }`, and config fields `source: GameSource`, `remembered_remote: RemoteSource` and `custom_site: CustomSite`.
- Consumes: nothing.

- [ ] **Step 1: Write failing serialisation and migration tests**

Mirror the two existing tests `test_ser_uwhportal` and `test_migrate_uwhportal` in the same module —
read them first and follow their shape exactly. Cover:
- a round trip of `source` for each of the three variants
- a round trip of `remembered_remote` for both variants
- a round trip of `[custom_site]` with a url and token
- migration of a config that has **no** `source` key → `GameSource::Manual`
- migration of a config that has **no** `remembered_remote` key → `RemoteSource::Portal`
- migration of a config that has **no** `[custom_site]` section → empty url and empty token

The no-key cases are the important ones: every existing installation is missing both.

- [ ] **Step 2: Run the tests to confirm they fail**

Run: `cargo test -p refbox config`
Expected: failures naming the missing `source` / `custom_site` items.

- [ ] **Step 3: Add the types and fields**

`GameSource` derives the same traits as the neighbouring config enums (look at how `Mode` is
declared and copy it), with `Manual` as `#[default]`. `CustomSite` mirrors the existing
`UwhPortal` struct, including its `migrate` function built from `get_string_value`.

Leave `UwhPortal { token }` alone — the portal keeps its own credential. That separation is a
design decision, not an accident.

- [ ] **Step 4: Run the tests to confirm they pass**

Run: `cargo test -p refbox config`

- [ ] **Step 5: Commit**

```bash
git add refbox/src/config.rs
git commit -m "feat(refbox): persist game source and custom site config"
```

---

### Task 2: Parse and validate the custom URL

**Files:**
- Create: `refbox/src/app/custom_site.rs` (with its own `#[cfg(test)]` module)
- Modify: `refbox/src/app/mod.rs` (add the `mod custom_site;` declaration only)

**Interfaces:**
- Produces: `parse_custom_site(input: &str) -> Result<ParsedSite, CustomSiteError>` where
  `ParsedSite { base_url: String, event_id: EventId }`, and `CustomSiteError` carrying enough
  detail for the UI to explain the failure.
- Consumes: `EventId` from `uwh_common::uwhportal::schedule`.

This is the only genuinely unit-testable piece of new logic. Keep it pure: no I/O, no app state.

- [ ] **Step 1: Write the failing tests**

The operator types one string carrying both site and event, e.g.
`http://scoreboard.local:8099/api/events/1234-A`. Cover, as separate cases:

Accepted:
- `http://scoreboard.local:8099/api/events/1234-A` → base `http://scoreboard.local:8099`, event `1234-A`
- the same with `https://`
- a trailing slash after the event ID
- an event ID of exactly three characters (`ABC`) — the minimum the contract allows
- a long hyphenated event ID (IDs are opaque beyond the length rule)

Rejected, each with a distinct error:
- no `/api/events/` segment at all
- nothing after `/api/events/`
- an event ID shorter than three characters (`AB`)
- a scheme that is neither `http` nor `https`
- empty input

The length and prefix rules are not arbitrary: `uwh-common` validates event IDs on deserialise and
a violation fails the **entire** response, so catching it at Apply is the whole point of this
function. Do not reimplement the validation by hand — construct the `EventId` through its public
constructor so the rules cannot drift apart.

- [ ] **Step 2: Run the tests to confirm they fail**

Run: `cargo test -p refbox custom_site`

- [ ] **Step 3: Implement the parser**

Split at the last `/api/events/` occurrence; everything before is the base URL, the next path
segment is the event ID. Trim a trailing slash from the base (the client also trims, but the SITE
row displays this string, so normalise it once here).

- [ ] **Step 4: Run the tests to confirm they pass**

Run: `cargo test -p refbox custom_site`

- [ ] **Step 5: Commit**

```bash
git add refbox/src/app/custom_site.rs refbox/src/app/mod.rs
git commit -m "feat(refbox): add custom site URL parsing and validation"
```

---

### Task 3: Replace the boolean with the three-way source

**Files:**
- Modify: `refbox/src/app/mod.rs` (~45 references)
- Modify: `refbox/src/app/view_builders/configuration.rs`
- Modify: `refbox/src/app/view_builders/game_info.rs`
- Modify: `refbox/src/app/view_builders/shared_elements.rs`
- Modify: `refbox/src/app/view_builders/game_info_table.rs`

**Interfaces:**
- Consumes: `GameSource` from Task 1.
- Produces: `source: GameSource` on both `RefBoxApp` and `EditableSettings`, replacing
  `using_uwhportal: bool`; plus a derived accessor `uses_remote(&self) -> bool` returning
  `!matches!(self.source, GameSource::Manual)`.

This is the largest task by diff size and the smallest by judgement required. The compiler finds
every site; the rule for converting them is below.

- [ ] **Step 1: Add the field and the accessor, keeping the old field temporarily**

Add `source: GameSource` alongside `using_uwhportal` so the build still passes, and add
`uses_remote()`. This keeps the task reviewable in stages rather than as one unbuildable jump.

- [ ] **Step 2: Convert every read site mechanically**

The rule: a site that asks "is the portal in use at all?" becomes `uses_remote()`. A site that
must distinguish the official portal from a custom one matches on `GameSource` explicitly. Almost
every site is the first kind.

Sites known to need an explicit match rather than `uses_remote()`:
- the startup link restore, which currently sets the flag true (`mod.rs` around line 1969) — it
  restores the **portal**, so set `GameSource::Portal`
- anything that builds the portal login/link flow, which is portal-only
- the event-list request, which Task 7 makes portal-only

- [ ] **Step 3: Remove `using_uwhportal` entirely**

Delete the field from `RefBoxApp` and `EditableSettings`. Keep `BoolGameParameter::UsingUwhPortal`
for now — Task 4 replaces the message that uses it.

- [ ] **Step 4: Verify**

Run: `just check`
Expected: clean. A behaviour change here would be a bug: this task is a pure representation change.

- [ ] **Step 5: Commit**

```bash
git add refbox/src/app/
git commit -m "refactor(refbox): replace portal boolean with a three-way game source"
```

---

### Task 4: The source control in row 1

**Files:**
- Modify: `refbox/src/app/view_builders/configuration.rs`
- Modify: `refbox/src/app/message.rs`
- Modify: `refbox/src/app/mod.rs` (handle the new message)
- Modify: `refbox/translations/*/refbox.ftl` (all 15)

**Interfaces:**
- Consumes: `GameSource`, `uses_remote()` from Task 3.
- Produces: a message that sets the source explicitly (not a toggle), e.g.
  `Message::SelectGameSource(GameSource)`.

- [ ] **Step 1: Add the Fluent keys, all 15 locales**

```
manual-games = MANUAL GAMES:
source-portal = { $portal }PORTAL
source-custom = CUSTOM
```

`source-portal` reuses the existing `$portal` tenant variable so it reads `UWH PORTAL` or
`UWR PORTAL` — see how `using-portal` at `en-US/refbox.ftl:107` passes it.

- [ ] **Step 2: Replace the toggle with the three-way control**

Row 1 is already a three-cell grid whose second and third cells are blank
(`configuration.rs:678`) — the two source buttons go there, so no layout change is needed.

- `MANUAL GAMES: YES/NO` occupies cell 1, using the existing `make_value_button` pattern.
- When the answer is NO, cells 2 and 3 hold the portal and custom buttons, with the active one
  visually selected. Follow the existing selected-button styling; do not invent a new one.
- When the answer is YES, cells 2 and 3 stay blank exactly as they are today.

Retire `BoolGameParameter::UsingUwhPortal` in favour of the explicit source message. A toggle
cannot express three states, and an explicit setter removes any ordering question about which
source a toggle would land on.

- [ ] **Step 3: Remember the remote choice across MANUAL**

Turning MANUAL GAMES **on** sets `source` to `Manual` and leaves `remembered_remote` untouched.
Turning it **off** sets `source` from `remembered_remote`. Choosing either source button while not
in manual sets both `source` and `remembered_remote` together.

So an operator who runs a custom site, switches to manual for a friendly, and switches back lands
on CUSTOM again rather than being bounced to the portal. Their URL and token were never lost —
those persist independently — and now the selection is not lost either.

`remembered_remote` defaults to `Portal`, so an installation that has never chosen a source behaves
exactly as it does today.

- [ ] **Step 4: Verify**

Run: `just check`, then launch and confirm by eye that the row renders in all three states and
that switching does not disturb the rows below.

- [ ] **Step 5: Commit**

```bash
git add refbox/src/app/ refbox/translations/
git commit -m "feat(refbox): add manual games and source selection control"
```

---

### Task 5: The SITE row and its edit page

**Files:**
- Modify: `refbox/src/app/view_builders/configuration.rs`
- Modify: `refbox/src/app/mod.rs` (new page state, text input handling)
- Modify: `refbox/src/app/message.rs`
- Modify: `refbox/translations/*/refbox.ftl` (all 15)

**Interfaces:**
- Consumes: `parse_custom_site` from Task 2; `GameSource` from Task 3.
- Produces: the SITE row, and an edit page reachable from it.

This introduces the **first text input in the whole application** — there is no `text_input`
anywhere in refbox today. Use iced's `text_input` widget and follow the theming in
`refbox/src/app/theme/`; do not introduce a new styling approach.

- [ ] **Step 1: Add the Fluent keys, all 15 locales**

```
custom-site = SITE:
custom-site-url-title = SITE URL
custom-site-invalid = That address is not usable. It must look like http://your-site/api/events/1234-A
```

- [ ] **Step 2: Add the SITE row, shown only for `GameSource::Custom`**

It takes the row that **EVENT** occupies under the portal — the event is in the URL now, so the
event picker has nothing to pick. TOKEN keeps its own row in both sources, and the page stays at
four rows either way:

```
portal:  [ MANUAL: NO ][ UWH PORTAL ][ CUSTOM ]   custom:  [ MANUAL: NO ][ UWH PORTAL ][ CUSTOM ]
         [ EVENT: ... ]                                    [ SITE:  ... ]
         [ TOKEN:  OK ]                                    [ TOKEN:  OK ]
         [ COURT:   A ]                                    [ COURT:   A ]
```

The row displays **the URL actually in use**, which means the environment override wins when it is
set. A typed value must never be silently ignored while a different site is really called; that is
the exact failure this feature exists to remove.

- [ ] **Step 3: Add the edit page**

Full-width text input, plus Cancel and Apply, matching the shape of the existing pages reached
from this settings page. Apply runs `parse_custom_site` and, on failure, keeps the page open and
shows `custom-site-invalid`. Rejecting here rather than failing later is deliberate: an operator on
the pool deck cannot debug a bad URL mid-game.

No spacebar conflict needs handling — the manual-alarm handler is already gated to the main screen
(`mod.rs:4364`), with a comment saying it exists so text inputs are unaffected.

- [ ] **Step 4: Verify**

Run: `just check`, then launch with a keyboard attached and confirm typing works, a good URL is
accepted, and each rejection case from Task 2 produces the message rather than a silent failure.

- [ ] **Step 5: Commit**

```bash
git add refbox/src/app/ refbox/translations/
git commit -m "feat(refbox): add custom site row and URL edit page"
```

---

### Task 6: Apply — swap the client, with guards

**Files:**
- Modify: `refbox/src/app/mod.rs`
- Modify: `refbox/translations/*/refbox.ftl` (all 15)

**Interfaces:**
- Consumes: `ParsedSite` from Task 2; `GameSource` from Task 3.
- Produces: the repoint path used by Task 7.

- [ ] **Step 1: Add the refusal messages, all 15 locales**

```
source-locked-clock = The game clock is running. Stop the clock before changing this.
source-locked-queue = Game results are still waiting to be sent. Send or discard them first.
```

- [ ] **Step 2: Guard the change**

Repointing is refused while the clock runs, and while any result is pending in the outbound queue.
`clock_running` is already a parameter of the settings view builder, so the first gate needs no new
plumbing. Both refusals must **say why** using the keys above — do not simply grey the control out,
because a silent refusal is indistinguishable from a broken button.

Both guards cover switching source **and** editing the URL: either repoints the live client.

- [ ] **Step 3: Swap the client on Apply**

Assign a freshly constructed `UwhPortalClient` through the existing mutex guard. The client is held
as `Option<Arc<Mutex<UwhPortalClient>>>` (`mod.rs:160`) and every request formats its address from
the private `base_url` at call time, so the next request uses the new site with no restart.

Construct it — do not attempt to mutate a URL string. The TLS requirement is fixed on the inner
HTTP client at construction, and it must be derived from the scheme of the URL actually in use
(`https://` requires TLS, `http://` does not). This is what removes the `--allow-http` requirement;
the sibling `schedule-processor/src/main.rs:63` already does exactly this.

In-flight requests need no handling: the existing pattern locks, builds the request, releases the
lock, then awaits, so a request already in flight completes against the site it was addressed to.

- [ ] **Step 4: Leave a custom site alone across a mode switch**

Switching hockey/rugby mode currently discards the portal link and tells the operator to reconnect
— there is an existing message for it, `mode-switch-portal-tenant`. That must keep happening for
`GameSource::Portal`, unchanged.

A custom site is **not** tenant-scoped in the same way, so a mode switch must leave it intact: its
URL, its token and the selected source all survive. Find the mode-switch handler and make the
discard conditional on the source being `Portal`.

The two sources therefore behave differently here. That is intentional, and it needs to be called
out in the release notes so it does not read as a bug.

- [ ] **Step 5: Verify**

Run: `just check`, then confirm by hand that Apply with a changed URL causes the next fetch to hit
the new site, that both guards refuse with their message, and that switching mode leaves a
configured custom site untouched while still clearing a portal link.

- [ ] **Step 6: Commit**

```bash
git add refbox/src/app/ refbox/translations/
git commit -m "feat(refbox): repoint the portal client on apply, with guards"
```

---

### Task 7: Give CUSTOM its schedule and teams

**Files:**
- Modify: `refbox/src/app/mod.rs`

**Interfaces:**
- Consumes: the repoint path from Task 6; `ParsedSite` from Task 2.
- Produces: a populated events-map entry, schedule and teams for the embedded event.

- [ ] **Step 1: Create the synthetic events-map entry**

Team data is stored into an entry in the events map, and the court picker reads its court list from
that same entry. Custom therefore needs one entry for the embedded event ID or teams and court data
have nowhere to land and the court picker stays permanently empty. This is a required step, not a
tidy-up.

- [ ] **Step 2: Trigger the fetches directly**

On applying a custom site, call the two existing per-event helpers for the embedded event:
`request_teams_list(event_id)` (`mod.rs:731`) and `request_schedule(event_id)` (`mod.rs:752`, which
fetches the privileged schedule and the referee names together). No new fetching machinery — Custom
simply becomes another caller of the functions the event-list path already loops over.

- [ ] **Step 3: Skip the event-list call under Custom**

The event is named in the URL, so the list is not needed. `RecvEventList` (`mod.rs:4089`) remains
the portal path's trigger and is untouched.

- [ ] **Step 4: Verify**

Run: `just check`, then confirm against the verification stub in the sibling branch's
`docs/third-party-stub/` that court and game become selectable and the game shows **both team
names**. Team names are the real check: they only appear if the schedule's team IDs match IDs the
teams call returned, and a mismatch shows the raw ID or `Unknown` with nothing logged.

- [ ] **Step 5: Commit**

```bash
git add refbox/src/app/
git commit -m "feat(refbox): fetch schedule and teams for a custom site"
```

---

### Task 8: Stop verifying a token we do not have

**Files:**
- Modify: `refbox/src/app/mod.rs`

**Interfaces:**
- Consumes: nothing new.
- Produces: no new interface. Behaviour change only.

This deliberately changes the **existing portal path** as well as the custom one. It is in scope
because a source display that can lie defeats the purpose of the feature.

- [ ] **Step 1: Gate the verify call on holding a token**

`check_uwhportal_auth` (`mod.rs:818`) currently calls `verify_token` unconditionally. When no token
is held, do not send the call and leave the indicator at `Some(false)` — FAILED.

Why this matters, observed live against a permissive stub: refbox correctly set the indicator to
FAILED with no token, then the site's `200` arrived and `RecvTokenValid` overwrote it to a green OK
(`mod.rs:4302`), with no `Authorization` header ever sent. Only the site can enforce a token, so
refbox must not present the site's answer as proof of its own credentials.

- [ ] **Step 2: Verify**

Run: `just check`, then with the stub running and **no** token saved, confirm the indicator reads
FAILED and **stays** FAILED, and that no verify request appears in the stub's log.

- [ ] **Step 3: Commit**

```bash
git add refbox/src/app/
git commit -m "fix(refbox): do not verify a token that is not held"
```

---

### Task 9: Walkthrough against the stub

**Files:** none — this task writes no code.

The verification stub lives on the sibling branch `docs/workspace/third-party-data-source` at
`docs/third-party-stub/stub_site.py`. It is dependency-free Python; run it directly.

**Before running anything:** back up `~/.config/refbox`. Every refbox build on the machine shares
that directory, so a stub run overwrites whatever link and token are saved there. Confirm no other
refbox is running first (`pgrep -ax refbox`) — only one may run at a time.

- [ ] **Step 1: Confirm each acceptance criterion from the spec, one at a time**

Work through the numbered acceptance criteria in the design spec and report the result of each
individually. Do not batch them into a single "all passed".

- [ ] **Step 2: Record the outcome**

Note anything that failed, and anything the spec did not anticipate, in the plan's Deviations
section below.

---

## Deviations and outcome

_(Record deviations from this plan here as execution proceeds. Per
`.claude/rules/plan-execution.md`, do not create standalone deviation commits — fold notes into the
code commit or record them here.)_

- **Task 1: followed `Mode`, not the plan's `#[default]` shorthand.** The plan says `GameSource`
  takes `Manual` as `#[default]`, but the neighbouring config enums use `Derivative`'s
  `#[derivative(Default)]` on the variant. The plan's own instruction is to "look at how `Mode` is
  declared and copy it", so `Mode` won. Both new enums also derive `EnumFromStr!` for the same
  reason `Mode` does: `Config::migrate` reads them back from strings.

- **Task 2: PLAN DEFECT — a parser with no caller fails the lint gate.** `just lint` runs
  `cargo clippy --all -- -D warnings`, and `refbox` is a binary crate, so `parse_custom_site`,
  `ParsedSite`, `CustomSiteError` and `EVENTS_SEGMENT` are all `dead_code` until Task 5 calls them.
  Verified: `just lint` exits 101 with four errors. Task 1 escaped this only because its new types
  are `Config` fields, which serde's derived code counts as reading.

  Resolved with the human's explicit approval (required by `.claude/rules/rust.md`, which forbids
  silencing warnings without discussion) by a module-level `#![allow(dead_code)]` in
  `custom_site.rs`, carrying a comment that names Task 5 as the point of deletion. The two
  alternatives considered and rejected were leaving Task 2 uncommitted until Task 5 (uncommitted
  work has been lost twice in this project) and reordering Task 3 ahead of Task 2 (throws away
  working tested code).

  **Task 5 must delete that attribute.** If it survives Task 5, the parser was never wired up.

  Future plans that introduce a pure helper ahead of its caller should either say how the lint gate
  will be satisfied, or order the caller into the same task.

- **Task 3: one file missing from the plan's list.** `refbox/src/app/view_builders/main_view.rs`
  also threaded the boolean (4 sites) and had to be included. 96 occurrences across 6 files, not 5.

- **Task 3: `PageEntrySnapshot` had to become three-way, not just renamed.** The Cancel/revert
  snapshot (`mod.rs`) stored `using_uwhportal: bool` and restored it onto `edited`. Left as a
  boolean it would have silently collapsed `Custom` to `Portal` on revert once Task 4 can select a
  custom site — a real defect rather than a naming one. Both variants now carry
  `source: GameSource`.

- **Task 3: the four view files were renamed as well, on the human's decision.** `main_view.rs`,
  `game_info.rs`, `game_info_table.rs` and `shared_elements.rs` never held the field — they only
  receive the yes/no answer — so the compiler did not require touching them (about a fifth of the
  96 sites). Renaming their parameter `using_uwhportal` -> `uses_remote` was chosen because after
  this change the value is `true` for a third-party site, which is emphatically not the UWH Portal,
  and a future reader trusting the old name would branch the wrong way.

- **Task 3: the portal-login and event-list sites were left as `uses_remote()` deliberately.** The
  plan lists them as needing an explicit `GameSource` match. They do not yet: `Portal` is currently
  the only remote, so `uses_remote()` is exactly behaviour-preserving, and narrowing them now would
  be the behaviour change this task is required not to make. Task 7 narrows the event list; the
  login flow narrows when a custom site can actually be selected.

- **Task 4: `remembered_remote` is recorded on Apply, not when a source button is tapped.** The plan
  (Step 3) says choosing a source "sets both `source` and `remembered_remote` together". Implemented
  instead as: the button sets only `source`, and the *apply* path records the remote actually
  applied. Two reasons. First, updating it on tap lets a choice the operator then **cancels** survive
  as a hidden preference. Second, avoiding that would mean adding the field to `PageEntrySnapshot`,
  whose `page_has_changes` comparison would then treat an invisible preference as a visible edit and
  could wrongly enable APPLY. The plan's stated goal still holds: an operator who *applies* CUSTOM,
  switches to manual, and switches back lands on CUSTOM. Only an un-applied, same-session sequence
  differs, which is a corner case.

- **Task 4: `remembered_remote` had to be plumbed onto `EditableSettings`.** The plan's file list
  omitted this. Task 1 put the field on `Config` only, so Task 4 added it to `EditableSettings`,
  seeded it from the config at all five real construction sites, and wrote it back at all four apply
  sites. Cheap in the end because the ~24 test literals use `..Default::default()`.

- **Task 4: also relabelled the token row, at the human's request.** `UWHPORTAL TOKEN:` became
  `ACCESS TOKEN:` — it is wrong for a custom site, and the spec's own sketch had already dropped
  "UWHPORTAL" without flagging it as a step. This uncovered a **pre-existing bug**: that label was
  `text("UWHPORTAL TOKEN:")`, the *only* hardcoded English string in `configuration.rs`, so it
  appeared untranslated in all 14 non-English locales. It is now the Fluent key `access-token`.

- **Task 4: the "cells 2 and 3 are blank" claim holds only in remote mode.** In manual mode row 1's
  other two cells already carry game settings (overtime allowed, etc.). That does not affect the
  design — the source buttons appear only when MANUAL GAMES is NO, which is exactly the branch with
  the blank cells — but the plan's wording ("stay blank exactly as they are today" under YES) is
  wrong about the manual branch.

- **Task 4: `trigger_event_list_fetch` removed from the `ToggleBoolParameter` handler.** The portal
  toggle was its only setter, so once that moved to `Message::SelectGameSource` the variable and its
  trailing `if` were dead and clippy rejected the unused `mut`. Deleted rather than silenced.

- **Task 5: the SITE row shows the TYPED URL, not the effective one — deferred to Task 6.** The plan
  requires the row to display "the URL actually in use", so that a typed value is never silently
  ignored while the environment override really points elsewhere. That safety property needs the
  effective URL, which is decided by the client swap in Task 6; the view has no access to it today.
  **Task 6 must complete this**, or the failure this feature exists to remove survives in the one
  place an operator would look.

- **Task 5: `custom_site` had to be plumbed onto `EditableSettings`** — the same omission as Task 4's
  `remembered_remote`. Seeded from the config at all five construction sites.

- **Task 5: added `PageEntrySnapshot::CustomSite`.** Needed for two things the plan assumes without
  saying: Cancel discarding a half-typed URL, and the APPLY gate greying until the URL actually
  changes. Unlike `remembered_remote` this one *is* snapshotted, because the typed URL is a visible
  edit rather than a hidden preference.

- **Task 5: one rejection message rather than five, on the human's decision.** Task 2 produces five
  distinct errors; the plan's Step 1 defines a single message naming the correct shape, and that was
  chosen deliberately over five specific messages (which would have cost 75 translations). The
  parser's distinctions are still there for a later pass if operators struggle. Typing clears the
  message, so a rejection never nags.

- **Task 4 follow-up folded into Task 5's commit:** the portal source button read `UWHPORTAL` because
  it reused the existing tenant wording verbatim. Changed to `{ $portal } PORTAL` in all 15 locales,
  so it reads `UWH PORTAL`, matching the spec's sketch. Decided at the Task 4 visual check.

- **Task 3: the ON/OFF toggle preserves today's meaning rather than using `remembered_remote`.**
  `BoolGameParameter::UsingUwhPortal` now sets `Manual` or `Portal` explicitly instead of flipping a
  boolean. Routing OFF->ON through `remembered_remote` would be a behaviour change and belongs to
  Task 4, which replaces this control.

## Out of scope

- Changing the verification stub's token handling.
- Fixing the two known documentation defects in `docs/third-party-integration.md` (tracked on its
  own branch): the permissive-site requirement, and the claim that an operator can type a token
  directly into refbox.
- The `TOKEN: OK` wording decision for the custom path. The indicator's behaviour is fixed in
  Task 8; whether the label should read something weaker than `OK` is an open copy decision for
  the human and must not be changed unilaterally.
