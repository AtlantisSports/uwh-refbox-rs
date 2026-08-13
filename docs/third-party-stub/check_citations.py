#!/usr/bin/env python3
"""Report what every source citation in the contract document points at.

The document cites source locations three ways, and all three drift whenever
the cited file changes:

    `uwh-common/src/uwhportal/mod.rs:671`   full path
    `schedule.rs:762`                       bare filename, path implied by context
    `:718`                                  bare line, both file and path implied

The last two are shorthand a human reader resolves from the nearest preceding
full citation. This resolves them the same way, so that a citation written as
shorthand is checked rather than silently skipped -- an early version of this
tool matched only the full form and missed a third of the citations in the
document.

A citation that resolves is necessary but not sufficient: it can still resolve
to the wrong thing. This narrows the reading; it does not replace it.

Standard library only, matching stub_site.py's constraint.
"""

import pathlib
import re
import sys

DOC = pathlib.Path("docs/third-party-integration.md")

EXT = r"(?:rs|ftl|toml|py)"
CITE = re.compile(
    r"`(?P<full>[A-Za-z0-9_.-]+(?:/[A-Za-z0-9_.-]+)+\." + EXT + r"):"
    r"(?P<fstart>\d+)(?:-(?P<fend>\d+))?`"
    r"|`(?P<base>[A-Za-z0-9_-]+\." + EXT + r"):"
    r"(?P<bstart>\d+)(?:-(?P<bend>\d+))?`"
    r"|`:(?P<lstart>\d+)(?:-(?P<lend>\d+))?`"
)

_cache = {}


def source_lines(path):
    if path not in _cache:
        p = pathlib.Path(path)
        _cache[path] = p.read_text().splitlines() if p.exists() else None
    return _cache[path]


def resolve(base, last_path, seen_paths):
    """Map a bare filename to a full path, the way a reader would."""
    if last_path and pathlib.Path(last_path).name == base:
        return last_path, None
    matches = sorted({p for p in seen_paths if pathlib.Path(p).name == base})
    if len(matches) == 1:
        return matches[0], None
    if not matches:
        return None, f"no full-path citation of {base} seen earlier"
    return None, f"ambiguous {base}: {', '.join(matches)}"


def main():
    if not DOC.exists():
        print(f"ERROR: {DOC} not found - run from the repository root", file=sys.stderr)
        return 2

    seen = set()
    seen_paths = []
    last_path = None
    problems = 0
    total = 0

    for doc_line_no, doc_line in enumerate(DOC.read_text().splitlines(), 1):
        for m in CITE.finditer(doc_line):
            note = None
            if m.group("full"):
                path = m.group("full")
                start, end = m.group("fstart"), m.group("fend")
                kind = "full"
                if path not in seen_paths:
                    seen_paths.append(path)
                last_path = path
            elif m.group("base"):
                start, end = m.group("bstart"), m.group("bend")
                kind = "base"
                path, note = resolve(m.group("base"), last_path, seen_paths)
                if path:
                    last_path = path
            else:
                start, end = m.group("lstart"), m.group("lend")
                kind = "bare"
                path = last_path
                if path is None:
                    note = "bare :line with no preceding file citation"

            start = int(start)
            end = int(end) if end else start

            if path is None:
                total += 1
                problems += 1
                print(f"=== doc:{doc_line_no}  [{kind}] :{start}")
                print(f"    !! UNRESOLVED - {note}")
                continue

            key = (path, start, end)
            if key in seen:
                continue
            seen.add(key)
            total += 1

            span = f"{start}" if end == start else f"{start}-{end}"
            print(f"=== doc:{doc_line_no}  [{kind}] {path}:{span}")

            src = source_lines(path)
            if src is None:
                print("    !! FILE NOT FOUND")
                problems += 1
                continue
            if start > len(src):
                print(f"    !! OUT OF RANGE - file has {len(src)} lines")
                problems += 1
                continue
            for n in range(start, min(end, len(src)) + 1):
                print(f"    {n}: {src[n - 1][:100]}")

    print(f"\n{total} citations checked, {problems} unresolvable")
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main())
