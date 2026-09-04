# Per-Event Access-Key Store Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** File every access key against the exact site and event that issued it, so re-selecting a previously-used event re-establishes the connection with no new login code.

**Architecture:** Replace the two single-key slots (`[uwhportal] token`, `[custom_site] token`) with one list of `(site, event, key)` entries in the settings file. The key is no longer chosen when the portal client is built — at startup the event is not yet known — but loaded by a single function whenever the event or the site changes, mirroring how `set_current_event_id` already owns "which event".

**Tech Stack:** Rust 2024, MSRV 1.85, `serde` + `toml` for the settings file, `iced` 0.13 for the UI (untouched by this branch).

**Spec:** the source-switch draft-model design — open question 2 and its "Sequencing and
delivery" section. This is branch 1 of two, and that spec describes branch 2, so it is
deliberately not carried here: it would put a document for unbuilt work into this PR's diff.
It is recoverable from `51f22f86` in this branch's history, and a working copy is kept
untracked in the main checkout under `docs/superpowers/specs/`.

## Global Constraints

- **Branch:** `feat/refbox/per-event-access-keys`, based on `master` **after PR #3082 merges**. #3082 is reviewed and walked; only the `cargo audit` failure holds it, fixed on `chore/deps/rtrb-advisory-fix`.
- **Keys are filed under `(site, event)`, never event alone.** Event ids collide between the Portal and a custom site by design, so filing by event alone could hand a custom site a Portal key.
- **`site` is the normalised base URL, no trailing slash** — byte-identical to `SiteTarget::base_url` (`refbox/src/app/mod.rs:424`). Never the display address, which for a custom site includes the event.
- **New format only.** Ruled by Eric 2026-09-01: nothing is written back into `[uwhportal] token` or `[custom_site] token` for the benefit of an older binary. Rolling back means one re-login.
- **No key expiry, no picker UI, no delete path.** A key silently vanishing is worse than a stale key sitting unused.
- **`[custom_site] url` stays.** It is an address, not a credential, and is still the saved custom site.
- **MSRV 1.85, Rust 2024, `-D warnings`.** No new dependencies. No `unwrap()`/`expect()` in production code without a comment explaining why it cannot panic.
- **Out of scope (branch 2):** the draft model itself, the source-button rework, the confirmation copy, the cascade.

---

## File Structure

| File | Responsibility in this branch |
|---|---|
| `refbox/src/config.rs` | New `AccessKey` type, `Config::access_keys` field, lookup/store helpers, forward migration. Legacy token fields become read-only. |
| `refbox/src/app/mod.rs` | `build_site_client` stops reading tokens; new `apply_access_key` owns loading; `set_current_event_id` and `repoint_client` call it; the login save writes to the store; startup adopts a legacy key. |

No new files. Both are large existing files following established patterns; this branch does not restructure them.

---

### Task 1: The `AccessKey` type and the store on `Config`

**Files:**
- Modify: `refbox/src/config.rs` (new type near `CustomSite` at :75-91; new field on `Config` at :330-355; helpers in `impl Config`)
- Test: `refbox/src/config.rs` (`#[cfg(test)] mod tests`, alongside `config_remembered_remote_round_trips` at :956)

**Interfaces:**
- Consumes: nothing from earlier tasks.
- Produces: `pub struct AccessKey { pub site: String, pub event: EventId, pub key: String }`; `Config::access_keys: Vec<AccessKey>`; `Config::access_key_for(&self, site: &str, event: &EventId) -> Option<&str>`; `Config::store_access_key(&mut self, site: &str, event: &EventId, key: String)`.

- [ ] **Step 1: Write the failing tests**

Add to the existing `mod tests` in `refbox/src/config.rs`:

