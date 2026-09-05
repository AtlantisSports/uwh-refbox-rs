# Court-Finished Behaviour — Second Rebase & Mandatory Re-Check Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this
> plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking. Subagent-driven execution
> is NOT recommended here — see "Why inline" below.

**Goal:** Rebase `fix/refbox/court-finished-behaviour` (36 commits) onto `origin/master`
(+108 commits), resolving the semantic conflicts master's `EventStore` and court-commit work
creates, then re-run the two mandatory pre-PR checks the rebase stales.

**Architecture:** The conflict surface is only three files. Master left
`refbox/src/tournament_manager/mod.rs` (648 of our changed lines), all of `uwh-common`,
`link_session.rs`, `game_info_table.rs` and `main_view.rs` completely untouched. The real work is
`refbox/src/app/mod.rs`, where master rewrote event storage from a flat
`Option<BTreeMap<EventId, Event>>` into a per-source `EventStore`, and moved when the court is
committed. Our 951 changed lines in that file must be ported onto the new API, not merely
de-conflicted.

**Tech Stack:** Rust 2024, MSRV 1.85, iced 0.13, `just` task runner.

**Spec:** `docs/superpowers/plans/2026-08-17-court-finished-behaviour.md`
**Ledger (15 rulings):** `.superpowers/sdd/2026-08-17-court-finished-behaviour/progress.md`
**Walkthrough results:** `docs/backlog/court-finished-panel-state/WALKTHROUGH-RESULTS-2026-08-31.md`

**Why inline, not subagent-driven:** every conflict below is a judgement call that needs the
branch's design intent in context, and two known traps make delegation actively unsafe here —
review/diff skills take their target from the session cwd rather than the worktree, and a
background or sandbox-off Bash run resets cwd and can build the wrong branch while reporting green.

## Global Constraints

