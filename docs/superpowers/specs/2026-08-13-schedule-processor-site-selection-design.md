# Schedule processor: choose UWH Portal or a custom site

**Date:** 2026-08-13
**Crate:** `schedule-processor` only
**Status:** approved design, not yet implemented
**Related, deliberately separate:** shipping this tool as a release artifact — its own spec, not
started. See "Why this is only half the story" below.

---

## Why this exists

The schedule processor can already be pointed at any address, but only by setting the
`UWH_PORTAL_URL_OVERRIDE` environment variable before launching. That is a reasonable affordance
for a developer and an unreasonable one for a tournament organiser with a laptop, who is the
tool's actual audience.

The refbox already models this properly: an operator picks the built-in portal or a custom site,
and types the address for a custom site. This brings the schedule processor into line with that,
so the two tools present the same idea the same way.

## What the tool does today, verified

- It is already interactive, using terminal prompts.
- It asks the sport (Underwater Hockey / Underwater Rugby), then the site (Production /
  Development / Local), and derives the address from that pair.
- `UWH_PORTAL_URL_OVERRIDE` (or `UWR_…` for rugby) replaces the derived address when set.
- Credentials: an organiser's email and password are prompted **only when an upload needs them**,
  exchanged for a key held in memory, and never written to disk. On upload failure the key is
  discarded.
- The sport choice feeds **nothing** except the derived address and the name of the override
  variable. It has no other effect anywhere in the tool.

That last point is load-bearing for the design below and was checked, not assumed.

## Decisions taken

| Decision | Choice |
|---|---|
| Site model | Ask "UWH Portal or Custom site" first, mirroring the refbox |
| Portal environments | Keep Production / Development / Local as a second question |
| Custom credentials | Paste an access key |
| Credential storage | Nothing stored, asked each run — already the behaviour today |
| Sport question for custom | Skipped |

## The design

### The flow

```
Select the site to connect to:
  > UWH Portal
    Custom site
```

**Portal** — unchanged from today. Environment, then sport, then the derived address, with the
existing override still applying. No existing workflow changes.

**Custom** — asks for the address, then the access key. The sport question is skipped, because the
only thing it feeds is the address the operator has just typed.

Whether a valid certificate is required is taken from the address as typed, reusing the rule the
override already applies: `https://` requires one, `http://` does not. This keeps a self-hosted or
on-LAN site usable without a special flag.

A blank access key is accepted and means "connect without one", so read-only work against an open
site still functions. If an action later needs a key and none was given, the tool asks for the
**access key** again at that point — not for an email and password, which belong to the portal
path and would mean nothing to a custom site.

### Credentials

Portal is untouched: email and password, prompted lazily, held in memory, discarded on failure.

For a custom site the pasted key is set on the client directly. The client already supports this,
so no shared-crate change is required.

An access key is chosen over email and password because a custom site is a third-party system
implementing the refbox contract. That contract never asked such a site for an organiser login
endpoint, so requiring one could fail against a site that is otherwise fully compliant. Pasting a
key asks nothing of the third party beyond honouring the key it already issued.

### The paste hazard

`uwh-common` builds its authorization header with
`HeaderValue::from_str(format!("Bearer {token}")).unwrap()`. That panics on any character invalid
in an HTTP header — a trailing newline from a copy-paste, a smart quote, a stray space.

Today the tool's key always comes from a login response, so it is server-clean and the panic is
unreachable. **A paste path makes it reachable with human input.**

So the pasted key is trimmed and validated before use, and a key that cannot form a header is
rejected with a plain message asking the operator to re-copy it.

The `unwrap` itself is deliberately **not** fixed here. It is reachable from the refbox's own
custom-site key entry, which makes it a shared-crate concern with a wider blast radius, its own
branch, and its own testing against the refbox. Fixing it inside this feature would bundle a
shared-crate change into a single-crate one.

### Code shape

Site selection becomes one function returning a resolved target — address, certificate
requirement, and how to authenticate — named to mirror the refbox's own model. The address
derivation and key validation become unit-testable, which the current inline prompt chain is not.

## Acceptance criteria

Observable or runnable:

1. Launching the tool asks whether to use the UWH Portal or a custom site, before anything else.
2. Choosing UWH Portal reproduces today's behaviour exactly: same environment question, same sport
   question, same address, and the existing override still works.
3. Choosing Custom asks for an address and an access key, and does not ask for the sport.
4. An address typed as `http://…` connects without requiring a valid certificate; `https://…`
   requires one.
5. A pasted key with a trailing newline or an invalid character produces a readable message, not a
   crash.
6. A blank access key is accepted, and work that needs no key proceeds.
7. Nothing is written to disk: after a run, no file holds the address, the key, or the password.
8. `just check` exit 0, all existing tests green.

## Out of scope

- Shipping the tool as a release artifact (separate spec).
- The `unwrap` in `uwh-common` (separate branch, refbox testing required).
- Renaming the executable.
- Scoresheet rendering, schedule validation, coin flips, uploads, and the CSV path.
- Remembering the address or key between runs — explicitly rejected: asked every run, stored never.
- Hiding the Development and Local environments from a shipped build. That is a packaging question
  and belongs to the release spec.

## Testing approach

Unit-testable, and tested: address derivation for every combination of site kind, environment and
sport; certificate requirement derived from the typed scheme; access-key validation including the
trailing-newline case that would otherwise panic; blank key accepted.

Not unit-testable: the prompts themselves, which need a terminal. Those are verified by running
the tool — once down the Portal path confirming nothing changed, once down the Custom path against
a local address.

## Why this is only half the story

The reason for this change is that other organisers should be able to use the tool, and that also
requires shipping it — it appears in no release workflow today. That work is deliberately separate
because it touches CI and cross-compilation, which this project treats as shared infrastructure.

One finding from this design work belongs to that spec: **scoresheet generation on macOS looks
broken for a shipped build.** The browser lookup handles Windows properly, with explicit Chrome
install paths and `microsoft-edge` as a fallback, but macOS resolves names via `PATH`, where
Chrome does not appear — it lives inside an application bundle. A Mac organiser would likely get
no scoresheets without setting an environment variable.