```rust
fn ev(id: &str) -> EventId {
    EventId::from_full(format!("events/{id}")).unwrap()
}

#[test]
fn access_key_is_found_by_site_and_event() {
    let mut config = Config::default();
    config.store_access_key("https://api.uwhportal.com", &ev("abc"), "KEY-A".into());
    assert_eq!(
        config.access_key_for("https://api.uwhportal.com", &ev("abc")),
        Some("KEY-A")
    );
}

#[test]
fn access_key_is_not_shared_across_events_on_one_site() {
    let mut config = Config::default();
    config.store_access_key("https://api.uwhportal.com", &ev("abc"), "KEY-A".into());
    assert_eq!(
        config.access_key_for("https://api.uwhportal.com", &ev("xyz")),
        None
    );
}

#[test]
fn access_key_is_not_shared_across_sites_with_a_colliding_event_id() {
    // Event ids collide between the Portal and a custom site by design. A
    // Portal key must never be handed to somebody else's server.
    let mut config = Config::default();
    config.store_access_key("https://api.uwhportal.com", &ev("abc"), "PORTAL-KEY".into());
    assert_eq!(config.access_key_for("https://scores.example.org", &ev("abc")), None);
}

#[test]
fn storing_twice_for_one_site_and_event_replaces_rather_than_appends() {
    let mut config = Config::default();
    config.store_access_key("https://api.uwhportal.com", &ev("abc"), "OLD".into());
    config.store_access_key("https://api.uwhportal.com", &ev("abc"), "NEW".into());
    assert_eq!(config.access_keys.len(), 1);
    assert_eq!(
        config.access_key_for("https://api.uwhportal.com", &ev("abc")),
        Some("NEW")
    );
}

#[test]
fn access_keys_round_trip_through_the_settings_file() {
    let mut config = Config::default();
    config.store_access_key("https://api.uwhportal.com", &ev("abc"), "KEY-A".into());
    config.store_access_key("https://scores.example.org", &ev("abc"), "KEY-B".into());
    let text = toml::to_string(&config).unwrap();
    let parsed: Config = toml::from_str(&text).unwrap();
    assert_eq!(parsed.access_keys, config.access_keys);
}

#[test]
fn a_settings_file_with_no_access_keys_loads_with_an_empty_store() {
    let parsed: Config = toml::from_str(&config_toml_without("access_keys")).unwrap();
    assert!(parsed.access_keys.is_empty());
}
```

- [ ] **Step 2: Run the tests and verify they fail**

Run: `cargo test -p refbox --bin refbox config::tests 2>&1 | tail -20`
Expected: FAIL — `no method named store_access_key found for struct Config`.

- [ ] **Step 3: Add the type, the field and the helpers**

Add `use uwh_common::uwhportal::schedule::EventId;` to the imports if not already present. Add near `CustomSite`:

```rust
/// One saved access key, filed against the exact site and event that issued it.
///
/// `site` is the normalised base URL with no trailing slash — the same string
/// `SiteTarget::base_url` carries. Filing by event alone would be wrong: event
/// ids collide between the Portal and a custom site by design, so a Portal key
/// could be handed to somebody else's server.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccessKey {
    pub site: String,
    pub event: EventId,
    pub key: String,
}
```

Add the field to `Config`. **It must be the last field in the struct:** `toml` serialises an array of tables as `[[access_keys]]`, and every plain value has to be emitted before any table or the file it writes cannot be read back.

```rust
    #[serde(default)]
    pub access_keys: Vec<AccessKey>,
```

Add to `impl Config`:

```rust
    /// The key held for this exact site and event, if any. `None` means the
    /// operator has never logged in to this event on this site, or the key was
    /// replaced by a later login elsewhere.
    pub fn access_key_for(&self, site: &str, event: &EventId) -> Option<&str> {
        self.access_keys
            .iter()
            .find(|k| k.site == site && k.event == *event)
            .map(|k| k.key.as_str())
    }

    /// File a key against the site and event that issued it, replacing any key
    /// already held for that pair. Deliberately never removes anything else:
    /// keys for other events stay, which is the whole point of the store.
    pub fn store_access_key(&mut self, site: &str, event: &EventId, key: String) {
        match self
            .access_keys
            .iter_mut()
            .find(|k| k.site == site && k.event == *event)
        {
            Some(existing) => existing.key = key,
            None => self.access_keys.push(AccessKey {
                site: site.to_string(),
                event: event.clone(),
                key,
            }),
        }
    }
```

Add `access_keys` to the destructuring and the returned literal in `Config::migrate` (:359-469), reading it with the existing helper:

```rust
        get_serde_value(old, "access_keys", &mut access_keys);
```

- [ ] **Step 4: Run the tests and verify they pass**

Run: `cargo test -p refbox --bin refbox config::tests 2>&1 | tail -20`
Expected: PASS, and every pre-existing config test still passes.

- [ ] **Step 5: Commit**

```bash
git add refbox/src/config.rs
git commit -m "feat(refbox): add a per-event access-key store to the config"
```

---

### Task 2: Stop writing the legacy token slots

