# Open New Display Button — Design Spec

**Date:** 2026-05-18
**Status:** Proposed (ready for implementation planning)

## Summary

Add a button to the Display Options tab labelled "Open new display" that
opens an additional panel-simulator window. The button is additive:
every press opens a new window. This covers two operator use cases:

1. Reopening a simulator window that was closed accidentally (today the
   only way is to restart the refbox).
2. Showing the simulated panel in more than one window at once.

Operationally, the simulator already supports multiple connected windows
— the refbox's binary TCP listener accepts unlimited clients and
broadcasts the same snapshot stream to all of them. What is missing is a
way to launch additional sim windows from inside the running refbox.

## Scope

In scope:

- New button in the Display Options config page in
  [refbox/src/app/view_builders/configuration.rs](../../../refbox/src/app/view_builders/configuration.rs)
- New `Message` variant in
  [refbox/src/app/message.rs](../../../refbox/src/app/message.rs)
- Handler in [refbox/src/app/mod.rs](../../../refbox/src/app/mod.rs)
- Extracted spawn helper used by both the startup spawn in
  [refbox/src/main.rs](../../../refbox/src/main.rs) and the new handler
- New translation key added to `refbox/translations/<locale>/refbox.ftl`
  for every supported locale (15 locales today)
- Field rename `sim_child: Option<Child>` → `sim_children: Vec<Child>`
  on `RefBoxApp` and `RefBoxAppFlags`
- Updates to the three cleanup sites that today kill `sim_child` so
  they walk the new `Vec`

Out of scope:

- Any change to CLI arguments — `--no-simulate`, `--is-simulator`, and
  port flags behave exactly as before
- Any change to the simulator process itself (`led-panel-sim` crate)
- Any change to wire format, snapshot broadcasting, or the binary
  listener — those already support multiple clients
- Window positioning — new windows open at the same hard-coded position
  as the original (the operator drags them where they need to go)
- Operator-visible error UI for spawn failure — see Error Handling
- A way to switch the new window's display mode (matrix vs sunlight).
  New windows inherit whatever mode the original was launched with
- A per-window cap on the number of open simulator windows

## Operator-Visible Behavior

After this change:

1. **New button on the Display Options tab.** Labelled "Open new
   display". Lives in one of the two currently-empty rows below the
   existing brightness/hide-time row.
2. **Pressing the button opens a fresh simulator window.** The window
   appears at the same screen position as the original startup window
   and immediately begins mirroring the same panel data.
3. **The button can be pressed any number of times.** Each press opens
   another window. There is no cap.
4. **Closing a simulator window has no effect on the refbox.** The
   refbox continues running, and the remaining sim windows continue
   updating.
5. **The button works regardless of `--no-simulate`.** If the refbox
   was started with `--no-simulate` (no startup sim window), pressing
   the button still opens a fresh sim window. The binary TCP listener
   runs unconditionally, so connections succeed.
6. **Quitting the refbox closes all simulator windows.** Same behaviour
   the original sim window has today, applied uniformly to every
   tracked window.
7. **Restarting the refbox in-app (Mode change, font change) closes
   all simulator windows.** Same behaviour as today, applied to every
   tracked window. The restarted refbox will spawn a fresh startup sim
   window (if `--no-simulate` was not used); any extras the operator
   had opened do not auto-restore.

## Architectural Sketch

### 1. New `SimSpawnConfig` struct

A small plain-data struct that carries everything needed to spawn a
simulator child process. Built once in `main()` from the parsed `Cli`,
threaded through `RefBoxAppFlags`, stored on `RefBoxApp`. Fields:

- `binary_port: u16`
- `scale: f32`
- `spacing: f32`
- `sunlight_mode: bool`
- `verbose: u8`
- `log_location: PathBuf`
- `log_max_file_size: u64`
- `num_old_logs: u32`

### 2. Extracted spawn helper

