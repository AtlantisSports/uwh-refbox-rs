# Beep-Test LED Panel Score Hiding — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** When the refbox is in BeepTest mode, the LED panel should hide one of the two score columns (the side determined by `white_on_right`), so only the lap-count area remains uncluttered on the active side. Today both columns render `0` because the transmitted `beep_test` bit is hardcoded `false`.

**Architecture:** Plumb a `beep_test: bool` flag from `config.mode == Mode::BeepTest` at refbox startup, through `UpdateSender::new()` → `Server::new()` → stored as a field on `Server` and passed into each `serial_worker_loop` task. Two `TransmittedData` construction sites in `refbox/src/app/update_sender.rs` (lines 171 and 441) read the plumbed value instead of hardcoding `false`. No change to `matrix-drawing/` — its draw logic already gates score-column rendering on this bit.

**Tech Stack:** Rust 2024, MSRV 1.85.

**Spec:** `docs/superpowers/specs/2026-05-21-beep-test-led-panel-score-hiding-design.md` (committed at `2752a84`).

**Process:** Heavy (per `.claude/rules/plan-execution.md` — change crosses the refbox↔LED-panel wire format). Per-task TDD: write the new test first, confirm it fails (compile failure is the fail-state here, since the new test references the not-yet-added `beep_test` argument), apply the plumbing + hardcode replacements + call-site update as a single coherent change, confirm the test passes. Final led-panel-sim walkthrough.

---

## File Structure

| File | Role | Tasks |
|------|------|-------|
| `refbox/src/app/update_sender.rs` | `UpdateSender::new` signature, `Server` struct + `Server::new`, `serial_worker_loop` signature, `Server::add_serial_sender` spawn site, `Server::encode_flash` hardcode replacement, the line-171 hardcode replacement inside `serial_worker_loop`, the `#[cfg(test)] mod tests` block (one new test, one existing test arg-update) | Task 1 |
| `refbox/src/app/mod.rs` | Single call site of `UpdateSender::new` at line 1222 — pass `config.mode == Mode::BeepTest` as new fifth argument | Task 1 |

Signature changes ripple from `UpdateSender::new` into `mod.rs`'s only call site, so the plumbing change and the call-site update must land together — the workspace will not compile in between. One implementation task plus one walkthrough task.

---

### Task 1: TDD — plumb `beep_test` flag through `UpdateSender` and fix both hardcodes

**Files:**
- Modify: `refbox/src/app/update_sender.rs`
- Modify: `refbox/src/app/mod.rs`

- [ ] **Step 1: Add the new failing test**

Append to the `#[cfg(test)] mod tests` block in `refbox/src/app/update_sender.rs`. Place it after the existing big integration test (the one that starts around line 671 with `let white_on_right = false;`). The new test follows the same pattern but constructs `UpdateSender` in beep-test mode and asserts the decoded `beep_test` bit is `true`:

```rust
// Test — when UpdateSender is constructed with beep_test=true (i.e. the
// refbox was launched in BeepTest mode), the binary frames it emits to
// any connected client carry `beep_test == true` so the LED panel
// renderer hides one score column. See
// docs/superpowers/specs/2026-05-21-beep-test-led-panel-score-hiding-design.md.
#[tokio::test]
async fn binary_port_emits_beep_test_flag_when_constructed_in_beep_test_mode() {
    const BINARY_PORT: u16 = 4_700; // pick a free port distinct from the other test
    const JSON_PORT: u16 = 4_701;
    const MAX_CONN_FAILS: u8 = 20;

    let update_sender =
        UpdateSender::new(vec![], BINARY_PORT, JSON_PORT, false, /* beep_test */ true);

    // Connect to the binary port (retry until the listener is up, mirroring
    // the existing integration test's pattern).
    let mut binary_conn;
    let mut fail_count = 0;
    loop {
        match TcpStream::connect(("localhost", BINARY_PORT)).await {
            Ok(conn) => { binary_conn = conn; break; }
            Err(e) => {
                if e.kind() == ErrorKind::ConnectionRefused {
                    assert_le!(fail_count, MAX_CONN_FAILS);
                    fail_count += 1;
                } else {
                    panic!("Unexpected connection error: {e:?}");
                }
            }
        };
    }

    // Send a snapshot through the sender so the server emits a frame.
    let snapshot = GameSnapshot {
        current_period: GamePeriod::FirstHalf,
        secs_in_period: 30,
        timeout: None,
        scores: BlackWhiteBundle { black: 0, white: 0 },
        penalties: BlackWhiteBundle { black: vec![], white: vec![] },
        warnings: BlackWhiteBundle { black: vec![], white: vec![] },
        fouls: OptColorBundle { black: vec![], white: vec![], equal: vec![] },
        is_old_game: false,
        game_number: "1".to_string(),
        next_game_number: "2".to_string(),
        event_id: None,
        recent_goal: None,
        next_period_len_secs: None,
        conf_pause_time: None,
    };
    update_sender
        .send_snapshot(snapshot.clone(), false /* white_on_right */, Brightness::Low)
        .unwrap();

    // Read one full TransmittedData frame from the binary port and decode it.
    let expected_bytes = TransmittedData {
        white_on_right: false,
        brightness: Brightness::Low,
        flash: false,
        beep_test: true,
        snapshot: snapshot.clone().into(),
    }
    .encode()
    .unwrap()
    .len();

    let mut buf = vec![0u8; expected_bytes];
    let mut read_so_far = 0;
    while read_so_far < expected_bytes {
        let n = binary_conn.read(&mut buf[read_so_far..]).await.unwrap();
        assert_ne!(n, 0, "binary port closed unexpectedly");
        read_so_far += n;
    }

    let decoded = TransmittedData::decode(&buf).unwrap();
    assert!(
        decoded.beep_test,
        "expected beep_test=true in transmitted frame, got false"
    );
}
```

If `TransmittedData::decode` is not the exact decode entry point (the existing test compares raw bytes rather than decoding), fall back to comparing the encoded `expected_bytes` byte-by-byte and assert equality — the existing test pattern at line ~854 does this. The point is to verify the `beep_test` bit flows end-to-end through the binary port.

- [ ] **Step 2: Run the new test; confirm it FAILS (compile error)**

```
cargo test -p refbox --lib app::update_sender::tests::binary_port_emits_beep_test_flag_when_constructed_in_beep_test_mode
```

Expected: the workspace fails to compile because `UpdateSender::new` does not yet accept a fifth `beep_test` argument. The compile error is the fail-state for this TDD step.

If the test compiles unexpectedly, stop — the signature may already have been changed.

- [ ] **Step 3: Apply the plumbing change in `update_sender.rs`**

Three signature changes, one struct field, two hardcode replacements, one existing-test arg update:

**3a. `UpdateSender::new` signature (line 38–43).** Replace:

```rust
pub fn new(
    initial: Vec<SerialPortBuilder>,
    binary_port: u16,
    json_port: u16,
    hide_time: bool,
) -> Self {
```

with:

```rust
pub fn new(
    initial: Vec<SerialPortBuilder>,
    binary_port: u16,
    json_port: u16,
    hide_time: bool,
    beep_test: bool,
) -> Self {
```

Inside the body (line 51), update the `Server::new` call to forward the flag:

```rust
let server_join = task::spawn(Server::new(rx, initial, hide_time, beep_test).run_loop());
```

**3b. `Server` struct (around line 312–326).** Add a `beep_test: bool` field. Append after the existing `hide_time: bool` field:

```rust
struct Server {
    // ...all existing fields unchanged...
    hide_time: bool,
    beep_test: bool,  // new
}
```

**3c. `Server::new` signature (around line 329).** Replace:

```rust
pub fn new(
    rx: mpsc::Receiver<ServerMessage>,
    initial: Vec<SerialStream>,
    hide_time: bool,
) -> Self {
```

with:

```rust
pub fn new(
    rx: mpsc::Receiver<ServerMessage>,
    initial: Vec<SerialStream>,
    hide_time: bool,
    beep_test: bool,
) -> Self {
```

