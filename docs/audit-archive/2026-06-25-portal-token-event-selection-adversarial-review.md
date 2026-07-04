# Adversarial Review — Portal Token / Event-Selection Flow

**Date:** 2026-06-25
**Scope:** The "primary function" flow: paste a uwhportal link/token → fetch events → select event/court/game → maintain the live portal link (send scores/stats, show connection health, restore after restart, retry failed sends).
**Review range:** `46ec0973` (last Tristan-authored commit) → `f4d52b21` (master, v0.4.5). All reviewed code is e-straily's; the `portal_manager/` module is entirely new.
**Method:** 7 parallel "attacker" agents (one per failure class) → dedup → every candidate finding independently re-checked by 3 skeptic agents (claim-accuracy / reachability / impact); a finding survives only if ≥2 uphold it.
**Result:** 25 raw → 22 distinct → **19 confirmed, 1 disputed, 2 rejected as false alarms.**

> Note: findings are reasoned from reading the code (each verified by 3 independent agents against the actual source), not all reproduced on hardware. Line numbers are approximate.

---

## Overall assessment

The flow is functional on a good day but **not yet rock-solid for a live tournament.** The confirmed issues cluster in two areas: (1) the on-screen connection/status indicators and buttons can lie or get stuck, and (2) a game whose stats the portal rejects can never cleanly leave the queue. Most confirmed bugs degrade the experience or mislead the operator rather than silently losing the actual game scores.

**Top priorities to fix first:**
1. **The stats-rejection family** (stuck-forever game + mislabeled DISCARD + non-idempotent re-sending) — one root cause: a game is "done" only when BOTH scores and stats succeed, with no give-up and no honest recovery wording. Fixing the design (stats best-effort, scores stand alone) addresses three findings at once. *This matches the already-approved backlog design "portal-stats-best-effort".*
2. **The settings-save crash on a bad SD card** — a worn/full SD card crashes the whole referee screen mid-game on routine actions. One-area fix, high field payoff.
3. **The misleading/stuck indicators** — REFRESH stuck on "Refreshing…", a wifi blip reported as "login expired", APPLY jamming after declining the portal-switch prompt, and the status page breaking exactly when a backlog finishes uploading.

---

## Confirmed findings

### HIGH severity

**H1. Saying "No" to the portal-switch prompt jams the App Options APPLY button**
`refbox/src/app/mod.rs` — apply_game_confirmation DiscardChanges arm (~1203–1216); configuration.rs:200–202, 434–435.
*Operator impact:* Cycle App Mode across a portal boundary (Hockey→Rugby) and press APPLY → "switch portal and restart?" → tap No → you're back on App Options but APPLY is greyed out and stays greyed for any further change. Settings silently won't save; only escape is backing out to the menu and re-opening.
*Why real:* Declining throws away the page's entry snapshot without re-taking one; APPLY only lights when current state differs from the snapshot, so with none it always reads "nothing changed." Sibling paths re-snapshot here; this one doesn't. Same class as fixed PRs #1264/#1275.
*Fix:* Re-capture the page-entry snapshot on the decline path (the call the page-entry/navigate-to-parent paths already make).

**H2. Game-Info REFRESH button can stick on "Refreshing…" forever**
`refbox/src/app/mod.rs` — RequestPortalRefresh (~2742–2751); failure returns NoAction (~715–718); reset only at ~3993–3998 under three guards; game_info.rs:37–41.
*Operator impact:* Tapping REFRESH shows "Refreshing…" and disables the button until done — but it never re-enables if you tapped during a live game (the common case: schedule is fetched/stored but button stays dead), if the network blips, or if the next game number no longer matches the court. Looks hung; only escape is leaving the page.
*Why real:* The flag is cleared in exactly one place, nested under three conditions (no edits in progress, "between games" state, next game found on court). The worst case needs no network problem. The offline case never delivers a result at all. No tick clears it.
*Fix:* Always clear the "refreshing" flag when the attempt ends — on both success and failure, regardless of game period or lookup result.

