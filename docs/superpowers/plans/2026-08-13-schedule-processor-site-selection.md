# Schedule processor site selection — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let the organiser choose the UWH Portal or a custom site at startup, typing the address
and pasting an access key for a custom site, instead of setting an environment variable.

**Architecture:** A new `site` module holds the site model and every pure decision — which address a
menu choice resolves to, whether a typed address requires a valid certificate, and whether a pasted
access key is usable. `main.rs` keeps only the prompting, which needs a terminal and cannot be unit
tested. The model mirrors the refbox's own: a `SiteKind` of `Portal` or `Custom`, and a resolved
`SiteTarget`.

**Tech Stack:** Rust 2024, MSRV 1.85, `inquire` for terminal prompts (already a dependency),
`uwh-common`'s `UwhPortalClient`. No new dependencies.

**Spec:** `docs/superpowers/specs/2026-08-13-schedule-processor-site-selection-design.md` — read it
first; this plan argues from it.

## Global Constraints

- `schedule-processor` crate only. No change to `uwh-common`, and no new dependency.
- **Do NOT fix** `HeaderValue::from_str(&format!("Bearer {}", token)).unwrap()` in
  `uwh-common/src/uwhportal/mod.rs` (~line 852). It panics on an invalid header character and is
  the reason keys are validated here, but it is reachable from the refbox's custom-site entry too,
  so it needs its own branch and its own refbox testing.
- Nothing is written to disk: no address, no access key, no password.
- The Portal path must behave **exactly** as it does today — same environment question, same sport
  question, same derived address, same `UWH_PORTAL_URL_OVERRIDE` / `UWR_PORTAL_URL_OVERRIDE`
  behaviour.
- **No test may require a terminal.** `inquire` prompts cannot run in `cargo test`; every test in
  this plan calls a pure function.
- Rust edition 2024, MSRV 1.85, `just check` must pass with zero warnings.
- Audience is a non-programmer: every failure message says what to do next, in plain words.

---

## File Structure

| File | Responsibility |
|---|---|
| `schedule-processor/src/site.rs` | **New.** The site model (`SiteKind`, `SiteTarget`), address derivation, certificate rule, access-key validation. All pure, all tested. |
| `schedule-processor/src/main.rs` | Prompting only: asks the questions, calls into `site`, and reports failures. Loses the inline address `match`. |

`main.rs` is already 1212 lines, so moving this logic out rather than adding to it is the right
direction. No other file changes.

---

## Task 1: The site module — address derivation

**Files:**
- Create: `schedule-processor/src/site.rs`
- Modify: `schedule-processor/src/main.rs` (add `mod site;` beside the existing module
  declarations at lines 19–27)

**Interfaces:**
- Consumes: the existing `apply_portal_override(default_url, default_require_https, override_value)
  -> (String, bool)` in `main.rs`, which stays where it is.
- Produces: `pub enum SiteKind { Portal, Custom }`,
  `pub struct SiteTarget { pub kind: SiteKind, pub base_url: String, pub require_https: bool }`,
  `pub fn portal_default_url(environment: &str, sport: &str) -> &'static str`,
  `pub fn override_var_name(sport: &str) -> &'static str`.

- [x] **Step 1: Write the failing tests**

Create `schedule-processor/src/site.rs` containing only this test module for now:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_menu_pair_resolves_to_its_address() {
        assert_eq!(
            portal_default_url("Production", "Underwater Hockey"),
            "https://api.uwhportal.com"
        );
        assert_eq!(
            portal_default_url("Production", "Underwater Rugby"),
            "https://api.uwrportal.com"
        );
        assert_eq!(
            portal_default_url("Development", "Underwater Hockey"),
            "https://api.dev.uwhportal.com"
        );
        assert_eq!(
            portal_default_url("Development", "Underwater Rugby"),
            "https://api.dev.uwrportal.com"
        );
        assert_eq!(portal_default_url("Local", "Underwater Hockey"), "http://localhost:9000");
        assert_eq!(portal_default_url("Local", "Underwater Rugby"), "http://localhost:9000");
    }

    #[test]
    fn the_override_variable_follows_the_sport() {
        assert_eq!(override_var_name("Underwater Hockey"), "UWH_PORTAL_URL_OVERRIDE");
        assert_eq!(override_var_name("Underwater Rugby"), "UWR_PORTAL_URL_OVERRIDE");
    }
}
```

- [x] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p schedule-processor site:: 2>&1 | tail -20`
Expected: FAIL — `cannot find function portal_default_url in this scope`. (If instead it reports
`file not found for module site`, add `mod site;` to `main.rs` first — see Step 3.)

