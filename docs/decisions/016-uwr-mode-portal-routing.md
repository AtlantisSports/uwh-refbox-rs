# 016 — UWR Mode Portal Routing

**Date:** 2026-04-23
**Status:** proposed

## Context

The refbox has always supported two modes in its `Mode` enum:

- `Mode::Hockey6V6` and `Mode::Hockey3V3` — underwater hockey.
- `Mode::Rugby` — underwater rugby.

The two sports are hosted on separate portal tenants that share a
codebase but deploy under different domains (per the
`uwh-portal` repo's `deploy/README.md`):

| Tenant | Web UI | API |
|--------|--------|-----|
| UnderwaterHockey | `uwhportal.com` | `api.uwhportal.com` |
| UnderwaterRugby  | `uwrportal.com` | `api.uwrportal.com` |

Staging/dev mirror the same structure (`dev.uwhportal.com` /
`api.dev.uwhportal.com` on the UWH side; a parallel UWR set exists).

The refbox's portal base URL is stored in `refbox/src/config.rs` as
a single `url: String` on the persisted `UwhPortal` config struct,
defaulting to `"https://api.uwhportal.com"`. Nowhere in the
codebase does `Mode` influence which URL is used. Consequently:

- A refbox instance running in `Mode::Rugby` still talks to the
  UWH portal (either production or whatever `UWH_PORTAL_URL_OVERRIDE`
  is set to).
- Login attempts, score submissions, and schedule fetches against
  the UWH portal from a UWR-mode refbox either 404, fail auth, or
  submit to the wrong tenant's database.
- The operator has no obvious signal that this is wrong, because
  the refbox's login screen does not display the target URL, and
  Task 22's portal health indicator (ADR 011) surfaces the symptom
  only after the operator has linked an event — which they cannot
  successfully do in the first place against the wrong portal.

The portal health indicator feature (commit `fd95806` on
`feat/refbox/portal-health-indicator`) addresses the *visual* side
— the status tile displays the UWR logo in Rugby mode, and the
three portal-related user-facing strings switch from "UWH" to "UWR"
in their sport-prefix label. That commit explicitly does **not**
change which URL the refbox contacts. The URL side is tracked here
because it has broader implications than a view-builder change.

## Open design questions (to resolve during implementation)

### Configuration shape

Today, `UwhPortal::url` is a single string. Options for UWR awareness:

- **A — two fields, runtime selection.** Add `url_uwr: String`
  alongside the existing `url`, and the portal client construction
  code picks the right field based on `self.config.mode` at
  construction time. Backwards-compatible with existing config
  files (the new field defaults to `https://api.uwrportal.com`).
- **B — one field, selected at config-load time.** Replace the
  single `url` with a `ParticipantRoleDictionary`-style
  per-sport lookup. Requires a config migration and is a bigger
  change but puts the sport-to-URL relationship in the data model
  rather than as application logic.
- **C — derive from mode, no user-facing field.** Remove the
  stored URL entirely and hard-code
  `match mode { Hockey* => "https://api.uwhportal.com",
  Rugby => "https://api.uwrportal.com" }` at the client
  construction site. Simplifies the happy path but loses the
  ability to redirect to a staging/dev/self-hosted instance
  without an environment variable override.

Option A is the smallest migration and preserves the existing
per-environment override knob.

### Mode switch at runtime

The refbox allows the operator to change `Mode` at runtime via the
App Options page. If the portal is currently linked to an event on
the UWH tenant and the operator switches to Rugby, the
saved `current_event_id`, queued retry items, and cached portal
token become stale against the newly-selected UWR portal. The
switch must either:

- **Unlink automatically** — clear `current_event_id`, drop the
  queue, invalidate the token — and prompt the operator to re-link
  against the UWR portal. Matches the dormant-until-linked rule
  documented in ADR 011 amendment 2026-04-23.
- **Block the switch** — require the operator to unlink manually
  before changing mode. More conservative; avoids silent data
  loss if an unlink-on-switch surprises them.

Unlink-automatically is more ergonomic but has a larger blast
radius. Blocking-the-switch is safer and easier to explain.

### Queue file semantics across tenants

`refbox/src/portal_manager/queue.rs` persists queued retry items
keyed on `(event_id, game_number)`. The `event_id` does not
encode which tenant the event belongs to. If an operator switched
mode with items in the queue, retrying them against the wrong
tenant would produce nonsensical errors. This ADR's implementation
must either:

- Tag each queued item with its tenant (schema version bump on
  the queue file), or
- Enforce a clean queue at mode-switch time (tied to the
  unlink-on-switch decision above).

## Out of scope for this ADR

- Changing which keys Fluent serves in UWR vs hockey mode. That
  work landed separately on the portal-health-indicator branch
  (commit `fd95806`), using the existing Fluent `{ $portal }`
  variable. This ADR does not revisit those keys.
- LED-panel, overlay, and wireless-remote interactions with UWR.
  Those subsystems are independent of the portal and already
  respect `Mode::Rugby` for game-rules purposes.

## Sequencing

This ADR sits behind the `feat/refbox/portal-health-indicator` PR.
Recommended order:

1. Land ADR 011's portal health indicator PR (makes the symptom
   visible; includes the logo and text-prefix changes).
2. On a new branch `fix/refbox/uwr-mode-portal-url` (or similar,
   scope chosen per the branch naming convention), pick one of the
   three config-shape options above, implement, write operator-
   facing changelog text, land behind its own PR.

## References

- `refbox/src/config.rs` — `UwhPortal::url` default and load path.
- `refbox/src/app/mod.rs` — `UwhPortalClient::new` construction
  site (around line 514) and `UWH_PORTAL_URL_OVERRIDE` read.
- `uwh-portal/deploy/README.md` — tenant URL matrix confirming
  both sports have their own API domain.
- `docs/decisions/011-portal-health-indicator.md` — the feature
  that surfaced the routing gap and added the partial (visual-only)
  mode awareness.
