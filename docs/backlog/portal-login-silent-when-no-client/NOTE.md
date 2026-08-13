# Backlog: the portal login silently does nothing when the refbox has no client

**Status:** NOT FILED, not started. Local note only.
**Surfaced:** 2026-08-13, while source-tracing finding 3 of the code-quality inventory
(`git show 87be104b:docs/audit-archive/2026-08-11-ai-slop-inventory.md`) and designing the fix in
`docs/superpowers/specs/2026-08-13-degraded-portal-startup-message-design.md`.
**Raised by:** Claude, during the trace. Every claim below was read in code, not assumed.

## The gap

If the refbox starts without a portal client — the "degraded mode" path, realistically caused by a
broken system certificate store on a Pi — the portal login flow is still fully reachable from
Settings, and it **fails completely silently**.

What happens when the operator enters their code:

1. The login keypad marks the request as sent (`refbox/src/app/mod.rs:4268`, `*requested = true`).
2. It calls `request_uwhportal_token` (`refbox/src/app/mod.rs:991-1011`), which is wrapped in
   `if let Some(client)` and returns `Task::none()` when there is no client.
3. No network request is made, so no reply message is ever produced.
4. Nothing updates the screen. No success, no error, no timeout, no log line the operator sees.

The comment at `refbox/src/app/mod.rs:4288` states the reply "will replace this once the network
request completes". It never completes. The operator types a code from the portal website, presses
Done, and the refbox behaves as though they did nothing at all.

## Why this is NOT covered by the degraded-startup fix

The 2026-08-13 fix stops the refbox *pushing* the operator into this login — it removes the false
"Access token expired — tap to re-login" prompt. It does not close the dead end itself. An operator
who sees the red dot and goes looking in Settings for a way to fix it will still land here.

## The ask

When there is no portal client, the login attempt should tell the operator something instead of
nothing. Anything truthful is an improvement: an error state on the keypad, or the login entry
point being unavailable with a reason.

## Scope when picked up

- `refbox/src/app/mod.rs` only, around the keypad-Done handler and `request_uwhportal_token`.
- Decide with the human whether the better shape is (a) the login attempt reporting a failure, or
  (b) the login entry point being closed off while no client exists. This is an operator-experience
  call, not a technical one.
- Check the same silent-no-op shape in the sibling request helpers in that file: `request_schedule`
  and the event-list fetch use the identical `if let Some(client) … else Task::none()` pattern.
  Whether their silence is equally harmful was **not** traced — verify before assuming.

## Related, and deliberately not fixed in the 2026-08-13 branch

Two code comments misdescribe when degraded mode is entered, and would mislead the next reader:

- `refbox/src/app/mod.rs:2389` — "only possible on a bad https-only config"
- `refbox/src/app/mod.rs:1112` — "only reachable from a bad https-only config"

Both are wrong. The https-only setting is a stored flag enforced per request
(`reqwest 0.12.23`, `src/async_impl/client.rs:2135-2136` sets it, `:2511-2512` enforces it), so
such a configuration builds a client successfully and fails each individual call instead. Every
real failure path inside that constructor is TLS/certificate related — e.g. "zero valid
certificates found in native root store". Correcting the comments is a two-line documentation fix
that belongs with whichever branch next touches this area.

## Explicitly NOT part of this

- Not the degraded-startup message itself (that is the 2026-08-13 spec, in progress).
- Not making the portal work without a client — it cannot; this is about honest feedback.
- Not ADR 011's missing failure counter (inventory finding 5) or the unreachable yellow indicator
  state (finding 4).