**H3. Turning the portal OFF mid-game silently turns itself back ON after any restart**
`refbox/src/app/mod.rs` — apply_switch_to_manual_confirmation (~1351–1435), clear_portal_selections_to_manual (~1026–1034); contrast persist_link_session at ~2814; restore block ~1835–1860.
*Operator impact:* Deliberately switch the portal OFF during a game → the saved "which event am I linked to" note is never deleted. After any restart (language change, crash, self-update, power cycle — all routine on the Pi) the portal silently turns back ON: dot reappears, schedule re-fetched, old game re-selected and counting down. Your explicit "off" is undone; scores may resume to an event you meant to disconnect.
*Why real:* Only the normal Apply path deletes the note when portal is off; the mid-game confirmation takes an early exit and skips it. With portal off, the heartbeat that would self-heal is disabled. Startup restore then re-enables everything. The between-games OFF path is fine; only the mid-game confirmation path is broken.
*Fix:* In the mid-game switch-to-manual confirmation (both keep-game and end-game choices), delete the saved-link note when turning the portal off, mirroring the Apply path.

**H4. Retry loop floods the portal and freezes the on-screen "attempt" counter**
`refbox/src/portal_manager/health.rs` — run_task poll loop (~132–167), attempt_item (~192–214), is_item_retry_eligible (~31–39); POLL_INTERVAL=2s, ITEM_RETRY_INTERVAL=15s; portal_detail.rs:150–171.
*Operator impact:* When the portal is unreachable/rejecting, the box re-sends each queued game's scores+stats roughly every 2s instead of ~15s, for up to 30 min per game — on congested poolside wifi this can worsen the outage and risk rate-limiting. The "(attempt N)" counter you're told to watch is frozen at "(attempt 0)" forever, so it looks like nothing is happening. A just-succeeded game can be sent twice in a narrow window.
*Why real:* The background task works from a copy of the queue and never stamps last-attempt-time or bumps the count on the real records, so the 15s throttle is effectively dead and the counter never moves.
*Fix:* Have the retry record its outcome on the authoritative queue (stamp last-attempt-time, bump count) so the throttle engages, the counter is honest, and a just-resolved item isn't re-eligible.

