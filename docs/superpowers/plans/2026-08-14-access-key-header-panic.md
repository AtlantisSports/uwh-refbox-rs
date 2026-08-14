# Access Key Header Panic — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A refbox holding an access key that an HTTP header cannot carry reports the problem and keeps running, instead of panicking on the next portal call.

**Architecture:** The key is checked once, where it arrives, and the client stores a finished `HeaderValue` instead of raw text. After that, `authenticated_request` performs no conversion at all, so none of the 10 authenticated calls can hit this fault again. The character rule lives in `uwh-common` and `schedule-processor` delegates to it.

**Correction to the brief:** there are **10** authenticated call sites, not 11 — `get_event_list` sends no authorization header at all. Verified with `grep -c "authenticated_request(&self.client"`. This matters for the live run: the refbox's startup event-list request is unauthenticated, so it does **not** trigger the crash (see Task 5).

**Tech Stack:** Rust 2024, MSRV 1.85, `reqwest` (client), `iced` 0.13 (refbox UI), Fluent `.ftl` (translations).

**Spec:** `docs/superpowers/specs/2026-08-14-access-key-header-panic-design.md`

## Global Constraints

- Work from the worktree `/home/estraily/projects/uwh-refbox-rs/.worktrees/access-key-header-panic`. Run every cargo/just command from there.
- **Heavy process** (`.claude/rules/plan-execution.md`): `uwh-common` is the highest-blast-radius crate. Verify per task; do not batch tasks.
- **Approval gate:** ask the human before every commit. Never push or open a PR without asking.
- Clippy is `-D warnings`. `cargo fmt --all` before every commit.
- No `unwrap()`/`expect()` in production code without a comment stating why it cannot panic.
- No new dependencies.
- `uwh-common::uwhportal` is already `#[cfg(feature = "std")]`, so no_std is not expected to change — it is still checked in Task 5.
- Rejected outright, at every layer: dropping the authorization header when it cannot be built. That sends the request unauthenticated and turns a loud crash into a silent wrong answer.
- The character rule, used everywhere: **printable ASCII only** — `matches!(c, ' '..='~')`.

---

### Task 1: The shared rule in `uwh-common`

**Files:**
- Modify: `uwh-common/src/uwhportal/mod.rs` (add public check + error type near the bottom, before the `#[cfg(test)]` modules at line 884)
- Test: `uwh-common/src/uwhportal/mod.rs` (new `#[cfg(test)] mod access_key_tests`)

**Interfaces:**
- Consumes: nothing.
- Produces: `pub struct UnsendableAccessKey { pub character: char }` (implements `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`, `Display`, `std::error::Error`) and `pub fn check_access_key(key: &str) -> Result<(), UnsendableAccessKey>`. Task 2, 3 and 4 all use these exact names.

- [ ] **Step 1: Write the failing tests**

Add at the end of `uwh-common/src/uwhportal/mod.rs`:

```rust
#[cfg(test)]
mod access_key_tests {
    use super::*;

    #[test]
    fn a_curly_quote_is_refused_and_named() {
        // The case this exists for: a key copied through a chat app or a word
        // processor, where a straight quote has been turned into a curly one.
        let err = check_access_key("abc\u{2019}123").unwrap_err();
        assert_eq!(err.character, '\u{2019}');
    }

    #[test]
    fn a_newline_or_tab_is_refused() {
        assert_eq!(check_access_key("abc\n123").unwrap_err().character, '\n');
        assert_eq!(check_access_key("abc\t123").unwrap_err().character, '\t');
    }

    #[test]
    fn a_normal_key_is_accepted() {
        // Letters, digits, and the punctuation base64 and JWTs use.
        let key = "eyJhbGciOiJI.UzI1NiIs-InR5cCI6_IkpXVCJ9~abc+/=";
        assert_eq!(check_access_key(key), Ok(()));
    }
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p uwh-common access_key_tests`
Expected: FAIL to compile — `cannot find function 'check_access_key' in this scope`.