- MSRV Rust 1.85; edition 2024. No APIs newer than 1.85.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` must be clean.
- No new `unwrap()`/`expect()` in production code without a comment proving it cannot panic.
- No new dependencies. No files touched outside the 32 already in this branch's diff.
- Do NOT fix the three pre-existing bugs found by the walkthrough — they are separate issues.
- Do NOT delete `.superpowers/sdd/2026-08-17-court-finished-behaviour/` (Ruling 15).
- Never `git stash` bare — this checkout's stash stack is shared with ~30 other worktrees and
  peer Claude sessions.
- Branch, commit, push, PR: all require Eric's explicit approval.

## Baseline facts (measured 2026-09-04, not assumed)

| Fact | Value |
|---|---|
| Our HEAD | `336267da`, 36 commits, tracked tree clean |
| Merge base | `56db10ef` |
| We are ahead | 36 commits |
| Master is ahead | **108** commits (handover said 105 — it moved) |
| Existing backup ref | `backup/pre-rebase-court-finished-20260829` = `3a66a785` |
| Master commits on `app/mod.rs` | 20 (+1769 lines) |
| Master commits on `shared_elements.rs` | 2 (+107) |
| Master commits on `configuration.rs` | 4 (+376) |
| Master commits on our other 29 files | **0** |

---

## Task 1: Safety net and baseline

**Files:** none modified.

- [ ] **Step 1: Take a dated backup ref for THIS rebase**

The existing backup is from the first rebase on 2026-08-29 and points at a pre-first-rebase tree.
It is not a safety net for this attempt.

```bash
git branch backup/pre-rebase-court-finished-20260904 HEAD
git rev-parse --short backup/pre-rebase-court-finished-20260904   # expect 336267da
```

- [ ] **Step 2: Confirm no peer session is mid-operation on this worktree**

```bash
git status --porcelain=v1 | grep -v '^??'   # expect NO output
ls .git/rebase-merge .git/rebase-apply 2>&1 # expect "No such file or directory" for both
```

- [ ] **Step 3: Record the pre-rebase test baseline so a regression is provable**

```bash
just test 2>&1 | tail -20 | tee /tmp/claude-1000/pre-rebase-tests.txt
```
Expected: 708 tests pass. Write the exact number down — Task 5 compares against it.

- [ ] **Step 4: Confirm the mutation-proven criterion-9 test exists and passes now**

```bash
cargo test -p refbox test_reset_to_manual_break_starts_the_break_when_schedule_linked
```
Expected: 1 passed. This is the only automated proof of criterion 9; if the rebase breaks it,
criterion 9 has no coverage at all.

- [ ] **Step 5: No commit.** Nothing changed yet.

---

## Task 2: Run the rebase and triage every conflict

**Files:** expect conflicts only in `refbox/src/app/mod.rs`,
`refbox/src/app/view_builders/shared_elements.rs`,
`refbox/src/app/view_builders/configuration.rs`.

- [ ] **Step 1: Start the rebase**

```bash
git rebase origin/master
```

- [ ] **Step 2: On each stop, triage before editing**

For every conflicted stop, run this and read it before touching the file:

```bash
git status --short | grep -E '^(UU|AA|DU|UD)'
git log --oneline -1 REBASE_HEAD          # which of OUR 36 commits is being replayed
```

Rule: resolve to **our behavioural intent expressed in master's structures**. Never resolve by
keeping both sides — the first rebase proved that trap when master renamed
`make_button` -> `make_chrome_button` and deleted the old name, so keeping both sides would not
compile. Neither name survives in either tree now; expect the equivalent to recur.

- [ ] **Step 3: If a conflict is in a file master never touched, STOP and re-read**

Master touched only the three files above. A conflict anywhere else means the rebase is replaying
onto something unexpected — abort and re-measure rather than resolving it:

```bash
git rebase --abort
```

- [ ] **Step 4: Do not commit yet.** Task 3 and 4 finish the semantics.

---

## Task 3: Port our 10 `self.events` touchpoints onto `EventStore`

**Files:**
- Modify: `refbox/src/app/mod.rs`
- Read for API: `refbox/src/app/event_store.rs` (new on master)

**Interfaces:**
- Consumes: master's `EventStore`. Confirmed call shapes in master's `mod.rs`:
  `self.events.get_mut(source, &event_id)`, `self.events.set_portal_list(e_map)`,
  `self.events.owns(source, id)`, `self.events.portal_list_loaded()`,
  `self.events.adopt_custom(Event { .. })`, `EventStore::selectable(..)`, `EventStore::default()`.
- Produces: nothing new. This task changes no behaviour by design — it re-expresses our reads and
  writes in master's API.

**Why this is not a textual merge:** ours holds `events: Option<BTreeMap<EventId, Event>>`;
master holds `events: EventStore`, keyed per `GameSource` and not `Option`-wrapped. Every one of
our touchpoints below has to choose a `GameSource`, and choosing wrong silently reads or writes
another site's data — the exact class of bug master's last four commits exist to fix.

- [ ] **Step 1: Read the new API before editing anything**

```bash
sed -n '1,120p' refbox/src/app/event_store.rs
grep -n 'fn ' refbox/src/app/event_store.rs
```

- [ ] **Step 2: Enumerate our surviving touchpoints after the rebase**

```bash
grep -n 'self\.events' refbox/src/app/mod.rs
```
Pre-rebase these were at ours-lines 210 (doc comment), 1410, 2916 (comment), 5327, 5348, 5428,
6382 (comment), 6397, 6516, 6538.

- [ ] **Step 3: Port each write site, choosing the source explicitly**

The two structural rewrites required:

```rust
// OURS (flat, Option-wrapped) — line ~1410:
let events = self.events.get_or_insert_with(BTreeMap::new);

