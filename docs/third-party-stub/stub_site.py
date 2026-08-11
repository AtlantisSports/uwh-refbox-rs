#!/usr/bin/env python3
"""A stand-in for the UWH Portal, built ONLY from third-party-integration.md.

Serves the eight HTTP calls that document says the refbox itself makes
during a tournament. Deliberately does not implement the other ten calls
(schedule-processor / overlay), since the document says those aren't needed
to run a game.

Standard library only: http.server + json for the actual server/wire-format
work. A few other standard-library modules (re, sys, urllib.parse, datetime)
are used for routing/logging plumbing -- none of them are third-party
packages, and the document's ban is specifically on frameworks like flask/
fastapi/requests, not on the rest of the standard library.

THIS IS NOT A REFERENCE IMPLEMENTATION. It is an executable check of the
message formats and the call sequence a real refbox will exercise -- nothing
more. It deliberately does NOT implement the security policy the document
requires of a real site, and the one place that matters is call 1: this stub
hands its access key to anyone who asks, while the document requires a real
site to bind a code to an admin-entered refBoxId and refuse everything else.
Copy the shapes from here; take the security rules from the document, not
from this file. See handle_link_refbox.
"""

import http.server
import json
import re
import sys
from datetime import datetime, timezone
from urllib.parse import parse_qs, urlsplit

PORT = 8099

# The one access key this stub hands out (call 1) and the only bearer token it
# accepts afterwards (calls 2, 5, 7, 8). See _require_bearer for why it is not
# simply "any token".
ACCESS_KEY = "stub-access-token"

# ---------------------------------------------------------------------------
# Hardcoded fake event: one court, two games, two teams of six players each.
# ---------------------------------------------------------------------------

EVENT_ID = "1234-A"
EVENT_ID_LONG = "events/1234-A"
EVENT_NAME = "Example Open 2026"
EVENT_SLUG = "example-open-2026"

TEAM_A_ID = "1234-A"
TEAM_A_LONG = "teams/1234-A"
TEAM_A_NAME = "Black Sheep"
TEAM_A_ROSTER = [
    {"rosterName": "Alice", "capNumber": 1},
    {"rosterName": "Bailey", "capNumber": 2},
    {"rosterName": "Casey", "capNumber": 3},
    {"rosterName": "Drew", "capNumber": 4},
    {"rosterName": "Emerson", "capNumber": 5},
    {"rosterName": "Finley", "capNumber": 6},
]

TEAM_B_ID = "5678-B"
TEAM_B_LONG = "teams/5678-B"
TEAM_B_NAME = "White Knights"
TEAM_B_ROSTER = [
    {"rosterName": "Ashley", "capNumber": 1},
    {"rosterName": "Blair", "capNumber": 2},
    {"rosterName": "Cameron", "capNumber": 3},
    {"rosterName": "Dakota", "capNumber": 4},
    {"rosterName": "Elliot", "capNumber": 5},
    {"rosterName": "Frankie", "capNumber": 6},
]
# NOTE: none of the eight refbox calls documented in third-party-integration.md
# ever return player-level roster data (the roster-fetch call is one of the
# "other ten" and is explicitly out of scope for a refbox-only stand-in). The
# rosters above exist to satisfy the "two teams of six players each" fake-data
# requirement; they are surfaced as an extra, unread field on each team entry
# in the /teams response (the document says extra fields there are harmless),
# but a real refbox never looks at them.

COURT = "A"

TIMING_RULE_NAME = "RR"
TIMING_RULE = {
    "name": TIMING_RULE_NAME,
    "teamTimeoutCount": 1,
    "teamTimeoutsCountedPerHalf": True,
    "overtimeAllowed": True,
    "suddenDeathAllowed": True,
    "last2minStopTime": False,
    "halfPlayDuration": 900,
    "halfTimeDuration": 180,
    "teamTimeoutDuration": 60,
    "overtimeHalfPlayDuration": 300,
    "overtimeHalfTimeDuration": 180,
    "preOvertimeBreak": 180,
    "preSuddenDeathDuration": 60,
    "minimumBreak": 240,
    # gameBlock omitted -- the document says this is optional and that refbox
    # will work it out itself from the other durations when it's absent.
}

REFEREE_USER_ID = "user-abc123"
REFEREE_USERNAME = "reef_ref"
REFEREE_ROSTER_NAME = "Casey"

GAMES = {
    "1": {
        "number": "1",
        "dark": {"teamId": TEAM_A_LONG},
        "light": {"teamId": TEAM_B_LONG},
        "startsOn": "2026-08-08T09:00:00Z",
        "court": COURT,
        "timingRule": {"name": TIMING_RULE_NAME},
        "refereeAssignments": [
            {"role": "Head Referee", "userId": REFEREE_USER_ID},
        ],
        "description": "Round robin opener",
    },
    "2": {
        "number": "2",
        "dark": {"teamId": TEAM_B_LONG},
        "light": {"teamId": TEAM_A_LONG},
        "startsOn": "2026-08-08T10:00:00Z",
        "court": COURT,
        "timingRule": {"name": TIMING_RULE_NAME},
        # No refereeAssignments / description -- the document's own worked
        # example shows a second game omitting both, so this stub mirrors that.
    },
}