- [ ] **Step 3: Write the implementation**

Add immediately above the `#[cfg(test)] mod coin_flip_tests` block:

```rust
/// A character an access key must not contain, because an HTTP header cannot
/// carry it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnsendableAccessKey {
    /// The first character in the key that cannot be sent.
    pub character: char,
}

impl std::fmt::Display for UnsendableAccessKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "the access key contains a character that cannot be sent to the site ({:?})",
            self.character
        )
    }
}

impl Error for UnsendableAccessKey {}

/// Check that `key` can be carried in an `Authorization` header.
///
/// Printable ASCII only. Everything a real access key contains — letters,
/// digits, and the punctuation used by base64 and JWTs — is in this range, and
/// everything outside it is what a header cannot carry: a newline, a tab, a
/// curly quote left by a chat app or a word processor.
///
/// Whitespace around the key is *not* trimmed here; callers that accept a
/// pasted key trim first, so that this reports only characters that are
/// genuinely part of the key.
pub fn check_access_key(key: &str) -> Result<(), UnsendableAccessKey> {
    match key.chars().find(|c| !matches!(c, ' '..='~')) {
        Some(character) => Err(UnsendableAccessKey { character }),
        None => Ok(()),
    }
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p uwh-common access_key_tests`
Expected: 3 passed.

- [ ] **Step 5: Format, lint, and ask before committing**

```bash
cargo fmt --all
cargo clippy -p uwh-common --all-targets --all-features -- -D warnings
```

Then ask the human for permission to commit. On approval:

```bash
git add uwh-common/src/uwhportal/mod.rs
git commit -m "feat(uwh-common): add the access-key character check"
```

---

### Task 2: The client stores a finished header

This is the task that removes the panic. Everything else is plumbing.

**Files:**
- Modify: `uwh-common/src/uwhportal/mod.rs` — struct field (line ~160), `new` (~166), `set_token` (~187), `has_token` (~195), `authenticated_request` (~842), and the 10 call sites listed below
- Test: `uwh-common/src/uwhportal/mod.rs` (`mod access_key_tests`, extended)

**Interfaces:**
- Consumes: `check_access_key`, `UnsendableAccessKey` from Task 1.
- Produces: `UwhPortalClient::set_token(&mut self, token: &str) -> Result<(), UnsendableAccessKey>` — **signature change, callers in Task 3 and Task 4**. `UwhPortalClient::new` keeps its existing signature and its `Result<Self, Box<dyn Error>>`, gaining a failure case. Private field renamed `access_token: Option<String>` → `auth_header: Option<HeaderValue>`. `has_token` and `clear_token` keep their signatures and meaning.

- [ ] **Step 1: Write the failing tests**

Add these to `mod access_key_tests` (the module from Task 1):

`Duration`, `Error`, `Method`, `AUTHORIZATION` and `HeaderValue` all arrive
through the module's existing `use super::*` — do not re-import them, clippy
runs with `-D warnings`.

```rust
    fn test_client(key: Option<&str>) -> Result<UwhPortalClient, Box<dyn Error>> {
        UwhPortalClient::new("https://example.test", key, true, Duration::from_secs(5))
    }

    #[test]
    fn a_good_key_becomes_exactly_one_bearer_header() {
        let client = test_client(Some("good-key")).unwrap();
        let request = authenticated_request(
            &client.client,
            Method::GET,
            "https://example.test/thing",
            &client.auth_header,
        )
        .build()
        .unwrap();
        assert_eq!(
            request.headers().get(AUTHORIZATION).unwrap(),
            "Bearer good-key"
        );
    }

    #[test]
    fn a_client_cannot_be_built_with_a_key_that_cannot_be_sent() {
        // On master this succeeded, and the panic waited until the first call.
        assert!(test_client(Some("abc\u{2019}123")).is_err());
    }

    #[test]
    fn a_refused_key_leaves_the_previous_key_in_place() {
        // A half-updated client would be worse than a refused one: it would
        // start sending calls with no credential at all.
        let mut client = test_client(Some("good-key")).unwrap();
        assert!(client.set_token("abc\u{2019}123").is_err());
        assert_eq!(
            client.auth_header,
            Some(HeaderValue::from_static("Bearer good-key"))
        );
        assert!(client.has_token());
    }

    #[test]
    fn a_key_copied_with_a_trailing_newline_is_trimmed_not_refused() {
        // Copying a key out of a web page is how the newline gets there.
        let mut client = test_client(None).unwrap();
        assert_eq!(client.set_token("  good-key \r\n"), Ok(()));
        assert_eq!(
            client.auth_header,
            Some(HeaderValue::from_static("Bearer good-key"))
        );
    }
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p uwh-common access_key_tests`
Expected: FAIL to compile — `no field 'auth_header' on type 'UwhPortalClient'`.

