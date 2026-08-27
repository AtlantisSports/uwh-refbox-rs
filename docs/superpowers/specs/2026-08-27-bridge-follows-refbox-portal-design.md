# The bridge follows refbox's portal

Status: approved in conversation 2026-08-27 (Eric), not yet built.
Supersedes: the earlier approved task "show which portal the bridge is using on the status page".

## 1. Why this exists

On 2026-08-26 the bridge served real team names from the wrong tournament, with no error
anywhere. Event ids are not unique across portal environments: `1889-B` is Kings Cup on the
development portal and "20th Annual Battle @ Altitude" on production. refbox was launched against
the development portal; the bridge was left on its own default, production. Both fetched
`events/1889-B`. Both succeeded. The names on the graphic were real people from a real event --
just not the event being played.

## 2. What is already synced, and what is not

The event id is **already** taken from refbox. refbox stamps its current event onto the game-state
update it broadcasts (`refbox/src/app/mod.rs:742`), and the bridge reads it out of the feed
(`overlay-bridge/src/server.rs:890`); the bridge has no event-id setting of its own. When the
event id changes the bridge rebuilds its team and roster cache
(`overlay-bridge/src/server.rs:901-928`).

The portal *address* is the unsynced half. It comes only from the bridge's own `--portal-url`,
defaulting to production (`overlay-bridge/src/main.rs:33`, and the test at `:209-211` pinning that
default). So the incident was not two settings disagreeing: it was one correct event id, looked up
on a portal refbox was not using.

## 3. Scope boundary

In scope:

- One new optional value on the shared game-state type, carrying the address refbox's own portal
  client is pointed at.
- refbox filling it in.
- The bridge using it, and losing `--portal-url`.

Explicitly out of scope:

- **No display.** Eric's call, 2026-08-27: "I don't know that these need to be displayed, just
  properly used." The status page is unchanged. Once the bridge can only look where refbox is
  looking, there is no divergence left for an operator to check.
- **No auto-detection, and no change to any default.** Nothing guesses an environment from an
  event id, a hostname, or anything else.
- **No tournament name.** The public schedule response the bridge already fetches carries only
  `games`, `teams` and `courtNames` (verified against the live capture in
  `overlay-bridge/tests/fixtures/schedule-response.json`). Showing "Kings Cup" would need a portal
  call the bridge does not make. Possible follow-up; not invented here.
- **No second field describing the *kind* of site.** Nothing needs it.

## 4. The change

### 4.1 `uwh-common` -- one new field

`GameSnapshot` (`uwh-common/src/game_snapshot.rs:42`) gains:

```rust
pub portal_base_url: Option<String>,
```

Named from the code's own vocabulary, not invented: the shared module is `uwhportal`, its client
holds a `base_url` (`uwh-common/src/uwhportal/mod.rs:159`), and a custom site is served through
that same client with a different base. So this field is literally "the `base_url` this refbox's
`UwhPortalClient` is using".

`GameSnapshot` is the `std`-only struct, so a `String` is allowed. The field is deliberately NOT
added to `GameSnapshotNoHeap`, the compact `no_std` form the LED panel reads: the panel has no use
for it and no heap to hold it. The LED wire format is untouched.

### 4.2 `refbox` -- the two outbound paths

There are **two** places refbox hands a snapshot to the update sender, not one -- worth stating
because a change made in only the obvious one would look complete while leaving a second path
sending updates with no address attached.

**The game path** (`refbox/src/app/mod.rs:745`, inside `apply_snapshot`) gains one line beside the
line that already stamps the event id:

```rust
new_snapshot.portal_base_url = Some(self.current_site.base_url.clone());
```

`current_site` is described in refbox's own code as "where `uwhportal_client` actually points,
kept in step with every change" (`refbox/src/app/mod.rs:170-173`). It already accounts for the
`UWH_PORTAL_URL_OVERRIDE` / `UWR_PORTAL_URL_OVERRIDE` developer override, for Hockey vs Rugby mode
resolving to different portals, and for a hand-typed custom site
(`refbox/src/app/mod.rs:472-499`). Nothing is recomputed and no new setting is introduced: refbox
reports the value it is already using, which is what makes disagreement impossible rather than
merely unlikely.