# ---------------------------------------------------------------------------
# Route handlers. Each takes (handler, path_match, query_dict, raw_body_bytes).
# ---------------------------------------------------------------------------


def _require_bearer(h):
    """Refuse the call unless it carries this stub's own access key.

    All four bearer calls of the refbox eight go through here: 2 (verify
    token), 5 (privileged schedule), 7 (push scores) and 8 (push stats).

    Why this is NOT "accept any bearer token", which is what the plan for this
    stub originally asked for: a site that accepts anything -- including a
    request carrying no Authorization header at all -- tells refbox its token is
    already fine. refbox then believes it is paired, shows UWHPORTAL TOKEN: OK
    in green, and never offers the link flow. The site is the only thing
    enforcing that a refbox is authorised for an event, so a permissive site
    silently disables pairing altogether, with no error surfaced anywhere.
    Refusing here is what makes the link flow reachable at all. Recorded in the
    integration document as gap 13.
    """
    header = h.headers.get("Authorization")
    if header == f"Bearer {ACCESS_KEY}":
        return True
    reason = "no Authorization header at all" if header is None else f"unknown token {header!r}"
    print(
        f"    -> 401 ({reason}) -- a real site must refuse this, "
        "or refbox never learns it needs to link",
        flush=True,
    )
    h._send_json(401, {"error": "unauthorized"})
    return False


def handle_link_refbox(h, m, query, raw_body):
    # DELIBERATELY WRONG, and the only place this stub knowingly diverges from
    # the document. It ignores the request body and hands the key to anyone.
    #
    # The document REQUIRES the negotiation: record the refBoxId an admin
    # entered so a pending link exists, issue a code bound only to that
    # refBoxId, reject anything else with the exact 400 reason strings
    # NoPendingLink / InvalidCode, expire codes, and keep every issued
    # accessKey revocable. An earlier revision of the document said a stand-in
    # site was free to skip that; it no longer does, and this stub was not
    # changed to follow, on purpose:
    #
    #   - refbox cannot tell the difference. It accepts whatever call 1
    #     returns, so implementing the negotiation here would verify nothing
    #     about refbox -- the only thing this stub exists to verify.
    #   - the negotiation would add a manual pairing step to every walkthrough,
    #     which is the one thing that makes the stub usable at the screen.
    #
    # So this is a happy-path fixture, not an example to copy. A site that
    # ships this behaviour is an open token dispenser: it hands a working
    # credential to any client that can reach it, which hollows out the
    # document's own "reject a token you did not issue" rule on call 2.
    #
    # This call is unauthenticated by design -- it is how a refbox obtains its
    # token in the first place.
    print(
        "    -> 200, key issued to ANY caller -- this stub skips the pairing "
        "negotiation on purpose. A real site MUST bind the code to an "
        "admin-entered refBoxId and reject the rest with NoPendingLink / "
        "InvalidCode. Do not copy this handler.",
        flush=True,
    )
    h._send_json(200, {"accessKey": ACCESS_KEY})


def handle_verify_token(h, m, query, raw_body):
    if not _require_bearer(h):
        return
    # Body is never parsed by refbox, so an empty 200 is enough.
    h._send_empty(200)


def handle_event_list(h, m, query, raw_body):
    payload = {
        "totalCount": 1,
        "items": [
            {
                "id": EVENT_ID_LONG,
                "name": EVENT_NAME,
                "slug": EVENT_SLUG,
                "dateRange": {
                    "startsOn": "2026-08-08T09:00:00Z",
                    "endsOn": "2026-08-08T18:00:00Z",
                },
            }
        ],
    }
    h._send_json(200, payload)


def handle_event_teams(h, m, query, raw_body):
    payload = {
        "teams": [
            {
                "team": {
                    "id": TEAM_A_LONG,
                    "name": TEAM_A_NAME,
                    "roster": TEAM_A_ROSTER,
                }
            },
            {
                "team": {
                    "id": TEAM_B_LONG,
                    "name": TEAM_B_NAME,
                    "roster": TEAM_B_ROSTER,
                }
            },
        ]
    }
    h._send_json(200, payload)


def handle_schedule(h, m, query, raw_body):
    if not _require_bearer(h):
        return
    payload = {
        "eventId": EVENT_ID_LONG,
        "games": GAMES,
        "nonGameEntries": [],
        "groups": [],
        "timingRules": [TIMING_RULE],
    }
    h._send_json(200, payload)