Inside the struct literal in the body (around line 334–347), add the new field after `hide_time`:

```rust
let mut server = Server {
    // ...existing fields...
    hide_time,
    beep_test,
};
```

**3d. `serial_worker_loop` signature (line 154–157).** Replace:

```rust
async fn serial_worker_loop(
    mut rx: mpsc::Receiver<SerialWorkerMessage>,
    mut write: SerialStream,
) -> Result<(), WorkerError> {
```

with:

```rust
async fn serial_worker_loop(
    mut rx: mpsc::Receiver<SerialWorkerMessage>,
    mut write: SerialStream,
    beep_test: bool,
) -> Result<(), WorkerError> {
```

**3e. The hardcode at line 171.** Inside `serial_worker_loop`, in the initial `TransmittedData` literal, replace:

```rust
let mut data = TransmittedData {
    snapshot,
    flash: false,
    beep_test: false,
    brightness,
    white_on_right,
};
```

with:

```rust
let mut data = TransmittedData {
    snapshot,
    flash: false,
    beep_test,
    brightness,
    white_on_right,
};
```

**3f. `Server::add_serial_sender` spawn site (line 384).** Replace:

```rust
let join = task::spawn(serial_worker_loop(rx, sender));
```

with:

```rust
let join = task::spawn(serial_worker_loop(rx, sender, self.beep_test));
```

**3g. The hardcode at line 441.** Inside `Server::encode_flash`, replace:

```rust
TransmittedData {
    white_on_right: self.white_on_right,
    flash: self.flash,
    beep_test: false,
    brightness: self.brightness,
    snapshot: self.snapshot.clone(),
}
```

with:

```rust
TransmittedData {
    white_on_right: self.white_on_right,
    flash: self.flash,
    beep_test: self.beep_test,
    brightness: self.brightness,
    snapshot: self.snapshot.clone(),
}
```

**3h. Update the existing integration test's `UpdateSender::new` call (line 671).** Replace:

```rust
let update_sender = UpdateSender::new(vec![], BINARY_PORT, JSON_PORT, false);
```

with:

```rust
let update_sender = UpdateSender::new(vec![], BINARY_PORT, JSON_PORT, false, false);
```

The existing test's `let beep_test = false;` (line 733) and its `TransmittedData { ..., beep_test, ... }` (line 835) already expect `false`, so the asserted bytes match without further change.

- [ ] **Step 4: Apply the call-site change in `mod.rs`**

In `refbox/src/app/mod.rs` at line 1222. Replace:

```rust
UpdateSender::new(serial_ports, binary_port, json_port, config.hide_time);
```

with:

```rust
UpdateSender::new(
    serial_ports,
    binary_port,
    json_port,
    config.hide_time,
    config.mode == Mode::BeepTest,
);
```

`Mode` is already imported in `mod.rs` (used at lines 1162, 1178, 1187, 1286, 3803, 3941) — no new `use` statement.

- [ ] **Step 5: Build and confirm the workspace compiles**

```
cargo build -p refbox --tests
```

Expected: clean compile.

- [ ] **Step 6: Run the new test and the existing big integration test; confirm both PASS**

```
cargo test -p refbox --lib app::update_sender::tests::binary_port_emits_beep_test_flag_when_constructed_in_beep_test_mode
cargo test -p refbox --lib app::update_sender::tests
```

Expected: both PASS. The new test asserts `beep_test=true` flows through; the existing integration test continues to confirm `beep_test=false` flows through when not in beep-test mode (its hardcoded expectation in `binary_expected`).

- [ ] **Step 7: Run `just check` to confirm fmt, clippy, and the full test suite**

```
just check
```

Expected: PASS.

- [ ] **Step 8: Commit**

```
git add refbox/src/app/update_sender.rs refbox/src/app/mod.rs
git commit -m "fix(refbox): plumb beep_test flag through UpdateSender for LED panel"
```

With Co-Authored-By footer:

```
Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
```

---

### Task 2: Walkthrough verification with led-panel-sim

