# Open New Display Button — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add an "Open New Display" button to the Display Options tab that spawns an additional panel-simulator window. Multiple windows can be opened; all are tracked and closed when the refbox quits or restarts.

**Architecture:** Extract the existing startup simulator-spawn logic in `main.rs` into a reusable helper backed by a `SimSpawnConfig` struct. Replace the single-slot `sim_child: Option<Child>` on `RefBoxApp` with a `sim_children: Vec<Child>`. Add a `Message::OpenNewDisplay` variant; its handler calls the helper and pushes the new child onto the vec. Add the button using the existing `make_button` + `light_gray_button` pattern, and add the translation key to all 15 supported locales.

**Tech Stack:** Rust 2024, `iced` 0.13, `tokio::process::Child`, `i18n-embed` for translations.

**Spec:** [docs/superpowers/specs/2026-05-18-open-new-display-button-design.md](../specs/2026-05-18-open-new-display-button-design.md)

**Process:** Lean (per `.claude/rules/plan-execution.md` — refbox UI feature, single-crate, no `uwh-common`/wire-format/state-machine impact). Feature-level code review at the end; per-task verification is compile + `just check` only.

---

## Task 1: Extract `SimSpawnConfig` + `spawn_sim_child` helper

Pure refactor. No behavior change. The existing startup spawn block is replaced by a call to the new helper; the resulting argv is identical.

**Files:**
- Modify: [refbox/src/main.rs](../../../refbox/src/main.rs) (the spawn block at lines 290–337)

- [ ] **Step 1: Add the `SimSpawnConfig` struct + `spawn_sim_child` helper near the top of `main.rs`**

Insert immediately above `fn main()` (around line 165):

```rust
/// All arguments needed to launch a panel-simulator child process. Built once
/// in `main()` from the parsed CLI, then reused for every sim window we spawn
/// (the startup one, and any opened later via the Display Options button).
#[derive(Debug, Clone)]
pub struct SimSpawnConfig {
    pub binary_port: u16,
    pub json_port: u16,
    pub scale: f32,
    pub spacing: f32,
    pub sunlight_mode: bool,
    pub verbose: u8,
    pub log_location: PathBuf,
    pub log_max_file_size: u64,
    pub num_old_logs: u32,
}

/// Build the argv that `spawn_sim_child` passes to the spawned process.
/// Factored out as a pure function so its construction can be unit-tested
/// without spawning.
pub fn build_sim_argv(config: &SimSpawnConfig) -> Vec<String> {
    let mut args = vec![
        "--is-simulator".to_string(),
        "--binary-port".to_string(),
        config.binary_port.to_string(),
        "--json-port".to_string(),
        config.json_port.to_string(),
        "--scale".to_string(),
        config.scale.to_string(),
        "--spacing".to_string(),
        config.spacing.to_string(),
        "--log-location".to_string(),
        // Matches the original main.rs behaviour. A non-UTF-8 log path would
        // already have panicked at startup before we got here.
        config.log_location.to_str().unwrap().to_string(),
        "--log-max-file-size".to_string(),
        config.log_max_file_size.to_string(),
        "--num-old-logs".to_string(),
        config.num_old_logs.to_string(),
    ];
    for _ in 0..config.verbose {
        args.push("--verbose".to_string());
    }
    if config.sunlight_mode {
        args.push("--simulate-sunlight-display".to_string());
    }
    args
}

pub(crate) fn spawn_sim_child(config: &SimSpawnConfig) -> std::io::Result<Child> {
    let bin_name = std::env::current_exe()?.into_os_string();
    let argv = build_sim_argv(config);
    info!("Spawning sim child, bin_name: {bin_name:?}, args: {argv:?}");
    Command::new(bin_name)
        .args(&argv)
        .stdin(Stdio::null())
        .spawn()
}
```

- [ ] **Step 2: Replace the existing inline spawn block with a call to the helper**

In `main()`, replace lines 290–337 (the `let child = if args.no_simulate { ... }` block) with:

```rust
    let sim_spawn_config = SimSpawnConfig {
        binary_port: args.binary_port,
        json_port: args.json_port,
        scale: args.scale,
        spacing,
        sunlight_mode: args.simulate_sunlight_display,
        verbose: args.verbose,
        log_location: log_base_path.clone(),
        log_max_file_size: args.log_max_file_size,
        num_old_logs: args.num_old_logs,
    };

    let startup_sim_children: Vec<Child> = if args.no_simulate {
        Vec::new()
    } else {
        info!("Starting child with binary port {}", args.binary_port);
        match spawn_sim_child(&sim_spawn_config) {
            Ok(child) => vec![child],
            Err(e) => {
                error!("Failed to spawn startup simulator: {e:?}");
                Vec::new()
            }
        }
    };
```

(The previous code propagated the spawn error with `?`. We now log and continue — startup with `no_simulate` already worked without a sim, so a runtime spawn failure is non-fatal. The operator can press the new button later to retry.)

- [ ] **Step 3: Verify imports**

The helper uses `Child`, `Command`, `Stdio`, `PathBuf`. Confirm all are already imported. The existing spawn block uses `Command`, `Stdio`, `Child` from the same scope; `PathBuf` is used elsewhere in main. Add any missing imports.

- [ ] **Step 4: Add a unit test asserting the argv is built correctly**

Append to `refbox/src/main.rs`:

```rust
#[cfg(test)]
mod sim_spawn_tests {
    use super::*;
    use std::path::PathBuf;

    fn make_test_config(verbose: u8, sunlight: bool) -> SimSpawnConfig {
        SimSpawnConfig {
            binary_port: 8001,
            json_port: 8000,
            scale: 4.0,
            spacing: 1.0,
            sunlight_mode: sunlight,
            verbose,
            log_location: PathBuf::from("/tmp/logs"),
            log_max_file_size: 5_000_000,
            num_old_logs: 3,
        }
    }

    #[test]
    fn argv_includes_required_flags() {
        let config = make_test_config(0, false);
        let argv = build_sim_argv(&config);
        assert!(argv.contains(&"--is-simulator".to_string()));
        assert!(argv.contains(&"--binary-port".to_string()));
        assert!(argv.contains(&"8001".to_string()));
    }

    #[test]
    fn argv_repeats_verbose_per_count() {
        let config = make_test_config(3, false);
        let argv = build_sim_argv(&config);
        assert_eq!(argv.iter().filter(|a| a.as_str() == "--verbose").count(), 3);
    }

    #[test]
    fn argv_includes_sunlight_flag_only_when_enabled() {
        let off = build_sim_argv(&make_test_config(0, false));
        let on = build_sim_argv(&make_test_config(0, true));
        assert!(!off.contains(&"--simulate-sunlight-display".to_string()));
        assert!(on.contains(&"--simulate-sunlight-display".to_string()));
    }
}
```

The test exercises `build_sim_argv` directly (a pure function, no process spawn). `spawn_sim_child` itself is not directly unit-tested because it calls `Command::spawn()`, which is exercised by the manual tests in Task 5.

- [ ] **Step 5: Run `just check`**

```bash
just check
```

Expected: PASS (format, lint, tests, audit all green).

- [ ] **Step 6: Commit**

```bash
git add refbox/src/main.rs
git commit -m "refactor(refbox): extract SimSpawnConfig and spawn_sim_child helper"
```

---

## Task 2: Replace `sim_child: Option<Child>` with `sim_children: Vec<Child>`

Pure refactor. The startup-spawned sim (when present) is just the first entry in the vec. All cleanup sites are updated to iterate while preserving the existing kill-vs-wait distinction.

**Files:**
- Modify: [refbox/src/main.rs](../../../refbox/src/main.rs) (one site: the `RefBoxAppFlags` initializer)
- Modify: [refbox/src/app/mod.rs](../../../refbox/src/app/mod.rs) (multiple sites)

- [ ] **Step 1: Update `RefBoxAppFlags` in `app/mod.rs`**