It cannot be blank: `portal_target` always produces a non-empty base, and `parse_custom_site`
rejects empty input, a missing host and any unsupported scheme
(`refbox/src/app/custom_site.rs:69-92`). No empty-string special case is added, because none can
occur -- and a malformed value from a hostile producer degrades on its own (an unusable address
fails the request, which the bridge already treats as a failed fetch).

**The beep-test path** (`refbox/src/app/mod.rs:5474`) synthesizes a `GameSnapshot` from a
beep-test snapshot purely to drive the LED panel, using `..Default::default()`. It is left
**unchanged**: during a beep test there is no game, no event and no portal call, so `None` is the
honest value. It needs no compile fix either, since it already falls through to the default.

No overlay or bridge is used during a beep test (Eric, 2026-08-28), so this is not a scenario to
design around. It is recorded because it is why the field must be optional at all, and why this
second path needs no change.

### 4.3 `overlay-bridge` -- use it, and drop the flag

- `--portal-url` is removed, and typing it becomes an error. This mirrors the existing treatment of
  `--white-on-right` and `--court`, which have a test asserting they are rejected
  (`overlay-bridge/src/main.rs`, `the_removed_side_of_pool_and_court_flags_are_rejected`): a stale
  shortcut must fail loudly, not appear to work.
- Nothing to migrate in the saved settings: `--portal-url` was never persisted. `config::Settings`
  holds only `refbox_host`, `refbox_port` and `port`.

**The rebuild rule, restated as one rule.** Today the bridge rebuilds its directory when a
*different known* event id arrives, and deliberately ignores a `None` on the wire so a transient
gap cannot throw away a cached event's names (`overlay-bridge/src/server.rs:901-928`). Rather than
bolt a second condition beside it, the directory becomes identified by the **pair** it is built
from:

- Remember the two halves of the identity **jointly**, and only from a snapshot that reported
  both. A gap in either half then leaves the whole remembered pair standing -- which is what the
  behaviour table requires for a momentary absence of *either* value.

  **This corrects an earlier version of this bullet**, which said to remember the address "the same
  way the event id is already remembered" -- that is, independently, under its own `is_some()`
  guard. The final review showed that independent halves can be married into a pair that was never
  reported together, reproducing the original incident: the bridge holds (dev, `1889-B`); the
  operator relaunches refbox against production, which without the override also wipes
  `portal_link.json`, so refbox comes up with no event selected; refbox stamps the address
  unconditionally (`refbox/src/app/mod.rs:748`), so its first snapshot is (production, *no event*);
  the address half updates alone; and `events/1889-B` is then fetched from production.
  `AppState::forget_game` does not save us -- it has exactly one call site, an operator *switching*
  refbox (`overlay-bridge/src/server.rs:638`), not a refbox restart, and nothing clears the
  remembered values on a disconnect. The same shape arrives from Manual-mode startup and from a
  Hockey/Rugby mode switch, both of which clear the event id while an address is still reported.
  Worse: neither this spec's own acceptance walkthrough nor any test written from it could tell the
  two rules apart, because step 2 relaunches against production with the *same* event id -- which
  is what the mixed pair produces anyway.
- `Directory` already holds the address and event it was built from, so it is its own record of
  that pair. Rebuild when the remembered pair is fully known and differs from the running
  directory's own. That single comparison covers the first address arriving, the address changing
  and the event changing; it cannot fire on an absent value, because it reads the remembered
  values and never the raw wire ones; and it needs no separate bookkeeping kept in step by hand.
- Until both halves are known, the bridge fetches nothing: team and player names stay blank. No
  default, no guess, no fallback.
- Choosing a different refbox already forgets the remembered event (`AppState::forget_game`), and
  with the rule above that is sufficient: the new refbox's first snapshot re-establishes the pair,
  and nothing is fetched before it does. All refboxes at an event point at the same portal (Eric,
  2026-08-28), so a mid-event switch does not change the address at all.