**H5. A game whose stats are permanently rejected gets stuck forever with no honest way to clear it**
`refbox/src/portal_manager/health.rs` — attempt_item (~192–214); mod.rs force_submit (~535–546), retry_all (~564–575), discard (~598–604).
*Operator impact:* A game leaves the queue only when BOTH scores AND stats succeed. For an event configured so the portal permanently rejects stats (real, reproduced — non-unique-cap events), scores go through fine but stats fail every time, so the game never clears. Dot goes yellow then red after 30 min; RETRY/RETRY ALL/FORCE all keep re-failing on stats. Only DISCARD clears it — worded as if abandoning the result — so the operator believes scores were lost (they weren't) and the red dot persists all tournament unless they discard each affected game.
*Why real:* No max-attempts give-up, no "scores done" marker, FORCE applies only to scores (stats has no force). Corroborated by the project's own backlog memory with the exact 400 response from a live test.
*Fix:* Decouple scores from stats — a game is done when scores land, stats best-effort (the approved backlog design). At minimum, stop endlessly re-failing on a permanent rejection and give an honestly-worded way to clear a game whose scores already succeeded.

**H6. Saving settings crashes the whole app if the SD card is full or read-only**
`refbox/src/app/mod.rs` — persist_config (~1494–1496); call sites incl. ApplyConfigPage (~2810), CycleDisplayMode (~2763), language paths (~3595, 4447); contrast graceful restart store at ~1313/~4706.
*Operator impact:* The save-to-disk routine crashes the entire refbox if the write fails. On a Pi with a full/worn SD card (a real tournament condition), confirming your portal setup with APPLY — or merely cycling display mode or changing language — makes the whole window vanish mid-game, with a crash instead of a "couldn't save settings" message.
*Why real:* The save treats any write failure as fatal with no handling. The same author handled this gracefully on the restart path, proving the write is known to fail. Broad blast radius (many routine actions), so high.
*Fix:* Catch the write error, log it, surface a plain "could not save settings" message, keep the app running — matching the restart path's graceful pattern.

**H7. Portal status page crashes (desktop) or shows a broken list (Pi) when the backlog drains while scrolled down**
`refbox/src/app/view_builders/shared_elements.rs` — make_scroll_list (~75); portal_detail.rs:47–81; never-clamped scroll position in mod.rs (~2160–2169, 4875–4876); on_item_resolved (mod.rs ~671–680).
*Operator impact:* Open the portal status page (tap the colored dot), scroll down to watch a backlog, and as games actually upload (the outcome you want) the list shrinks under your scroll position. Desktop/dev build crashes; the Pi release build collapses the list into a garbage-sized area you can't read or tap — exactly when you need to manage unsent results.
*Why real:* The layout subtracts row count and scroll position with no lower bound, and the scroll position is never re-clamped when the list shrinks (only reset on page re-entry). The list shrinks on the normal recovery path (sent games removed; "recent successes" capped at 5), underflowing the math. Pi is release mode (wraps to garbage); dev crashes.
*Fix:* Clamp the scroll position to the current row count on every render, and/or use saturating/checked subtraction in the scroll-list size math.

### MEDIUM severity

**M1. Re-linking to a different event (same timings) can show a false red connection dot**
`refbox/src/app/mod.rs` — apply_game_options between-games game-number-changed branch (~1171); contrast set_current_event_id (~872–883) and sibling branches at ~964, ~1132.
*Impact:* Link the wrong event, then correct it to a different event with the same standard timings → the background check keeps checking the OLD event's authorization → red dot + greyed REFRESH despite a valid login. Persists until another event change or restart. Scores still go to the correct event (read from the queued item), so misleading-indicator only.
*Fix:* Route this branch's event write through set_current_event_id (one line), like its two siblings.

**M2. Switching games mid-game then crashing can restore the wrong game**
`refbox/src/app/mod.rs` — apply_game_confirmation EndGameAndApply/KeepGameAndApply arms (~1218–1273); persist_link_session game-number derivation (~903–931); heartbeat ~2670.
*Impact:* Change which scheduled game is running mid-game, confirm, then crash/self-update within minutes → the box re-links to and counts down to the game you switched AWAY from. Confusing wrong-game state at poolside; no scores lost. Narrow window (heartbeat self-corrects every ~5 min), hence medium.
*Fix:* Immediately refresh the saved-link note after the mid-game game-change confirmation commits (same persist routine the Apply/heartbeat paths use).

**M3. A late token-check reply for a previous event can paint a false green "OK"**
`refbox/src/app/mod.rs` — RecvTokenValid arm (~4068–4073); check_uwhportal_auth (~771–790); armed at ParameterSelected::Event (~3407–3410) and enter_game_config (~1512–1514); message.rs:165.
*Impact:* Open Game Options on one event, pick another; the per-event auth checks can finish out of order. If event A's "valid" lands after event B's "invalid", the token box shows green "OK" for B though your login is rejected for it. You apply, then every submission fails. Self-corrects only once real submissions fail (red dot).
*Why real:* The result carries only yes/no with no event tag; the handler applies whichever lands last. The maintainers added exactly this event-id guard on the sibling schedule and auto-court paths — it's missing here.
*Fix:* Tag the token-check result with its event and drop any reply that doesn't match the current event, like the schedule/auto-court paths.

**M4. Scores are re-sent on every retry, and a FORCE can keep overwriting the server score**
`refbox/src/portal_manager/health.rs` — attempt_item (~197–213); post_game_scores (uwh-common/src/uwhportal/mod.rs ~213–257); force_submit (mod.rs ~535–546).
*Impact:* Every retry re-sends scores even after they landed (usually harmless — same numbers). The hazard: once you FORCE a game, the force flag is never cleared, so each retry re-sends with force ON. If an admin corrects that game's score in the portal web UI while the item is still queued, the box silently reverts their correction on the next retry.
*Fix:* Record per-leg success (don't re-send landed scores) and clear the force flag after a successful score post (or scope force to a single attempt).

**M5. DISCARD is mislabeled when the scores actually succeeded**
`refbox/src/app/view_builders/portal_attention_action.rs` — build_portal_attention_action (~28–114, note text ~59–80); discard semantics (mod.rs ~595–604); stats-fail path (health.rs ~209–212).
*Impact:* For a stuck game where scores landed but stats keep failing, the page says "The game result has not been accepted" (false — the score WAS accepted) and the only clearing button is "Discard this game result." The operator is told to discard a result that's already correctly recorded, and may re-enter or escalate needlessly.
*Fix:* Surface the "scores succeeded, stats failed" state and re-word the clear action so it doesn't read as throwing away a landed result. Best fixed with H5's decoupling.

**M6. If the portal client fails to start, every game shows a fake green "submitted" while nothing is sent**
`refbox/src/portal_manager/mod.rs` — NullIo impl (~27–40); startup fallback (app/mod.rs ~1725–1730, ~1663–1671).
*Impact:* On a box where the portal client can't initialize (broken TLS/cert env — plausible on a locked-down field Pi or loaner laptop), the app falls back to a "do nothing" uploader that fakes success for everything. Every game shows green "submitted", dot stays green, nothing reaches the portal. You can finish a whole tournament believing all results were sent. Fake-success deletes queue items, so a restart won't re-send.
*Fix:* Route a failed portal-client construction into the visible degraded (red, not-sending) mode instead of a silent fake-success uploader.

**M7. An ordinary wifi drop is reported as "Portal login expired" and greys out REFRESH**
`refbox/src/portal_manager/health.rs` — verify_token branch (~155–164); on_token_status (mod.rs ~612–615); recompute_indicator (mod.rs ~383–396); game_info.rs:34/46; portal_detail.rs:131–137.
*Impact:* During a normal blip with a valid login, the app says your login expired and pushes you to re-log-in (impossible offline, pointless — token is fine), while greying out REFRESH, the one control that would re-pull the schedule on reconnect. Wastes time chasing a non-existent login problem. Clears on reconnect.
*Why real:* The check collapses every failure (real rejection, outage, DNS, server error) into one flag treated as "token expired", which both shows the re-login row and disables REFRESH. The code comment even wrongly asserts refresh "can only fail while the token is expired."
*Fix:* Distinguish a genuine auth rejection from a connectivity/server failure; show "re-login" only for a real auth failure; an outage should read "offline" and not disable REFRESH.

### LOW severity

**L1. Queue-snapshot updates can arrive out of order, briefly desyncing the uploader**
`refbox/src/portal_manager/mod.rs` — push_queue_snapshot (~486–492); callers ~525, 544, 573, 590, 602, 629, 686.
*Impact:* After a rapid burst of portal actions the background uploader can briefly work from a stale queue copy → at worst one redundant re-send of an already-resolved/discarded game (mostly absorbed by existing safeguards) or a brief delay. Self-heals on the next change. (Operator-visible UI is driven by the always-correct main-thread queue.)
*Fix:* Deliver snapshots in order (send sequentially, or version them and ignore older-than-applied).

**L2–L5. Minor robustness warts** (folded into the assessment by the synthesizer): a ~5-minute window where the dot can read green after a change before the heartbeat catches up; sluggishness on very slow networks; portal state split across more than one on-disk file; missing directory fsync after writes. Genuine but minor.

---

## Disputed (1) — worth a human look, not a flow defect

**D1. Startup config write can panic on first launch / migration when the config dir isn't writable**
`refbox/src/main.rs:537` (`get_configuration_file_path().unwrap()`) and `:562` (`confy::store().unwrap()`), migration branch.
Only 1 of 3 reviewers upheld it: this is pre-existing 2023 startup code the portal feature never touched, and the trigger is a narrow legacy corner (config load fails AND the dir is unwritable). A general hardening item, not introduced by this flow.

## Rejected (2) — investigated and found to be false alarms

Two candidate findings were traced to the actual code and rejected because the code was correct (the reviewers found the guard/behavior the original finder had missed). Listing the confirmed set is therefore filtered, not inflated.

---

## Connections to existing tracked work
- **H5 / M4 / M5** are the same root cause as the approved backlog item *"portal-stats-best-effort"* (stats POST 400 → stuck game). This review confirms it's real and surfaces two related sub-issues. Fixing per the approved design closes all three.
- **H1** is the same class as the already-fixed APPLY-stuck portal-config bugs (PRs #1264 / #1275) — a path that path was missed.
- **M1 / M3** are fixable by reusing event-id guards that already exist elsewhere in the same file.