**Files:**
- Modify: `refbox/src/config.rs` (`UwhPortal` :62-73, `CustomSite` :78-91)
- Test: `refbox/src/config.rs` (`mod tests`)

**Interfaces:**
- Consumes: `Config::access_keys` from Task 1.
- Produces: `UwhPortal::token` and `CustomSite::token` still parse from an existing file but disappear from a freshly written one once empty. `CustomSite::url` is unchanged.

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn legacy_token_slots_are_read_but_never_written_back() {
    let old = r#"
        [uwhportal]
        token = "LEGACY-PORTAL"
        [custom_site]
        url = "https://scores.example.org"
        token = "LEGACY-CUSTOM"
    "#;
    let table: toml::Table = toml::from_str(old).unwrap();
    let config = Config::migrate(&table);
    // Still read, so Task 3 can adopt them.
    assert_eq!(config.uwhportal.token, "LEGACY-PORTAL");
    assert_eq!(config.custom_site.token, "LEGACY-CUSTOM");
    // The address is not a credential and stays.
    assert_eq!(config.custom_site.url, "https://scores.example.org");

    // Once adopted and blanked, they leave the file entirely.
    let mut adopted = config.clone();
    adopted.uwhportal.token.clear();
    adopted.custom_site.token.clear();
    let text = toml::to_string(&adopted).unwrap();
    assert!(!text.contains("LEGACY-PORTAL"));
    assert!(!text.contains("token"), "no empty token key should be written:\n{text}");
    assert!(text.contains("https://scores.example.org"));
}
```

- [ ] **Step 2: Run the test and verify it fails**

Run: `cargo test -p refbox --bin refbox legacy_token_slots 2>&1 | tail -20`
Expected: FAIL — the serialised text still contains `token = ""`.

- [ ] **Step 3: Make the legacy fields skip when empty**

```rust
pub struct UwhPortal {
    /// Legacy single-slot key. Read on load so a settings file written before
    /// the per-event store can be adopted once (see `adopt_legacy_access_key`),
    /// then blanked. Never written back: rolling back to an older refbox means
    /// logging in again, which Eric ruled acceptable on 2026-09-01.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub token: String,
}
```

Apply the identical attribute and a matching comment to `CustomSite::token`. Leave `CustomSite::url` exactly as it is.

- [ ] **Step 4: Run the tests and verify they pass**

Run: `cargo test -p refbox --bin refbox config 2>&1 | tail -20`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add refbox/src/config.rs
git commit -m "feat(refbox): stop writing the legacy single-key token slots"
```

---

### Task 3: One function owns which key is loaded

**Files:**
- Modify: `refbox/src/app/mod.rs` — `build_site_client` :541-556, `repoint_client` :1169-1195, `set_current_event_id` :1489-1515
- Test: `refbox/src/app/mod.rs` (`#[cfg(test)] mod tests`)

**Interfaces:**
- Consumes: `Config::access_key_for` from Task 1.
- Produces: `fn build_site_client(target: &SiteTarget, config: &Config) -> Option<UwhPortalClient>` — same signature, but it no longer reads any token. `RefBoxApp::apply_access_key(&mut self)` — loads the key for the current site and event into the live client, or clears it.

**Why not choose the key when the client is built:** at startup `current_event_id` is `None` (`app/mod.rs:2396-2399`) and is filled later by the link restore, so the event simply is not known at `build_site_client` time.

- [ ] **Step 1: Write the failing test**

Goes in the `#[cfg(test)] mod tests` block in `refbox/src/app/mod.rs` that holds the `decide_restore` tests. It needs its own `ev` helper: the one added in Task 1 lives in `config.rs`'s test module and is not visible from here.

```rust
fn ev(id: &str) -> EventId {
    EventId::from_full(format!("events/{id}")).unwrap()
}

#[test]
fn a_client_is_built_with_no_key_because_the_event_is_not_known_yet() {
    let mut config = Config::default();
    config.store_access_key("https://api.uwhportal.com", &ev("abc"), "KEY-A".into());
    let target = portal_target(Mode::Hockey6V6, false);
    let client = build_site_client(&target, &config).unwrap();
    assert!(
        !client.has_token(),
        "the event is unknown at build time, so no key can be chosen"
    );
}
```

- [ ] **Step 2: Run the test and verify it fails**

Run: `cargo test -p refbox --bin refbox a_client_is_built_with_no_key 2>&1 | tail -20`
Expected: FAIL — the client still carries `config.uwhportal.token`.