- The last known address **holds** through a refbox dropout, exactly like every other value the
  bridge serves. A quiet refbox never blanks it. (The bridge goes silent for ~25s on a stopped
  clock; silence never means disconnected.)

## 5. Compatibility, both directions

Verified, not assumed:

- **New refbox, older consumer** (a v0.5.0 stream overlay reading a newer refbox): safe *against
  unknown fields*. Nothing in `uwh-common` uses `serde(deny_unknown_fields)`, so an unrecognised key
  is ignored -- verified against the shipped v0.5.0 tag, not only against current source. The
  overlay deserializes the feed at `overlay/src/network.rs:500`.

  **But the mechanism that actually threatens that consumer is size, not field names, and this
  change spends part of the remaining margin.** The deployed Pi overlay reads into a fixed
  1024-byte buffer with no newline framing, and per this repo's own
  `docs/backlog/overlay-feed-reader-defects/NOTE.md` (defect 2), a line over 1024 bytes does not
  cost one dropped update -- the reader misaligns and stops for the rest of the game. This field
  adds ~46 bytes to *every* line (50 for the dev host, more for a long custom site). Against the
  repo's own 7-entry capture (793 bytes), headroom falls from ~231 bytes to ~181: roughly four more
  recorded fouls/warnings entries, down to roughly three. Nothing breaks today, because that note
  also records that fouls and warnings have not yet been used in earnest. What it does mean is that
  the note's own conclusion -- fix the reader before that rollout, not after -- has less slack than
  when it was written. Whether the reader fix now moves ahead of the fouls/warnings work is Eric's
  call, recorded here rather than silently absorbed.
- **Older refbox, new bridge**: safe, and blank rather than wrong. A missing field reaches serde's
  `missing_field`, whose deserializer implements only `deserialize_option` and returns `visit_none`
  (`serde-1.0.228/src/private/de.rs:24-49`; 1.0.228 is the version this workspace resolves in
  `Cargo.lock`). So the value arrives as `None` -- absent, never substituted -- and the bridge
  fetches nothing. No `#[serde(default)]` needed.
- **Nothing persisted**: `GameSnapshot` is never written to disk by refbox, so there is no saved
  file format to migrate.

## 6. Blast radius and cost

`uwh-common` is the highest-blast-radius crate in the workspace, so this is **heavy process** per
`.claude/rules/plan-execution.md`: per-task verification, per-task review, strict deviation
tracking.

The mechanical cost: 65 places build a `GameSnapshot` by hand, across 12 files in `uwh-common`,
`refbox`, `overlay` and `overlay-bridge`. A large share already end in `..Default::default()` and
need no change; the rest stop compiling until the field is added. The exact split is whatever the
compiler reports -- the point is that it is mechanical and exhaustively found, never silent.

## 7. Behaviour table

| Situation | What the bridge does |
|---|---|
| Never connected to a refbox | No address, no event: fetches nothing, names blank |
| Connected, refbox reports address + event | Fetches from that address for that event |
| Connected, refbox too old to report an address | Fetches nothing, names blank (not production) |
| refbox reports a different address **together with an event** | Cache rebuilt for that whole pair |
| refbox reports a different event **together with an address** | Cache rebuilt for that whole pair |
| refbox reports a new address with **no** event (a restart before an event is chosen) | Cache KEPT. The new address is never married to the previously remembered event — that is the incident vector, see §4.3 |
| refbox momentarily reports no event | Cache kept (unchanged from today) |
| refbox momentarily reports no address | Cache kept — one joint rule covers both halves |
| refbox enters beep-test mode | Not a scenario: no overlay or bridge is used during a beep test |
| Operator points the bridge at a different refbox | Remembered event forgotten; the new refbox's first snapshot re-establishes the pair |
| refbox goes quiet (stopped clock, dropout) | Last known address and names hold |
| refbox is on a hand-typed custom site | Bridge follows it there, unauthenticated |

## 8. What changes for vMix