At [refbox/src/app/mod.rs:148](../../../refbox/src/app/mod.rs#L148), change:

```rust
pub sim_child: Option<Child>,
```

to:

```rust
pub sim_children: Vec<Child>,
pub sim_spawn_config: crate::SimSpawnConfig,
```

- [ ] **Step 2: Update `RefBoxApp` struct field in `app/mod.rs`**

At [refbox/src/app/mod.rs:105](../../../refbox/src/app/mod.rs#L105), change:

```rust
sim_child: Option<Child>,
```

to:

```rust
sim_children: Vec<Child>,
sim_spawn_config: crate::SimSpawnConfig,
```

- [ ] **Step 3: Update `RefBoxApp::new` to take the new fields**

In the `let RefBoxAppFlags { ... } = flags;` destructure (around line 1062–1069), add `sim_children` and `sim_spawn_config` to the destructure. In the struct literal at the bottom of `new` (around line 1209), pass `sim_children` and `sim_spawn_config` through.

- [ ] **Step 4: Update the three cleanup sites to iterate**

Site A — [refbox/src/app/mod.rs:958](../../../refbox/src/app/mod.rs#L958) (RestartAndApply for Mode change). Replace:

```rust
if let Some(mut child) = self.sim_child.take() {
    let _ = child.kill();
}
```

with:

```rust
for mut child in self.sim_children.drain(..) {
    let _ = child.kill();
}
```

Site B — [refbox/src/app/mod.rs:2538](../../../refbox/src/app/mod.rs#L2538) (Language restart). Same replacement as Site A.

Site C — [refbox/src/app/mod.rs:1053](../../../refbox/src/app/mod.rs#L1053) (`Drop` for `RefBoxApp`, normal shutdown). Replace:

```rust
impl Drop for RefBoxApp {
    fn drop(&mut self) {
        if let Some(mut child) = self.sim_child.take() {
            info!("Waiting for child");
            child.wait().unwrap();
        }
    }
}
```

with:

```rust
impl Drop for RefBoxApp {
    fn drop(&mut self) {
        for mut child in self.sim_children.drain(..) {
            info!("Waiting for sim child");
            child.wait().unwrap();
        }
    }
}
```

- [ ] **Step 5: Update `main.rs` to pass new fields into `RefBoxAppFlags`**

In `main()` where `RefBoxAppFlags` is constructed (around line 418), change:

```rust
sim_child: child,
```

to:

```rust
sim_children: startup_sim_children,
sim_spawn_config: sim_spawn_config.clone(),
```

(The `.clone()` keeps the config available; the original is consumed when we no longer need it. Alternatively, move it directly — choose whichever borrow-checks cleanly during implementation.)

- [ ] **Step 6: Run `just check`**

```bash
just check
```

Expected: PASS. The refactor changes no behavior — startup spawn still produces exactly one sim window, cleanup paths behave the same.

- [ ] **Step 7: Commit**

```bash
git add refbox/src/main.rs refbox/src/app/mod.rs
git commit -m "refactor(refbox): track simulator children in a vec"
```

---

## Task 3: Add `open-new-display` translation key in all 15 locales

The `fl!` macro is checked at compile time against every locale's fluent file. The key must exist in every locale before any code referencing it can compile.

**Files:** All 15 of:
- `refbox/translations/de-DE/refbox.ftl`
- `refbox/translations/en-US/refbox.ftl`
- `refbox/translations/es/refbox.ftl`
- `refbox/translations/fr/refbox.ftl`
- `refbox/translations/id-ID/refbox.ftl`
- `refbox/translations/it-IT/refbox.ftl`
- `refbox/translations/ja-JP/refbox.ftl`
- `refbox/translations/ko-KR/refbox.ftl`
- `refbox/translations/ms-MY/refbox.ftl`
- `refbox/translations/nl-NL/refbox.ftl`
- `refbox/translations/pt-PT/refbox.ftl`
- `refbox/translations/th-TH/refbox.ftl`
- `refbox/translations/tl-PH/refbox.ftl`
- `refbox/translations/tr-TR/refbox.ftl`
- `refbox/translations/zh-CN/refbox.ftl`

- [ ] **Step 1: Add the key to every locale**

Append to each file's section that contains the existing `display-options` / `sound-options` keys (or near the bottom of the file, matching that file's organisation). The line for each locale is exactly:

| Locale | Line to add |
|---|---|
| `de-DE` | `open-new-display = NEUE ANZEIGE ÖFFNEN` |
| `en-US` | `open-new-display = OPEN NEW DISPLAY` |
| `es` | `open-new-display = ABRIR NUEVA PANTALLA` |
| `fr` | `open-new-display = OUVRIR NOUVEL AFFICHAGE` |
| `id-ID` | `open-new-display = BUKA TAMPILAN BARU` |
| `it-IT` | `open-new-display = APRI NUOVO DISPLAY` |
| `ja-JP` | `open-new-display = 新しい表示を開く` |
| `ko-KR` | `open-new-display = 새 화면 열기` |
| `ms-MY` | `open-new-display = BUKA PAPARAN BARU` |
| `nl-NL` | `open-new-display = NIEUWE WEERGAVE OPENEN` |
| `pt-PT` | `open-new-display = ABRIR NOVO ECRÃ` |
| `th-TH` | `open-new-display = เปิดการแสดงผลใหม่` |
| `tl-PH` | `open-new-display = BUKSAN ANG BAGONG DISPLAY` |
| `tr-TR` | `open-new-display = YENİ GÖRÜNTÜ AÇ` |
| `zh-CN` | `open-new-display = 打开新显示` |

Each translation reuses the same word for "display" that the locale already uses in `display-options`, for consistency. The CJK locales (`ja-JP`, `ko-KR`, `zh-CN`) and `th-TH` do not use ALL CAPS — that's deliberate; the existing sibling keys in those files also don't (e.g. `display-options = 显示选项`).

- [ ] **Step 2: Run `just check`**

```bash
just check
```

Expected: PASS. The key isn't referenced yet — this step only verifies the fluent files still parse.

- [ ] **Step 3: Commit**

```bash
git add refbox/translations/
git commit -m "feat(refbox): add open-new-display translation key in all locales"
```

**Note for reviewer:** A native speaker should verify the non-English translations during PR review. Adjustments are simple key edits and need no code change.

---

## Task 4: Add `Message::OpenNewDisplay` + handler + button

The user-visible change. Adds the message variant, wires its handler to spawn a new sim and push onto `sim_children`, and adds the button to the Display Options page.

**Files:**
- Modify: [refbox/src/app/message.rs](../../../refbox/src/app/message.rs)
- Modify: [refbox/src/app/mod.rs](../../../refbox/src/app/mod.rs)
- Modify: [refbox/src/app/view_builders/configuration.rs](../../../refbox/src/app/view_builders/configuration.rs)

- [ ] **Step 1: Add the message variant**

Open [refbox/src/app/message.rs](../../../refbox/src/app/message.rs). Add a new variant to the `Message` enum, placed alphabetically near the other `Open*` variants (after `OpenPortalDetailPage`):

```rust
OpenNewDisplay,
```

- [ ] **Step 2: Add the handler in `update()`**

In [refbox/src/app/mod.rs](../../../refbox/src/app/mod.rs), find the `update()` function and add a new arm in the `match message { ... }`. Place it near the other `Message::Open*` arms (after `Message::OpenPortalDetailPage` around line 1930):

```rust
Message::OpenNewDisplay => {
    match crate::spawn_sim_child(&self.sim_spawn_config) {
        Ok(child) => {
            info!("Opened new sim window; total now {}", self.sim_children.len() + 1);
            self.sim_children.push(child);
        }
        Err(e) => {
            error!("Failed to spawn new sim window: {e:?}");
        }
    }
    Task::none()
}
```

- [ ] **Step 3: Add the button to the Display Options page**

In [refbox/src/app/view_builders/configuration.rs](../../../refbox/src/app/view_builders/configuration.rs), find `make_display_config_page` (line 875). Locate the two empty `row![horizontal_space()].height(Length::Fill)` rows at lines 950–951. Replace the **first** of them with:

```rust
        row![
            make_button(fl!("open-new-display"))
                .style(light_gray_button)
                .on_press(Message::OpenNewDisplay),
        ]
        .spacing(SPACING)
        .height(Length::Fill),
```

Leave the second empty row in place to preserve the existing vertical layout proportions.

The resulting `column!` body (lines 924–953) will read:

```rust
column![
    make_game_time_button(
        snapshot,
        false,
        false,
        mode,
        clock_running,
        portal_indicator
    ),
    row![sides_btn].spacing(SPACING).height(Length::Fill),
    row![
        make_value_button(
            fl!("hide-time-for-last-15-seconds"),
            bool_string(*hide_time),
            (false, true),
            Some(Message::ToggleBoolParameter(BoolGameParameter::HideTime))
        ),
        make_value_button(
            fl!("player-display-brightness"),
            fl!("brightness", brightness = brightness.to_string()),
            (false, true),
            Some(Message::CycleParameter(CyclingParameter::Brightness))
        )
    ]
    .spacing(SPACING)
    .height(Length::Fill),
    row![
        make_button(fl!("open-new-display"))
            .style(light_gray_button)
            .on_press(Message::OpenNewDisplay),
    ]
    .spacing(SPACING)
    .height(Length::Fill),
    row![horizontal_space()].height(Length::Fill),
    make_cancel_apply_footer(ConfigPage::Display, settings, page_entry_snapshot),
]
```

Verify that `make_button` is in scope (it's defined in `shared_elements.rs` and the rest of `configuration.rs` already uses it; no import change should be needed).

- [ ] **Step 4: Run `just check`**

```bash
just check
```

Expected: PASS. The button now compiles, references `Message::OpenNewDisplay`, and references `fl!("open-new-display")` (which exists in every locale from Task 3).

- [ ] **Step 5: Commit**

```bash
git add refbox/src/app/message.rs refbox/src/app/mod.rs refbox/src/app/view_builders/configuration.rs
git commit -m "feat(refbox): add \"open new display\" button to Display Options"
```

---

## Task 5: Manual verification on the running app

The non-programmer operator drives the UI for these checks. Per memory rule, refbox is launched as a background process with `dangerouslyDisableSandbox:true` and `WAYLAND_DISPLAY=` unset (for WSL X11 fallback). The user reports observations back.

- [ ] **Step 1: Launch refbox**

Run:

```bash
WAYLAND_DISPLAY= cargo run -p refbox
```

(in the background, with `dangerouslyDisableSandbox: true`.)

- [ ] **Step 2: Operator-driven verification (per spec § Manual tests)**

The operator confirms each of:

1. **Golden path.** Open Display Options → press "Open New Display" → second sim window appears and mirrors the first.
2. **Multiple windows.** Press the button two more times → four windows total, all mirroring the same content.
3. **Close-one independence.** Close one sim window via its OS close button → refbox keeps running → other windows continue updating.
4. **Reopen after close.** Close all sim windows manually → press button → new sim window appears.
5. **Cleanup on quit.** With multiple windows open, quit the refbox → all sim windows close.
6. **Cleanup on in-app restart.** Change App Mode (e.g. Hockey ↔ Rugby) → press Restart → all sim windows close, refbox restarts with a fresh single sim.

If any test fails, return to the failing task, fix, re-run `just check`, and re-verify.

- [ ] **Step 3: (No commit at this step — verification only.)**

---

## Task 6: Final code review and PR readiness

- [ ] **Step 1: Run `just check` one final time**

```bash
just check
```

- [ ] **Step 2: Invoke `superpowers:requesting-code-review`**

Per lean process, code review runs once at the end of the feature, not per task.

- [ ] **Step 3: Address review feedback inline, then prepare PR**

Once review is clean, the operator is asked for PR approval per the project's branch/PR rules. Do not push, open, or merge a PR without explicit operator approval.

---

## Self-Review Notes

- **Spec coverage.** Every numbered item in the spec's "Operator-Visible Behavior" (1–7) and "Architectural Sketch" (1–6) maps to a task: state change → Task 2; SimSpawnConfig → Task 1; spawn helper → Task 1; field rename → Task 2; new Message variant → Task 4; new button → Task 4; new translation key → Task 3. Manual tests → Task 5.
- **Type consistency.** Field name is `sim_children` (not `sim_child`/`sim_kids`/etc.) at every reference. Helper is `spawn_sim_child` (singular noun, since each call spawns one). Struct is `SimSpawnConfig`. Message variant is `Message::OpenNewDisplay`.
- **No placeholders.** Every step has actual code or an exact command. Translations are concrete strings, not "TBD".
- **Order rationale.** Task 1 (extract helper) and Task 2 (field rename) are pure refactors that compile and behave identically to today — safe to commit independently. Task 3 (translation key) must precede Task 4 because `fl!` is compile-time-checked. Task 4 is where the user-visible change appears. Task 5 is verification. Task 6 is the review/PR gate.