- [x] **Step 3: Implement**

Add `mod site;` to `main.rs` alongside the other module declarations (lines 19–27), then put this
above the test module in `site.rs`:

```rust
//! Which site the tool talks to, and how it authenticates there.
//!
//! Mirrors the refbox's model: the operator picks the built-in portal or a
//! custom site, and a custom site is one address they type. Every decision
//! here is a pure function so it can be tested — `main.rs` keeps only the
//! prompting, which needs a terminal.

/// Which kind of site the tool is talking to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SiteKind {
    /// The built-in UWH/UWR portal, chosen from the environment menu.
    Portal,
    /// An address the operator typed, authenticated with a pasted access key.
    Custom,
}

/// A resolved site: where to connect, and what is expected of it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SiteTarget {
    pub kind: SiteKind,
    pub base_url: String,
    /// Whether a valid TLS certificate is required. Taken from the address's
    /// own scheme, so a local or on-LAN `http://` site still works.
    pub require_https: bool,
}

/// The portal address for a menu pair. The menu strings come straight from the
/// prompts in `main.rs`; anything else is a programming error, so this falls
/// back to production hockey rather than panicking on a typo.
pub fn portal_default_url(environment: &str, sport: &str) -> &'static str {
    match (environment, sport) {
        ("Local", _) => "http://localhost:9000",
        ("Development", "Underwater Rugby") => "https://api.dev.uwrportal.com",
        ("Development", _) => "https://api.dev.uwhportal.com",
        (_, "Underwater Rugby") => "https://api.uwrportal.com",
        _ => "https://api.uwhportal.com",
    }
}

/// The environment variable that can replace the menu-selected address.
pub fn override_var_name(sport: &str) -> &'static str {
    match sport {
        "Underwater Rugby" => "UWR_PORTAL_URL_OVERRIDE",
        _ => "UWH_PORTAL_URL_OVERRIDE",
    }
}
```

Note the fallbacks replace the current `unreachable!()`: an unknown menu string now resolves to
production hockey instead of aborting the tool mid-run.

- [x] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p schedule-processor site:: 2>&1 | tail -10`
Expected: PASS, both tests.

- [x] **Step 5: Commit**

```bash
git add schedule-processor/src/site.rs schedule-processor/src/main.rs
git commit -m "refactor(schedule-processor): move portal address derivation into a site module"
```

---

## Task 2: A typed address becomes a target

**Files:**
- Modify: `schedule-processor/src/site.rs`

**Interfaces:**
- Consumes: `SiteKind`, `SiteTarget` from Task 1.
- Produces: `pub fn custom_target(typed_url: &str) -> Result<SiteTarget, String>`.

- [x] **Step 1: Write the failing tests**

```rust
    #[test]
    fn a_typed_https_address_requires_a_certificate() {
        let t = custom_target("  https://scores.example.org  ").unwrap();
        assert_eq!(t.kind, SiteKind::Custom);
        assert_eq!(t.base_url, "https://scores.example.org", "surrounding spaces are trimmed");
        assert!(t.require_https);
    }

    #[test]
    fn a_typed_http_address_does_not_require_a_certificate() {
        // A club server on the local network is a real case; refusing it
        // would make the custom option useless there.
        let t = custom_target("http://192.168.1.50:9000").unwrap();
        assert!(!t.require_https);
    }

    #[test]
    fn an_address_without_a_scheme_is_refused_in_plain_words() {
        let err = custom_target("scores.example.org").unwrap_err();
        assert!(
            err.contains("http://") && err.contains("https://"),
            "the message must tell the operator what to type; got: {err}"
        );
    }

    #[test]
    fn a_blank_address_is_refused() {
        assert!(custom_target("   ").is_err());
    }

    #[test]
    fn a_trailing_slash_is_dropped_so_paths_do_not_double_up() {
        let t = custom_target("https://scores.example.org/").unwrap();
        assert_eq!(t.base_url, "https://scores.example.org");
    }
```

