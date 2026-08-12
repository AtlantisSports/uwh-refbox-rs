# Source-neutral health wording — design

**Date:** 2026-08-12
**Status:** approved by Eric, not yet implemented
**Scope:** `refbox` only — 15 translation files and two argument deletions

## The problem

Since the game-source picker merged (PR #2168), an operator can point refbox at a third-party
site instead of the UWH Portal. Five screens still tell that operator about "the Portal".

The word does not come from the site they chose. `{ $portal }` resolves through
`portal_name_for_mode`, which returns the **game mode** — `UWH` or `UWR` — so the strings read
"UWH Portal" no matter what the game source is. Two of the five strings interpolate it; the other
three hardcode the English word "Portal" outright.

Eric spotted this on the game-end advisory during the post-merge walkthrough.

## The decision

**Drop the word rather than re-parameterise it.** These five strings are about the *connection*,
so neutral wording is correct for UWH Portal operators too — nobody loses information.

Re-parameterising was considered and rejected: refbox never learns a custom site's name, only the
address the operator typed. The best it could substitute is a URL, and "has not been accepted on
`https://your-site/api/1234-A` Portal" is worse than saying nothing.

**Vocabulary — "connection".** All five strings land on "connection" or on plain wording that
needs no noun for the far end at all.

**Key names do not change.** `portal-summary-title` and the rest stay as they are. Key names are
never shown to anyone; renaming them would multiply the diff across 15 files and 4 call sites for
no operator-visible benefit, and would leave the block half-renamed alongside the `portal-row-*`
siblings this change does not touch.

## The five strings

| # | Key | Where the operator sees it | After |
|---|-----|---------------------------|-------|
| 1 | `portal-advisory-at-game-end` | Advisory at game end | **Connection issue detected.** Score will still be queued — find an admin to resolve. |
| 2 | `portal-summary-title` | Health page title | **CONNECTION STATUS** |
| 3 | `portal-row-token-expired` | Red row on the health page | **Access token expired — tap to re-login** |
| 4 | `portal-page-attention-info` | Submission-error page | **The game result has not been accepted** |
| 5 | `uwhportal-token-no-pending-link` | Failed link attempt | **The connection is not expecting communication.** |

Everything not shown in bold is unchanged. String 1 keeps its second sentence; string 5 keeps its
existing second line, "Please try again."

Three of these need a note, because the reasoning is not recoverable from the text:

- **String 3 names the access token deliberately.** Tapping that row does not open the login
  keypad — it lands the operator on the game settings page and leaves them to find the control,
  which is the row labelled `ACCESS TOKEN:` (renamed from `UWHPORTAL TOKEN:` in PR #2168). Naming
  the same thing two different ways is the exact confusion this change exists to remove.
- **String 4 drops its trailing phrase rather than replacing it.** The page it sits on is already
  titled "Game { $game } submission error", so the sentence does not need to re-establish what
  failed.
- **String 5 is Eric's literal**, chosen over the proposed "The game source is not expecting a
  connection." Implement it exactly as written. Known and accepted: the message appears when the
  far end has no pending link request — nobody has added this Refbox on the site yet — and neither
  the old wording nor the new one tells the operator that. Not a regression; out of scope here.

## What is explicitly not in scope

- **`portal-login-instructions`.** It walks the operator through the UWH Portal's own menus
  (`Event Management >> Referee Management >>` the `+` button), so it needs different text per
  source, not neutral text. That is gap 15 of the third-party contract document — a separate job.
- **`{ $portal }` itself and `portal_name_for_mode`.** Both stay exactly as they are; they are
  still used by other strings.
- **The `portal-row-*` siblings** (`portal-row-stuck`, `portal-row-pending`,
  `portal-row-stats-pending`, `portal-row-recent`). They are already source-neutral.
- **Any behavioural change** to health checks, linking, the queue, or the token indicator.

## Files

**Translations — 5 values × 15 locales.** `refbox/translations/*/refbox.ftl`. Every locale gets
real translated text, no English placeholders, following each language's own capitalisation
convention for the block it sits in.

**Rust — two deleted arguments, nothing else.**

- `refbox/src/app/view_builders/portal_detail.rs` — drop `portal = portal_name_for_mode(mode)`
  from the `portal-summary-title` call.
- `refbox/src/app/view_builders/portal_attention_action.rs` — drop the same argument from the
  `portal-page-attention-info` call.

Nothing else in either file needs touching, and this was checked rather than assumed:

- `mode` stays in use in both files — it is passed to `make_game_time_button`.
- No import needs removing. Both files bring `portal_name_for_mode` in through `use super::*;`,
  and Rust does not warn on unused glob imports, so `-D warnings` stays satisfied. The function
  itself remains in use at eight other call sites and is not touched.

## Acceptance criteria

1. With the game source set to CUSTOM, none of the five screens says "Portal".
2. With the game source set to UWH PORTAL, the same five screens read correctly — the neutral
   wording is not merely tolerable there, it is accurate.
3. No locale contains an untranslated English placeholder for the five keys.
4. `just check` passes: formatting, `-D warnings`, the full test suite, and the security scan.

Strings 1–4 are reachable against the verification stub. String 5 needs a link attempt the far end
rejects with `NoPendingLink`.