- [ ] **Step 3: Change the client to hold a header**

In `uwh-common/src/uwhportal/mod.rs`, replace the field:

```rust
pub struct UwhPortalClient {
    base_url: String,
    // The finished header, not the raw key. Building it here, once, is what
    // keeps a key that cannot be sent out of the 11 calls that use it.
    auth_header: Option<HeaderValue>,
    client: Client,
    id: OnceCell<u32>,
}
```

In `new`, replace `access_token: access_token.map(|s| s.to_string()),` with a checked conversion. The `?` runs before `Self` is built, so a client that exists always holds a usable key:

```rust
        let auth_header = match access_token {
            Some(token) => Some(build_auth_header(token)?),
            None => None,
        };

        Ok(Self {
            base_url,
            auth_header,
            client,
            id: OnceCell::new(),
        })
```

Replace `set_token` and `has_token`:

```rust
    /// Replace the access key.
    ///
    /// The key is checked and converted before it is stored, so a key that
    /// cannot be sent is refused here — the previous key is kept and the
    /// client is never left half-updated.
    pub fn set_token(&mut self, token: &str) -> Result<(), UnsendableAccessKey> {
        self.auth_header = Some(build_auth_header(token)?);
        Ok(())
    }

    pub fn clear_token(&mut self) {
        self.auth_header = None;
    }

    pub fn has_token(&self) -> bool {
        self.auth_header.is_some()
    }
```

Add the private helper next to `authenticated_request`:

```rust
/// Turn an access key into the `Authorization` header value, or say which
/// character stops it.
///
/// Surrounding whitespace is dropped first: a key copied out of a web page
/// arrives with a trailing newline, and that is worth accepting rather than
/// refusing.
fn build_auth_header(token: &str) -> Result<HeaderValue, UnsendableAccessKey> {
    let token = token.trim();
    check_access_key(token)?;
    // why this cannot panic: `check_access_key` has just proved every
    // character is in `' '..='~'` (0x20..=0x7E), which is exactly the range
    // `from_str` accepts, and "Bearer " is itself in that range.
    Ok(HeaderValue::from_str(&format!("Bearer {token}"))
        .expect("a printable-ASCII key always makes a valid header value"))
}
```

Replace `authenticated_request` — this is the line that panicked:

```rust
fn authenticated_request(
    client: &Client,
    method: Method,
    url: &str,
    auth_header: &Option<HeaderValue>,
) -> RequestBuilder {
    let mut request = client.request(method, url);
    if let Some(value) = auth_header {
        request = request.header(AUTHORIZATION, value.clone());
    }
    request
}
```

- [ ] **Step 4: Update the 10 call sites**

Each is a single-word change, `&self.access_token` → `&self.auth_header`:

| Line | Method |
|---|---|
| 309 | `verify_token` |
| 333 | `post_game_stats` |
| 366 | `post_game_scores` |
| 410 | `get_event_schedule_privileged` |
| 589 | `push_event_schedule` |
| 624 | `push_team_map` |
| 703 | `get_coin_flips` |
| 737 | `get_event_referee_name_map` |
| 781 | `get_game_referee_name_map` |
| 826 | `set_coin_flip_result` |