- [x] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p schedule-processor site:: 2>&1 | tail -20`
Expected: FAIL — `cannot find function custom_target in this scope`.

- [x] **Step 3: Implement**

```rust
/// Turn an address the operator typed into a target, or explain why it cannot
/// be used. Surrounding spaces and a trailing `/` are cleaned up silently —
/// both are ordinary copy-paste artefacts, not mistakes worth a message.
pub fn custom_target(typed_url: &str) -> Result<SiteTarget, String> {
    let url = typed_url.trim().trim_end_matches('/');
    if url.is_empty() {
        return Err("No address entered. Type the full web address of your site.".to_string());
    }
    let require_https = if url.starts_with("https://") {
        true
    } else if url.starts_with("http://") {
        false
    } else {
        return Err(format!(
            "\"{url}\" does not look like a web address. It needs to start with https:// \
             (or http:// for a site on your own network)."
        ));
    };
    Ok(SiteTarget {
        kind: SiteKind::Custom,
        base_url: url.to_string(),
        require_https,
    })
}
```

- [x] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p schedule-processor site:: 2>&1 | tail -10`
Expected: PASS, all seven tests so far.

- [x] **Step 5: Commit**

```bash
git add schedule-processor/src/site.rs
git commit -m "feat(schedule-processor): turn a typed address into a site target"
```

---

## Task 3: A pasted access key that cannot crash the tool

**Files:**
- Modify: `schedule-processor/src/site.rs`

**Interfaces:**
- Consumes: nothing from earlier tasks.
- Produces: `pub fn validate_access_key(raw: &str) -> Result<Option<String>, String>` —
  `Ok(None)` means "no key given, connect anonymously".

- [x] **Step 1: Write the failing tests**

```rust
    #[test]
    fn a_pasted_key_with_a_trailing_newline_is_cleaned_up_not_refused() {
        // THE case this function exists for. uwh-common builds its
        // authorization header with an unwrap that panics on a newline, and a
        // trailing newline is what copying from a web page gives you.
        assert_eq!(
            validate_access_key("abc123XYZ\n").unwrap(),
            Some("abc123XYZ".to_string())
        );
        assert_eq!(
            validate_access_key("  abc123XYZ \r\n").unwrap(),
            Some("abc123XYZ".to_string())
        );
    }

    #[test]
    fn a_blank_key_means_connect_without_one() {
        assert_eq!(validate_access_key("").unwrap(), None);
        assert_eq!(validate_access_key("   \n").unwrap(), None);
    }

    #[test]
    fn a_key_with_a_character_that_cannot_be_sent_is_refused_in_plain_words() {
        // A smart quote is what you get when a key is pasted through a word
        // processor or a chat app.
        let err = validate_access_key("abc\u{2019}123").unwrap_err();
        assert!(
            err.to_lowercase().contains("copy"),
            "the message must tell the operator to copy it again; got: {err}"
        );

        // An embedded newline would reach the header builder and panic.
        assert!(validate_access_key("abc\n123").is_err());
        // A tab likewise.
        assert!(validate_access_key("abc\t123").is_err());
    }

    #[test]
    fn an_ordinary_key_passes_through_unchanged() {
        let key = "eyJhbGciOi.JIUzI1NiIs-InR5cCI6_IkpXVCJ9";
        assert_eq!(validate_access_key(key).unwrap(), Some(key.to_string()));
    }
```

