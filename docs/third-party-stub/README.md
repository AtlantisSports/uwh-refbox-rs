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
document was perfect. The blind build found nine; a desk check of the stub against the real client
found three more, and running a real refbox against the stub found a thirteenth.

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

One hardcoded event: two teams of six players, one court, two games sharing one timing rule. It
ignores `force`, `filter` and `limit`, and answers `200` to every call it accepts — which is
deliberate, since `200` is the only status refbox treats as success.

**It does not accept any bearer token, and that is the point.** On the four authenticated calls it
accepts only the access key it handed out itself during linking, and answers `401` to anything
else — including a request with no `Authorization` header, which is exactly what refbox sends when
it holds no token. An earlier version accepted everything, and the result was that refbox decided
it was already paired, showed the token as `OK`, and never offered to link. That is gap 13 in the
document, and refusing is what makes the link flow reachable.

## Scope

Not part of the cargo workspace. Never built, run, or checked in CI. Nothing in the workspace
imports it, and no test depends on it. It has no dependencies beyond the Python 3 standard
library, so it needs no install step.

Expect it to rot. It is a snapshot of what the document described on 2026-08-10; the document is
the artefact under maintenance, not this.