// BECOMES (per-source; a custom site is adopted, not inserted into the portal list):
self.events.adopt_custom(Event { /* fields as master's line ~1622 builds them */ });
```

```rust
// OURS — lines ~5348 and ~5428:
if let Some(ref mut events) = self.events {

// BECOMES — resolve against the COMMITTED source, per master's own comment at its line ~5890:
let source = self.reply_source();
if let Some(event) = self.events.get_mut(source, &event_id) {
```

Use `self.reply_source()` — not the staged source — for anything reacting to a reply. Master's
comment states why: staging alone never moves the client, so a reply arriving after a merely
staged source change still belongs to the committed one.

- [ ] **Step 4: Port the three read sites**

Ours-lines 6397, 6516, 6538 pass `self.events.as_ref()`. Master passes `&self.events`
(its lines 7093, 7119). Drop the `as_ref()` and the `Option` handling that surrounds it.

- [ ] **Step 5: Fix the three stale doc comments, do not delete them**

Ours-lines 210, 2916, 6382 describe the old flat map by name. Each states a real ordering
constraint that still holds; only the mechanism name changed. Reword to name `EventStore`.
Line 210's claim — "the schedule arrives after `self.events` is populated" — must be re-checked
against master's restructure, not just renamed.

- [ ] **Step 6: Compile only this crate for a fast loop**

```bash
cargo check -p refbox 2>&1 | tail -30
```
Expected: no errors. Warnings are handled in Task 5.

---

## Task 4: Re-verify the anchor-clearing order against master's commit points

**Files:** Modify: `refbox/src/app/mod.rs`

**This is the highest-risk task in the plan and it cannot be caught by any test we have.**

`clear_anchor_if_event_or_court_changing` works by comparing the *current* committed values with
the *incoming* ones:

```rust
if self.current_event_id != *new_event_id || self.current_court != *new_court {
    self.last_played = None;
    self.last_played_start = None;
}
```

Master now commits the court inside the confirmed-apply path
(`3b1c706e fix(refbox): commit the court when an apply is confirmed`, its line ~1468
`self.set_current_event_id(event_id); self.current_court = court;`). If the rebase lands any such
commit-write **above** one of our four call sites, the comparison reads new-against-new, is always
false, and the anchor is never cleared. That compiles, lints, and passes every test we have while
doing nothing — the same failure shape as `end_game` not being the end-of-game seam.

- [ ] **Step 1: Locate all four call sites after the rebase**

```bash
grep -n 'clear_anchor_if_event_or_court_changing' refbox/src/app/mod.rs
```
Pre-rebase: definition at 1767, call sites at 1442, 1841, 2061, 2110.

- [ ] **Step 2: For each call site, prove the ordering holds**

For each of the four, print the surrounding 40 lines and answer in writing:

```bash
for L in $(grep -n 'self\.clear_anchor_if_event_or_court_changing' refbox/src/app/mod.rs | cut -d: -f1); do
  echo "=== call site line $L ==="
  sed -n "$((L-30)),$((L+10))p" refbox/src/app/mod.rs
done
```

For each site record: **is there any write to `self.current_event_id` (directly or via
`set_current_event_id`) or `self.current_court` between the start of this handler and the call?**
If yes, the call must move above that write, or take the pre-write values as arguments.

- [ ] **Step 3: Write a failing test for the one site the walkthrough exercised**

Criterion 4 (refresh-adoption) reaches the site at ours-line 1442
(`court_after_adopt`). Add a test asserting the anchor is cleared when adoption changes the court.
Mutate the *code* to prove the test can fail — never adjust the fixture to make it pass.

```bash
cargo test -p refbox anchor_cleared -- --nocapture
```

- [ ] **Step 4: Commit the port and the ordering fix**

```bash
git add refbox/src/app/mod.rs
git commit -m "fix(refbox): port the court-finished work onto the per-source event store"
```

---

## Task 5: Prove the tree is green on the rebased base

- [ ] **Step 1: Finish the rebase if it is still in progress**

```bash
git rebase --continue
git rev-list --count origin/master..HEAD    # expect 36 (or 37 with Task 4's commit)
git status --porcelain=v1 | grep -v '^??'   # expect no output
```

- [ ] **Step 2: Run the full gate**

```bash
just check 2>&1 | tail -40
```
Expected: fmt, lint, tests, audit all clean.

Three known ways this gate lies, all of which apply here:
- `just check` is **host-only** — a Windows-target break is invisible locally.
- `just lint` is **not** `--all-targets`; the strict form trips a pre-existing `player_grid.rs`
  error that CI does not hit, so a failure there is not necessarily ours.
- `just audit` can go red with no code change when the advisory DB moves.

- [ ] **Step 3: Compare against the Task 1 baseline**

Test count must be >= 708 (plus Task 4's new test). A *lower* count means tests were lost in
conflict resolution, not that they passed.

- [ ] **Step 4: Re-run the criterion-9 regression test specifically**

```bash
cargo test -p refbox test_reset_to_manual_break_starts_the_break_when_schedule_linked
```

- [ ] **Step 5: Confirm what the rebase actually kept**

```bash
git diff --stat origin/master...HEAD | tail -5
```
Compare to the pre-rebase 32 files / +5551 / -95. A large drop in insertions means work was lost.

---

## Task 6: Mandatory check 1 — automated code review of the rebased diff

Per `.claude/rules/pr-review.md` a rebase stales this check; it must be re-run, not carried over.

- [ ] **Step 1: Run the built-in `code-review` skill with an EXPLICIT target**

Do not run it bare. Two known traps: the skill diffs against a **stale local master**, and review
skills take their diff from the **session cwd**, not this worktree — and passing the path alone
does not fix that. Give it an explicit branch target of `origin/master...HEAD` and confirm from
its output that the file list matches Task 5 Step 5 before trusting a single finding.

- [ ] **Step 2: Triage every finding — fix or answer, none left silent**

- [ ] **Step 3: Note for the disclosure**

There is no `citizen-review` and no project review skill in this repo — that is uwh-portal's.
The built-in `code-review` skill plus `just check` is the whole of leg 1 here. No vendored or
third-party code is in this diff, so `security-review` is not triggered.

---

## Task 7: Mandatory check 2 — walkthrough with Eric (his leg, not mine)

Agreed scope is **not all ten criteria** — only **2, 4, 5 and 9**: the two restart Criticals,
refresh-adoption, and 9 which has never been walked. ~15 minutes.

I write numbered steps; Eric clicks and reports back. I never perform this leg myself, and my own
driving of the app would be leg 3, which cannot be done on this machine anyway — there is no
screen-capture tool here.

- [ ] **Step 1: Rebuild the binary first** — the walkthrough rig points at `./target/debug/refbox`
      and a stale binary silently walks the old code.

```bash
cargo build -p refbox 2>&1 | tail -5
```

- [ ] **Step 2: Start the mock portal on 8100** (8099 belongs to another session — leave it alone)

```bash
cd docs/backlog/court-finished-panel-state/mock-portal && python3 server.py 8100
```

- [ ] **Step 3: Seed `portal_link.json` only while refbox is stopped**, and set
      `token = "walkthrough-test-key"` under `[uwhportal]` or REFRESH greys out.

- [ ] **Step 4: Launch with an isolated config so the real portal link is never touched**

```bash
XDG_CONFIG_HOME=$SP/cfg UWH_PORTAL_URL_OVERRIDE=http://127.0.0.1:8100 WAYLAND_DISPLAY= \
  ./target/debug/refbox --allow-http --no-simulate --json-port 8010 --binary-port 8011
```

Without `UWH_PORTAL_URL_OVERRIDE` the app hits production and wipes `portal_link.json`.

- [ ] **Step 5: Walk criterion 9 from the GAME page, not the App page** — the App-page route
      leaves numbering at last+1 rather than 1, so it proves the wrong thing.

- [ ] **Step 6: Criterion 9 needs Eric's decision first.** It cannot start while WSL audio is
      dead (`snd_pcm_open ... Connection refused`, exit 101). The fix is `wsl --shutdown` from
      Windows, which kills every WSL session including peer Claude sessions. That is Eric's call.

- [ ] **Step 7: Record results in
      `docs/backlog/court-finished-panel-state/WALKTHROUGH-RESULTS-2026-08-31.md`**, one scenario
      at a time — report a unit, then wait, rather than batching all four.

---

## Task 8: Reconcile the docs the rebase invalidated

**Files:**
- Modify: `docs/backlog/court-finished-panel-state/NOTE.md`
- Modify: `.superpowers/sdd/2026-08-17-court-finished-behaviour/progress.md` (Deviations section)
- Modify: the bug-(a) issue draft under `.claude/issue-drafts/`

- [ ] **Step 1: Re-word bug (a) — the bug survives, its mechanism moved**

Master did **not** fix it: `portal_list_loaded()` only selects which error message is logged, and
the schedule is still dropped with no retry. But our stated mechanism is now stale. The draft
must describe master's shape — the schedule handler nested inside
`if let Some(event) = self.events.get_mut(source, &event_id)` — or a reader will look for code
that no longer exists.

- [ ] **Step 2: Confirm bugs (b) and (c) are unchanged by the rebase**

(b) refresh-adopted game not written to the note for ~5 min (heartbeat only);
(c) no audio device panics the whole refbox. Neither is in master's 108 commits' territory —
verify rather than assume, then leave both unfixed.

- [ ] **Step 3: Record the deviation, do not create a standalone deviation commit**

Per `.claude/rules/plan-execution.md`, deviations go in the plan's Deviations section or the PR
body — not `docs(workspace): record Task N deviations` commits.

- [ ] **Step 4: Commit the doc reconciliation**

```bash
git add docs/backlog/court-finished-panel-state/NOTE.md
git commit -m "docs(refbox): re-anchor the walkthrough notes onto the per-source event store"
```

---

## Task 9: Report, and surface the three decisions owed

- [ ] **Step 1: Do not propose the PR until all three decisions are ruled**

1. **Rescheduled anchor:** an already-played game moved later can make a live court read
   "finished". The safer failure direction was chosen; the spec does not settle which clock wins.
2. **Custom sites** now need a game picked after every restart — their notes are never read back.
3. Should changing court/game **during a running countdown** warn? It currently does not.

A relayed ruling is not a decision — if any of these arrives second-hand, mark it UNRESOLVED
and ask Eric directly.

- [ ] **Step 2: Disclose all three pre-PR checks, unprompted**

Leg 1 automated review: state when it ran and what it found. Leg 2 walkthrough: Eric's, with the
criterion-9 blocker named. Leg 3 Claude-driven: **cannot be done on this machine** — no
screen-capture tool — so it is "not done", with that as the reason, and Eric's screenshots are
leg 2 evidence, never leg 3. A green `just check` is not evidence for any of the three.

## Deviations

**1. Master was 108 commits ahead, not the 105 in the handover.** It moved again while the branch
sat. Merge base `56db10ef`.

**2. The conflict surface was much narrower than feared, and Task 3 was largely subsumed.** Master
touched only 3 of our 32 files. `refbox/src/tournament_manager/mod.rs` (648 of our changed lines),
all of `uwh-common`, `link_session.rs`, `game_info_table.rs` and `main_view.rs` replayed untouched.
Task 3's premise — porting our ten `self.events` touchpoints onto `EventStore` — turned out not to
apply: all ten sat in code master had itself rewritten, and our surviving block reads
`self.schedule` / `self.current_event_id`, never the event store. The source is resolved by
master's own `reply_source()`. No per-source decision fell to us.

**3. Anchor clearing moved into the commit funnel rather than four call sites.** Master
consolidated every "commit source + event + court + schedule" sequence into
`commit_link_selection`, documented as the one funnel every APPLY path goes through. The clearing
went inside it, before the writes. This was checked, not assumed, against the accusation of scope
creep: at the original base `apply_game_confirmation` only ever did
`set_current_event_id(None)`, so the two extra funnel sites are paths **master added**. Covering
only the four our commit knew about would have shipped a hole in the branch's own stated invariant
— a stale anchor making a fresh court read "finished". `adopt_custom_event` keeps its own separate
call, as our commit intended; it bypasses the funnel.

`switch_to_source` also bypasses the funnel and does not clear. Left alone deliberately: it nulls
the event id, nothing reads the anchor with no committed event, and every path that re-commits an
event or court goes through one of the two clearing sites.

**4. Stale `.unwrap()` calls were dropped as encountered, not left to commit 33.** Master's
`SharedGame::lock()` returns the guard directly, so `.unwrap()` does not compile. Our commit
`c8e24687` exists to remove these; where a resolution would otherwise not have compiled the unwrap
and its poison-safety comment were dropped in place. Commit 33 still had four sites left to do.

**5. A 37th commit was added: `fix(refbox): drop the font-family helper the rebase re-added`.**
Reconciling the no-next-game commit swept in `font_family_id`, which master had deleted when it
centralised font selection into `app/languages.rs`. Unreferenced in the binary build, so `just
lint` failed `-D warnings` on it — while `cargo check --all-targets` passed, because a test still
names it. That is the known `just lint` / `--all-targets` divergence. An `--autosquash` into the
originating commit conflicted against later commits, so it landed as a tip commit instead. A scan
for other symbols in this class (defined at the old base, deleted by master, still present here)
found none.

**6. Task 4 Step 3's test could not be written, and the gap is worse than the plan assumed.**
Deleting the anchor-clearing call outright leaves all 764 refbox tests green — proven by mutation,
not inferred. No test constructs a `RefBoxApp` (53 fields, does I/O in `new`), and every test
module in `app/mod.rs` tests extracted pure functions instead. Adding app-level test
infrastructure is well outside a rebase's scope, so this is reported as a finding rather than
fixed. Extracting the predicate to a pure function would not have caught this mutation either. The
only real cover is walkthrough criteria 4 and 10.

**7. Two rebase attempts were aborted before the third succeeded.** Attempt 1: a hardcoded
closing-brace count produced a syntax error at step 27, and — my error — I staged and continued
past the failing compile, baking it into two commits. Attempt 2: a whole-file resolver silently
dropped step 10's entire `mod.rs` hunk (the manual-switch guard) while still compiling perfectly —
the "green but does nothing" class. Attempt 3 fixed the method and added the gate that mattered:
after every replayed commit, its changed-file list is compared against the original's, so a
dropped hunk cannot pass unnoticed. That gate then caught a genuine (benign) difference at step 9
and one real problem, and both were read rather than waved through.

**8. Intermediate commits 29-32 do not compile.** They carry `.unwrap()` calls that commit 33
removes. This is the pre-existing shape of the branch — commit 33 was written during the *first*
rebase for exactly this reason — and not something this rebase introduced. Final tree is green.

**9. Verification numbers.** Baseline before the rebase: 861 workspace tests passing, 4 ignored
(the handover's "708" is the refbox unit-test binary alone, which matched exactly). After:
`just check` exit 0, **1180 passing, 0 failing, 5 ignored**; the rise reflects master's own added
tests. Diff against master: 32 files, +5559/-95, versus the pre-rebase +5551/-95 — the 8 extra
lines are the funnel comment in deviation 3. Formatting clean. Criterion 9's regression test
passes. The plan's `--exact` test invocations were wrong (that flag needs the fully-qualified
module path) and were corrected in place.

---

## Deviations — 2026-09-04 (walkthrough session)

- **Branch renamed** to `fix/uwh-common/court-finished-behaviour` (review finding 12), with Eric's
  yes. Free: nothing pushed, no PR. The worktree directory and `.superpowers/sdd/` path keep the
  old name; harmless.
- **Task 7 partially done.** Criteria **2, 4 and 5 walked and PASSED**; criterion **9 NOT walked**.
  Full evidence: `docs/backlog/court-finished-panel-state/WALKTHROUGH-RESULTS-2026-09-04.md`.
- **Step 6 of task 7 is settled as a finding, not a pending decision.** The WSL audio panic has
  **no workaround** — tested: an `ALSA_CONFIG_PATH` null-device override of `default` does nothing
  (cpal opens `pulse` by name), overriding `pulse` too only postpones the same exit-101 panic, no
  sound daemon is installed to substitute, and audio init is not gated by config or CLI flag. Only
  `wsl --shutdown` (Eric's call; kills every WSL session) unblocks criterion 9.
- **Criterion 2 was walked more strictly than planned** — two shutdown kinds (SIGKILL and a tidy
  window close) rather than two tidy closes.
- **One attempt was voided and re-run**: `pgrep -f` matched the harness's bash wrapper, so a
  SIGTERM left the real app running and a second instance came up beside it. A later kill loop over
  `pgrep -x refbox` killed a peer session's refbox. Both recorded in the results doc; the rule
  adopted is to gate every signal on `readlink /proc/<pid>/exe`.
- **Docs to reconcile (task 8):** `WALKTHROUGH-2026-08-29.md` claims a restart onto a finished court
  "looks identical to finishing normally". It does not — the Prior Game row reads 0 with a blank
  score after a restart, because that row is session-scoped by design.

---

## Deviations — 2026-09-04 (rebase and third review)

- **Rebased onto master**, which had moved +22 since the branch's previous base. All 51 commits
  replayed with **no conflicts**. Backup taken first: `backup/pre-rebase-court-finished-20260904b`.
  `just check` green on the rebased branch: 1217 tests, 0 failures.
- **Third code review** run against the rebased diff at high effort. Three findings, all triaged
  below. No dependency or vendored-code changes in the diff, so `security-review` was not triggered.
- **Finding A — fixed, and it had two instances, not the one the review named.** The mid-break reset
  was not armed by `reset_to_manual_break`, so a court revived by switching the source off the portal
  held the finished game's score and penalties on screen for the whole break instead of clearing them
  `post_game_duration` before kickoff — the same defect already fixed for `apply_next_game_start`
  (finding 9). Checking the whole class rather than the reported case found `reset_for_site_switch`'s
  else branch with the identical hole. Both now start their break through a new `start_nominal_break`,
  which sets the clock and arms the reset in one place, so a future revive path cannot forget.
  Tests: `a_manual_break_after_a_finished_court_clears_the_old_score_early`,
  `a_site_switch_off_a_finished_court_clears_the_old_score_early`.
  **Note:** the first of those sits exactly on criterion 9's route, which has still never been walked.
- **Finding B — fixed. DELAY grew without bound on a finished court.** Parking a court left
  `next_scheduled_start` holding the Game Block slot the next game would have used. With the break
  clock parked at 0:00 the projected start is simply `now`, so the DELAY figure read the gap to that
  phantom slot and grew for as long as the refbox stayed open. Cleared in `set_no_next_game` and in
  `end_game`'s finished branch; `reset_for_site_switch`'s parked branch already cleared it via
  `clear_portal_next_game`. Test: `a_finished_court_does_not_show_a_growing_delay`.
- **Finding C — ruled out by Eric**, not a code change. Recorded in
  `REVIEW-FINDINGS-OPEN-2026-09-04.md` under "Raised by review and ruled out".
- All three fixes were written test-first; each test was watched failing for the right reason before
  the fix went in. `just check` green afterwards: **1220 tests, 0 failures**.

---

## Deviations — 2026-09-04 (peer rebase mid-walkthrough, and the re-walk)

- **A peer session rebased this branch onto `1229c396` at 21:08:07, mid-walkthrough.** Master had
  moved +8 (roster-before-kickoff merged). Nothing was lost and the branch's own diff stayed intact,
  but it staled the review and the walkthrough that had just been completed, and both had to be
  redone. `just check` on the rebased branch: 1228 tests, 0 failures.
- **A screen error that looked exactly like a product bug was an artefact of that rebase.** The
  refbox displayed `No localization for id: "schedule-end"` where END belongs. The key is defined in
  all 15 locales. In a debug build `rust-embed` reads the translation files from disk at runtime, and
  the app happened to read `en-US/refbox.ftl` while the rebase was replaying commits, catching it at
  a point in history before that key was added. Every run since, on a stable tree, shows zero
  occurrences — including a repeat of the same offline scenario that first produced it. The string
  itself comes from `i18n-embed-0.16.0/src/fluent.rs:186`, not from this project.
- **Criteria 2, 4, 5 and 9 re-walked and all PASSED** at `808b146b`, with HEAD verified unmoved
  before and after every step. Evidence in `WALKTHROUGH-RESULTS-2026-09-04.md`.
- **Criterion 9 walked for the first time in four sessions**, and it exercises the fix directly. The
  blocker recorded in the previous session — that refbox could not start at all because of WSL audio
  — no longer held; the app launched and ran normally, and no `wsl --shutdown` was needed.
