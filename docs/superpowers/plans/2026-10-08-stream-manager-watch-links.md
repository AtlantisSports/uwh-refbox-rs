# Stream Manager watch links Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Stream Manager links itself to a portal event with a one-time code, then fills each
prepared game's watch link on the portal schedule and clears it again when test videos are
deleted.

**Architecture:** One new module, `stream-manager/src/watch_links.rs`, holds everything that
talks to the portal for this: the saved key file, the code exchange and the watch-link update.
Prepare (control page and command line) and Delete test videos call it after they finish. The
control page gets two endpoints and one Settings card.

**Tech Stack:** Rust 2024 (MSRV 1.85), tokio, axum 0.8, reqwest 0.12, serde. No new dependencies.

**Spec:** uwhportal `docs/superpowers/specs/2026-10-08-stream-manager-watch-links-design.md`,
Part 2 (worktree `~/projects/uwh-portal-stream-watch-links`). Decision record: ADR 026,
Amendment 9 (`docs/decisions/026-per-game-youtube-streams.md`).

**Branch:** `feat/workspace/stream-manager-watch-links`, stacked on `feat/workspace/stream-manager`
(#3521, at 1f09f000). #3521 itself is not changed.

**Process:** lean (`.claude/rules/plan-execution.md`). Stream Manager is not one of the heavy
crates. Code review runs once, at the end (`code-review` skill), plus the human walkthrough
(`pr-review.md`). Deviations go in a section at the bottom of this file, and the ADR changes
once, in Task 3.

## Global Constraints

- No portal email or password is stored anywhere (ADR 026 Amendment 9).
- The portal endpoints, exactly as the uwhportal PR builds them:
  - `POST {portal}/api/events/{event_slug}/access-keys/stream-manager`, no sign-in.
    - Body: `{"streamManagerId": "<id>", "code": "<code>"}`.
    - 200 `{"accessKey": "<key>"}`.
    - 400 `{"reason": "NoPendingLink"}` or `{"reason": "InvalidCode"}`.
    - Bare 400 when the event is unknown.
  - `PUT {portal}/api/events/{event_slug}/schedule/watch-urls`, header
    `Authorization: Bearer <key>`.
    - Body: `{"watchUrlsByGameNumber": {"14": "https://youtu.be/<id>", "15": null}}`.
    - 204 on success.
    - 401/403 when the key is no good (removed on the portal, or expired after the event).
    - 404 for an unknown event or schedule.
    - 400 for an unknown game or a non-YouTube link. One bad game refuses the **whole** request.
- The link form is `https://youtu.be/{broadcast_id}`, the form the chat messages use already
  (`live.rs:671`).
- The Stream Manager ID is a random 6-digit number (100000–999999), made once and saved in the
  settings. It never changes.
- The key is a secret. It never appears in a log line, an error message or the settings JSON.
- Prepare and Delete test videos never fail because of the portal: a portal problem is one log
  line, and the job's own result is unchanged. Links are never re-sent automatically.
- Practice mode does not stop the links.
- The Live/Dev portal choice is unchanged (PO, 2026-10-08).
- Words on the page and in the log are plain English for a tournament volunteer (see the exact
  strings in each task).
- `cargo fmt --all`, `cargo clippy --workspace --all-targets --all-features -- -D warnings` and
  `cargo test -p stream-manager` pass. No `unwrap()`/`expect()` outside tests without a comment
  saying why it can't panic.

## Review Focus

1. **A game that has a video but is no longer on the portal schedule.** One unknown game makes
   the portal refuse the whole update, so every other game would lose its link. Only send games
   that are in the schedule as loaded. Tested in Task 2.
2. **Delete test videos stopping half-way** (a YouTube error). The games already deleted must
   still have their links cleared. Tested in Task 2.
3. **The event or portal changed after linking.** A key for event A must never be sent for
   event B. Each send checks that the saved key's portal and event match the current settings,
   and changing either deletes the key file. Tested in Tasks 1 and 3.
4. **A key removed on the portal** (401/403). It is forgotten and the log says what to do, and
   the next Prepare doesn't try again with it. Tested in Task 2.
5. **A broken or hand-edited key file.** It reads as "not linked", never as a crash or a failed
   Prepare. Tested in Task 1.

---

### Task 1: The ID, the saved key and the portal calls

**Files:**
- Create: `stream-manager/src/watch_links.rs`
- Modify: `stream-manager/src/main.rs` (`mod watch_links;`, and `load_config` makes the ID)
- Modify: `stream-manager/src/config.rs` (`stream_manager_id` field, default empty)
- Modify: `stream-manager/src/access.rs` (`new_stream_manager_id()`)
- Modify: `stream-manager/src/app.rs` (`App::link_file()`)

**Interfaces (produced, for Tasks 2–3):**

```rust
// config.rs: next to button_key, with a doc comment.
/// This Stream Manager's ID for linking to a portal event (6 digits), made on first start.
/// Not a secret: the code the portal shows for it is.
#[serde(default)]
pub stream_manager_id: String,

// access.rs
/// A new Stream Manager ID: 6 random digits, 100000–999999.
pub fn new_stream_manager_id() -> Result<String, getrandom::Error>;

// watch_links.rs
pub const LINK_FILE: &str = "portal-watch-links.json";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortalLink {
    pub portal_url: String,
    pub event_slug: String,
    pub access_key: String,
}

/// The saved link, if there is one for this portal and event. A missing, unreadable or
/// unparsable file, or one for another portal or event, is `None`.
pub fn link_for(path: &Path, portal_url: &str, event_slug: &str) -> Option<PortalLink>;
/// Saves the link with `prepare::write_atomically`.
pub fn save(path: &Path, link: &PortalLink) -> Result<(), BoxError>;
/// Deletes the file; a file that isn't there is fine.
pub fn forget(path: &Path) -> Result<(), BoxError>;

pub enum LinkError { NoPendingLink, InvalidCode, Other(String) }
/// Swaps the portal's code for a key.
pub async fn exchange_code(
    portal_url: &str, event_slug: &str, stream_manager_id: &str, code: &str,
) -> Result<String, LinkError>;

pub enum SendError { KeyRefused, Other(String) }
/// One PUT of `links` (game number → link, `None` clears it).
pub async fn send_watch_urls(
    portal_url: &str, event_slug: &str, access_key: &str,
    links: &BTreeMap<String, Option<String>>,
) -> Result<(), SendError>;

pub fn watch_url(broadcast_id: &str) -> String; // "https://youtu.be/{broadcast_id}"

// app.rs
pub fn link_file(&self) -> PathBuf; // self.config_dir.join(watch_links::LINK_FILE)
```

**Notes:**

- **The ID.** `load_config` makes it when it is empty, exactly like `button_key`: save it with
  `app::save_config`, then log `info!("Created the Stream Manager ID {id}")`. Use
  `getrandom::fill` on 4 bytes, `u32::from_le_bytes`, then `100_000 + n % 900_000`. The tiny
  modulo bias doesn't matter for an ID.
- `settings_from` (web.rs) already keeps every field the page doesn't send (`..current.clone()`),
  so a settings save keeps the ID. Add `stream_manager_id` to the
  `a_settings_save_keeps_the_key_and_pin_in_use` test.
- **Requests.** Use `crate::http_client()` (it has the timeout), `trim_end_matches('/')` on the
  portal address, and reqwest's `.bearer_auth(key)` for the key. Read a 400's reason from
  `body["reason"]`, the way `uwh-common/src/uwhportal/mod.rs` `login_to_portal` does for the
  refbox (the API sends camelCase).
- **Error text.** `LinkError::Other` and `SendError::Other` carry text a person can read:
  `"Couldn't reach the portal: {e}"`, `"The portal said {status}: {first 200 chars of body}"`.
  They never include the key.
- `send_watch_urls` with an empty map returns `Ok(())` and sends nothing.

**Tests** (in `watch_links.rs`, `#[cfg(test)]`). Add a small mock portal: an axum `Router` on
`127.0.0.1:0`, as `web.rs` tests do. It records each request's path, `Authorization` header and
JSON body in an `Arc<Mutex<Vec<…>>>`, and answers with a status and body the test chooses.

- [ ] `exchange_code` returns the key on 200, and sends `streamManagerId` and `code` to
  `/api/events/cup-2026/access-keys/stream-manager`.
- [ ] It maps 400 `NoPendingLink` and 400 `InvalidCode` to those variants. A bare 400 and a
  500 give `Other`.
- [ ] `send_watch_urls` sends `Bearer <key>` and the exact body, with `null` for a cleared
  game. 204 gives `Ok`, 401 and 403 give `KeyRefused`, and 400/404/500 give `Other`.
- [ ] An empty map sends nothing (the mock saw no request).
- [ ] `link_for`:
  - a saved link reads back for its own portal and event;
  - another event or another portal gives `None`;
  - a missing file, a file holding `not json`, and a file holding `{}` all give `None`
    (Review Focus 3 and 5).
- [ ] `forget` on a missing file is `Ok`.
- [ ] `new_stream_manager_id` is always 6 digits (run it 1,000 times) and two calls differ.
- [ ] A config with no `stream_manager_id` loads with it empty (the serde default).
  `load_config` on a fresh folder saves a 6-digit ID, and loading again keeps the same one.
- [ ] Run `cargo test -p stream-manager watch_links access config`. Then run `cargo fmt --all`
  and clippy. Expected: pass, no warnings.

---

### Task 2: Fill links after Prepare, clear them on Delete test videos

**Files:**
- Modify: `stream-manager/src/watch_links.rs` (`publish`)
- Modify: `stream-manager/src/prepare.rs` (`cleanup` reports what it deleted; `cleanup_cli`)
- Modify: `stream-manager/src/web.rs` (`prepare_run`, `cleanup`)
- Modify: `stream-manager/src/main.rs` (CLI `Prepare` and `Cleanup`)

**Interfaces:**

```rust
// watch_links.rs
/// What to send after Prepare: every recorded video whose game is on `plan`, as its link.
/// Games no longer on the schedule are left out (the portal would refuse the whole update).
pub fn links_after_prepare(plan: &EventPlan, state: &EventState) -> BTreeMap<String, Option<String>>;
/// What to send after deleting videos: `None` for each deleted game that is on `plan`.
pub fn links_after_cleanup(plan: &EventPlan, deleted: &[String]) -> BTreeMap<String, Option<String>>;

/// Sends `links` with the saved key for `config`'s portal and event, and logs one line:
/// - not linked:  "Portal watch links: this Stream Manager isn't linked to the event, so none were set (Settings → Portal watch links)"
/// - nothing to send: no line, no request
/// - success:     "Portal: watch links set for N games"  /  "Portal: watch links cleared for N games"
/// - key refused: forgets the key file, then
///                "Portal: the event refused this Stream Manager's key (it was removed on the portal, or the event is over). Link it again in Settings to set watch links."
/// - other error: "Portal: watch links weren't set: {error}. Run Prepare again to send them."
///                (cleanup: "...weren't cleared: {error}")
pub async fn publish(
    link_file: &Path, config: &Config, links: &BTreeMap<String, Option<String>>,
    purpose: Purpose, log: &mut (dyn FnMut(String) + Send),
);
pub enum Purpose { Set, Clear }

// prepare.rs: the new cleanup signature. `deleted` gets each game number as its video is
// deleted, so a caller still knows them when cleanup stops on an error.
pub async fn cleanup(
    youtube: &mut YouTube, state_file: &Path, event_slug: &str,
    deleted: &mut Vec<String>, log: &mut (dyn FnMut(String) + Send),
) -> Result<(), BoxError>;
```

**Notes:**

- **`prepare_run` (web.rs).** After `prepare::run(...)` returns, whether it worked or failed
  (videos made before a failure still deserve links):
  1. reload the state file;
  2. build `links_after_prepare(&plan, &state)` with the plan the job used;
  3. `publish(&app.link_file(), &config, …, Purpose::Set, &mut log)`;
  4. then hand the run's own result to `end_job` as now.

  If reloading the state file fails, log `Portal: watch links weren't set: {e}` and carry on.
- **`cleanup` (web.rs).** Make `let mut deleted = Vec::new()` before the job's async block and
  pass it in. Afterwards, whatever the result, clear the links: use `app.plan()` if it is
  loaded, otherwise fetch it with `portal::fetch_event_plan`. If neither works, log
  `Portal: watch links weren't cleared: the schedule couldn't be loaded ({e})`. Do this before
  `end_job`, and outside the `lock_all_courts` guard (drop the guard first, since the portal
  call doesn't need it).
- **Command line.**
  - `CliCommand::Prepare`: after `run_cli` returns `Ok`, publish the same way, logging with
    `info!`. A cancelled run (the user typed no) changes nothing, but publishing after it is
    harmless and keeps it simple.
  - `CliCommand::Cleanup`: `cleanup_cli` gains `config: &Config` and `link_file: &Path`, fetches
    the plan only when something was deleted, and publishes with `Purpose::Clear`.
  - The link file path is `config_dir.join(watch_links::LINK_FILE)`, beside `token_file`.
- Don't hold a court lock or `RECORD_LOCK` across a portal call.

**Tests:**

- [ ] `links_after_prepare`:
  - two recorded games on the plan give two `youtu.be` links;
  - a recorded game that isn't on the plan is left out (Review Focus 1).
- [ ] `links_after_cleanup` leaves out a deleted game that isn't on the plan.
- [ ] `publish` against the mock portal from Task 1:
  - linked, 204: the right body is sent, and the log says `watch links set for 2 games`;
  - not linked (no file): no request, and the "isn't linked" line;
  - a link file for another event: no request, the "isn't linked" line, and the file is kept
    (Review Focus 3);
  - 401: the file is deleted and the "refused" line is logged. A second `publish` sends
    nothing (Review Focus 4);
  - 500: the file is kept, and the "weren't set … Run Prepare again" line is logged;
  - an empty map: no request, no line.
- [ ] `cleanup` keeps what it deleted when it stops half-way (Review Focus 2). With a fake
  YouTube this needs the existing test seam in `prepare.rs`; if there is none, test
  `links_after_cleanup` plus a unit test showing `deleted` is pushed before the state update,
  and record the gap under Deviations.
- [ ] Run `cargo test -p stream-manager`, fmt and clippy. Expected: pass, no warnings.

---

### Task 3: Settings card, link and unlink, and the docs

**Files:**
- Modify: `stream-manager/src/web.rs` (routes `POST`/`DELETE /api/portal-link`; `get_settings`)
- Modify: `stream-manager/src/app.rs` (`update_settings` forgets the key when the event or portal changes)
- Modify: `stream-manager/web/index.html` (the card and its script)
- Modify: `docs/streaming-setup.md`
- Modify: `docs/decisions/026-per-game-youtube-streams.md`

**Behaviour:**

- `GET /api/settings` adds `"portal_linked": true|false` at the top level, next to `is_local`.
  It is true when `link_for(link_file, portal_url, event_slug)` is `Some`. `settings` already
  carries `stream_manager_id`, because it is part of `Config`. The key is never in the reply.
- `POST /api/portal-link` takes body `{"code": "482197"}`.
  - Trim the code. It must be 6 digits, otherwise 400 `Enter the 6-digit code the portal
    showed`.
  - No event saved: 400 `Choose and save an event first`.
  - Otherwise call `exchange_code` with the settings in use and save the `PortalLink`.
  - Replies:
    - success: `{"ok": true}`, and log `info!("Linked to the portal for {slug}")`;
    - `NoPendingLink`: 400 `The portal isn't expecting this Stream Manager. On the event's
      Manage Event → Stream management tab, add Stream Manager ID {id}, then type the code it
      shows.`;
    - `InvalidCode`: 400 `That code isn't right, or it has expired. Codes last 15 minutes;
      add the Stream Manager again on the portal for a new one.`;
    - `Other(e)`: 400 `Couldn't link: {e}`.
- `DELETE /api/portal-link` forgets the key and replies `{"ok": true}`. It doesn't touch the
  portal: the organiser removes the Stream Manager from the portal's list there.
- Both are signed-in endpoints, like the others. They go through `authorize`, and other devices
  may use them, since the code is what makes linking safe.
- In `App::update_settings`, after `save_config` succeeds and `event_or_portal_changed` is true,
  call `watch_links::forget(&self.link_file())`. A failure there is only a `warn!`, because
  `link_for` already refuses a key for another event.

**The card** goes in `index.html` straight after the Event card, in the same column, with the
id `card-portal-link`. It uses the page's existing `card`, `field`, `row`, `muted` and `small`
classes and its `api()` helper. The words are exactly these:

```
 ┌ Portal watch links ────────────────────────┐
 │ Stream Manager ID:  482917                 │
 │ Code  [ ______ ]  [ Link ]                 │
 └────────────────────────────────────────────┘
          linked:
 ┌ Portal watch links ────────────────────────┐
 │ Linked ✓ — each game's link goes on the    │
 │ portal after Prepare.          [ Unlink ]  │
 └────────────────────────────────────────────┘
```

- **No event saved:** under the ID, in muted text, `Choose and save an event first.`, and Link
  is disabled.
- **Link:** POSTs the code. On success it reloads the settings. On an error it shows the
  message under the box in the page's error style (as other cards do).
- **Unlink:** `confirm('Unlink from the portal? Watch links stop being set until you link
  again.')`, then DELETE and reload the settings.
- `loadSettings()` renders the card from `settings.stream_manager_id`, `settings.event_slug` and
  `r.portal_linked`. Escape everything with `esc()`.

**Docs:**

- **`docs/streaming-setup.md`:**
  - B5, Settings tab: a step for linking. On the portal, open Manage Event → Stream management
    → (+), enter this Stream Manager's ID (shown in the Portal watch links box), and type the
    code it shows into the box.
  - Files table: add `portal-watch-links.json`, "the portal key for setting watch links (secret)".
  - C1 **Check:** each prepared game's watch link shows on the portal schedule.
  - Troubleshooting:
    - "The log says the event refused this Stream Manager's key": link it again.
    - "Links weren't set": run Prepare again.
- **ADR 026:** on the "Follow-up" line in Scope and in Amendment 9, say the follow-up is built in
  this PR (stacked on #3521) together with the uwhportal PR. Add a Deviations bullet for anything
  that changed from Amendment 9 during building, or "none".

**Tests** (in `web.rs`; mock portal from Task 1; the test config's `portal_url` is the mock's
address, set directly in `Config` because `App::new` doesn't run `validate_for_save`):

- [ ] Settings shows a 6-digit `stream_manager_id` and `portal_linked: false`. It contains no
  `access_key`.
- [ ] Linking:
  - link with a good code: `portal_linked` becomes true, and the settings reply still has no
    key;
  - a mock `InvalidCode` gives the "isn't right" message;
  - `NoPendingLink` gives the message with the ID in it;
  - `12ab` is refused without calling the portal.
- [ ] Unlink: `portal_linked` becomes false and the file is gone.
- [ ] Saving settings with another `event_slug` deletes the link file, and `portal_linked` is
  false. A save that changes only the privacy keeps it (Review Focus 3).
- [ ] Run `cargo test -p stream-manager` and `just check`. Expected: pass, no warnings.
- [ ] Open the page by hand once (`cargo run -p stream-manager -- --no-browser`, browse to
  `http://127.0.0.1:8090`). Check that the card shows the ID and the no-event note. Point the
  portal at nothing; this only checks the layout.

---

## After the tasks (controller)

1. `code-review` skill on the whole branch diff against `feat/workspace/stream-manager`
   (mandatory check 1).
2. Human walkthrough steps for the PO: link against the dev portal once the uwhportal PR is on
   dev, Prepare, see the links on the schedule, Delete test videos, see them gone.
3. Draft PR stacked on #3521. Push only on the PO's OK. Its description says it needs the
   uwhportal PR live first.

## Deviations

- **Lint scope.** Clippy ran as `cargo clippy -p stream-manager --all-targets --all-features`, plus
  `just check`. The workspace-wide `--all-targets` form fails on errors already in the `refbox`
  crate that no gate runs. Added: `cargo check -p stream-manager --all-targets --target
  x86_64-pc-windows-gnu`, clean.
- **Task 1.** The mock portal is `watch_links::test_portal` (`#[cfg(test)] pub(crate)`), so the
  `web.rs` tests in Task 3 can use it.
- **Task 2, cleanup wording.** When clearing fails, the log says `Portal: watch links weren't
  cleared: {error}` with no "Run Prepare again".
- **Task 2, half-way cleanup (Review Focus 2).** There is no fake YouTube, so this uses the
  plan's fallback: a `note_deleted` test and a `links_after_cleanup` test. A YouTube error
  half-way is checked by reading the code only.
- **Task 3.** Three messages the plan didn't give:
  - `Couldn't link: {e}` when the key can't be saved;
  - `Couldn't unlink: {e}`;
  - the log line `Unlinked from the portal`.

  The plan's open-the-page-by-hand check moved to the human walkthrough.
- **Final code review (10 findings, all fixed):**
  - Prepare and Delete test videos on the command line share one publishing path with the
    page. The command-line Prepare now sends links after a failure too.
  - A cancelled command-line Prepare sends nothing. The plan had called sending after a cancel
    harmless; that is reversed.
  - Page jobs use the state file and schedule of the job's own event.
  - A refused key is deleted only if it is still the saved one.
  - A key whose exchange outlived an event or portal change isn't saved. The reply is `The
    event or portal changed while linking. Link again.`
  - Link refuses while the event or portal on the page isn't saved. The message is `Save
    settings first: the event or portal on this page isn't saved yet.` The portal is compared
    only when the dropdown shows one.
  - Link and Unlink redraw only their own box. The plan had said to reload the whole Settings
    form.
  - The Link button is disabled while its request runs.
  - Clearing while not linked has its own log line: `…so the deleted games' links weren't
    cleared (Settings → Portal watch links)`.
  - A reply body that can't be read is reported as `Couldn't reach the portal`.