- [x] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p schedule-processor site:: 2>&1 | tail -20`
Expected: FAIL — `cannot find function validate_access_key in this scope`.

- [x] **Step 3: Implement**

```rust
/// Clean up and check an access key the operator pasted.
///
/// `Ok(None)` means no key was given, which is allowed — an open site can be
/// read without one, and anything that later needs a key asks for it then.
///
/// This check is not decoration. `uwh-common` builds its authorization header
/// with `HeaderValue::from_str(...).unwrap()`, which panics on any character a
/// header cannot carry. Until that is fixed on its own branch, a pasted key is
/// the one place a person's typing reaches it, so it is checked here first: a
/// bad paste has to produce a sentence, never a crash.
pub fn validate_access_key(raw: &str) -> Result<Option<String>, String> {
    let key = raw.trim();
    if key.is_empty() {
        return Ok(None);
    }
    // Printable ASCII only. Everything a key normally contains — letters,
    // digits, and the punctuation used by base64 and JWTs — is in this range,
    // and everything outside it is what a header cannot carry.
    if let Some(bad) = key.chars().find(|c| !matches!(c, ' '..='~')) {
        return Err(format!(
            "That access key contains a character that cannot be sent to the site ({bad:?}). \
             Copy the key again, straight from where your site shows it."
        ));
    }
    Ok(Some(key.to_string()))
}
```

- [x] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p schedule-processor site:: 2>&1 | tail -10`
Expected: PASS, all eleven tests.

- [x] **Step 5: Commit**

```bash
git add schedule-processor/src/site.rs
git commit -m "feat(schedule-processor): validate a pasted access key before it reaches the client"
```

---

## Task 4: Ask the questions

The only task that touches the prompts. Nothing here is unit-testable — `inquire` needs a
terminal — so it ends with a run of the tool instead.

**Files:**
- Modify: `schedule-processor/src/main.rs:108–160` (the sport and site prompts and the address
  derivation), and the credential prompt at `main.rs:515`

**Interfaces:**
- Consumes: `site::{SiteKind, SiteTarget, portal_default_url, override_var_name, custom_target,
  validate_access_key}` from Tasks 1–3, and the existing `apply_portal_override`.
- Produces: a `SiteTarget` and an `Option<String>` access key, both used to build the client.

- [x] **Step 1: Replace the site prompts**

In `main.rs`, replace the sport and site `Select` blocks (currently lines 108–131, ending with the
`default_url` match) with:

```rust
    let options = vec!["UWH Portal", "Custom site"];
    let site_kind_choice = Select::new("Select the site to connect to:", options)
        .prompt()
        .unwrap_or_else(|_| {
            error!("No site selected. Exiting.");
            std::process::exit(1);
        });

    // A custom site is one address the operator types, so the sport question is
    // skipped there: its only job is to pick between the hockey and rugby
    // portal addresses, which a typed address replaces.
    let (target, mut pasted_key) = if site_kind_choice == "Custom site" {
        let target = loop {
            let typed = Text::new("Enter the web address of your site:")
                .prompt()
                .unwrap_or_else(|_| {
                    error!("No address entered. Exiting.");
                    std::process::exit(1);
                });
            match site::custom_target(&typed) {
                Ok(t) => break t,
                Err(why) => error!("{why}"),
            }
        };
        let key = loop {
            let typed = Text::new("Paste your access key (leave blank if your site needs none):")
                .prompt()
                .unwrap_or_else(|_| {
                    error!("No access key entered. Exiting.");
                    std::process::exit(1);
                });
            match site::validate_access_key(&typed) {
                Ok(k) => break k,
                Err(why) => error!("{why}"),
            }
        };
        (target, key)
    } else {
        let options = vec!["Underwater Hockey", "Underwater Rugby"];
        let sport_choice = Select::new("Select the sport for the schedule:", options)
            .prompt()
            .unwrap_or_else(|_| {
                error!("No sport selected. Exiting.");
                std::process::exit(1);
            });

        let options = vec!["Production", "Development", "Local"];
        let site_choice = Select::new("Select the uwhportal site to connect to:", options)
            .prompt()
            .unwrap_or_else(|_| {
                error!("No site selected. Exiting.");
                std::process::exit(1);
            });

        let default_url = site::portal_default_url(site_choice, sport_choice);
        let override_var = site::override_var_name(sport_choice);
        let default_require_https = !matches!(site_choice, "Local");
        let (site_url, require_https) = apply_portal_override(
            default_url,
            default_require_https,
            std::env::var(override_var).ok(),
        );
        if site_url != default_url {
            info!("{override_var} active: using {site_url}");
        }
        (
            site::SiteTarget {
                kind: site::SiteKind::Portal,
                base_url: site_url,
                require_https,
            },
            None,
        )
    };

    info!("Using URL: {}", target.base_url);
    info!("Fetching event list from uwhportal...");

    let mut portal_client = UwhPortalClient::new(
        &target.base_url,
        pasted_key.as_deref(),
        target.require_https,
        std::time::Duration::from_secs(10),
    )?;
```