`get_event_list` is *not* in this list — it sends no authorization header. Let the compiler confirm the set:

Run: `cargo build -p uwh-common 2>&1 | grep -c "no field"`
Expected: 0 once every call site is updated.

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cargo test -p uwh-common access_key_tests`
Expected: 7 passed (3 from Task 1, 4 from this task).

- [ ] **Step 6: The mutation check — prove the tests are load-bearing**

Do not skip this. Delete the line `check_access_key(token)?;` from `build_auth_header`, then:

Run: `cargo test -p uwh-common access_key_tests`
Expected: RED. `a_client_cannot_be_built_with_a_key_that_cannot_be_sent` and `a_refused_key_leaves_the_previous_key_in_place` both fail, and the panic re-appears in the test output from the `expect` — which is the original crash, now caught by a test instead of an operator.

Then restore the line and re-run:

Run: `cargo test -p uwh-common access_key_tests`
Expected: 7 passed.

Record in the plan's Deviations section that the mutation check was run and what went red.

- [ ] **Step 7: Format, lint, and ask before committing**

```bash
cargo fmt --all
cargo clippy -p uwh-common --all-targets --all-features -- -D warnings
```

Ask the human. On approval:

```bash
git add uwh-common/src/uwhportal/mod.rs
git commit -m "fix(uwh-common): check the access key before it reaches the header"
```

---

### Task 3: `schedule-processor` delegates to the shared rule

**Files:**
- Modify: `schedule-processor/src/site.rs:65-90` (`validate_access_key` — the doc comment and the character check)
- Modify: `schedule-processor/src/main.rs` — the 5 `set_token` calls at lines 573, 615, 770, 1080, 1130
- Test: `schedule-processor/src/site.rs` (existing tests at lines 171-212 must keep passing **unchanged** — they are the proof the delegation preserved behaviour)

**Interfaces:**
- Consumes: `uwh_common::uwhportal::check_access_key` (Task 1), `set_token -> Result` (Task 2).
- Produces: nothing new. `validate_access_key(raw: &str) -> Result<Option<String>, String>` keeps its exact signature, wording, trimming and `Ok(None)` behaviour.

- [ ] **Step 1: Run the existing tests first, so the baseline is known**

Run: `cargo test -p schedule-processor site::`
Expected: PASS. These are the tests that must still pass at the end.

- [ ] **Step 2: Delegate the character question**

In `schedule-processor/src/site.rs`, replace the doc comment paragraph and the character check inside `validate_access_key`. The comment currently says the panic is unfixed — this branch fixes it, so the sentence becomes false and must go.

```rust
/// Clean up and check an access key the operator pasted.
///
/// `Ok(None)` means no key was given, which is allowed — an open site can be
/// read without one, and anything that later needs a key asks for it then.
///
/// The character rule lives in `uwh-common`, which owns the header this key
/// ends up in, so the two cannot drift apart. `uwh-common` refuses a bad key
/// too; this check exists so the operator gets a sentence about their paste at
/// the moment they paste it, rather than a failure later.
pub fn validate_access_key(raw: &str) -> Result<Option<String>, String> {
    let key = raw.trim();
    if key.is_empty() {
        return Ok(None);
    }
    if let Err(why) = uwh_common::uwhportal::check_access_key(key) {
        return Err(format!(
            "That access key contains a character that cannot be sent to the site \
             ({:?}). Copy the key again, straight from where your site shows it.",
            why.character
        ));
    }
    Ok(Some(key.to_string()))
}
```

- [ ] **Step 3: Run the existing tests to confirm nothing changed**

Run: `cargo test -p schedule-processor site::`
Expected: PASS, all of them, unmodified. If `a_pasted_key_with_a_smart_quote_is_refused` fails on its message assertion, the wording above drifted — fix the wording, not the test.

- [ ] **Step 4: Handle the refusal at the 5 key-setting sites**

`set_token` now returns a `Result`. Each site gets the treatment its neighbours already use.

Line ~573 — the key came from `validate_access_key`, so a refusal here means the two rules disagree. Log it rather than ignore it:

```rust
                                Ok(Some(key)) => {
                                    if let Err(why) = portal_client.set_token(&key) {
                                        error!("{why}");
                                        continue 'outer;
                                    }
                                }