- [ ] **Step 3: Remove token selection from `build_site_client`**

Replace :542-546 with a single `None`, and record why in place of the old comment:

```rust
fn build_site_client(target: &SiteTarget, config: &Config) -> Option<UwhPortalClient> {
    // No key here on purpose. Keys are filed per (site, event) and the event is
    // not known when the client is built at startup, so `apply_access_key` is
    // the one place that decides which key is loaded — called whenever the
    // event or the site changes.
    let token = None;
```

`config` stays in the signature: `https_policy_conflict` and the rest of the body still use it.

- [ ] **Step 4: Add `apply_access_key` and call it from both places**

Add to `impl RefBoxApp`, next to `repoint_client`:

```rust
    /// Load the key for the site and event the refbox is currently on, or clear
    /// the client's key when none is held. The single owner of "which key is
    /// loaded", so the client can never be left holding a key belonging to a
    /// different event.
    fn apply_access_key(&mut self) {
        let Some(shared) = self.uwhportal_client.as_ref() else {
            return;
        };
        let key = self
            .current_event_id
            .as_ref()
            .and_then(|event| self.config.access_key_for(&self.current_site.base_url, event))
            .map(str::to_owned);
        // why this cannot panic: the guard is held only across the synchronous
        // set_token/clear_token calls below, neither of which panics.
        let mut guard = shared.lock().unwrap();
        match key {
            Some(key) => {
                if let Err(why) = guard.set_token(&key) {
                    // Only reachable from a hand-edited settings file: a key is
                    // refused before it is ever stored. Clear rather than keep,
                    // so no request goes out with a broken credential.
                    warn!("A saved access key cannot be sent and was not loaded: {why}");
                    guard.clear_token();
                }
            }
            None => guard.clear_token(),
        }
    }
```

Call it as the last statement of `repoint_client` (after `self.current_site = target;`) and as the last statement of `set_current_event_id`.

- [ ] **Step 5: Run the tests and verify they pass**

Run: `cargo test -p refbox --bin refbox 2>&1 | tail -20`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add refbox/src/app/mod.rs
git commit -m "feat(refbox): load the access key for the current site and event"
```

---

### Task 4: Save a newly-issued key against its site and event

**Files:**
- Modify: `refbox/src/app/mod.rs` :5177-5183 (the `PortalTokenResponse::Success` arm)
- Test: `refbox/src/config.rs` (`mod tests`) — the assertion is about the store, so it belongs with the store and uses the `ev` helper from Task 1.

**Interfaces:**
- Consumes: `Config::store_access_key` (Task 1), `apply_access_key` (Task 3).
- Produces: no new public names.

- [ ] **Step 1: Replace the kind-based slot write**

The existing `match self.current_site.kind { … }` at :5180-5183 writes to one of two slots. Replace it with a write to the store, keyed by the site the login actually went to and the event it was issued for:

```rust
                            // Save it against the exact site AND event that
                            // issued it. The key is only ever valid for that
                            // event — `login_to_portal` posts to
                            // /api/events/{event}/access-keys/ref-box — so
                            // filing it by site alone would hand the next event
                            // a key that cannot work.
                            match self.current_event_id.clone() {
                                Some(event) => {
                                    self.config.store_access_key(
                                        &self.current_site.base_url,
                                        &event,
                                        token,
                                    );
                                    self.apply_access_key();
                                }
                                None => {
                                    // Not reachable from the UI: the login is
                                    // only offered once an event is selected.
                                    // Dropping the key is the safe branch — a
                                    // key with no event cannot be filed, and
                                    // guessing an event could send it to one
                                    // that never issued it.
                                    warn!(
                                        "An access key arrived with no event selected; it was not saved"
                                    );
                                }
                            }
```

- [ ] **Step 2: Write the test**

```rust
#[test]
fn a_new_key_is_filed_against_the_site_and_event_that_issued_it() {
    let mut config = Config::default();
    config.store_access_key("https://api.uwhportal.com", &ev("first"), "KEY-1".into());
    config.store_access_key("https://api.uwhportal.com", &ev("second"), "KEY-2".into());
    // Logging in to the second event must not disturb the first.
    assert_eq!(
        config.access_key_for("https://api.uwhportal.com", &ev("first")),
        Some("KEY-1")
    );
    assert_eq!(
        config.access_key_for("https://api.uwhportal.com", &ev("second")),
        Some("KEY-2")
    );
}
```

- [ ] **Step 3: Run the tests and verify they pass**

Run: `cargo test -p refbox --bin refbox 2>&1 | tail -20`
Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add refbox/src/app/mod.rs refbox/src/config.rs
git commit -m "feat(refbox): file a new access key against its site and event"
```

