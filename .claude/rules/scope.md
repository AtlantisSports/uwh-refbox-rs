# Scope Enforcement

These rules prevent scope creep — the tendency to make changes beyond what was asked. Scope
creep is the primary source of AI-generated problems in this codebase.

## Core Rules

**Before touching any file, list every file you plan to modify and state why each one is
necessary.** Do not begin editing until the scope is clear from the user's request or from the
branch that has been created.

**One branch = one concern.** If a second, unrelated issue is discovered while working, stop.
Document it as a suggestion only — do not fix it on the current branch. Propose a separate branch
for it.

**Never modify files outside the stated scope**, even if:
- The nearby code looks wrong
- Fixing it would be quick
- It would "clean things up"
- It seems obviously related

**If scope is unclear, ask before starting** — not halfway through, and not after the fact.

## What Is Never Acceptable Without Explicit Request

- Opportunistic refactoring of code that wasn't the subject of the task
- Renaming variables, functions, or types "for clarity"
- Adding comments, docstrings, or inline documentation to untouched code
- Adding error handling for cases not relevant to the fix
- Updating dependencies unless the task is specifically a dependency update
- Reformatting or reordering code that isn't being changed for another reason
- Adding abstractions, traits, or helper functions "for future use"

## What to Do When You Notice Something Outside Scope

If you notice a bug, improvement opportunity, or code smell outside the current scope:

1. Finish the current task first
2. After completing it, say: *"I also noticed [issue] in [file]. Would you like me to address
   that in a separate branch?"*
3. Wait for the user's answer before doing anything

## Scope and Branch Creation

Creating a branch is the user's approval of that scope. The branch name encodes what the scope
is. For example:

- `fix/refbox/confirm-score-timing` → scope is the confirm-score timing bug in the refbox crate
- `feat/schedule-processor/list-of-placements` → scope is a new feature in schedule-processor

Do not touch `uwh-common` or `overlay` on the first branch above unless the fix demonstrably
requires it — and if it does, say so before touching those files.
