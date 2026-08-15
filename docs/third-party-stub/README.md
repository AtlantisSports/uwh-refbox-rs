# Third-party stub site

A throwaway fake Portal, in one Python file with no dependencies. It exists for exactly one
reason: **to prove that `docs/third-party-integration.md` is complete enough to build against.**

It is not a test fixture, not a mock library, and not something to build on. If you want to know
what the Portal API looks like, read the document — not this file.

## Why it was written the way it was

The stub was written by an agent that was allowed to read the integration document **and nothing
else**. It was forbidden from opening any Rust source file, and it worked in a directory that
contained only a copy of the document, so the rule held structurally rather than on trust.

That blindness is the entire point. A stub written with the source code open would prove only
that the code works; a stub written from the document alone proves whether a stranger could
actually stand up a site from the document. Every question the document failed to answer became a
finding, and those findings were fixed in the document rather than worked around here.

For the same reason, **finding zero gaps would have meant the exercise failed**, not that the
document was perfect. Four rounds have been run, the later ones against a corrected document and
by agents who had not seen the earlier attempts, plus two runs of a real refbox against a live
stub. Every round found something. The most recent round's ledger — what was found, what was
fixed, and what is still open — is `docs/superpowers/plans/2026-08-13-round-4-findings.md`.

## One thing this stub deliberately gets wrong

**It does not implement call 1's pairing negotiation.** It hands its access key to any caller that
asks for one. The document requires a real site to record an admin-entered `refBoxId`, bind a code
to it, and refuse everything else — and refbox cannot tell the difference, which is precisely why
the stub skips it: implementing the negotiation here would verify nothing about refbox, and would
add a manual pairing step to every walkthrough. `stub_site.py`'s module banner and
`handle_link_refbox` both say so at length.

Copy the message shapes from this file. Take the security rules from the document. Do not copy
that handler.

## Running it

Start the stub:

```bash
python3 docs/third-party-stub/stub_site.py
```

It prints `serving on http://localhost:8099` and logs every request path and body to stdout, so
a call arriving in a shape you didn't expect is visible immediately.

Point refbox at it, in a second terminal:

```bash
UWH_PORTAL_URL_OVERRIDE=http://localhost:8099 cargo run -p refbox -- --allow-http
```

`--allow-http` is not optional. Without it refbox refuses to send a plain-HTTP request at all and
the stub sits in silence, looking exactly like a server that is down. refbox also has to be in one
of the hockey modes — in Beep Test mode nothing ever calls the Portal. Both points are explained
in "Pointing refbox at your site" in the document.

## Before you run it: back up your config

**Every refbox build on the machine shares `~/.config/refbox`.** Running against the stub will
overwrite the saved Portal link and token there with the stub's fake ones. Back the directory up
first and restore it afterwards:

```bash
cp -a ~/.config/refbox ~/.config/refbox.backup
```

Start the run unlinked: clear `uwhportal.token` in `default-config.toml` and delete
`portal_link.json` first. A saved link points at whichever event was last used, which this stub
does not serve, and starting clean is what puts the link flow at the front of the run.

## What it serves

All nine calls the document says a stand-in site must answer for the refbox itself — including the
team roster (`GET /api/admin/get-event-team`), which fills the player-number grid and whose absence
is the one failure the operator gets no warning about.

One hardcoded event: one court, two games sharing one timing rule, two teams of six playing
members. The rosters are deliberately awkward, because `roles` is the easiest field to get wrong:

- one member of the second team is **both** `Player` and `Coach`, and **must** still reach the grid.
  The role test is an inclusion test, so a site that implements it as "exclude anyone labelled
  Coach" wrongly hides a playing coach.
- one member is `Coach` only, and carries a cap number anyway. It must **not** reach the grid —
  refbox drops the entry on the role filter before it ever looks at the number.

It ignores `force`, `filter` and `limit`, which a real site is expected to apply itself. Where it
does serve a call it answers `200` and never `204`, deliberately, since `200` is the only status
refbox treats as success. It refuses with `401` on a bad or absent token and `404` for a team or
route it does not know.

**It accepts exactly one bearer token — the access key it issued itself — and refuses everything
else, including a request carrying no `Authorization` header at all.** That refusal is the point.
An earlier version accepted anything, and the result was that refbox decided it was already
paired, showed the token row as `OK` in green, and never offered to link. Your site is the only
thing enforcing that a refbox is authorised for an event, so a permissive stand-in silently
disables pairing altogether, with nothing reported anywhere.

## Scope

Not part of the cargo workspace. Never built, run, or checked in CI. Nothing in the workspace
imports it, and no test depends on it. It has no dependencies beyond the Python 3 standard
library, so it needs no install step.

Expect it to rot. It was last rebuilt against the document on 2026-08-13; the document is the
artefact under maintenance, not this file.
