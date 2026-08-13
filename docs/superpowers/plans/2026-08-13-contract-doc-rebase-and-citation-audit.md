# Contract Document Rebase and Citation Audit — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Bring the third-party contract document branch onto current master and prove every source citation in the document points at the code it claims, so the document is ready to open as a pull request.

**Architecture:** Two phases with a hard boundary between them. Phase one is a mechanical rebase of 16 unpushed commits onto a master that is 72 commits ahead — verified conflict-free in advance, because the branch touches only `docs/` paths that master has never modified. Phase two generates a machine-produced report of what every `file:line` citation in the document actually resolves to on the new base, then corrects the ones that moved. The audit is scripted rather than manual because there are 49 unique citations across 10 files, and eyeballing them is exactly the kind of check that silently degrades.

**Tech Stack:** git, Python 3 (standard library only, matching the stub's existing constraint), ripgrep.

**Spec:** No separate spec document. This plan implements the follow-up gate recorded in the commit message of `b252c079` and in memory `project_contract_doc_get_event_team_gap`.

## Global Constraints

- **Worktree:** `/home/estraily/projects/refbox-third-party-contract`. All commands run from there.
- **Branch:** `docs/workspace/get-event-team-attribution` (16 commits, none pushed, no remote copy exists).
- **Do not push, and do not open the PR.** Both need separate approval from the human.
- **Do not re-run the sealed-room build rounds.** That is a separate gate the human decides on.
- **Do not rewrite behaviour claims.** Correcting a line number that moved is bookkeeping. If a citation reveals that the *behaviour* the document describes has actually changed, stop and report it — that is a contract change and needs the human.
- **Do not touch `pre-rebase-backup-20260812`.** Unrelated branch, holds 5 commits not upstream.
- **Commit convention:** `type(scope): description`, lowercase, imperative, no trailing period, ~72 chars. This work is `docs(workspace): ...`.
- **The pre-commit hook rejects a detached HEAD.** A rebase leaves you detached mid-flight; only commit once the rebase has finished and you are back on a named branch.

---

### Task 1: Back up and rebase onto master

The branch has no remote copy. If this rebase goes wrong there is nothing to recover from, so the backup is not optional ceremony — it is the only copy.

**Files:**
- No file edits. Git state only.

**Interfaces:**
- Consumes: nothing.
- Produces: `docs/workspace/get-event-team-attribution` rebased onto `origin/master`, with the same 16 commits and an identical resulting tree. Task 2 depends on the working tree containing current master's source files.

- [ ] **Step 1: Record the pre-rebase state so the result can be proved identical**

```bash
cd /home/estraily/projects/refbox-third-party-contract
git rev-parse HEAD > /tmp/pre-rebase-head.txt
git log --oneline origin/master..HEAD | wc -l          # expect: 16
git rev-parse HEAD^{tree} > /tmp/pre-rebase-tree.txt
cat /tmp/pre-rebase-head.txt /tmp/pre-rebase-tree.txt
```

Expected: 16 commits, and two hashes written to files.

- [ ] **Step 2: Create the backup branch**

```bash
git branch pre-rebase-backup-contract-doc-20260813
git branch --list 'pre-rebase-backup-*'
```

Expected: both `pre-rebase-backup-20260812` (pre-existing, leave alone) and the new
`pre-rebase-backup-contract-doc-20260813` are listed.

- [ ] **Step 3: Confirm the rebase is conflict-free before starting it**

The branch touches only `docs/` files. Verify master has not touched any of them:

```bash
BASE=$(git merge-base origin/master HEAD)
comm -12 \
  <(git diff --name-only $BASE HEAD | sort) \
  <(git diff --name-only $BASE origin/master | sort)
```

Expected: **no output.** Empty means no shared files, so no conflicts. If this prints
any filename, STOP and report it — the rebase is no longer the trivial operation this
plan assumes.

- [ ] **Step 4: Fetch and rebase**

```bash
git fetch origin
git rebase origin/master
```

Expected: `Successfully rebased and updated refs/heads/docs/workspace/get-event-team-attribution.`
If the rebase stops for a conflict, run `git rebase --abort` and report — do not resolve
conflicts, because Step 3 predicted there would be none and a conflict means the
assumption is wrong.

- [ ] **Step 5: Prove the rebase preserved the work exactly**

Same 16 commits, and the document content is byte-identical to before:

```bash
git log --oneline origin/master..HEAD | wc -l          # expect: 16
git diff $(cat /tmp/pre-rebase-head.txt) HEAD -- docs/ | head
```

Expected: 16 commits, and **empty diff output** for `docs/`. An empty diff proves the
rebase changed no documentation content — only the base it sits on. If the diff is
non-empty, report exactly what changed before going further.

- [ ] **Step 6: Confirm the branch is now current with master**

```bash
git rev-list --count HEAD..origin/master               # expect: 0
git merge-base --is-ancestor origin/master HEAD && echo "master is an ancestor - current"
```

Expected: `0`, then `master is an ancestor - current`.

No commit in this task. The rebase itself is the deliverable and it rewrites existing
commits rather than adding one.

---

### Task 2: Generate the citation audit report

The document makes 49 unique `file:line` citations across 10 files. This task builds the
tool that says what each one now points at. It does not fix anything — separating the
report from the repair keeps the repair reviewable.

**Files:**
- Create: `docs/third-party-stub/check_citations.py`
- Test: the script's own output, reviewed against the document.

**Interfaces:**
- Consumes: the rebased working tree from Task 1.
- Produces: `check_citations.py`, runnable as `python3 docs/third-party-stub/check_citations.py`,
  printing one block per unique citation containing the document line number, the cited
  path and line range, and the actual source text at that location. Task 3 consumes this
  output.

- [ ] **Step 1: Write the audit script**

Create `docs/third-party-stub/check_citations.py`:

```python
#!/usr/bin/env python3
"""Report what every file:line citation in the contract document points at.

The document cites source locations as `path/to/file.rs:123` or
`path/to/file.rs:123-145`. Those line numbers drift whenever the cited file
changes. This prints each citation next to the text actually at that line, so a
reviewer can confirm the citation still lands on what the document claims.

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
```

- [ ] **Step 2: Run it and confirm it finds the expected number of citations**

```bash
cd /home/estraily/projects/refbox-third-party-contract
python3 docs/third-party-stub/check_citations.py > /tmp/citation-report.txt
tail -1 /tmp/citation-report.txt
```

Expected: a final line reading `49 unique citations, N unresolvable`. If the count is
not 49, the regex is missing citation formats present in the document — fix the regex
before continuing, because a citation the script cannot see is a citation nobody checks.

- [ ] **Step 3: Confirm the script detects a broken citation**

The script is worthless if it cannot fail. Prove it reports a bad line number:

```bash
python3 - <<'EOF'
import pathlib
p = pathlib.Path("docs/third-party-integration.md")
t = p.read_text()
p.write_text(t.replace("`uwh-common/src/uwhportal/mod.rs:671`",
                       "`uwh-common/src/uwhportal/mod.rs:999999`", 1))
EOF
python3 docs/third-party-stub/check_citations.py | grep -A1 "999999"
git checkout docs/third-party-integration.md
```

Expected: the report shows `!! OUT OF RANGE` for the injected citation, and the final
`git checkout` restores the document. Confirm `git status` is clean for that file
afterwards.

- [ ] **Step 4: Commit the script**

```bash
git add docs/third-party-stub/check_citations.py
git commit -m "docs(workspace): add a citation checker for the contract document"
```

---

### Task 3: Triage and fix drifted citations

**Files:**
- Modify: `docs/third-party-integration.md` (line numbers only)

**Interfaces:**
- Consumes: `/tmp/citation-report.txt` from Task 2.
- Produces: a document whose citations all resolve, and a written list of any citation
  where the *behaviour* changed rather than just the line number.

- [ ] **Step 1: Read the report and classify every citation**

```bash
less /tmp/citation-report.txt
```

For each block, compare the source text shown against what the document says at that
point. Sort every citation into exactly one of three buckets:

1. **Correct** — points at the thing the document describes. Leave alone.
2. **Drifted** — the same item still exists, at a different line. Fix the number.
3. **Changed** — the item is gone, renamed, or now behaves differently from what the
   document claims. **Do not edit these.** Collect them for Step 4.

The 17 `uwh-common/src/uwhportal/mod.rs` citations in the inventory table were already
confirmed to resolve on master before the rebase (each lands on its `pub fn`), so expect
those in bucket 1. The 9 `overlay/src/network.rs` and 8 `refbox/` citations are the
unverified ones and the most likely to have moved.

- [ ] **Step 2: Fix the drifted line numbers**

For each bucket-2 citation, find the item's new line and update the document. Example of
locating a moved function:

```bash
grep -n "pub fn get_team_roster" uwh-common/src/uwhportal/mod.rs
```

Edit the document so the citation matches. Change **only** the number inside the
backticks — no surrounding prose.

- [ ] **Step 3: Re-run the report until every citation resolves**

```bash
python3 docs/third-party-stub/check_citations.py > /tmp/citation-report-2.txt
tail -1 /tmp/citation-report-2.txt
```

Expected: `49 unique citations, 0 unresolvable`.

Note that zero unresolvable is necessary but not sufficient — a citation can resolve to
the wrong thing. Confirm by re-reading the blocks you changed in the new report.

- [ ] **Step 4: Report any bucket-3 citations and STOP if there are any**

If any citation fell into bucket 3, write them up plainly — for each: what the document
claims, what the code now does, and which call it affects. Then stop and hand that list
to the human. A changed behaviour in a contract document is a contract change and is not
this task's to make.

If bucket 3 is empty, say so explicitly and continue.

- [ ] **Step 5: Commit the corrections**

Only if Step 4 found nothing requiring the human:

```bash
git add docs/third-party-integration.md
git commit -m "docs(workspace): repoint source citations at the rebased base"
```

If no citations drifted, skip this commit and say so — an empty commit would imply work
that did not happen.

---

### Task 4: Final verification

**Files:**
- No edits. Verification only.

**Interfaces:**
- Consumes: the finished branch.
- Produces: evidence the document is internally consistent and ready to be offered as a PR.

- [ ] **Step 1: Confirm all internal links still resolve**

```bash
cd /home/estraily/projects/refbox-third-party-contract
python3 - <<'EOF'
import re, pathlib
t = pathlib.Path("docs/third-party-integration.md").read_text()
heads = set()
for line in t.splitlines():
    m = re.match(r'^#+\s+(.*)$', line)
    if m:
        a = re.sub(r'[`*]', '', m.group(1).lower())
        a = re.sub(r"[^\w\s-]", '', a)
        heads.add(re.sub(r'\s+', '-', a.strip()))
