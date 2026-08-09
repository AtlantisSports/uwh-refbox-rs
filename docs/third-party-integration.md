# Third-Party Integration: Running Your Own Site Instead of the UWH Portal

Accurate as of refbox v0.4.9. This is a best-effort description of what the software does
today. It carries no stability promise; a future release may change any of it without notice.

## You probably need eight calls, not eighteen

The refbox application, the pre-tournament schedule tool, and the stream overlay together
make eighteen different calls to the UWH Portal. If you only want to run a site that stands
in for the Portal on the day of the tournament — the thing the referee's box actually talks
to at poolside — you only need to support eight of them:

| # | Operation | Path | Auth |
|---|---|---|---|
| 1 | Link a refbox | `POST /api/events/{eventId}/access-keys/ref-box` | none |
| 2 | Verify token | `GET /api/events/{eventId}/access-keys/verify` | bearer |
| 3 | Event list | `GET /api/events` | none |
| 4 | Event teams | `GET /api/events/{eventId}/teams` | none |
| 5 | Schedule (privileged) | `GET /api/events/{eventId}/schedule/privileged` | bearer |
| 6 | Referees | `GET /api/events/{eventId}/referees` | none |
| 7 | Push scores | `POST /api/events/{eventId}/schedule/games/{gameNumber}/scores` | bearer |
| 8 | Push stats | `POST /api/admin/events/stats` | bearer |

The other ten calls serve two separate programs that are not the refbox itself: the
pre-tournament admin tool (`schedule-processor`), which uploads and manages a schedule before
the tournament starts, and the stream overlay, which pulls attachments for the video overlay.
If you're only standing up something for the refbox to talk to during a game, you can ignore
those ten.

## Full inventory

All eighteen calls the refbox ecosystem makes to the Portal today, across all three programs.
"Auth" is "bearer" when the call requires a bearer token in the `Authorization` header, and
"none" when it does not.

| # | Method | Path | Caller(s) | Auth | Source |
|---|---|---|---|---|---|
| 1 | POST | `/api/events/{eventId}/access-keys/ref-box` | refbox | none | `uwh-common/src/uwhportal/mod.rs:206` |
| 2 | POST | `/api/authentication` | schedule-processor | none | `uwh-common/src/uwhportal/mod.rs:264` |
| 3 | GET | `/api/events/{eventId}/access-keys/verify` | refbox | bearer | `uwh-common/src/uwhportal/mod.rs:300` |
| 4 | POST | `/api/admin/events/stats` | refbox | bearer | `uwh-common/src/uwhportal/mod.rs:325` |
| 5 | POST | `/api/events/{eventId}/schedule/games/{gameNumber}/scores` | refbox | bearer | `uwh-common/src/uwhportal/mod.rs:353` |
| 6 | GET | `/api/events/{eventId}/schedule/privileged` | refbox + schedule-processor | bearer | `uwh-common/src/uwhportal/mod.rs:399` |
| 7 | GET | `/api/events/{eventId}/referees` | refbox + schedule-processor | none | `uwh-common/src/uwhportal/mod.rs:449` |
| 8 | GET | `/api/events/{eventId}/teams` | refbox + schedule-processor | none | `uwh-common/src/uwhportal/mod.rs:501` |
| 9 | GET | `/api/events` | refbox + schedule-processor | none | `uwh-common/src/uwhportal/mod.rs:537` |
| 10 | POST | `/api/events/{eventSlug}/schedule` | schedule-processor | bearer | `uwh-common/src/uwhportal/mod.rs:580` |
| 11 | POST | `/api/events/{eventSlug}/schedule/map-teams` | schedule-processor | bearer | `uwh-common/src/uwhportal/mod.rs:614` |
| 12 | GET | `/api/events/{eventId}/schedule` | schedule-processor | none | `uwh-common/src/uwhportal/mod.rs:647` |
| 13 | GET | `/api/admin/get-event-team` | schedule-processor | bearer | `uwh-common/src/uwhportal/mod.rs:671` |
| 14 | GET | `/api/events/{eventSlug}/schedule/coin-flips` | schedule-processor | none | `uwh-common/src/uwhportal/mod.rs:694` |
| 15 | GET | `/api/events/{eventId}/participants` | schedule-processor | bearer | `uwh-common/src/uwhportal/mod.rs:726` |
| 16 | GET | `/api/admin/events/game-referees` | schedule-processor | bearer | `uwh-common/src/uwhportal/mod.rs:772` |
| 17 | POST | `/api/events/{eventSlug}/schedule/coin-flips` | schedule-processor | bearer | `uwh-common/src/uwhportal/mod.rs:816` |
| 18 | GET | `/api/admin/events/{eventId}/overlay-attachments` | overlay | bearer | `overlay/src/network.rs:174` |

These eighteen operations sit on sixteen distinct paths: the coin-flips endpoint serves both
a read and a write, and the schedule endpoint serves both a public read and an upload, each
under the same path with a different HTTP method.

## The refbox eight

_(To be filled in.)_

## Data formats

_(To be filled in.)_

## The other ten

_(To be filled in.)_

## Keeping this document honest

This document can drift from the code. The check below catches one specific kind of drift —
the set of paths — by comparing every `/api/...` path found in the source files against every
`/api/...` path found in this document, after normalising placeholder names (like
`{eventId}`) to a common `{}` on both sides so that naming differences don't cause false
alarms:

```bash
diff \
  <(rg -o -N '/api/[A-Za-z0-9/{}_-]+' uwh-common/src/uwhportal/mod.rs overlay/src/network.rs \
     | sed 's/^[^:]*://; s/{[^}]*}/{}/g' | sort -u) \
  <(rg -o '/api/[A-Za-z0-9/{}_-]+' docs/third-party-integration.md \
     | sed 's/{[^}]*}/{}/g' | sort -u) \
  && echo "IN SYNC"
```

This only proves the *paths* still match — it says nothing about whether the request or
response bodies documented here still match what the code sends and expects. The real test
of that is rebuilding a working stub server from this document alone (Task 5) and confirming
it actually stands in for the Portal.
