# 006 — Multi-remote alarm buttons

**Date:** 2026-04-18
**Status:** proposed

## Context

v0.4.0 (PR #718) added a single on-screen "Alarm" button that triggers the manual
buzzer. In practice a tournament deployment often has multiple paired wireless
remotes (one per referee, up to 4), each with its own buzzer sound assigned in
the "Manage Remotes" config page.

Today the operator has no way to manually trigger a specific remote's sound from
the refbox — the single on-screen alarm plays one fixed sound, and there is no
on-screen indication of which remote button a referee just pressed. Two
shortcomings follow:

1. If the operator needs to replace a missed remote press with the matching
   sound (e.g. a dead battery, a muffled underwater button), there is no way
   to do so — the one on-screen alarm plays the wrong sound.
2. When a referee presses a remote and nothing happens, the operator cannot
   tell whether the radio link is at fault or the buzzer itself is — there is
   no visible confirmation that the refbox received the packet.

All underlying plumbing already exists:

- The LoRa frame carries a 4-byte `RemoteId` per packet (see
  `refbox/src/sound_controller/button_handler/mod.rs`).
- Config stores `Vec<RemoteInfo>` with per-remote sound assignments.
- Sound dispatch already looks up the remote ID and plays its configured sound.

What is missing is only the refbox UI wiring.

## Decision

Replace the single manual alarm tile with a dynamically-sized row of up to 4
tiles, one per paired remote. Each tile both **triggers** and **reflects** the
state of its remote:

1. **Tile count matches the number of paired remotes.** 0 paired → no tiles
   (same as manual alarm disabled); 1 paired → one tile (same visual footprint
   as today); 2–4 paired → a row of same-width tiles.
2. **Pressing a tile** (mouse or touch) triggers that remote's configured sound,
   as if the physical remote had been pressed.
3. **Incoming radio packets visually flash the matching tile** in the "pressed"
   state briefly — the same visual as an on-screen press. This gives the
   operator immediate visual confirmation that the radio path is working.
4. **Keyboard:** digits `1`–`4` trigger their corresponding tile. Spacebar
   behaviour is an open question — see below.
5. **Tile label:** the remote's assigned-sound name by default; optionally
   an operator-set short label captured when pairing.
6. **Active-play red/blue colour logic** (from PR #718) applies identically
   across all tiles.

## Open design questions (to resolve during implementation)

- **Spacebar:** retire entirely, keep as "trigger primary remote," or rebind
  to "trigger all remotes"? Operators under poolside stress benefit from one
  large target; retiring spacebar may cost more than it gains.
- **Label strategy:** sound-name labels are self-documenting but long. A
  short operator-set label is cleaner but adds a config step during pairing.
- **Keyboard collisions:** verify `1`–`4` are not bound elsewhere on the main
  page before claiming them.

## Consequences

**Becomes easier:**

- Operator can manually trigger the exact sound associated with any specific
  remote without reconfiguration.
- Dead-remote diagnosis is possible at a glance: if no tile flashes when a
  referee presses their button, the radio path is at fault.

**Becomes harder / constrained:**

- The main-page layout must accommodate a variable-width alarm region.
- Per-remote pairing UX gains importance: if a tile's label is meaningless at
  poolside, the feature's diagnostic value is reduced. May pull forward the
  need for operator-facing remote labels.

**Scope:**

- Refbox-only change. No touches to `uwh-common`, `wireless-modes`, or the
  wireless-remote firmware.
- Estimated effort: 1–2 days once design questions are resolved.

## References

- PR #718 — original manual alarm button feature.
- `refbox/src/sound_controller/button_handler/mod.rs` — LoRa frame handling,
  per-remote ID extraction.
- `refbox/src/sound_controller/mod.rs` — `RemoteInfo`, per-remote sound
  configuration, `WirelessRemoteReceived` dispatch.
- `refbox/src/app/view_builders/configuration.rs::make_remote_config_page` —
  existing remote-management UI.