---

### Task 5: Adopt an existing key on first launch after the upgrade

**Files:**
- Modify: `refbox/src/app/mod.rs` — the startup link restore at :2580-2585 (`link_session::load_or_none`, then `decide_restore`)
- Test: `refbox/src/app/mod.rs` (`#[cfg(test)] mod tests`, the block holding the `decide_restore` tests). These cannot live in `config.rs`: `adopt_legacy_access_key` and `SiteKind` are both defined in `app/mod.rs`. Reuse the `ev` helper added in Task 3.

**Interfaces:**
- Consumes: `Config::store_access_key` (Task 1), the blanked legacy fields (Task 2).
- Produces: `fn adopt_legacy_access_key(config: &mut Config, site: &SiteTarget, event: &EventId) -> bool` — returns whether anything was adopted, so the caller knows to save the config.

**Why this exists:** without it, everyone who upgrades is sent back to the Portal website for a fresh code even though they hold a perfectly good key. The event to attribute it to comes from `portal_link.json`, which records the event the refbox is currently linked to.

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn a_legacy_portal_key_is_adopted_for_the_linked_event_and_then_blanked() {
    let mut config = Config::default();
    config.uwhportal.token = "LEGACY-PORTAL".into();
    let adopted = adopt_legacy_access_key(
        &mut config,
        SiteKind::Portal,
        "https://api.uwhportal.com",
        &ev("abc"),
    );
    assert!(adopted);
    assert_eq!(
        config.access_key_for("https://api.uwhportal.com", &ev("abc")),
        Some("LEGACY-PORTAL")
    );
    assert!(config.uwhportal.token.is_empty());
}

#[test]
fn adopting_does_not_overwrite_a_key_already_in_the_store() {
    let mut config = Config::default();
    config.store_access_key("https://api.uwhportal.com", &ev("abc"), "CURRENT".into());
    config.uwhportal.token = "LEGACY-PORTAL".into();
    let adopted = adopt_legacy_access_key(
        &mut config,
        SiteKind::Portal,
        "https://api.uwhportal.com",
        &ev("abc"),
    );
    assert!(adopted, "the legacy slot is still cleared");
    assert_eq!(
        config.access_key_for("https://api.uwhportal.com", &ev("abc")),
        Some("CURRENT"),
        "the store wins: it was written by this version and is known-good"
    );
    assert!(config.uwhportal.token.is_empty());
}

#[test]
fn nothing_is_adopted_when_the_legacy_slot_is_empty() {
    let mut config = Config::default();
    assert!(!adopt_legacy_access_key(
        &mut config,
        SiteKind::Portal,
        "https://api.uwhportal.com",
        &ev("abc"),
    ));
    assert!(config.access_keys.is_empty());
}
```

- [ ] **Step 2: Run the tests and verify they fail**

Run: `cargo test -p refbox --bin refbox adopt 2>&1 | tail -20`
Expected: FAIL — `cannot find function adopt_legacy_access_key`.

- [ ] **Step 3: Implement it**

```rust
/// Move a pre-upgrade single-slot key into the per-event store, filing it
/// against the event the refbox is linked to. Runs once: the legacy slot is
/// cleared either way, so a key that could not be attributed is not offered
/// again on the next launch.
///
/// Returns whether the config changed and should be saved.
fn adopt_legacy_access_key(
    config: &mut Config,
    kind: SiteKind,
    base_url: &str,
    event: &EventId,
) -> bool {
    let legacy = match kind {
        SiteKind::Portal => std::mem::take(&mut config.uwhportal.token),
        SiteKind::Custom => std::mem::take(&mut config.custom_site.token),
    };
    if legacy.is_empty() {
        return false;
    }
    // A key written by this version is known-good and known-attributed; a
    // legacy one is neither, so it never displaces it.
    if config.access_key_for(base_url, event).is_none() {
        config.store_access_key(base_url, event, legacy);
    }
    true
}
```

Call it at `app/mod.rs:2580-2585`, inside the arm where `load_or_none` returned a note and `decide_restore` accepted it — that arm is the one place at startup where the restored event id is known. Pass `new.current_site.kind`, `&new.current_site.base_url` and the note's `event_id`. When it returns `true`, call `apply_access_key()` and save the config, so the legacy slot is gone from the file by the next launch.

- [ ] **Step 4: Run the tests and verify they pass**

Run: `cargo test -p refbox --bin refbox 2>&1 | tail -20`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add refbox/src/app/mod.rs refbox/src/config.rs
git commit -m "feat(refbox): adopt a pre-upgrade access key for the linked event"
```