Pull the existing child-spawn block at
[refbox/src/main.rs:290-337](../../../refbox/src/main.rs#L290-L337) into a
free function:

```rust
fn spawn_sim_child(config: &SimSpawnConfig) -> std::io::Result<Child>
```

`main()` calls it for the startup spawn. The new message handler also
calls it. Single source of truth for the argv layout.

### 3. Field rename: `sim_child` → `sim_children`

`Option<Child>` → `Vec<Child>` on both `RefBoxApp` and `RefBoxAppFlags`.
The startup-spawned sim (when present) is just the first entry in the
vec. The three existing cleanup sites become loops over the vec,
preserving the existing semantic distinction between them:

- [refbox/src/app/mod.rs:958](../../../refbox/src/app/mod.rs#L958)
  (`RestartAndApply` for Mode change): `kill()` every child. Comment
  in the existing code explains: "Kill the simulator child so it does
  not linger as an orphan after the iced runtime closes its windows."
- [refbox/src/app/mod.rs:2538](../../../refbox/src/app/mod.rs#L2538)
  (Language change requiring a font-family restart): `kill()` every
  child. Same rationale.
- [refbox/src/app/mod.rs:1053](../../../refbox/src/app/mod.rs#L1053)
  (`Drop` for `RefBoxApp`, the normal shutdown path): `wait()` for
  every child. The sims exit gracefully on their own once the refbox's
  TCP listener task drops (each sim's reader sees `read == 0` and
  emits `Message::Stop`, which calls `iced::exit`). Waiting ensures
  the refbox process doesn't exit before its children.

### 4. New `Message` variant

`Message::OpenNewDisplay`. The handler in `update()`:

1. Calls `spawn_sim_child(&self.sim_spawn_config)`.
2. On `Ok(child)`, pushes onto `self.sim_children`.
3. On `Err(e)`, logs at `error!` level and returns `Task::none()`.

### 5. New button in Display Options

Added to `make_display_config_page` in
[refbox/src/app/view_builders/configuration.rs:875](../../../refbox/src/app/view_builders/configuration.rs#L875).
Uses the `make_button` helper from
[refbox/src/app/view_builders/shared_elements.rs:950](../../../refbox/src/app/view_builders/shared_elements.rs#L950)
(the right helper for an action-only button — `make_value_button` is
for value-displaying buttons like brightness and hide-time, which
doesn't fit here). Placed in one of the two currently-empty rows
(lines 950–951 of `configuration.rs`).

### 6. New translation key

Key: `open-new-display`. Value style: ALL CAPS, matching the existing
pattern (e.g. `display-options = DISPLAY OPTIONS`,
`sound-options = SOUND OPTIONS`).

Added to every locale file under `refbox/translations/<locale>/refbox.ftl`.
The 15 locales today are: `de-DE`, `en-US`, `es`, `fr`, `id-ID`,
`it-IT`, `ja-JP`, `ko-KR`, `ms-MY`, `nl-NL`, `pt-PT`, `th-TH`, `tl-PH`,
`tr-TR`, `zh-CN`. English value: `OPEN NEW DISPLAY`. Other locale
values translated to match each file's existing tone and style.

## Error Handling

If `spawn_sim_child` fails at runtime (rare; would mean the binary the
refbox launched from is no longer reachable on disk), log the error at
`error!` level and return `Task::none()`. No operator-visible error
page in v1.

Rationale: the failure surface is the operator pressing the button and
no new window appearing — recoverable, and the operator can try again
or restart the refbox. Adding a confirmation page for a near-impossible
runtime failure costs more than it pays. If this turns out to be a
real problem in practice, a follow-up branch can add a confirmation
page using the existing `ConfirmationKind::Error(...)` mechanism.

## Testing

### Unit test

In `refbox/src/main.rs` (or wherever `spawn_sim_child` lives), a test
that constructs a `SimSpawnConfig` with known values and asserts the
resulting argv matches the expected list. The test inspects the
`Command`'s captured args via `Command::get_args()` (available on
stable) without actually spawning a process.

### Manual tests

The non-programmer operator can verify all of the following from the
running refbox:

1. **Golden path.** Launch the refbox normally → open Display Options
   → press "Open new display" → confirm a second window appears and
   shows the same content as the original.
2. **Multiple windows.** From step 1, press the button two more times
   → confirm four windows total are open and all mirror the same
   snapshot.
3. **Independence on close.** Close one sim window with its OS close
   button → confirm the refbox keeps running and the other windows
   keep updating.
4. **Reopening after close.** Close all sim windows manually → press
   the button → confirm a new sim window appears.
5. **Cleanup on quit.** With multiple windows open, quit the refbox →
   confirm all sim windows close.
6. **Cleanup on in-app restart.** With multiple windows open, change
   the App Mode (Hockey ↔ Rugby) and press Restart → confirm all sim
   windows close as the refbox restarts, and the new refbox spawns its
   normal startup sim window.

## Scope Boundary

- Touches **`refbox` crate only**.
- Does **not** touch `uwh-common`, `led-panel-sim`, `matrix-drawing`,
  `wireless-remote`, or any of the utility crates.
- Does **not** change wire formats, serialization, or the TCP listener.
- Does **not** add new dependencies.

Blast radius is contained to the refbox application binary. Per
[.claude/rules/plan-execution.md](../../../.claude/rules/plan-execution.md),
this is lean-process work — feature-level code review at the end, not
per-task.