This is the one place a served value can change: team names can go from wrong-tournament to
correct, or (against a refbox older than this change) from wrong-tournament to blank. That is the
purpose of the change, but it is a deliberate exception to "the bridge serves what it served
before" and is recorded here as such.

## 9. Testing

Writable unit tests, all of them regression guards for something above:

- `update_sender`: the full-snapshot test carries a populated address, so the JSON path's length
  accounting is exercised with a real string rather than a `null`
  (`refbox/src/app/update_sender.rs:806` onward). **Corrected after the final review:** this test
  does NOT guard the field's presence on the wire, as an earlier version of this section claimed.
  It builds its expectation with the same `serde_json::to_string` call the code under test uses, so
  it is invariant to *which* fields serialize and would still pass with `#[serde(skip)]` on the
  field. The real wire-presence guard is the missing-key test in
  `uwh-common/src/game_snapshot.rs`, which removes the key from a genuine serialization and asserts
  it comes back absent.
- The compatibility pair: a snapshot line **without** the field deserializes with the address
  absent; a line carrying an unknown extra field still deserializes.
- The bridge: no address reported means no portal request is made at all -- specifically not one to
  production.
- The bridge: same event id, different address, rebuilds the directory.
- The bridge: an absent address (an older refbox, or any transient gap) does **not** rebuild and
  does **not** clear the cached names.
- The bridge: choosing a different refbox re-establishes the pair from that refbox's first
  snapshot rather than continuing to serve the previous one's names.
- The bridge: `--portal-url` is rejected.
- The incident itself, as a guard: given a feed reporting the development portal, the request URL
  the bridge builds is the development one.

**Not unit-tested, deliberately:** that refbox stamps `current_site` onto the game path. The stamp
is a field read inside a method on `RefBoxApp`, which no test constructs (the type owns the portal
client, the update sender and the sound controller). Extracting a helper to make it testable would
buy a test asserting that a function copies what it is handed, while the thing that could actually
go wrong -- passing the *wrong source* -- lives in the untested caller either way. It is covered by
the acceptance walkthrough below instead, and that coverage is real: step 2 fails loudly if the
stamp is missing or stale.

## 10. Acceptance -- what Eric can observe

1. Launch refbox against the development portal with an event selected; start the bridge with no
   portal flag at all. The names vMix receives are the development portal's.
2. Stop refbox, relaunch it against production with the same event id, leaving the bridge running
   and untouched. The names follow to production. Under today's code they would have stayed
   production in step 1 and been wrong.
3. `--portal-url` on the bridge is refused with a clear message.
4. The stopped-clock test still passes: stop the clock 30+ seconds; the dot stays green and every
   value holds.

## 11. Rejected

- **Keep `--portal-url` as an override, preferred only when refbox reports nothing.** Rejected: it
  restores two sources of truth for one value, which is the exact structure that caused the
  incident. One rule, one implementation.
- **Display the portal on the status page instead of syncing it** (the previously approved task).
  Rejected by Eric 2026-08-27: it makes the mistake visible rather than impossible.
- **Have the bridge read refbox's own settings file.** Rejected: the bridge runs on the vMix PC,
  usually a different machine from the refbox, so it would work only in the local-testing case --
  and it would couple two programs through a file neither documents to the other.
- **Send the *kind* of site (official portal vs custom) as a second field.** Rejected: nothing
  needs it, and nothing displays it.
- **Forgetting the remembered address when a different refbox is chosen.** Considered and dropped:
  the pair comparison re-establishes itself from the new refbox's first snapshot anyway, and all
  refboxes at an event point at the same portal (Eric, 2026-08-28), so the case it guarded against
  does not arise.
- **Treating beep-test mode as a state the bridge must ride out.** Dropped: no overlay or bridge is
  used during a beep test (Eric, 2026-08-28). The beep-test path still reports no address, which is
  simply the honest value; nothing is designed around it.
- **Add a second rebuild condition beside the event-id one.** Rejected in favour of the single
  pair comparison in section 4.3: two conditions over the same cache is how the momentary-`None`
  protection gets forgotten on one of them.