---

### Task 6: Prove the whole behaviour, then run the gate

**Files:**
- Test: `refbox/src/config.rs` (`mod tests`)

- [ ] **Step 1: Write the end-to-end store test**

```rust
#[test]
fn returning_to_a_previously_used_event_finds_its_key() {
    // The behaviour this branch exists for: run event A, move to event B,
    // come back to A. A's key must still be there.
    let portal = "https://api.uwhportal.com";
    let mut config = Config::default();
    config.store_access_key(portal, &ev("aaa"), "KEY-A".into());
    config.store_access_key(portal, &ev("bbb"), "KEY-B".into());

    let text = toml::to_string(&config).unwrap();
    let reloaded: Config = toml::from_str(&text).unwrap();

    assert_eq!(reloaded.access_key_for(portal, &ev("aaa")), Some("KEY-A"));
    assert_eq!(reloaded.access_key_for(portal, &ev("bbb")), Some("KEY-B"));
}
```

- [ ] **Step 2: Run it**

Run: `cargo test -p refbox --bin refbox returning_to_a_previously_used_event 2>&1 | tail -20`
Expected: PASS.

- [ ] **Step 3: Run the full gate**

Run: `just check`
Expected: exit code 0. Check the exit code directly — do not pipe to `tail`, which masks it.

- [ ] **Step 4: Commit**

```bash
git add refbox/src/config.rs
git commit -m "test(refbox): cover returning to a previously used event"
```

---

## Manual walkthrough (Eric, after the branch builds)

Claude launches the app; Eric performs these and reports back. Needs the dev portal and two events.

1. Launch, connect to event A with a login code, pick a court and a game. Confirm the access-token indicator is green.
2. Open settings, switch to event B, log in with a fresh code, pick a court and a game. Confirm green.
3. Switch back to event A. **Expected: it re-establishes with no login code requested.** This fails on master today.
4. Quit and relaunch. Confirm event A is still connected without a login code.
5. Open `~/.config/refbox/default-config.toml` (confy's name for the settings file -- there is
   no `config.toml`, and looking for one finds nothing) and confirm two `[[access_keys]]`
   entries. A pre-existing `token =` under `[uwhportal]` stays exactly where it is: deviation 3
   dropped adoption, so the legacy slot is retained and never read, not removed.

## Flagged, not addressed in this branch

- **Upgrading with results already queued — known issue, ruled 2026-09-04: accept and document.**
  Adoption was dropped (deviation 3), so after an upgrade the queue can hold results for an event
  whose key is no longer readable. Every upload is refused with `no access key held for event …`
  and the portal indicator goes red until the operator logs in again *for that event*. If that
  event has already finished it will not issue a refbox code, so those results cannot be delivered
  at all: they are archived to `portal_queue.expired.json` after `EXPIRY_THRESHOLD` (120h) rather
  than sent. The window is narrow — it needs an upgrade between events with results still pending
  — and the archive means nothing is destroyed, but it is a real path to results never reaching
  the Portal. Eric ruled against reintroducing adoption for this case; do not reopen it without
  new evidence.

- **The mode-switch warning may become pessimistic.** `mode-switch-portal-tenant` says "you must re-connect to {$to_portal} Portal". Each mode has its own portal base URL (`portal_target`, `app/mod.rs:474-494`), so once keys are filed per site, switching back to a tenant you have used will re-establish on its own and the warning will overstate. Copy change needs Eric's ruling.
- **A queued result for an old event.** Queue items carry an event but no site, and the client holds one key at a time. A result queued for event A and sent while linked to event B already goes out with B's key today; this branch does not change that, but it does make it fixable.
- **`post_game_stats` is not event-scoped in its URL** (`/api/admin/events/stats`, `uwh-common/src/uwhportal/mod.rs:344`) while every other authenticated refbox call is. Worth confirming which key it expects before branch 2.

---

## Deviations from this plan (recorded during execution, 2026-09-01)

