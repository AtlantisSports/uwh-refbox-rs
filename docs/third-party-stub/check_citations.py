#!/usr/bin/env python3
"""Report what every file:line citation in the contract document points at.

The document cites source locations as `path/to/file.rs:123` or
`path/to/file.rs:123-145`. Those line numbers drift whenever the cited file
changes. This prints each citation next to the text actually at that line, so a
reviewer can confirm the citation still lands on what the document claims.

A citation that resolves is necessary but not sufficient -- it can still resolve
to the wrong thing. This tool narrows the reading; it does not replace it.

Standard library only, matching stub_site.py's constraint.
"""

import pathlib
import re
import sys

DOC = pathlib.Path("docs/third-party-integration.md")
CITE = re.compile(
    r"`([A-Za-z0-9_.-]+/[A-Za-z0-9_/.-]+\.(?:rs|ftl|toml|py)):(\d+)(?:-(\d+))?`"
)

_cache = {}


def source_lines(path):
    if path not in _cache:
        p = pathlib.Path(path)
        _cache[path] = p.read_text().splitlines() if p.exists() else None
    return _cache[path]


def main():
    if not DOC.exists():
        print(f"ERROR: {DOC} not found - run from the repository root", file=sys.stderr)
        return 2

    seen = set()
    problems = 0
    total = 0

    for doc_line_no, doc_line in enumerate(DOC.read_text().splitlines(), 1):
        for m in CITE.finditer(doc_line):
            path = m.group(1)
            start = int(m.group(2))
            end = int(m.group(3)) if m.group(3) else start
            key = (path, start, end)
            if key in seen:
                continue
            seen.add(key)
            total += 1

            span = f"{start}" if end == start else f"{start}-{end}"
            print(f"=== doc:{doc_line_no}  {path}:{span}")

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

    print(f"\n{total} unique citations, {problems} unresolvable")
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main())
