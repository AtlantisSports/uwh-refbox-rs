# Beep-Test LED Panel Score Hiding — Design

**Date:** 2026-05-21
**Status:** Approved through brainstorming; awaiting implementation plan
**Owner:** e-straily (with Claude)
**Branch:** `feat/refbox/beep-test-redesign` (continuing after Chunk 5 at `8d238cb`)
**Chunk:** 6 of 6 follow-on improvements to the beep-test mode
**Process gate:** Heavy (per `.claude/rules/plan-execution.md`) — touches the refbox↔LED-panel wire format. Per-task verification, unit test required.

---

## Goal

When the refbox is in beep-test mode, the LED panel should hide one of the
two score columns so only the lap count remains visible on the active side.
Today both score columns render in beep-test mode, both showing `0`, because
the wire bit that tells the panel "this is beep-test, hide one side" is
hardcoded `false` and never updated to reflect the actual mode.

---

## Motivation

Operator walkthrough of Chunk 5 using `led-panel-sim` confirmed: in
beep-test mode, the LED panel renders two `0` blocks (blue on left, white on
right), exactly as in game mode. The intended behaviour — already implemented
by the existing draw logic — is for one side to be blank, leaving the lap
count area clear.

The drawing logic is already correct: at `matrix-drawing/src/drawing.rs:239`
and `:272`, the left/right score draws are gated on
`!beep_test || (!)white_on_right`. When `beep_test == true` and
`white_on_right == true`, the left (blue) score block is suppressed; when
`white_on_right == false`, the right (blue) block is suppressed. Either way,
the white side stays. The fix is purely on the transmit side: the `beep_test`
field in the outgoing `TransmittedData` struct must reflect the actual
operating mode, not a hardcoded `false`.

The hardcoded `false` lives in two spots in
`refbox/src/app/update_sender.rs`:

- Line 171 — `serial_worker_loop()` builds the initial `TransmittedData`
  for a serial connection (one-shot at worker startup; subsequent serial
  frames mutate `data.white_on_right` in place but leave `data.beep_test`
  alone).
- Line 441 — `Server::encode_flash()` rebuilds `TransmittedData` from
  `Server` state for every broadcast frame on the binary TCP port.

Both must be replaced with a value plumbed from the application's mode at
construction time. The mode is fixed at refbox startup — the operator
restarts the app to switch between Hockey6V6, Hockey3V3, and BeepTest — so a
one-time construction-time plumb is correct. No per-snapshot field on the
existing `ServerMessage::NewSnapshot` or `SerialWorkerMessage::NewSnapshot`
variants is needed.

---

## Scope

### Files touched

- `refbox/src/app/update_sender.rs` — add a `beep_test: bool` field to the
  `Server` struct, plumb it through `Server::new()` and `UpdateSender::new()`,
  pass it as a new argument into `serial_worker_loop()` via
  `Server::add_serial_sender()`, replace the two hardcodes at lines 171 and
  441. Existing integration test (around line 671) extended; one new unit
  test added (heavy process).
- `refbox/src/app/mod.rs` — single call site for `UpdateSender::new()`
  (line 1222); pass `config.mode == Mode::BeepTest` as the new fifth
  argument.

### Not touched

- `matrix-drawing/src/drawing.rs` — the score-hiding logic is already
  correct.
- `matrix-drawing/src/transmitted_data.rs` — the wire format already carries
  a `beep_test` bit at byte 0 bit 2. No serialization change.
- `led-panel-sim` — once the correct bit flows over the wire, the sim's
  decoder picks it up automatically (verified path: it decodes
  `TransmittedData` and forwards `beep_test` into the same `drawing.rs`).
- The operator-controlled `white_on_right` config — operator continues to
  choose which side is the hidden one; this spec does not force or change
  `white_on_right` in beep-test mode.
- The sound controller, the cadence engine, the view layer.
- `wireless-remote` (no firmware change).

---

## Design

### Approach A — set at construction (approved)

The `beep_test` flag is set once, at `UpdateSender::new()` time, from
`config.mode == Mode::BeepTest`. It is stored as a field on `Server`,
captured into the spawn closure for each `serial_worker_loop()` task as a
parameter, and read by `Server::encode_flash()` from the struct field. Both
`TransmittedData` construction sites read the stored value instead of
hardcoding `false`.

#### `UpdateSender::new()` signature change

Today (line 38–43):