Note the client is now built **with** the pasted key, so a custom site is authenticated from the
first request. `pasted_key` stays in scope for Step 2. Add `Text` to the `inquire` imports if it is
not already there, and drop any import left unused by the removed code.

- [x] **Step 2: Send the right credential prompt**

At `main.rs:515`, the `if !portal_client.has_token()` block asks for an email and password. That is
the portal's login and means nothing to a custom site, so branch on the site kind:

```rust
                if !portal_client.has_token() {
                    match target.kind {
                        site::SiteKind::Custom => {
                            // Custom sites have no organiser login — the access
                            // key is the only credential they issued.
                            let typed = match Text::new(
                                "This step needs an access key. Paste it now:",
                            )
                            .prompt()
                            {
                                Ok(t) => t,
                                Err(_) => {
                                    error!("No access key provided. Please try again.");
                                    continue 'outer;
                                }
                            };
                            match site::validate_access_key(&typed) {
                                Ok(Some(key)) => {
                                    portal_client.set_token(&key);
                                    pasted_key = Some(key);
                                }
                                Ok(None) => {
                                    error!("An access key is needed for this step.");
                                    continue 'outer;
                                }
                                Err(why) => {
                                    error!("{why}");
                                    continue 'outer;
                                }
                            }
                        }
                        site::SiteKind::Portal => {
                            // unchanged: the existing email + password prompt
                            // and login_with_email_and_password call go here
                        }
                    }
                }
```

Move the existing email/password block verbatim into the `Portal` arm. Do not change it.

- [x] **Step 3: Check it builds and the suite is green**

Run: `cargo test -p schedule-processor 2>&1 | tail -5`
Expected: PASS. If `pasted_key` is reported as never read, keep it: it is assigned in Step 2 so the
key survives a later `clear_token`, which is behaviour, not dead code. If the compiler is right that
nothing reads it, delete the field rather than silencing the warning.

- [x] **Step 4: Run the tool down both paths**

This is the verification that replaces a unit test.

```bash
cargo run -p schedule-processor
```

Portal path: choose "UWH Portal" → the sport and environment questions appear exactly as before,
and the log line reads `Using URL: https://api.uwhportal.com`. Ctrl-C is fine once the address is
confirmed.

Custom path: choose "Custom site" → type `scores.example.org` and confirm it is refused with a
message naming `https://`; type `http://localhost:9000` and confirm it is accepted; paste a key
with a trailing newline and confirm it is accepted silently; paste `abc"smart-quote"123` using a
real `\u{2019}` character and confirm it is refused with a message telling you to copy it again.

Record what you saw in the Deviations section below.

- [x] **Step 5: Full gate and commit**

```bash
just check
git add schedule-processor/src/main.rs docs/superpowers/plans/2026-08-13-schedule-processor-site-selection.md
git commit -m "feat(schedule-processor): ask for the portal or a custom site"
```

---

## Acceptance criteria check (from the spec)

Confirm each before opening the PR:

| # | Criterion | Proven by |
|---|---|---|
| 1 | Site question comes first | Task 4 Step 4, Portal path |
| 2 | Portal path unchanged | Task 1 tests + Task 4 Step 4 |
| 3 | Custom asks address + key, never the sport | Task 4 Step 4, Custom path |
| 4 | `http://` allowed, `https://` requires a certificate | `a_typed_http_address_does_not_require_a_certificate` |
| 5 | Bad paste gives a message, not a crash | `a_key_with_a_character_that_cannot_be_sent_is_refused_in_plain_words` |
| 6 | Blank key accepted | `a_blank_key_means_connect_without_one` |
| 7 | Nothing written to disk | No code writes it; confirm no new file appears after a run |
| 8 | `just check` exit 0 | Task 4 Step 5 |

## Deviations

Record anything that diverged from this plan here rather than in separate commits, per
`.claude/rules/plan-execution.md`.

**1. `pasted_key` is not kept after the client is built (Task 4, Steps 1–3).**
The plan kept it `mut` and reassigned it in Step 2 "so the key survives a later `clear_token`".
The compiler's `value assigned to pasted_key is never read` warning was correct: the token lives
inside `UwhPortalClient` (`set_token` / `has_token` / `clear_token` all read `self.access_token`),
and nothing reads the local back. Per this plan's own instruction, the assignment was deleted
rather than silenced, and the binding is no longer `mut`. Observable behaviour: after an upload
failure calls `clear_token`, the operator is asked for the access key again — which is exactly
what the Portal path already does with email and password.

**2. Task 1's commit carries two transient warnings.** `SiteKind` and `SiteTarget` are defined in
Task 1 but not used until Task 2's tests construct them, so that one commit reports
`enum SiteKind is never used`. Gone from Task 2 onward; the branch as a whole is warning-free and
`just check` exits 0.

**3. The literal trailing-newline paste is not reachable through a terminal (Task 4, Step 4).**
Pressing Enter after pasting a key *is* the submit, so `inquire` never delivers a trailing `\n`
to the program — the case cannot be reproduced interactively by construction. The reachable
equivalent was verified instead: `   goodkey123   ` was accepted and trimmed. The trailing- and
embedded-newline cases remain covered by
`a_pasted_key_with_a_trailing_newline_is_cleaned_up_not_refused` and
`a_key_with_a_character_that_cannot_be_sent_is_refused_in_plain_words`.

**4. Verification order matters.** The bad key must be entered *before* the good one: the key
prompt loops until a key validates, so a valid key ends the prompt and there is no second chance
to test a refusal.

**5. Driven through a pseudo-terminal.** The session had no keyboard attached, so both runs used
`printf '<keystrokes>' | script -qec './target/debug/schedule-processor' /dev/null`.

**6. Extra check not in the plan — mutation of the access-key guard.** Before trusting the Task 3
tests, the character check was disabled (range widened to accept every `char`) and the suite
re-run: exactly one test failed
(`a_key_with_a_character_that_cannot_be_sent_is_refused_in_plain_words`), and the guard was then
restored. This confirms the test fails for the reason it claims to.

### What the run actually showed

Portal path — the new question comes first, then the two original questions unchanged:

```
? Select the site to connect to: UWH Portal
? Select the sport for the schedule: Underwater Hockey
? Select the uwhportal site to connect to: Production
[INFO] Using URL: https://api.uwhportal.com
? Select the event to process:            <- event list fetched successfully
```

Custom path — no sport question anywhere:

```
? Select the site to connect to: Custom site
? Enter the web address of your site: scores.example.org
[ERROR] "scores.example.org" does not look like a web address. It needs to start with
        https:// (or http:// for a site on your own network).
? Enter the web address of your site: http://localhost:9000
? Paste your access key (leave blank if your site needs none): abc’123
[ERROR] That access key contains a character that cannot be sent to the site ('’').
        Copy the key again, straight from where your site shows it.
? Paste your access key (leave blank if your site needs none):    goodkey123
[INFO] Using URL: http://localhost:9000
```

The smart quote produced a sentence and a fresh prompt, not a crash — the whole point of Task 3.

**Cosmetic note, not changed:** the refusal renders the offending character as `('’')` because
`{bad:?}` adds its own quotes inside the parentheses. It is legible and it does show the
character, but the doubled punctuation is slightly awkward if a plainer `’` is preferred.
