# Access key header panic — design

**Date:** 2026-08-14
**Branch:** `fix/uwh-common/access-key-header-panic`
**Crates touched:** `uwh-common` (primary), `schedule-processor`, `refbox`

---

## The defect

`uwh-common/src/uwhportal/mod.rs`, inside `authenticated_request`:

```rust
HeaderValue::from_str(&format!("Bearer {}", token)).unwrap()
```

`HeaderValue::from_str` rejects any character an HTTP header cannot carry. The
`unwrap` turns that rejection into a panic, which takes the whole refbox down
mid-tournament with no message the operator can act on.

**Which characters actually panic** (measured, not assumed — a `reqwest` probe
over `Bearer <key>`):

| Character | `HeaderValue::from_str` |
|---|---|
| newline `\n` | **rejected — panics** |
| carriage return `\r` | **rejected — panics** |
| NUL, DEL | **rejected — panics** |
| curly quote `’` (U+2019) | accepted |
| `é` (U+00E9) | accepted |
| tab | accepted |

This corrects the original brief, which named a newline, a tab and a curly
quote as equally fatal. Only the newline (and its control-character siblings)
ever panicked — which is the likeliest of them anyway, since a key copied from
a web page or a chat message carries a trailing newline.

All 11 authenticated portal calls build their request through this one
function, so every one of them inherits the panic.

## Why it is reachable

The original brief said an operator can paste an access key into the refbox's
custom-site screen. **That is not what master does.** The custom-site page has
exactly one text box — the site address
(`refbox/src/app/view_builders/configuration.rs`, the `custom-site-placeholder`
input) — and it is the only text input in the whole settings UI. The phrase
"access key" appears nowhere in refbox's source or its English text.

`custom_site.token` is written in exactly one place, `refbox/src/app/mod.rs`
(`Message::RecvPortalToken` → `PortalTokenResponse::Success`), and the value
comes from the site's own server answering the link-code exchange (the
`accessKey` field of its reply).

So the panic is reachable by two routes, neither of which is a paste box:

1. **The site's server returns a header-hostile key.** A *custom* site is a
   third party's server. Refbox trusts whatever string it returns and puts it
   straight into the header.
2. **A hand-edited `config.toml`.** `~/.config/refbox/config.toml` holds
   `[custom_site] token = "..."` as plain text. Pasting a key into a config
   file from a chat app or a word processor is exactly how a curly quote or a
   trailing newline gets in.

Both routes end at the same `unwrap`.

## Rejected outright

**Dropping the authorization header when it fails to build.** That would send
the request unauthenticated: reads would quietly succeed with public data and
writes would fail with a permission error, turning a loud crash into a silent
wrong answer. Not an option at any layer of this design.

## Approach

Refuse an unusable key **once, where the key arrives**, rather than at each of
the 11 places it is used.

Considered and rejected:

- *Check at every call* — `authenticated_request` returns a result and each of
  the 11 methods propagates it. The refbox survives, but the operator gets the
  same failure repeatedly and forever (retrying can never fix a bad key), and
  the message arrives detached from the thing that caused it. It also leaves
  eleven places that must each remember to handle the fault.
- *Let reqwest carry the error* — hand the header value to reqwest as text and
  let it report a build error at send time. One line, no signature changes, but
  the message is generic ("builder error"), it still fails on every call
  forever, and nothing in the code records that the key was ever checked.

The chosen shape makes the bad state unrepresentable inside the client: the
client only ever holds a key it has already proved it can send.

## Design

### 1. One shared rule, in `uwh-common`

A public check in `uwh-common::uwhportal` answers one question: *can this key be
carried in a request header?* The rule is the one `schedule-processor` already
uses — printable ASCII only (`' '..='~'`). Everything a real access key
contains (letters, digits, and the punctuation used by base64 and JWTs) is in
that range. The failure names the offending character so the log line and the
operator message can both be specific.

This rule is deliberately **stricter** than the set of characters that actually
panic. It refuses a curly quote and an accented letter too, which `HeaderValue`
would have carried. That is kept on purpose: an access key is base64 or a JWT
and is ASCII by construction, so nothing legitimate is refused, and a key with a
curly quote in it would otherwise be sent and come back as a mystifying
permission error rather than a sentence naming the character. It is also the
rule `schedule-processor` already ships and its tests already assert.

`schedule-processor::site::validate_access_key` keeps its own operator-facing
wording, its own trimming, and its own `Ok(None)` "no key is fine" behaviour,
but delegates the character question to this shared rule so the two cannot
drift apart. Its comment claiming the panic is unfixed becomes false with this
branch and is corrected in the same edit.