```rust
pub fn new(
    initial: Vec<SerialPortBuilder>,
    binary_port: u16,
    json_port: u16,
    hide_time: bool,
) -> Self
```

After:

```rust
pub fn new(
    initial: Vec<SerialPortBuilder>,
    binary_port: u16,
    json_port: u16,
    hide_time: bool,
    beep_test: bool,
) -> Self
```

The new `beep_test` argument is forwarded to `Server::new()`.

#### `Server` struct and `Server::new()` change

Add a field to `Server` (struct around line 312):

```rust
struct Server {
    // ...existing fields...
    hide_time: bool,
    beep_test: bool,  // new
}
```

`Server::new()` (around line 329) accepts a `beep_test: bool` parameter and
stores it on the struct.

#### `serial_worker_loop()` signature change

Today (line 154–157):

```rust
async fn serial_worker_loop(
    mut rx: mpsc::Receiver<SerialWorkerMessage>,
    mut write: SerialStream,
) -> Result<(), WorkerError>
```

After:

```rust
async fn serial_worker_loop(
    mut rx: mpsc::Receiver<SerialWorkerMessage>,
    mut write: SerialStream,
    beep_test: bool,
) -> Result<(), WorkerError>
```

At line 171, replace `beep_test: false,` with `beep_test,`.

#### `Server::add_serial_sender()` update

This is the single spawn site for `serial_worker_loop` (line 384). Pass
`self.beep_test` as the third argument:

```rust
let join = task::spawn(serial_worker_loop(rx, sender, self.beep_test));
```

#### `Server::encode_flash()` update

At line 441, replace `beep_test: false,` with `beep_test: self.beep_test,`.

#### `mod.rs` call site update

Today (line 1222):

```rust
UpdateSender::new(serial_ports, binary_port, json_port, config.hide_time);
```

After:

```rust
UpdateSender::new(
    serial_ports,
    binary_port,
    json_port,
    config.hide_time,
    config.mode == Mode::BeepTest,
);
```

`Mode` is already imported in `mod.rs` (used at lines 1162, 1178, 1187,
1286, 3803, 3941). No new use-statement needed.

### Unit test

One new test in the existing `#[cfg(test)] mod tests` block of
`update_sender.rs`, modelled on the existing test that begins around line
671:

**`server_emits_beep_test_flag_when_constructed_in_beep_test_mode`**

- Construct `UpdateSender::new(vec![], BINARY_PORT, JSON_PORT, false, true)`
  on a fresh pair of ports.
- Connect a binary TCP client.
- Call `update_sender.send_snapshot(snapshot, false /* white_on_right */,
  Brightness::Low)`.
- Read the binary frame and decode it as `TransmittedData`.
- Assert `data.beep_test == true`.

The existing integration test at line 671 is also updated:

- Pass `false` as the new fifth argument to `UpdateSender::new(...)` (it
  was implicitly false-by-hardcode before).
- The local `let beep_test = false;` (line 733) and the
  `binary_expected = TransmittedData { ..., beep_test, ... }` (line 835)
  already expect `false`, so no further test-body change is needed beyond
  the constructor argument. That test continues to exercise the
  not-in-beep-test case end-to-end.

---

## Acceptance criteria

Walking through the running refbox + `led-panel-sim`:

1. **Launch refbox in beep-test mode + led-panel-sim.** The panel renders
   the beep-test layout. Only ONE side shows a number column; the other
   side is blank.
   - If `white_on_right == true` (operator's preferred setting): blue
     (left) side is blank; the lap count area remains uncluttered on that
     half.
   - If `white_on_right == false`: white (left) side is blank.
2. **Launch refbox in Hockey6V6 or Hockey3V3 mode + led-panel-sim.** Both
   score columns render as before — no regression in game mode.
3. **`just check` passes** with the new unit test and the updated existing
   test.

Heavy-process verification: `just check` is the first gate; the
led-panel-sim walkthrough is the second gate.

---

## Out of scope (intentionally deferred)

- Forcing `white_on_right` to `true` in beep-test mode. The operator
  continues to control this via the existing settings page. If a future
  request asks for an auto-set, it is its own spec.
- Any change to `drawing.rs` (logic is already correct).
- Any change to the wire format encoding/decoding in `transmitted_data.rs`.
- The audio-delay-on-buzzer concern from the Chunk 4 walkthrough — a
  separate investigation, captured in the Branch 2 project memory.
- LED panel work for non-beep-test modes (Branch 3 territory).
