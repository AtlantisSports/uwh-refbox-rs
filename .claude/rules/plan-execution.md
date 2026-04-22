# Plan Execution

These rules govern how implementation plans are executed. They exist to keep the process
right-sized for the work — more discipline for high-blast-radius code, less ceremony for
lower-risk feature work.

## Default process (lean)

For feature work in lower-risk crates — `refbox` UI, `overlay` layout, translations,
`schedule-processor` CLI — use the lean process:

1. **No per-task deviation commits.** If execution deviates from the plan, note it in the PR
   description or a running "Deviations" section at the bottom of the plan file. Do not create
   standalone `docs(workspace): record Task N deviations` commits.
2. **Code review once per feature, at the end.** Run `superpowers:requesting-code-review` when
   the feature is complete and ready for PR review, not after every task.
3. **Skip verification ceremony on mechanical tasks.** Translation-key additions, message-enum
   variant wire-up, field-passthrough tasks, and similar no-behaviour-change work do not need
   `superpowers:verification-before-completion`. Compilation + `just check` is enough.
4. **Do not amend ADRs mid-execution.** If execution reveals an ADR was wrong or incomplete,
   note it in the plan's Deviations section and address the ADR in a single amendment at the
   end of the feature — or leave the ADR accurate-as-of-approval and record what changed in
   the PR.

## Heavy process (when warranted)

Use the full ceremony — per-task verification, per-task code review, strict deviation
tracking — only when blast radius is high:

- Any change to `uwh-common` (shared types, wire format, serialization)
- Any change to `wireless-remote` (embedded firmware)
- Any change to a state machine (game clock, tournament manager, penalty tracking)
- Any change that crosses the wire format between refbox and the LED panel, wireless remote,
  or stream overlay

The trigger is blast radius, not complexity. A one-line change to `uwh-common` warrants more
care than a 200-line refactor inside `refbox/src/app/view_builders/`.

## Writing future plans

Keep plans right-sized. For typical UI, layout, translation, or CLI features, a plan of
~200–500 lines is usually sufficient. It should cover:

- **Goal and scope boundary** (which crates, what is explicitly out of scope)
- **Acceptance criteria** (what the human can observe or run to confirm it works)
- **Architectural sketch** (which files and types change and why)
- **A rough task list** — not step-by-step scripts for every line of code

Plans over ~800 lines are a signal of over-specification. The cost of keeping an over-detailed
plan in sync with reality during execution exceeds its planning value — and it produces
noise-commits that amend the plan mid-stream. When in doubt, write a shorter plan and expect
the executing subagent to make reasonable decisions inside the sketch.
