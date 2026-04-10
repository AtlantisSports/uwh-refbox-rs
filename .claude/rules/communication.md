# Communication

These rules govern how Claude communicates with the human in this project. The human is a
domain expert and tournament organizer, not a programmer. All collaboration must account for this.

## Language

**Use plain English for all explanations.** Do not assume programming knowledge. If a technical
term is necessary, define it the first time it appears.

When something is technically complex, explain the trade-off in terms of **outcomes and
behaviour**, not implementation details. For example:

- Instead of: *"We need to call `end_confirm_pause` before `start_clock` to prevent a race
  condition in the state machine."*
- Say: *"Without this fix, the game can get stuck in a half-finished state after the score
  confirmation screen closes. This change ensures the game transitions cleanly to the next stage."*

## Approval Gates

**Always ask before:**
- Creating a new branch
- Making a commit
- Pushing to the remote repository
- Opening a pull request
- Making any change that affects shared infrastructure (Cargo.toml dependencies, CI workflows,
  cross-compilation configuration)

The human cannot easily undo a pushed commit or a merged PR. When in doubt, pause and confirm.

## Before Every Commit

Provide a plain-language summary that includes:
1. **What changed** — in plain English, describing the behaviour, not the code
2. **Why** — the reason this change was needed
3. **How to verify** — what the human can do or observe to confirm it worked

This summary becomes the PR description. The human uses it to review the change via
`docs/review-checklist.md`.

## Handling Ambiguity

**Never assume intent.** If a request could be interpreted in more than one way, ask for
clarification before starting. State the ambiguity clearly: *"I could do X or Y here — which do
you mean?"*

This is especially important for requests that could affect `uwh-common` or the wireless-remote,
where the blast radius of a wrong assumption is high.

## When CI Fails

Explain what failed in plain English before suggesting a fix. For example:

- Instead of: *"The clippy check failed with `warning: unused variable: tm`."*
- Say: *"The automated code check found a small issue: there's a variable named `tm` that was
  created but never used. I'll remove it."*

Then fix it. The human should not need to interpret CI output themselves.

## Verifiability

**Never recommend an action the human cannot verify without reading code.** Every proposed change
should come with at least one of:
- A visible behaviour change the human can observe by running the software
- A test that passes (and would have failed before the fix)
- A CI check that is now green (and was red before)

If a change is purely internal and has no observable effect, say so explicitly and explain why it
is still necessary.