```

Lines ~615, ~770, ~1130 — the key came from a login response. Same shape as the `Err` arm beside each one, which does `continue 'outer`:

```rust
                            if let Err(why) = portal_client.set_token(&token) {
                                error!("The site returned an access key that cannot be used: {why}");
                                continue 'outer;
                            }
```

At line ~770 and ~1130 this replaces `Ok(token) => portal_client.set_token(&token),` inside a `match`, so it becomes:

```rust
                        Ok(token) => {
                            if let Err(why) = portal_client.set_token(&token) {
                                error!("The site returned an access key that cannot be used: {why}");
                                continue 'outer;
                            }
                        }
```

Line ~1080 — its neighbouring `Err` arm says "Proceeding without login", so this one warns and carries on rather than restarting the loop:

```rust
                                    Ok(token) => {
                                        if let Err(why) = portal_client.set_token(&token) {
                                            error!(
                                                "The site returned an access key that cannot be \
                                                 used. Proceeding without login. Reason: {why}"
                                            );
                                        }
                                    }
```

- [ ] **Step 5: Build and lint**

Run: `cargo clippy -p schedule-processor --all-targets --all-features -- -D warnings`
Expected: clean, no `unused_must_use`.

Run: `cargo test -p schedule-processor`
Expected: PASS.

- [ ] **Step 6: Format and ask before committing**

```bash
cargo fmt --all
```

Ask the human. On approval:

```bash
git add schedule-processor/src/site.rs schedule-processor/src/main.rs
git commit -m "refactor(schedule-processor): use the shared access-key check"
```

---

### Task 4: The refbox refuses a bad key from the site and says so

**Files:**
- Modify: `refbox/src/app/mod.rs:43` (import), `:359` (new `ConfirmationKind` variant), `:1449` (debug-only key setter), `:4740-4744` (the comment listing which kinds reach that match), `:5102-5120` (`PortalTokenResponse::Success` arm)
- Modify: `refbox/src/app/view_builders/confirmation.rs:24-40` (header text) and `:117-119` (buttons)
- Modify: `refbox/translations/<15 locales>/refbox.ftl` — one new key each
- Test: none automated (refbox is a bin crate and this path needs a live client); proven by Task 5's live run and by the `uwh-common` tests from Task 2

**Interfaces:**
- Consumes: `check_access_key`, `UnsendableAccessKey` (Task 1), `set_token -> Result` (Task 2).
- Produces: `ConfirmationKind::UwhPortalKeyUnusable` (no payload) and the Fluent key `uwhportal-token-unusable-key`.

- [ ] **Step 1: Add the translation key to all 15 locales**

The dialog's two sibling reasons are both two-line, so this one matches. Add after `uwhportal-token-no-pending-link` in each file:

`refbox/translations/en-US/refbox.ftl`:
```
uwhportal-token-unusable-key = The site sent an access key this refbox cannot use.
    Ask the site for the key again.
```

`de-DE`:
```
uwhportal-token-unusable-key = Die Website hat einen Zugangsschlüssel gesendet, den diese Refbox nicht verwenden kann.
    Fordern Sie den Schlüssel erneut bei der Website an.
```

`es`:
```
uwhportal-token-unusable-key = El sitio envió una clave de acceso que esta Refbox no puede usar.
    Solicite la clave al sitio de nuevo.
```

`fr`:
```
uwhportal-token-unusable-key = Le site a envoyé une clé d'accès que cette Refbox ne peut pas utiliser.
    Demandez à nouveau la clé au site.
```