links = set(re.findall(r'\]\(#([^)]+)\)', t))
bad = sorted(l for l in links if l not in heads)
print("internal links:", len(links))
print("BROKEN:", bad if bad else "none")
EOF
```

Expected: `internal links: 10`, `BROKEN: none`.

- [ ] **Step 2: Confirm the call numbering is still self-consistent**

```bash
sed -n '/^## The refbox nine/,/^## Data formats/p' docs/third-party-integration.md | grep "^#### "
sed -n '/^## The other nine/,/^## Keeping this document honest/p' docs/third-party-integration.md | grep "^#### "
```

Expected: entries numbered 1–9 in each section, with no gaps or repeats. Nine plus nine
is the eighteen the document claims.

- [ ] **Step 3: Confirm the stub still runs and serves call 9**

The rebase brought 72 commits of source change underneath the stub; prove it still works
against the current refbox rather than assuming it.

```bash
cd docs/third-party-stub && python3 -m py_compile stub_site.py && echo "syntax OK"
python3 stub_site.py > /tmp/stub-verify.log 2>&1 &
sleep 3
curl -s --get --data-urlencode "teamId=teams/5678-B" \
  http://localhost:8099/api/admin/get-event-team
pkill -f stub_site.py
```

Expected: a `roster` array of 7 entries, including `Frankie` with `["Player", "Coach"]`
and `Gabriel` with `["Coach"]`.

- [ ] **Step 4: Confirm the branch state is clean and ready**

```bash
cd /home/estraily/projects/refbox-third-party-contract
git status --short                                     # expect: clean
git rev-list --count HEAD..origin/master               # expect: 0
git log --oneline origin/master..HEAD | wc -l          # expect: 16, 17, or 18
```

The commit count is 16 plus whatever Tasks 2 and 3 added: 17 if only the checker was
committed, 18 if citations also needed fixing.

- [ ] **Step 5: Report to the human and stop**

Summarise in plain English: whether any citations had drifted and how many, anything in
bucket 3, and that the branch is rebased and current. Then **stop** — pushing and opening
the PR need separate approval and are not part of this plan.

---

## Deviations

**Task 1, Step 5 — the verification command was too broad.** The plan said to check
`git diff <old-head> HEAD -- docs/` and expect empty output. That compares the old branch tip
against the new one across *all* of `docs/`, which includes every documentation change that
arrived with master's 72 commits — 277KB of unrelated diff. Replaced with a diff limited to the
five files this branch actually touches, which was correctly empty. `git range-diff` was added
alongside it and confirmed 16 of 16 commits identical.

**Task 2, Step 2 — the "expect 49 citations" check was circular.** The plan told the executor to
confirm the checker finds 49 citations, treating a mismatch as evidence of a bad regex. But 49 was
itself derived from the same regex, so agreement proved nothing. The document also cites
`schedule.rs:762` with the path implied by context, and `:718` with both file and path implied.
Twenty more citations — just under a third of the total — were invisible to both the count and the
tool. The checker was extended to resolve both shorthand forms the way a reader does, against the
nearest preceding full citation, bringing the total to 69. **A count derived from the tool under
test cannot validate that tool; the expected value has to come from somewhere else.**

**Task 3 — six citations had drifted, all in `refbox/`.** `uwh-common` and `overlay` had not moved
at all. Bucket 3 (behaviour changed rather than line moved) was empty, so nothing required the
human. One drift was worse than a stale number: `refbox.ftl:181-184` had come to span out of the
portal linking instructions and into the custom-site key added since, citing the wrong string
entirely.
