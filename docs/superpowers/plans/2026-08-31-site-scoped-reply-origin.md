# Site-Scoped Reply Origin Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Stamp every site-scoped reply with the site generation it was issued under, so a reply arriving from a site the refbox has left is dropped instead of being attributed to the site it has arrived at.

**Architecture:** One `site_generation: u64` counter on `RefBoxApp`, bumped only where `repoint_client` actually assigns `self.current_site`. Each request-issuing function reads it at the moment it issues and moves the value into the reply message. Each handler compares on arrival through a single free function `reply_is_current`, accepting on equal and `warn!`-and-dropping otherwise. `RecvTeamRoster`'s existing `GameSource` tag is deleted and replaced by the stamp, so there is one mechanism rather than two.

**Tech Stack:** Rust 2024, MSRV 1.85, iced 0.13. Crate: `refbox` only.

**Spec:** `docs/superpowers/specs/2026-08-28-site-scoped-reply-origin-design.md`

**Suggested branch:** `fix/refbox/site-scoped-reply-origin` (branch creation needs Eric's approval before Task 1).

## Global Constraints

- Crate scope is `refbox` only. Do **not** touch `uwh-common`, `overlay`, or `wireless-remote`.
- Rust edition 2024, MSRV 1.85. No APIs newer than 1.85.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` must stay clean.
- No new `unwrap()`/`expect()` in production code without a comment justifying why it cannot panic.
- No new dependencies.
- Commit format: `type(scope): description`, lowercase, imperative, no trailing period, ~72 chars.
- The counter is a `u64` starting at `0`. Record the counter-vs-address trade-off in a code comment (spec: "Why a counter and not the site address") so the next reader does not mistake the rejection of `portal → custom → portal` for an oversight.

---

## DEVIATION FROM THE SPEC — read before Task 1

The spec's scope table lists **five** site-scoped replies and calls `RecvEventList` "the one that is currently wholly unguarded and the one most likely to be missed". **This plan deliberately stamps four, not five, and leaves `RecvEventList` unstamped.**

Reason, from the merged code: `request_event_list` (`refbox/src/app/mod.rs:1105-1128`) does **not** use the live client. It builds its own client against `portal_target(...)` and its doc comment states the intent verbatim — *"A client built here for the portal keeps the list loading whatever the refbox is committed to"*. That was made deliberate by commit `297ff166` ("stop guarding the event-list fetch on the live client's site"). The reply is therefore always portal data, and `set_portal_list` always files it in the portal bucket, whatever source the refbox is on.

Stamping it would mean a **valid** portal event list gets dropped whenever a source switch happened while it was in flight — breaking the very property `297ff166` added, and emptying the event picker in a way that looks like a network fault (the spec's own Risk 2).

The spec is internally inconsistent on this point: its Risks section says "**Four handlers, one rule**". This plan follows the Risks section. Task 6 records the exclusion in the code so a future reader does not "finish the job" by stamping it.

**RULED BY ERIC 2026-08-31 — four, not five.** In his words: *"an event list only comes from the Portal; if using Custom, the event is provided in the URL and only the court/game lists come through."* The code says the same thing verbatim at `adopt_custom_event` (`mod.rs:1482`): *"A custom site never calls the event list — its event is named in the URL."*

So the event list is not site-scoped data at all: there is only ever one source for it. It is excluded by construction, not by judgement call, and Task 6 stays a documentation task.

The court and game lists Eric names ride inside `RecvSchedule`, which **is** stamped (Task 3) — that is the picker-contamination path, and it stays closed.

---

## File Structure

| File | Responsibility | Change |
|---|---|---|
| `refbox/src/app/mod.rs` | `RefBoxApp` state, request issuers, `update()` handlers | Add `site_generation` field; add free fn `reply_is_current` + its tests; bump in `repoint_client`; stamp 4 issuers; guard 4 handlers |
| `refbox/src/app/message.rs` | The `Message` enum | Add `u64` to 4 variants; update `is_repeatable`, the manual `eq`, and the catch-all fallthrough arm for each |

No new files. Both files are large and established; follow their existing patterns rather than restructuring.

**Every variant touched needs FOUR edits in `message.rs`:**
1. The variant declaration (~line 190-206)
2. `is_repeatable` (~line 352-365)
3. The manual `eq` impl (~line 698-706)
4. The catch-all `(Self::X(..), _)` fallthrough (~line 787-793)

Missing #4 is a compile error; missing #2 or #3 is a silent behaviour change. `cargo build` catches all four because the arity changes.

---

### Task 1: The mechanism — counter, free function, and the bump

**Files:**
- Modify: `refbox/src/app/mod.rs` (struct field ~line 179; constructor ~line 3113; `repoint_client` ~line 1434; new free fn near `reply_source` ~line 601)
- Test: `refbox/src/app/mod.rs` (existing `#[cfg(test)]` module, alongside the `source_tap_outcome` tests ~line 8474)

**Interfaces:**
- Produces: `fn reply_is_current(issued_at: u64, now: u64) -> bool` — free function, module-private. Tasks 2-5 all call it.
- Produces: `RefBoxApp.site_generation: u64` — read by issuers, bumped by `repoint_client`.

- [ ] **Step 1: Write the failing test**

Add to the existing `#[cfg(test)]` module in `refbox/src/app/mod.rs`:

```rust
#[test]
fn reply_from_the_current_site_is_accepted() {
    assert!(reply_is_current(0, 0));
    assert!(reply_is_current(7, 7));
}

#[test]
fn reply_from_a_departed_site_is_rejected() {
    // The refbox has moved on since the request went out.
    assert!(!reply_is_current(0, 1));
    assert!(!reply_is_current(3, 9));
}

#[test]
fn a_stamp_from_the_future_is_also_rejected() {
    // Cannot happen today, but the rule is equality, not "not older than".
    // Anything but an exact match is data of uncertain origin, which this
    // guard exists to refuse.
    assert!(!reply_is_current(5, 2));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p refbox source_tap_tests 2>&1 | tail -20`
Expected: FAIL — `cannot find function 'reply_is_current' in this scope`

- [ ] **Step 3: Write minimal implementation**

Add the free function next to `reply_source` in `refbox/src/app/mod.rs` (~line 607):

```rust
/// Whether a site-scoped reply still belongs to the site the refbox is on.
///
/// Every request that goes to a *site* reads `site_generation` as it is issued
/// and carries that value on its reply; `repoint_client` bumps the counter when
/// it moves the client. Equal means the refbox has not moved since; anything
/// else means the answer came from a site it has left, and applying it would
/// attribute one site's data to another.
///
/// Why a counter and not the site address: an address would let
/// `portal -> custom -> portal` accept a reply issued on the first portal
/// visit, because the address matches again. That reply is *correct* data, so
/// an address is strictly more precise — but it costs a heap `String` on every
/// one of the dozens of messages an event fires, and the cost of rejecting it
/// is one wasted fetch (a fresh fetch always follows a switch), not a wrong
/// answer. Erring toward dropping is the right default for a guard whose whole
/// purpose is to refuse data of uncertain origin. This is a deliberate
/// trade-off, not an oversight.
fn reply_is_current(issued_at: u64, now: u64) -> bool {
    issued_at == now
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p refbox source_tap_tests 2>&1 | tail -20`
Expected: PASS, 3 tests

- [ ] **Step 5: Add the field and the bump**

In the `RefBoxApp` struct (~line 179, next to `current_site`):

```rust
    current_site: SiteTarget,
    /// Bumped every time the client is actually pointed at a different site.
    /// Site-scoped requests carry the value current when they were issued, and
    /// their handlers drop a reply whose stamp no longer matches — see
    /// [`reply_is_current`].
    site_generation: u64,
```

In the constructor (~line 3113, alongside `events: EventStore::default(),`):

```rust
            site_generation: 0,
```

In `repoint_client` (~line 1434), bump **only** beside the assignment:

```rust
        *shared.lock().unwrap() = new_client;
        self.current_site = target;
        // Only here. The two early returns above leave the client exactly where
        // it was, so replies in flight are still from the site the refbox is on
        // and must NOT be invalidated. (Same asymmetry `1f4bdc62` fixed for the
        // portal fetch; keep the two consistent.)
        self.site_generation = self.site_generation.wrapping_add(1);
```

- [ ] **Step 6: Verify it compiles clean**

Run: `cargo clippy -p refbox --all-targets -- -D warnings 2>&1 | tail -20`
Expected: no warnings. `site_generation` is written but not yet read — if clippy flags it as unused, that is expected until Task 2 and is the only acceptable point at which it may be so.

- [ ] **Step 7: Commit**

```bash
git add refbox/src/app/mod.rs
git commit -m "feat(refbox): count how many times the client has been repointed"
```

---

### Task 2: Stamp `RecvTokenValid`

Taken first because it is acceptance criterion 3 — the ACCESS TOKEN row painting a false "Connected" — which is the most visible of the five and the one that cost Eric time on 2026-08-28.

**Files:**
- Modify: `refbox/src/app/message.rs` (4 edits, see File Structure)
- Modify: `refbox/src/app/mod.rs` — `check_uwhportal_auth` (~line 1708), handler (~line 6037)

**Interfaces:**
- Consumes: `reply_is_current` and `self.site_generation` from Task 1.
- Produces: `RecvTokenValid(EventId, bool, u64)` — the `u64` is the site generation the check was issued under.

- [ ] **Step 1: Change the message variant**

In `refbox/src/app/message.rs`, replace the variant and extend its doc comment:

```rust
    /// Result of a portal token-validity check for a specific event. Carries
    /// the `EventId` it was checked for so a late reply for a previously
    /// selected event can be dropped instead of overwriting the current one,
    /// and the site generation it was issued under so a reply from a site the
    /// refbox has left cannot paint a verdict about the site it is on now.
    RecvTokenValid(EventId, bool, u64),
```

Then update the other three sites to the new arity:
- `is_repeatable`: `| Self::RecvTokenValid(_, _, _)`
- manual `eq`: `(Self::RecvTokenValid(a, b, c), Self::RecvTokenValid(d, e, f)) => a == d && b == e && c == f`
- fallthrough: `| (Self::RecvTokenValid(_, _, _), _)`

- [ ] **Step 2: Run the build to find every call site**

Run: `cargo build -p refbox 2>&1 | grep -E '^error' -A 4 | head -40`
Expected: errors at `check_uwhportal_auth` (3 construction sites) and the handler arm. Use this list — do not hunt by hand.

- [ ] **Step 3: Stamp the issuer**

In `check_uwhportal_auth` (`refbox/src/app/mod.rs` ~line 1708), read the generation once at issue time:

```rust
    fn check_uwhportal_auth(&self, event_id: &EventId) -> Task<Message> {
        if let Some(client) = &self.uwhportal_client {
            // The site this check goes out against, carried on the reply. Read
            // here, at the moment of issue, because that is the only point at
            // which the answer is known.
            let issued_at = self.site_generation;
            // why this cannot panic: the guard is held only for a synchronous
            // `has_token()` call and dropped immediately.
            let has_token = client.lock().unwrap().has_token();
            if !has_token {
                // Never ask a site to vouch for a credential we do not hold.
                // Only the site can enforce a token, and a permissive one
                // answers an unauthenticated probe with `200` — which arrives
                // as a green "Connected" painted over nothing. Report the
                // rejected state here instead, without sending the request.
                return Task::done(Message::RecvTokenValid(event_id.clone(), false, issued_at));
            }
```

and in the async block, both arms become `Message::RecvTokenValid(event_id, true, issued_at)` and `Message::RecvTokenValid(event_id, false, issued_at)`.

- [ ] **Step 4: Guard the handler**

Replace the handler arm (~line 6037):

```rust
            Message::RecvTokenValid(event_id, valid, issued_at) => {
                if !reply_is_current(issued_at, self.site_generation) {
                    warn!(
                        "Discarding the token verdict for event {}: it was checked \
                         against site generation {}, and the refbox is now on {}",
                        event_id.full(),
                        issued_at,
                        self.site_generation
                    );
                    return Task::none();
                }
                if let Some(ref mut settings) = self.edited_settings {
                    // Drop a stale reply for an event the operator has since
                    // switched away from, so a late "valid" for a previous
                    // event can't paint a false OK for the current one. The
                    // schedule and auto-court paths already guard on event id.
                    if settings.current_event_id.as_ref() == Some(&event_id) {
                        settings.uwhportal_token_valid = Some(valid);
                    }
                }
                Task::none()
            }
```

- [ ] **Step 5: Verify**

Run: `cargo clippy -p refbox --all-targets -- -D warnings 2>&1 | tail -20`
Expected: clean.

- [ ] **Step 6: Commit**

```bash
git add refbox/src/app/message.rs refbox/src/app/mod.rs
git commit -m "fix(refbox): drop a token verdict from the site the refbox has left"
```

---

### Task 3: Stamp `RecvSchedule`

**Files:**
- Modify: `refbox/src/app/message.rs` (4 edits)
- Modify: `refbox/src/app/mod.rs` — `request_schedule` (~line 1245), handler (~line 5814)

**Interfaces:**
- Consumes: `reply_is_current`, `self.site_generation`.
- Produces: `RecvSchedule(EventId, Schedule, u64)`.

- [ ] **Step 1: Change the variant**

```rust
    /// An event's schedule. Carries the site generation it was fetched under so
    /// a reply from a site the refbox has left cannot fill the court and game
    /// pickers of the site it is on now.
    RecvSchedule(EventId, Schedule, u64),
```

Update `is_repeatable` (`| Self::RecvSchedule(_, _, _)`), the manual `eq` (`(Self::RecvSchedule(a, b, c), Self::RecvSchedule(d, e, f)) => a == d && b == e && c == f`), and the fallthrough (`| (Self::RecvSchedule(_, _, _), _)`).

- [ ] **Step 2: Run the build to find every call site**

Run: `cargo build -p refbox 2>&1 | grep -E '^error' -A 4 | head -40`
Expected: `request_schedule` and the handler arm.

- [ ] **Step 3: Stamp the issuer**

In `request_schedule`, immediately inside the `if let Some(client)`:

```rust
            // The site this request goes out against, carried on the reply.
            // Read at the moment of issue — the only point at which it is known.
            let issued_at = self.site_generation;
```

and the tail of the async block becomes:

```rust
                info!("Got schedule");
                Message::RecvSchedule(event_id, schedule, issued_at)
```

- [ ] **Step 4: Guard the handler**

The guard goes **first in the arm, before the REFRESH spinner is cleared** — a schedule from a departed site must not be treated as the refresh completing:

```rust
            Message::RecvSchedule(event_id, mut schedule, issued_at) => {
                if !reply_is_current(issued_at, self.site_generation) {
                    warn!(
                        "Discarding the schedule for event {}: it was fetched from \
                         site generation {}, and the refbox is now on {}",
                        event_id.full(),
                        issued_at,
                        self.site_generation
                    );
                    return Task::none();
                }
                // A manual REFRESH (RequestPortalRefresh) spins the Game Info
                // button until a schedule arrives. Clear it for every success
                // path here, not just the between-games branch below.
                if let AppState::GameDetailsPage(ref mut is_refreshing) = self.app_state {
                    *is_refreshing = false;
                }
```

- [ ] **Step 4b: Delete this handler's KNOWN GAP paragraph**

There is a second edit in this arm, well below the head — at `mod.rs:5878`, immediately above `let source = self.reply_source();`. Delete the bare `//` separator line and the whole block from `// KNOWN GAP, deliberately not closed here.` down to and including `// own branch.` — that is exactly what this task closes.

**Keep** the paragraph above it ("Resolves against the COMMITTED source, not the staged one: ...") — it is still true and still load-bearing.

The identical paragraph also appears at `mod.rs:5763` in `RecvTeamsList`; that copy is Task 4's to remove. **Two copies, word for word — removing one and not the other is precisely the "fixing three of four" failure the spec names as its top risk.**

- [ ] **Step 5: Verify**

Run: `cargo clippy -p refbox --all-targets -- -D warnings 2>&1 | tail -20`
Expected: clean.

- [ ] **Step 6: Commit**

```bash
git add refbox/src/app/message.rs refbox/src/app/mod.rs
git commit -m "fix(refbox): drop a schedule from the site the refbox has left"
```

---

### Task 4: Stamp `RecvTeamsList`

**Files:**
- Modify: `refbox/src/app/message.rs` (4 edits)
- Modify: `refbox/src/app/mod.rs` — `request_teams_list` (~line 1162), handler (~line 5757)

**Interfaces:**
- Consumes: `reply_is_current`, `self.site_generation`.
- Produces: `RecvTeamsList(EventId, TeamList, u64)`.

- [ ] **Step 1: Change the variant**

```rust
    /// The teams entered in an event. Carries the site generation it was
    /// fetched under so a reply from a site the refbox has left cannot be
    /// filed against the site it is on now.
    RecvTeamsList(EventId, TeamList, u64),
```

Update `is_repeatable`, the manual `eq`, and the fallthrough to the new arity exactly as in Task 3.

- [ ] **Step 2: Run the build to find every call site**

Run: `cargo build -p refbox 2>&1 | grep -E '^error' -A 4 | head -40`

- [ ] **Step 3: Stamp the issuer**

In `request_teams_list`, immediately inside the `if let Some(client)`:

```rust
            // The site this request goes out against, carried on the reply.
            let issued_at = self.site_generation;
```

and the success arm becomes `Message::RecvTeamsList(event_id, teams, issued_at)`.

- [ ] **Step 4: Guard the handler, and delete the KNOWN GAP comment**

This arm's KNOWN GAP paragraph is at `mod.rs:5763` and is word-for-word the copy Task 3 removed from `RecvSchedule`. Delete the bare `//` separator and everything from `// KNOWN GAP, deliberately not closed here.` through `// own branch.` — **keeping** the "Resolves against the COMMITTED source" paragraph above it — and add the guard at the arm's head:

```rust
            Message::RecvTeamsList(event_id, teams, issued_at) => {
                if !reply_is_current(issued_at, self.site_generation) {
                    warn!(
                        "Discarding the teams list for event {}: it was fetched from \
                         site generation {}, and the refbox is now on {}",
                        event_id.full(),
                        issued_at,
                        self.site_generation
                    );
                    return Task::none();
                }
                // Resolves against the COMMITTED source, not the staged one:
                // staging alone never moves the client, so a reply arriving
                // after a merely staged source change still belongs to the
                // committed one.
                let source = self.reply_source();
```

Leave the `if let Some(event) = ...` body below it untouched.

- [ ] **Step 5: Verify**

Run: `cargo clippy -p refbox --all-targets -- -D warnings 2>&1 | tail -20`
Expected: clean.

- [ ] **Step 6: Commit**

```bash
git add refbox/src/app/message.rs refbox/src/app/mod.rs
git commit -m "fix(refbox): drop a teams list from the site the refbox has left"
```

---

### Task 5: Replace `RecvTeamRoster`'s source tag with the stamp

This is the collapse the spec calls for: two mechanisms become one, and it closes the custom-site-to-custom-site case (acceptance criterion 4) that a `GameSource` tag structurally cannot reach, because the source does not change.

**Files:**
- Modify: `refbox/src/app/message.rs` (4 edits — arity stays 3, but the first field's *type* changes)
- Modify: `refbox/src/app/mod.rs` — `request_team_roster` (~line 1190), handler (~line 5790)

**Interfaces:**
- Consumes: `reply_is_current`, `self.site_generation`.
- Produces: `RecvTeamRoster(TeamId, Vec<u8>, u64)` — **`GameSource` is removed.**

- [ ] **Step 1: Change the variant**

```rust
    /// A team's roster arrived from the site in use, reduced to the cap numbers
    /// on it. Players with no cap number are dropped at the fetch — there is
    /// nothing to tap for them.
    ///
    /// Tagged with the site generation the request went out under. The tag is
    /// what lets the handler drop a reply that lands after a source switch: the
    /// roster cache is keyed by team id alone, and a team id is whatever text
    /// the serving site chose to send. This used to be a `GameSource`, which
    /// could not tell one custom site from another — the generation can.
    RecvTeamRoster(TeamId, Vec<u8>, u64),
```

Update `is_repeatable` (arity unchanged: `| Self::RecvTeamRoster(_, _, _)`), the manual `eq` (unchanged shape), and the fallthrough (unchanged shape). Only the declaration's types move — **the other three sites need no edit for this variant**, which is exactly why Step 2 matters.

- [ ] **Step 2: Run the build to find every call site**

Run: `cargo build -p refbox 2>&1 | grep -E '^error' -A 4 | head -40`
Expected: type errors at `request_team_roster` and the handler. Because the arity did not change, the compiler is the only thing that will catch a missed reorder — read every error rather than assuming two.

- [ ] **Step 3: Stamp the issuer, dropping the source read**

In `request_team_roster`, replace the `let source = self.source;` block:

```rust
            // The site this request goes out against, carried on the reply so
            // the handler can tell a roster from the site the refbox is on now
            // from one still arriving from the site it has left. Read here, at
            // the moment the request is issued, because that is the only point
            // at which the answer is known. This was a `GameSource` until the
            // reply-origin work: a source tag cannot distinguish one custom
            // site from another, and rosters are exactly where that bites,
            // because the cache is keyed by team id alone.
            let issued_at = self.site_generation;
```

and the success arm becomes `Message::RecvTeamRoster(team_id, numbers, issued_at)`.

- [ ] **Step 4: Guard the handler**

```rust
            Message::RecvTeamRoster(team_id, numbers, issued_at) => {
                // Roster fetches go out in a batch, one per team in the
                // schedule, so a switch made while they are in flight would
                // otherwise let the departed site's replies refill the cache
                // `switch_to_source` has just cleared. `RecvSchedule` skips
                // re-fetching any team it already holds, so such an entry would
                // then shadow the new site's numbers for the rest of the
                // session and survive a REFRESH.
                if reply_is_current(issued_at, self.site_generation) {
                    self.team_rosters.insert(team_id, numbers);
                } else {
                    warn!(
                        "Discarding the roster for team {}: it was fetched from site \
                         generation {}, and the refbox is now on {}",
                        team_id.full(),
                        issued_at,
                        self.site_generation
                    );
                }
                Task::none()
            }
```

- [ ] **Step 5: Confirm `GameSource` is gone from this path**

Run: `grep -n 'RecvTeamRoster' refbox/src/app/mod.rs refbox/src/app/message.rs`
Expected: no occurrence mentions `GameSource` or `source`. If `use` of `GameSource` is now unused anywhere, clippy will say so in Step 6 — do not pre-emptively delete the import, it is used elsewhere.

- [ ] **Step 6: Verify**

Run: `cargo clippy -p refbox --all-targets -- -D warnings 2>&1 | tail -20`
Expected: clean.

- [ ] **Step 7: Commit**

```bash
git add refbox/src/app/message.rs refbox/src/app/mod.rs
git commit -m "fix(refbox): tag rosters with the site rather than the source"
```

---

### Task 6: Prove the guard, record the exclusion, and run the full gate

**Files:**
- Modify: `refbox/src/app/mod.rs` — `request_event_list` doc comment (~line 1105)

**Interfaces:**
- Consumes: everything from Tasks 1-5. Produces nothing.

- [ ] **Step 1: Mutation-test the guard — this is the step that proves the tests are real**

A check never seen failing is not a check. Temporarily invert the free function:

```rust
fn reply_is_current(issued_at: u64, now: u64) -> bool {
    issued_at != now   // DELIBERATELY WRONG — revert after this step
}
```

Run: `cargo test -p refbox source_tap_tests 2>&1 | tail -20`
Expected: **FAIL**. If any test still passes, that test is not asserting what it claims — fix the test, not the code.

Then revert the inversion and re-run:

Run: `cargo test -p refbox source_tap_tests 2>&1 | tail -20`
Expected: PASS, 3 tests.

- [ ] **Step 2: Record why `RecvEventList` is deliberately unstamped**

Append to the `request_event_list` doc comment (~line 1105), so the exclusion survives the next reader:

```rust
    /// Deliberately NOT stamped with `site_generation`, unlike every other
    /// reply in this group. This fetch does not use the live client at all — it
    /// builds its own against the portal, so its answer is portal data whatever
    /// source the refbox is committed to, and `set_portal_list` files it in the
    /// portal bucket either way. Stamping it would drop a perfectly good event
    /// list whenever a source switch happened while it was in flight, emptying
    /// the event picker in a way that looks like a network fault — and would
    /// undo `297ff166`, which made the list load regardless of source on
    /// purpose. The guard belongs on replies whose meaning depends on which
    /// site answered; this one's does not.
```

- [ ] **Step 3: Grep-audit that no site-scoped reply was left unstamped**

This is the spec's stated mitigation for its own top risk ("Four handlers, one rule. The failure mode is fixing three.").

Run:
```bash
grep -n 'Recv.*(EventId\|Recv.*(TeamId\|Recv.*(GameSource' refbox/src/app/message.rs
grep -c 'if !reply_is_current\|if reply_is_current' refbox/src/app/mod.rs
```
Expected: every `Recv*` variant carrying an `EventId` or `TeamId` ends in `, u64)`, and the **production guard count is exactly 4** — one per stamped handler. A count of 3 means a handler was missed.

**Count the guards, not the identifier.** A bare `grep -c 'reply_is_current'` returns 11, not 4: one doc-comment mention on the struct field, one definition, four guards, and five assertions inside the unit tests. Counting the identifier makes this audit unreadable and hides exactly the miss it exists to catch. `RecvTeamRoster`'s guard uses the positive form (`if reply_is_current(...)`, if/else) while the other three use `if !reply_is_current(...)` with an early return, which is why the pattern matches both.

Then confirm both stale gap notes are gone:

```bash
grep -c 'KNOWN GAP' refbox/src/app/mod.rs
```
Expected: **0**. A count of 1 means Task 3 or Task 4 removed only its own copy.

- [ ] **Step 4: Run the full gate**

Run: `just check`
Expected: fmt, lint, tests, audit all clean.

Note for the executor: `just check` is **host-only** — it does not prove the Windows or macOS builds. Those are CI's job. Do not report cross-platform success from a green local run.

- [ ] **Step 5: Commit**

```bash
git add refbox/src/app/mod.rs
git commit -m "docs(refbox): record why the event list is not site-stamped"
```

---

## Acceptance criteria — walkthrough, not automated

**Be honest about coverage.** The unit tests cover `reply_is_current` only. `RefBoxApp` has no test harness in this crate — every `#[cfg(test)]` module in `app/mod.rs` tests free functions, and the app is only ever built by its real startup path. **The four handler guards and the `repoint_client` bump are not reachable by any automated test in this repo.** They are proved by walkthrough or not at all.

All five need two sites configured with **identical event numbering** — see `reference_local_mock_portal_recipe` and `reference_overlay_test_server_mock_portal`. Each fails before the change:

1. Switch source while a **schedule** fetch is in flight. The departed site's schedule must not appear in the new site's court or game pickers.
2. Same for the **team list**.
3. With a portal **token check** in flight, switch to a custom site **with no saved access key**. The ACCESS TOKEN row must not read "Connected". *(For whoever writes this up: the green is a real verification against the portal, not a fabricated one — the fault is attribution, not invention. Getting that wrong in a report cost two rounds of Eric's time on 2026-08-28.)*
4. Switch from one custom site to a **different** custom site with a colliding team id, rosters in flight. The departed site's cap numbers must not seed the player-number grid. **This is the case the old `GameSource` tag could not reach.**
5. A repoint that **fails** (`build_site_client` returns `None`) must not invalidate replies in flight — the client did not move, so pickers must still fill.

Criterion 5 is the counter-test for Risk 2 ("dropping too much"). Do not skip it: if the bump were placed above the early returns, 1-4 would all still pass and only 5 would catch it.

## Explicitly out of scope

- **The persisted results queue.** `ItemId` is `{ event_id, game_number }` with no site at all (`portal_manager/mod.rs:306`). Same root cause, higher stakes (a result posted to the wrong server), and it needs a `portal_queue.json` format migration plus a decision about items already on disk. It is Eric's separate queued-results backlog item. **Do not fold it in.**
- Anything about *which* site is correct to talk to. This only ensures an answer is attributed to the site that gave it.
- `uwh-common`, `overlay`, `wireless-remote`.

## Deviations

Record here if execution diverges from this plan; do not create standalone deviation commits (`.claude/rules/plan-execution.md`).

- **Pre-execution:** `RecvEventList` is excluded from stamping — four handlers, not five. Raised as a spec/code conflict, **ruled by Eric on 2026-08-31** on domain grounds: an event list only ever comes from the Portal, because a custom site names its event in the URL and serves only court/game lists. See the DEVIATION section at the top.