1. **Task 4 was retargeted.** The plan was written against master before PR #3082 merged, which
   replaced the inline `match self.current_site.kind { … }` credential write with a guarded
   `file_login_key()`. Task 4 extended that function instead of reintroducing an inline match.
2. **Task 4 also stamps the login reply with its `EventId`.** The plan filed the key under
   `current_event_id` read at reply time, which files it under whatever event is selected when the
   answer lands. `Message::RecvPortalToken` now carries the `EventId` captured at issue.
3. **Task 5 (adopt a pre-upgrade key) was built and then REMOVED.** The whole-branch review showed
   it could file the existing key under the wrong event: the link note records the last *linked*
   event, not the last *logged-in* one. Where those differ the key was consumed and worked for
   neither — a regression against master. Eric ruled on 2026-09-01 to drop adoption: everyone logs
   in once after upgrading, which is predictable, rather than a rare silent key-burn.
   The legacy `token` fields therefore remain parsed and written-while-non-empty, but nothing in
   this version reads them.
4. **`build_site_client` lost its `config` parameter** rather than keeping it as `_config`; the
   plan wrongly claimed `https_policy_conflict` still needed it.
5. **`apply_access_key()` runs before** the `#[cfg(debug_assertions)]` scramble block in
   `set_current_event_id`, not as the last statement as the plan said, so
   `UWH_PORTAL_SCRAMBLE_TOKEN` keeps the last word.
6. **Tasks 5-6 ran lean** (controller-implemented, one whole-branch review) rather than a per-task
   gated loop, per `.claude/rules/plan-execution.md`.

7. **The walkthrough found a defect the branch had shipped with, and the fix changed shape twice.**
   On 2026-09-04 Eric's login filed its key correctly and the court list then hung. The privileged
   schedule request was coming back `401`, ~160ms after the app reported the login succeeded.
   `apply_access_key()` resolved the key from `current_event_id` -- the *linked* event -- while the
   login files under the event it was issued for and `request_schedule` fetches for the *drafted*
   one. On a first login to a newly picked event those differ, the lookup missed, and the `None`
   arm cleared the key `set_token()` had installed moments earlier.

   The same root cause also broke this branch's headline feature: picking an event already logged
   in to fetched its schedule with the *previous* event's key, so the court list hung there too and
   APPLY never enabled -- which meant the operator could never reach the APPLY that would have
   loaded the right key. Both whole-branch reviews called the branch clean. Nothing in the suite
   exercised login-then-fetch; the tests covered the store, which was always correct.

   It also re-reads walkthrough part 1: the identical `401` against production event `2243-A` on
   2026-09-01 was recorded as "upcoming production events have no schedules". It was this bug.

   The first fix pushed the key out at each transition that changes the working event. Eric hit the
   gap within minutes -- the ACCESS TOKEN row reported an event with a good key on file as
   disconnected, until he re-picked it. Eleven places stage or clear the event and only two read
   the result, so the key is now settled at those two reads instead: immediately before the
   privileged schedule fetch (`request_schedule`), and when the settings editor opens
   (`enter_game_config`). `working_event` and `key_for_event` are the single definition.

## Walkthrough as run (2026-09-04, dev portal, isolated `XDG_CONFIG_HOME`)

Six scenarios, all confirmed by Eric driving the app, each corroborated in the log or the settings
file rather than from the screen alone:

1. **Fresh login fills the court list.** Was the reported defect; `Got schedule` 200 after the fix.
2. **Returning to an event already logged in to** reconnects with no login code, court list fills.
3. **A second event's login does not evict the first.** Two `[[access_keys]]` entries on one site
   (`events/1889-B`, `events/1601-C`); the second login's schedule fetch returned 200.
4. **Restart** restores the link and reconnects with no code; the ACCESS TOKEN row opens green.
5. **Upgrade from a legacy token.** Seeded a *working* dev key into the old `[uwhportal] token`
   slot with an empty store: refbox asked for a code anyway, filed a different key in the new
   store, and left the legacy value untouched. Adoption is genuinely gone.
6. **Cross-site separation.** Custom site pointed at `https://api.uwhportal.com/api/1889-B` --
   production host, same event id, which collides by design. Refbox did not reuse the dev key:
   the production request came back `401` with no credential sent, and switching back to the
   Portal re-authenticated cleanly.

Not covered: a genuine third-party server (the "custom site" was the portal at another address),
and any use of a real tournament schedule under load.