**Files:** none. Smoke-test the running refbox alongside the LED panel simulator.

- [ ] **Step 1: Launch the LED panel simulator**

In one terminal window/tab:

```
cargo run --manifest-path /home/estraily/projects/uwh-refbox-rs/.worktrees/feat-refbox-beep-test-redesign/Cargo.toml -p led-panel-sim
```

(If the sim has connection-target flags or auto-connects to the refbox's binary port, use the project's standard sim launch — defer to the operator if uncertain.)

- [ ] **Step 2: Launch the refbox in BeepTest mode**

In another terminal:

```
WAYLAND_DISPLAY= cargo run --manifest-path /home/estraily/projects/uwh-refbox-rs/.worktrees/feat-refbox-beep-test-redesign/Cargo.toml -p refbox
```

If the refbox is not already configured for BeepTest mode, go Settings → Mode and choose BeepTest, then restart the refbox so the new `UpdateSender` is constructed with `beep_test=true`. (BeepTest mode plumbing is set once at startup — switching mode at runtime without restart will NOT update the binary stream's beep_test bit. This is consistent with the spec's design choice.)

- [ ] **Step 3: Verify the LED panel sim's display in BeepTest mode**

Walk through the spec's three acceptance criteria:

1. **Refbox in BeepTest mode + led-panel-sim running.** The panel renders the beep-test layout. Only ONE side shows a number column; the other side is blank.
   - With operator's preferred `white_on_right == true`: the LEFT (blue) side is blank; the right side shows the lap count.
   - With `white_on_right == false`: the LEFT (white) side is blank; the right side shows the lap count.
2. **Restart the refbox in Hockey6V6 or Hockey3V3 mode (Settings → Mode → Hockey, then restart).** Both score columns render on the panel sim — no regression in game mode.
3. **`just check` already passed** (from Task 1 Step 7) — no further command required here.

Report any scenario that fails.

- [ ] **Step 4: Hand back to operator**

Report walkthrough results. Do NOT push — Branch 2 is held for stacked PR with Branch 3 per the project memory.

---

## Deviations

(Append notes here if execution deviates from the plan. Per `.claude/rules/plan-execution.md`, fold deviation notes into the code commit that introduced the deviation; no standalone doc-only deviation commits.)

---

## Self-review notes

- **Spec coverage:** Spec §Design `UpdateSender::new()` signature → Task 1 Step 3a. Spec §Design `Server` field → Step 3b/3c. Spec §Design `serial_worker_loop` signature → Step 3d/3e/3f. Spec §Design `Server::encode_flash` → Step 3g. Spec §Design `mod.rs` call site → Step 4. Spec §Unit test → Steps 1, 3h, 6. Spec §Acceptance criteria → Task 2.
- **No type-system surprises:** `bool` is `Copy`, so `self.beep_test` can be passed by value into the spawn closure without borrow gymnastics. The new field on `Server` is plain data, no `Debug`/`Clone` concerns since the struct already derives `Debug` via existing fields' impls.
- **Compile-order constraint:** Signature changes to `UpdateSender::new` ripple to its only caller (`mod.rs:1222`). Both must land in one commit or the workspace won't build — Task 1 batches them accordingly.
- **No placeholders:** all steps show concrete code/commands.
- **Heavy process:** new unit test added; per-task verification via `just check`; final walkthrough.
- **Test isolation:** new test uses distinct ports (4_700, 4_701) from the existing integration test's `BINARY_PORT`/`JSON_PORT` to avoid bind conflicts when both tests run concurrently. The existing test's port constants live in the same `mod tests` scope — check them before merging to confirm no collision.
- **Test imports:** the new test uses `TcpStream`, `BlackWhiteBundle`, `GamePeriod`, `GameSnapshot`, `OptColorBundle`, `PenaltySnapshot`, `PenaltyTime`, `Infraction`, `InfractionSnapshot`, `Brightness`, `TransmittedData`, `assert_le!`, `ErrorKind`. All are already used by the existing big integration test in the same `mod tests` scope, so the imports are in place.