`id-ID`:
```
uwhportal-token-unusable-key = Situs mengirim kunci akses yang tidak dapat digunakan Refbox ini.
    Mintalah kunci itu lagi dari situs Anda.
```

`it-IT`:
```
uwhportal-token-unusable-key = Il sito ha inviato una chiave di accesso che questa Refbox non può usare.
    Richiedi di nuovo la chiave al sito.
```

`ja-JP`:
```
uwhportal-token-unusable-key = サイトから送られたアクセスキーは、この Refbox では使用できません。
    サイトにキーをもう一度要求してください。
```

`ko-KR`:
```
uwhportal-token-unusable-key = 사이트에서 보낸 액세스 키를 이 Refbox에서 사용할 수 없습니다.
    사이트에 키를 다시 요청하세요.
```

`ms-MY`:
```
uwhportal-token-unusable-key = Tapak ini menghantar kunci akses yang tidak boleh digunakan oleh Refbox ini.
    Minta kunci itu semula daripada tapak anda.
```

`nl-NL`:
```
uwhportal-token-unusable-key = De site heeft een toegangssleutel gestuurd die deze Refbox niet kan gebruiken.
    Vraag de sleutel opnieuw op bij de site.
```

`pt-PT`:
```
uwhportal-token-unusable-key = O site enviou uma chave de acesso que esta Refbox não consegue usar.
    Peça novamente a chave ao site.
```

`th-TH`:
```
uwhportal-token-unusable-key = เว็บไซต์ส่งคีย์การเข้าถึงที่ Refbox นี้ใช้ไม่ได้
    โปรดขอคีย์จากเว็บไซต์อีกครั้ง
```

`tl-PH`:
```
uwhportal-token-unusable-key = Nagpadala ang site ng access key na hindi magagamit ng Refbox na ito.
    Hilingin muli ang key sa site.
```

`tr-TR`:
```
uwhportal-token-unusable-key = Site, bu Refbox'ın kullanamayacağı bir erişim anahtarı gönderdi.
    Anahtarı siteden tekrar isteyin.
```

`zh-CN`:
```
uwhportal-token-unusable-key = 站点发送的访问密钥无法在此 Refbox 上使用。
    请再次向站点索取密钥。
```

- [ ] **Step 2: Add the confirmation variant**

In `refbox/src/app/mod.rs`, beside `UwhPortalLinkFailed`:

```rust
    UwhPortalLinkFailed(PortalTokenResponse),
    // Raised when the site's reply carried an access key this refbox cannot
    // put in a header. Not a `PortalTokenResponse` variant: the server never
    // says this, the refbox concludes it, and that type should keep meaning
    // "what the server replied".
    UwhPortalKeyUnusable,
```

Extend the import on line 43 with `check_access_key`.

- [ ] **Step 3: Render it in the dialog**

In `refbox/src/app/view_builders/confirmation.rs`, add to the `header_text` match after the `UwhPortalLinkFailed(PortalTokenResponse::Success(_))` arm:

```rust
        ConfirmationKind::UwhPortalKeyUnusable => fl!("uwhportal-token-unusable-key"),
```

and widen the buttons arm so it offers the same single OK:

```rust
        ConfirmationKind::UwhPortalLinkFailed(_) | ConfirmationKind::UwhPortalKeyUnusable => {
            vec![(fl!("ok"), green_button, ConfirmationOption::GoBack)]
        }
```

`ConfirmationOption::GoBack` returns the operator to the link-code keypad, which is where they can ask the site for a fresh key — the behaviour the other link failures already have.

- [ ] **Step 4: Refuse the key before it is stored or saved**

In `refbox/src/app/mod.rs`, the `PortalTokenResponse::Success(token)` arm. Check first, and wrap the existing body in the `else`:

```rust
                    PortalTokenResponse::Success(token) => {
                        // The site's reply is not trusted text. A custom site is
                        // somebody else's server, and a key carrying a character a
                        // header cannot hold used to take the refbox down on the
                        // next call. Refusing it here also keeps it out of the
                        // config file, so it cannot come back at the next launch.
                        let token = token.trim().to_string();
                        if let Err(why) = check_access_key(&token) {
                            warn!("The site returned an access key that cannot be sent: {why}");
                            AppState::ConfirmationPage(ConfirmationKind::UwhPortalKeyUnusable)
                        } else {
                            info!("Portal token request succeeded");
                            if let Some(client) = self.uwhportal_client.as_ref() {
                                // why this cannot panic: the guard is held only for a
                                // synchronous `set_token()` call and dropped immediately.
                                if let Err(why) = client.lock().unwrap().set_token(&token) {
                                    // Cannot happen: the key was just checked with the
                                    // same rule `set_token` applies. Logged rather than
                                    // ignored so a future divergence is not silent.
                                    error!("Client refused a key that passed the check: {why}");
                                }
                            }

                            // ... the rest of the existing arm, unchanged, from
                            // `match self.current_site.kind {` through to
                            // `AppState::EditGameConfig(ConfigPage::Game)` ...
                        }
                    }
```

Keep the whole existing body — the config save, `uwhportal_token_valid`, `portal_manager.token_refreshed()`, the schedule request, and the final `AppState::EditGameConfig(ConfigPage::Game)` — inside the `else`, untouched apart from indentation.

- [ ] **Step 5: Update the stale comment about which kinds reach the final match**

Around line 4740 the comment says only `Error` and `UwhPortalLinkFailed` reach that match. Add the new variant:

```rust
                // After ADR 009 Task 13 retired the global apply path, only
                // `ConfirmationKind::Error` (which offers DiscardChanges) and
                // `ConfirmationKind::UwhPortalLinkFailed` /
                // `ConfirmationKind::UwhPortalKeyUnusable` (which offer GoBack)
                // reach this match. The Game-related and PortalTenantSwitch
                // confirmations are dispatched to apply_game_confirmation above.
```

- [ ] **Step 6: Fix the debug-only key setter**

At line ~1449, `set_token` now returns a `Result`:

```rust
                // A literal printable-ASCII key, so the check cannot refuse it;
                // this is the debug scramble path, not an operator's key.
                let _ = guard.set_token("invalid-debug-token");
```

- [ ] **Step 7: Build, lint, and check the translations**

Run: `cargo clippy -p refbox --all-targets --all-features -- -D warnings`
Expected: clean, and no non-exhaustive-match errors left from the new variant.

Run: `cargo test -p refbox`
Expected: PASS, including the translation-consistency test — it fails if any of the 15 locales is missing the new key.

- [ ] **Step 8: Format and ask before committing**

```bash
cargo fmt --all
```

Ask the human. On approval:

```bash
git add refbox/src refbox/translations
git commit -m "fix(refbox): refuse an access key the site cannot be sent"
```

---

### Task 5: Whole-workspace verification and the live run

**Files:** none modified (unless a check fails).

**Interfaces:**
- Consumes: everything from Tasks 1-4.
- Produces: the evidence for the PR's "how to verify" section.

- [ ] **Step 1: The downstream compile list**

`uwh-common` is the highest-blast-radius crate, so every dependent is checked (`uwh-common/CLAUDE.md`):

```bash
cargo check -p refbox
cargo check -p schedule-processor
cargo check -p overlay
cargo check -p led-panel-sim
cargo build -p uwh-common --no-default-features
```

Expected: all clean. The last one proves the no_std build is unaffected — `uwhportal` is `std`-gated, so this is confirmation, not a risk.

- [ ] **Step 2: The full gate**

Run: `just check`
Expected: fmt, lint, tests and audit all clean.

Note: `just lint` is not `--all-targets`. The stricter form fails on a pre-existing `player_grid.rs` error that does not fail CI — if that appears, it is not from this branch.

- [ ] **Step 3: Build the real binary for the live run**

`just check` builds a *test* binary, which is not what gets launched:

```bash
cargo build -p refbox
```

- [ ] **Step 4: Prove the crash on master first**

Ask the human before launching — they drive the refbox UI. Two things matter about how this is launched:

- The config directory is **shared**: only one refbox at a time, and back up `~/.config/refbox/config.toml` before editing it.
- **Never launch bare.** Without the override the refbox talks to the *production* portal and rewrites `portal_link.json`. Always:
  `UWH_PORTAL_URL_OVERRIDE=https://api.dev.uwhportal.com WAYLAND_DISPLAY= ./target/debug/refbox`

Put a **newline** into the saved key in `~/.config/refbox/config.toml`. TOML turns the `\n` escape into a real newline:

```toml
[uwhportal]
token = "abc\n123"
```

**Use a newline, not a curly quote.** Measured with a `reqwest` probe over `Bearer <key>`: `HeaderValue::from_str` rejects a newline, a carriage return, NUL and DEL — and *accepts* a curly quote, `é` and a tab. Only the control characters ever panicked. A curly quote here would produce a false "master didn't crash". A trailing newline is the likeliest bad character in real life anyway: it is what copying a key out of a web page gives you.

Then, from a separate checkout at `origin/master`, `cargo build -p refbox` and launch as above. The event list loads (that call carries no authorization header, so it survives) — then **select an event**, which requests the schedule. That is an authenticated call, and it is built synchronously on the UI thread.

Expected on master: the refbox dies the moment the event is selected. Capture the exit code and the panic line. Exit 101 at startup can also mean "no audio device" — confirm the panic message names `HeaderValue`, not audio.

- [ ] **Step 5: Run the fixed binary against the same config**

Same config, same launch line, this branch's build.

Expected:
- The refbox starts and stays up. Selecting an event is impossible because there is no client, which is the point: nothing is sent, and nothing is sent unauthenticated either.
- The log contains `Failed to start the client for ...` naming the character `'\n'` — that message comes from `UnsendableAccessKey`'s `Display` through the existing `build_site_client` error path, which needed no code change.
- The portal indicator is red.
- Everything unrelated to the portal works normally.

Then repair the key in the config (remove the curly quote), relaunch, and confirm the refbox connects and lists events as usual — proving the refusal is specific to the bad key and not a general breakage.

Kill it with `pkill -x refbox`. Restore the backed-up config afterwards.

- [ ] **Step 6: Record what was and was not proven live**

The config-file route exercises the startup path, so the live run proves the crash is gone but does **not** display the new dialog. State this plainly in the PR: the dialog reason is covered by test and code reading only. Proving it live needs the mock portal server to answer a link-code exchange with a bad `accessKey`, which was deliberately deferred.

- [ ] **Step 7: Code review, then ask about the PR**

Run `superpowers:requesting-code-review` for the whole branch. Then write the PR body in the project's four-section format (What changed / Why / Scope / How to verify) and **ask the human before pushing or opening it**.

---

## Deviations

Record anything that diverged from this plan here, and fold the note into the
relevant code commit rather than making a standalone commit for it.

- **The brief's three example characters were wrong in two of three cases.** Measured with a throwaway `reqwest` probe over `Bearer <key>`: `HeaderValue::from_str` **rejects** newline, carriage return, NUL and DEL; it **accepts** a curly quote (U+2019), `é` and a tab. Only the control characters ever panicked. Consequences, all folded into the tasks they affect rather than committed separately:
  - The printable-ASCII rule is kept as specified, which makes it deliberately stricter than the panic set. An access key is base64 or a JWT and is ASCII by construction, so nothing legitimate is refused, and a curly-quoted key would otherwise come back as a mystifying permission error instead of a sentence naming the character. It is also the rule `schedule-processor` already ships and its tests assert.
  - Task 2 gained a newline-based client test in fix round 1, so that deleting the guard reproduces the original panic rather than a plain assertion failure.
  - Task 5's live run uses `token = "abc\n123"`, not a curly quote, which would otherwise have produced a false "master didn't crash".
  - The spec was corrected in place with the measured table.