def handle_referees(h, m, query, raw_body):
    payload = {
        "tournamentReferee": None,
        "referees": {
            "dedicated": [
                {
                    "user": {"id": REFEREE_USER_ID, "username": REFEREE_USERNAME},
                    "rosterName": REFEREE_ROSTER_NAME,
                }
            ],
            "hybrid": [],
            "timeOrScoreKeeper": [],
        },
    }
    h._send_json(200, payload)


def handle_push_scores(h, m, query, raw_body):
    if not _require_bearer(h):
        return
    game_number = m.group(2)
    if game_number not in GAMES:
        print(
            f"    NOTE: game number {game_number!r} is not one of this stub's "
            f"known games ({', '.join(GAMES)}) -- accepting it anyway",
            flush=True,
        )
    # Body is never parsed by refbox on success; only the status code matters.
    h._send_empty(200)


def handle_push_stats(h, m, query, raw_body):
    if not _require_bearer(h):
        return
    try:
        events = json.loads(raw_body) if raw_body else []
    except json.JSONDecodeError:
        print("    NOTE: stats body did not parse as JSON at all", flush=True)
        h._send_empty(200)
        return
    if isinstance(events, list):
        kinds = [e.get("$type", "?") for e in events if isinstance(e, dict)]
        print(
            f"    Stats push contained {len(events)} event(s): {kinds}",
            flush=True,
        )
    else:
        print(
            "    NOTE: stats body was not a bare JSON array, contrary to "
            "the document's description of this call",
            flush=True,
        )
    h._send_empty(200)


ROUTES = [
    ("POST", re.compile(r"^/api/events/([^/]+)/access-keys/ref-box$"), handle_link_refbox),
    ("GET", re.compile(r"^/api/events/([^/]+)/access-keys/verify$"), handle_verify_token),
    ("GET", re.compile(r"^/api/events$"), handle_event_list),
    ("GET", re.compile(r"^/api/events/([^/]+)/teams$"), handle_event_teams),
    ("GET", re.compile(r"^/api/events/([^/]+)/schedule/privileged$"), handle_schedule),
    ("GET", re.compile(r"^/api/events/([^/]+)/referees$"), handle_referees),
    (
        "POST",
        re.compile(r"^/api/events/([^/]+)/schedule/games/([^/]+)/scores$"),
        handle_push_scores,
    ),
    ("POST", re.compile(r"^/api/admin/events/stats$"), handle_push_stats),
]


class Handler(http.server.BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"

    def _read_body(self):
        try:
            length = int(self.headers.get("Content-Length", 0) or 0)
        except ValueError:
            length = 0
        return self.rfile.read(length) if length else b""

    def _log(self, method, raw_path, raw_body):
        timestamp = datetime.now(timezone.utc).isoformat()
        auth = self.headers.get("Authorization", "<none>")
        print(f"[{timestamp}] {method} {raw_path}", flush=True)
        print(f"    Authorization: {auth}", flush=True)
        if raw_body:
            try:
                parsed = json.loads(raw_body)
                print(f"    Body: {json.dumps(parsed)}", flush=True)
            except json.JSONDecodeError:
                print(f"    Body (not valid JSON): {raw_body!r}", flush=True)
        else:
            print("    Body: <empty>", flush=True)

    def _send_json(self, status, payload):
        body = json.dumps(payload).encode("utf-8")
        self.send_response(status)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def _send_empty(self, status):
        self.send_response(status)
        self.send_header("Content-Length", "0")
        self.end_headers()

    def log_message(self, fmt, *args):
        # Replace the default stderr access log with our own stdout logging
        # done in _log(), so all visibility lives in one place.
        pass

    def do_GET(self):
        self._dispatch("GET")

    def do_POST(self):
        self._dispatch("POST")

    def _dispatch(self, method):
        split = urlsplit(self.path)
        path = split.path
        query = parse_qs(split.query, keep_blank_values=True)
        raw_body = self._read_body()
        self._log(method, self.path, raw_body)

        for route_method, pattern, handler in ROUTES:
            if route_method != method:
                continue
            match = pattern.match(path)
            if match:
                handler(self, match, query, raw_body)
                return

        print("    -> 404 (no route in the refbox eight matched)", flush=True)
        self._send_json(404, {"error": "not found"})


def main():
    # Bind on all interfaces so a real refbox running on separate hardware
    # (e.g. a Raspberry Pi on the same network) can reach this stub, even
    # though the printed URL below -- required verbatim -- says "localhost".
    server = http.server.ThreadingHTTPServer(("0.0.0.0", PORT), Handler)
    print(f"serving on http://localhost:{PORT}", flush=True)
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        pass
    finally:
        server.server_close()


if __name__ == "__main__":
    sys.exit(main())