### 2. The client stores a finished header, not raw text

`UwhPortalClient` currently stores the key as `Option<String>` and converts it
to a header on every call. It will instead convert **once, when the key
arrives**, and store the finished header value.

- Surrounding whitespace is trimmed before the check, so a key copied with a
  trailing newline works — matching what `schedule-processor` already does for
  a pasted key.
- `UwhPortalClient::new` already returns a pass/fail result, so **its signature
  does not change**; it gains a failure case for an unusable key.
- `set_token` gains a result. On failure the client keeps whatever key it had
  before rather than half-updating.
- `has_token` and `clear_token` are unchanged. Nothing outside the client reads
  the raw key back, so storing the header instead of the string costs no
  caller anything.

The point of this shape: after it, `authenticated_request` performs no
conversion at all, so the 11 calls *cannot* fail this way again. This is a
structural guarantee, not eleven remembered checks.

### 3. What the operator sees

| Route | Behaviour |
|---|---|
| Bad key in `config.toml` at startup | The client is not built, which is the refbox's existing "no portal connection" state: red indicator, nothing sent, never sent unauthenticated. The log line names the bad character. The refbox otherwise runs normally. |
| Site's server returns a bad key after a link code | The existing link-failed dialog appears with a third reason. The key is **not** saved to the config and the token is **not** marked valid, so a bad key cannot lie dormant until the next launch. |

The new English sentence, literal:

```
The site sent an access key this refbox cannot use. Ask the site for the key again.
```

It is added to all 15 locales. The dialog reason is a new refbox-side
confirmation variant rather than a new `PortalTokenResponse` variant: the
server never says "unusable key", the refbox concludes it, and the response
type should keep meaning "what the server replied".

## Files to change

| File | Why |
|---|---|
| `uwh-common/src/uwhportal/mod.rs` | the shared check, the stored header, `set_token` result, tests |
| `schedule-processor/src/site.rs` | delegate to the shared rule; correct the stale comment |
| `schedule-processor/src/main.rs` | the 5 places that set a key now handle a refusal |
| `refbox/src/app/mod.rs` | link-reply handling; new dialog reason; the debug-only key setter; build-failure log wording |
| `refbox/src/app/view_builders/confirmation.rs` | render the new reason |
| `refbox/translations/<15 locales>/refbox.ftl` | one new sentence in each |

Out of scope: the login/token-refresh flow, any other `unwrap` in
`uwhportal/mod.rs` not on this path, and adding an access-key entry box to the
refbox UI.

## Acceptance criteria

1. A refbox launched with a newline in its saved access key **does not crash**.
   On master it does. (A curly quote is refused too, but it never crashed —
   see the table above.)
2. The refbox with a bad key never sends an authenticated call without the
   header — it sends nothing at all and shows the red indicator.
3. A key copied with a trailing newline is accepted, not refused.
4. A good key still produces exactly the header `Bearer <key>`.
5. `schedule-processor` still refuses a bad pasted key with its own sentence,
   with its existing tests unchanged and passing.

## How this is verified

**Tests (`uwh-common`):** a curly quote, a newline and a tab are each refused;
the client's existing key survives a refused update; a trailing newline is
trimmed and accepted; a good key produces exactly `Bearer <key>`.

**Mutation check:** delete the line the check lives on and confirm a named test
goes red. No new test is trusted until it has been seen to fail.

**Downstream:** `just check`, plus `cargo check` for `refbox`,
`schedule-processor`, `overlay` and `led-panel-sim`, plus
`cargo build -p uwh-common --no-default-features` for the no-std guarantee.
(`uwhportal` is already `std`-gated, so no-std is not expected to be affected —
the check confirms it.)

**Live, in the refbox:** put a key containing a newline into
`~/.config/refbox/config.toml` (`token = "abc\n123"` — TOML turns that escape
into a real newline), launch, and trigger an authenticated portal call. On
master: panic. With the fix: a log line naming the character, red indicator,
refbox still running.

The event-list request at startup is **unauthenticated**, so it does not
trigger the crash. Selecting an event does — that requests the schedule, which
is authenticated and is built on the UI thread.

**Known limit:** the config-file route exercises the *startup* path, so the
live run proves the crash is gone but does not display the new dialog. The
dialog reason is covered by test and code reading only. Proving it live would
need the mock portal server to answer a link-code exchange with a bad
`accessKey`, which was deliberately deferred.
