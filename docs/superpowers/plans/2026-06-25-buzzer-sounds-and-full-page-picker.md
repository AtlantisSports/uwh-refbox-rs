# Buzzer Sounds + Full-Page Picker Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add 7 new buzzer sounds and replace the buzzer carousel with a full-page picker (matching the Languages page), reachable from both the main Sound settings page and the beep test settings — with a Cancel | Test | Apply footer.

**Architecture:** New `BuzzerSound` variants + embedded `.raw` clips in the sound engine. A new `ConfigPage::Buzzer` sub-page reuses the existing `ApplyConfigPage`/`CancelConfigPage` + `PageEntrySnapshot` machinery (mirroring `make_language_select_page`). A parallel `BeepTestConfigPage::Buzzer` mirrors the beep test's existing Language-picker copy. A new sound-controller `test_buzzer(sound)` plays an arbitrary sound for the Test button.

**Tech Stack:** Rust 2024, iced 0.13, `web-audio-api`, fluent (`fl!`) translations. `refbox` crate only.

## Global Constraints

- MSRV Rust 1.85; edition 2024. Do not use newer APIs.
- Clippy clean: `cargo clippy -p refbox -- -D warnings` (mirrors CI/`just lint`; do NOT use `--all-targets` locally — it surfaces ~90 pre-existing test-only lints that are not failures).
- `refbox` is bin-only: test with `cargo test -p refbox` (no `--lib`).
- No new dependencies. No new `unwrap()`/`expect()` in production code without a justifying comment.
- Sound assets: mono, 32-bit float, little-endian, 44,100 Hz `.raw` files in `refbox/resources/sounds/` (same format as `buzz.raw`).
- New sound NAMES are English-only (via `Display`, like existing sounds) — NOT localized. New UI button label `test` ("TEST") MUST be translated in all 15 locales (no English placeholders).
- Final sound set is 12, order: Buzz, Whoop, Crazy, DeDeDu, TwoTone (existing), then Airhorn, Pipes, Klaxon, Pip, Pulse, Siren, Trill (new).
- Rebuild the real binary before any walkthrough: `cargo build -p refbox` (clippy/test build a different binary).
- Approval required before any branch/commit/push (the human is a non-programmer). Per project convention, the design spec & this plan are local working docs — do NOT add them to the feature branch/PR.
- Reference spec: `docs/superpowers/specs/2026-06-25-buzzer-sounds-and-full-page-picker-design.md`.

---

## Task 1: Sound assets + reproducible regen script

**Files:**
- Create: `refbox/resources/sounds/regen-buzzer-sounds.py`
- Create (generated): `refbox/resources/sounds/{airhorn,pipes,klaxon,pip,pulse,siren,trill}.raw`

**Interfaces:**
- Produces: 7 single-cycle loop-element `.raw` files (mono f32 LE @44,100 Hz) and a script that regenerates them deterministically (the font `regen-cjk-font.py` is the precedent for a committed regen script).

- [ ] **Step 1: Write the regen script**

```python
#!/usr/bin/env python3
"""Regenerate the 7 synthesized buzzer loop-element .raw files.

Each file is a single-cycle loop element (mono, 32-bit float LE, 44,100 Hz) —
the same format/role as buzz.raw etc. The app loops the element to fill the
auto-buzzer window and the held alarm. Elements are designed to loop with an
even rhythm / continuous phase so the repeat seam is imperceptible, and to land
near the ~2.15s auto window (3 cycles for most). See the design spec.
"""
import numpy as np, os
SR = 44100
OUT = os.path.dirname(os.path.abspath(__file__))

def wave_at(freq, dur, kind="square"):
    n = int(round(dur * SR)); t = np.arange(n) / SR; ph = 2 * np.pi * freq * t
    if kind == "sine":   return np.sin(ph)
    if kind == "square": return np.sign(np.sin(ph))
    return 2 * (t * freq - np.floor(0.5 + t * freq))  # saw

def glide(farr, kind="sine"):
    ph = 2 * np.pi * np.concatenate(([0.0], np.cumsum(farr)[:-1])) / SR
    nxt = ph[-1] + 2 * np.pi * farr[-1] / SR
    k = max(1, round(nxt / (2 * np.pi)))
    ph = ph * (2 * np.pi * k / nxt)
    if kind == "saw": return 2 * ((ph / (2 * np.pi)) % 1.0) - 1.0
    return np.sin(ph)

def edge(x, ms=3):
    k = int(SR * ms / 1000)
    if k < 1 or k * 2 >= len(x): return x
    r = 0.5 * (1 - np.cos(np.linspace(0, np.pi, k))); x = x.copy()
    x[:k] *= r; x[-k:] *= r[::-1]; return x

def sil(dur): return np.zeros(int(round(dur * SR)))
def cat(*p): return np.concatenate(p)
def norm(x):
    p = np.max(np.abs(x)); return x / p * 0.95 if p > 0 else x

def e_airhorn():
    def honk(d):
        return edge(norm(wave_at(215, d, "saw") + wave_at(286, d, "saw")
                         + 0.4 * wave_at(107, d, "saw") + 0.5 * wave_at(218, d, "saw")), 10)
    return cat(honk(0.50), sil(0.20))                                    # 0.70s

def e_pipes():
    def clang(f0, d):
        n = int(round(d * SR)); t = np.arange(n) / SR
        ratios = [1.0, 2.76, 5.40, 8.93, 11.34]; amps = [1.0, 0.6, 0.35, 0.2, 0.12]
        x = sum(a * np.sin(2 * np.pi * f0 * r * t) for r, a in zip(ratios, amps))
        env = np.exp(-t / (d * 0.32)); k = int(0.0015 * SR); env[:k] *= np.linspace(0, 1, k)
        x = x * env; kf = int(0.003 * SR); x[-kf:] *= np.linspace(1, 0, kf); return x
    return norm(clang(470, 0.215))                                       # 0.215s

def e_klaxon():
    n = int(round(0.58 * SR)); h = n // 2
    farr = np.concatenate([np.linspace(300, 520, h), np.linspace(520, 300, n - h)])
    return cat(edge(glide(farr, "saw"), 8), sil(0.12))                   # 0.70s

def e_pip():
    return cat(edge(wave_at(1700, 0.07, "square"), 2), sil(0.07))        # 0.14s

def e_pulse():
    return cat(edge(wave_at(330, 0.42, "square"), 4), sil(0.28))         # 0.70s

def e_siren():
    n = int(round(0.70 * SR)); h = n // 2
    farr = np.concatenate([np.linspace(500, 1500, h), np.linspace(1500, 500, n - h)])
    return glide(farr, "sine")                                           # 0.70s

def e_trill():
    per = int(round(0.0625 * SR))
    farr = np.concatenate([np.full(per, f) for f in [1000, 1300] * 4])
    return glide(farr, "sine")                                           # 0.50s

ELEMENTS = {"airhorn": e_airhorn, "pipes": e_pipes, "klaxon": e_klaxon,
            "pip": e_pip, "pulse": e_pulse, "siren": e_siren, "trill": e_trill}
for name, fn in ELEMENTS.items():
    norm(fn()).astype("<f4").tofile(os.path.join(OUT, name + ".raw"))
    print("wrote", name + ".raw")
```

- [ ] **Step 2: Run it to generate the files**

Run: `python3 refbox/resources/sounds/regen-buzzer-sounds.py`
Expected: prints `wrote airhorn.raw` … `wrote trill.raw` (7 lines).

- [ ] **Step 3: Verify the files are valid f32 elements with the expected lengths**

Run:
```bash
python3 -c "
import numpy as np, os
d='refbox/resources/sounds'
exp={'airhorn':30870,'pipes':9481,'klaxon':30870,'pip':6174,'pulse':30870,'siren':30870,'trill':22050}
for n,e in exp.items():
    a=np.fromfile(f'{d}/{n}.raw',dtype='<f4'); assert len(a)==e,(n,len(a),e); assert abs(a).max()<=0.95+1e-6
print('ok')"
```
Expected: `ok` (lengths within ±1 sample are fine; adjust the assert to `abs(len-e)<=2` if rounding differs on the platform).

- [ ] **Step 4: Commit**

```bash
git add refbox/resources/sounds/regen-buzzer-sounds.py refbox/resources/sounds/*.raw
git commit -m "feat(refbox): add 7 synthesized buzzer sound clips + regen script"
```

---

## Task 2: `BuzzerSound` enum — variants, Display, ALL list, Ord

**Files:**
- Modify: `refbox/src/sound_controller/sounds.rs:51-74` (enum + Display)
- Test: `refbox/src/sound_controller/sounds.rs` (new `#[cfg(test)]`)

**Interfaces:**
- Produces: `BuzzerSound::{Airhorn,Pipes,Klaxon,Pip,Pulse,Siren,Trill}`; `BuzzerSound::ALL: [BuzzerSound; 12]`; `BuzzerSound` now derives `PartialOrd, Ord` (needed by `SoundId` in Task 4).

- [ ] **Step 1: Write the failing test**

Add to a `#[cfg(test)] mod tests` in `sounds.rs`:
```rust
#[test]
fn all_buzzer_sounds_round_trip_via_serde() {
    for s in BuzzerSound::ALL {
        let toml = toml::to_string(&Wrap { s }).unwrap();
        let back: Wrap = toml::from_str(&toml).unwrap();
        assert_eq!(back.s, s, "round-trip failed for {s:?}");
    }
    assert_eq!(BuzzerSound::ALL.len(), 12);
}
#[derive(serde::Serialize, serde::Deserialize, PartialEq, Eq, Debug)]
struct Wrap { s: BuzzerSound }
```

- [ ] **Step 2: Run it — expect FAIL (variants/ALL missing → does not compile)**

Run: `cargo test -p refbox sounds:: 2>&1 | tail -20`
Expected: compile error — `no variant ... Airhorn`, `no associated item ALL`.

- [ ] **Step 3: Add variants, derives, Display, and ALL**

In the `macro_attr! { ... enum BuzzerSound { ... } }` block add `PartialOrd, Ord` to the derive list and the 7 variants after `TwoTone`:
```rust
        TwoTone,
        Airhorn,
        Pipes,
        Klaxon,
        Pip,
        Pulse,
        Siren,
        Trill,
```
Extend `impl Display for BuzzerSound` with:
```rust
            Self::Airhorn => write!(f, "Airhorn"),
            Self::Pipes => write!(f, "Pipes"),
            Self::Klaxon => write!(f, "Klaxon"),
            Self::Pip => write!(f, "Pip"),
            Self::Pulse => write!(f, "Pulse"),
            Self::Siren => write!(f, "Siren"),
            Self::Trill => write!(f, "Trill"),
```
Add (right after the `impl Display` block):
```rust
impl BuzzerSound {
    /// All buzzer sounds, in picker display order (existing first, new last).
    pub const ALL: [BuzzerSound; 12] = [
        BuzzerSound::Buzz, BuzzerSound::Whoop, BuzzerSound::Crazy,
        BuzzerSound::DeDeDu, BuzzerSound::TwoTone, BuzzerSound::Airhorn,
        BuzzerSound::Pipes, BuzzerSound::Klaxon, BuzzerSound::Pip,
        BuzzerSound::Pulse, BuzzerSound::Siren, BuzzerSound::Trill,
    ];
}
```

- [ ] **Step 4: Run the test — expect PASS**

Run: `cargo test -p refbox sounds:: 2>&1 | tail -20`
Expected: PASS. (Note: `SoundLibrary`'s `Index` match is now non-exhaustive — Task 3 fixes it; if compiling the whole crate now, expect that single known error. Run the targeted test which compiles the module under test.)

- [ ] **Step 5: Commit**

```bash
git add refbox/src/sound_controller/sounds.rs
git commit -m "feat(refbox): add 7 BuzzerSound variants, ALL list, Ord derive"
```

---

## Task 3: Embed the clips + wire `SoundLibrary`

**Files:**
- Modify: `refbox/src/sound_controller/sounds.rs` (embeds ~28-47; `SoundLibrary` struct ~76-83; `Index` ~85-97; `new()` ~99-127)

**Interfaces:**
- Consumes: `BuzzerSound` variants (Task 2), `.raw` files (Task 1).
- Produces: `SoundLibrary` returns an `AudioBuffer` for every `BuzzerSound` (exhaustive `Index` match — compiler-enforced coverage).

- [ ] **Step 1: Add the embedded arrays**

After the `TWO_TONE` block (~line 47) add, for each new sound (pattern shown for one; repeat for pipes, klaxon, pip, pulse, siren, trill):
```rust
const AIRHORN_LEN: usize = include_bytes!("../../resources/sounds/airhorn.raw").len() / 4;
static AIRHORN: [f32; AIRHORN_LEN] =
    process_array(include_bytes!("../../resources/sounds/airhorn.raw"));
```

- [ ] **Step 2: Extend `SoundLibrary` struct + `Index` + `new()`**

Add 7 fields (`airhorn: AudioBuffer`, …) to `struct SoundLibrary`. Add 7 arms to `impl Index<BuzzerSound>`:
```rust
            BuzzerSound::Airhorn => &self.airhorn,
            BuzzerSound::Pipes => &self.pipes,
            BuzzerSound::Klaxon => &self.klaxon,
            BuzzerSound::Pip => &self.pip,
            BuzzerSound::Pulse => &self.pulse,
            BuzzerSound::Siren => &self.siren,
            BuzzerSound::Trill => &self.trill,
```
In `new()` add, per sound (pattern shown for one):
```rust
        let mut airhorn = context.create_buffer(1, AIRHORN_LEN, SAMPLE_RATE);
        airhorn.copy_to_channel(&AIRHORN, 0);
```
and add each to the returned struct literal.

- [ ] **Step 3: Build to verify exhaustiveness/compile**

Run: `cargo build -p refbox 2>&1 | tail -20`
Expected: builds (the `Index` match and struct are now complete).

- [ ] **Step 4: Commit**

```bash
git add refbox/src/sound_controller/sounds.rs
git commit -m "feat(refbox): embed 7 new buzzer clips in SoundLibrary"
```

---

## Task 4: `test_buzzer()` playback API for the Test button

**Files:**
- Modify: `refbox/src/sound_controller/mod.rs` (`enum SoundId` ~222-231; the `msg_rx` match ~341-412; `start_sound` ~444-528; public methods ~599-619; `whole_cycles` test ~995-1003)

**Interfaces:**
- Produces: `SoundController::test_buzzer(&self, sound: BuzzerSound)` — plays `sound` once as the timed ~2.15s auto-buzzer through the live audio path. Used by the picker Test button.

- [ ] **Step 1: Add a whole-cycles test for the new periods (guards the ~2.15s fit)**

Add to the existing `whole_cycles_rounds_to_nearest_whole_cycle` test (or a new test) in `mod.rs`:
```rust
    assert_eq!(whole_cycles_for(0.14, 2.15), 15); // Pip
    assert_eq!(whole_cycles_for(0.215, 2.15), 10); // Pipes
    assert_eq!(whole_cycles_for(0.70, 2.15), 3);  // Airhorn/Klaxon/Pulse/Siren
    assert_eq!(whole_cycles_for(0.50, 2.15), 4);  // Trill
```
Run: `cargo test -p refbox whole_cycles 2>&1 | tail` → expect PASS (function already exists; this just documents the new periods).

- [ ] **Step 2: Add the `SoundId` variant and `SoundMessage`**

In `enum SoundId` add `TestBuzzer(BuzzerSound)` (the enum derives `Ord`; Task 2 made `BuzzerSound: Ord`, so this compiles). Find the `enum SoundMessage` definition (in this file or `button_handler`) and add `TestBuzzer(BuzzerSound)`.

- [ ] **Step 3: Handle the message + build the sound**

In the `msg_rx.recv()` match, add:
```rust
                                    SoundMessage::TestBuzzer(sound) => {
                                        let id = SoundId::TestBuzzer(sound);
                                        if !sound_queue.contains(&id) {
                                            sound_queue.push_back(id);
                                        }
                                    }
```
In `start_sound`, add a `SoundId::TestBuzzer(sound)` arm mirroring `SoundId::AutoBuzzer` but using the carried `sound` and NOT flashing:
```rust
                        SoundId::TestBuzzer(sound) => {
                            info!("Testing buzzer sound {sound:?}");
                            let volumes = ChannelVolumes::new(&settings, false);
                            Sound::new(context.clone(), volumes, library[sound].clone(), true, true)
                        }
```

- [ ] **Step 4: Add the public method**

Near `trigger_buzzer` (~611):
```rust
    pub fn test_buzzer(&self, sound: BuzzerSound) {
        self.msg_tx.send(SoundMessage::TestBuzzer(sound)).unwrap()
    }
```

- [ ] **Step 5: Build + test**

Run: `cargo build -p refbox 2>&1 | tail -5 && cargo test -p refbox sound 2>&1 | tail -5`
Expected: builds; tests pass.

- [ ] **Step 6: Commit**

```bash
git add refbox/src/sound_controller/mod.rs
git commit -m "feat(refbox): add SoundController::test_buzzer for picker preview"
```

---

## Task 5: Main full-page buzzer picker (end-to-end, one compilable unit)

Adding `ConfigPage::Buzzer` makes every exhaustive `ConfigPage` match non-compiling until updated, so the variant and all its arms land together.

**Files:**
- Modify: `refbox/src/app/message.rs` (`enum ConfigPage` ~765-775; add `Message` variants + their arms in the discriminant/eq matches ~362+, 439+, 691+)
- Modify: `refbox/src/app/mod.rs` (`PageEntrySnapshot` ~362-398 + `revert_into` ~404-466; `capture_snapshot_for` ~1409-1446; `navigate_to_parent` ~1532-1547; `ApplyConfigPage`/`CancelConfigPage` ~2765-2804; new `Message::SelectBuzzer`/`TestBuzzer` handlers; remove `CyclingParameter::BuzzerSound` arm ~3550)
- Modify: `refbox/src/app/view_builders/configuration.rs` (remove `impl Cyclable for BuzzerSound` ~126-149; replace buzzer tile in `make_sound_config_page` ~1240-1249; add `make_buzzer_select_page`; add view dispatch arm ~347; add `page_has_changes` arm ~202)

**Interfaces:**
- Consumes: `BuzzerSound::ALL`, `SoundController::test_buzzer` (Tasks 2/4).
- Produces: `ConfigPage::Buzzer`; `Message::SelectBuzzer(BuzzerSound)`, `Message::TestBuzzer`; `make_buzzer_select_page(...)`.

- [ ] **Step 1: Add `ConfigPage::Buzzer` and the `Message` variants**

In `enum ConfigPage` add `Buzzer` after `Language`. In `enum Message` add:
```rust
    /// Operator tapped a sound in the buzzer picker (stages the selection).
    SelectBuzzer(BuzzerSound),
    /// Operator pressed TEST in the buzzer picker — plays the staged sound.
    TestBuzzer,
```
Add these two and `ConfigPage::Buzzer` to the relevant arms in the `Message` discriminant/eq/`From` matches in `message.rs` (mirror how `SelectLanguage`/`ConfigPage::Language` appear). Build to find every site: `cargo build -p refbox 2>&1 | grep -n "non-exhaustive\|not covered" | head`.

- [ ] **Step 2: Add the `PageEntrySnapshot::Buzzer` variant + revert + capture + change-detection + navigation**

`PageEntrySnapshot` (mod.rs): add
```rust
    Buzzer {
        buzzer_sound: BuzzerSound,
    },
```
`revert_into`: add
```rust
            PageEntrySnapshot::Buzzer { buzzer_sound } => {
                edited.sound.buzzer_sound = buzzer_sound;
            }
```
`capture_snapshot_for`: add
```rust
            ConfigPage::Buzzer => PageEntrySnapshot::Buzzer {
                buzzer_sound: edited.sound.buzzer_sound,
            },
```
`page_has_changes` (configuration.rs): add
```rust
        (ConfigPage::Buzzer, PageEntrySnapshot::Buzzer { buzzer_sound }) => {
            edited.sound.buzzer_sound != *buzzer_sound
        }
```
`navigate_to_parent` (mod.rs): add `ConfigPage::Buzzer` to the arm that returns `ConfigPage::Sound` (Buzzer's parent is the Sound page):
```rust
            ConfigPage::Display | ConfigPage::Sound => ConfigPage::User,
            ConfigPage::Buzzer => ConfigPage::Sound,
```

- [ ] **Step 3: Apply/Cancel/Select/Test handlers**

In `Message::ApplyConfigPage(page)` match, add a `ConfigPage::Buzzer` arm BEFORE the shared persist+navigate tail. It commits ONLY the buzzer (so unrelated in-progress Sound edits aren't force-committed), updates the controller, then falls through to the shared `persist_config` + `navigate_to_parent`:
```rust
                    ConfigPage::Buzzer => {
                        let bs = self.edited_settings.as_ref().map(|e| e.sound.buzzer_sound);
                        if let Some(bs) = bs {
                            self.config.sound.buzzer_sound = bs;
                        }
                        self.sound.update_settings(self.config.sound.clone());
                    }
```
(`CancelConfigPage(ConfigPage::Buzzer)` needs no new code — the generic `revert_from_snapshot()` + `navigate_to_parent` handle it via the new snapshot variant.)

Add the two new handlers (near the other config handlers):
```rust
            Message::SelectBuzzer(sound) => {
                if let Some(edited) = self.edited_settings.as_mut() {
                    edited.sound.buzzer_sound = sound;
                }
                Task::none()
            }
            Message::TestBuzzer => {
                if let Some(edited) = self.edited_settings.as_ref() {
                    self.sound.test_buzzer(edited.sound.buzzer_sound);
                }
                Task::none()
            }
```
Remove the `CyclingParameter::BuzzerSound => settings.sound.buzzer_sound.cycle(),` arm (~3550). If that leaves `CyclingParameter::BuzzerSound` unused, also remove the variant from `enum CyclingParameter` and any remaining match arms (build will point them out).

- [ ] **Step 4: Remove the carousel, add the entry tile + the picker view + dispatch**

Remove `impl Cyclable for BuzzerSound` (configuration.rs ~126-149).

In `make_sound_config_page`, replace the buzzer `make_value_button` (~1240-1249) with a value-button that opens the picker (still shows the current sound):
```rust
            make_value_button(
                fl!("buzzer-sound"),
                sound.buzzer_sound.to_string().to_uppercase(),
                (false, true),
                if sound.sound_enabled {
                    Some(Message::ChangeConfigPage(ConfigPage::Buzzer))
                } else {
                    None
                },
            ),
```

Add the picker builder. **Read `make_language_select_page` (configuration.rs ~1764-2012) first and mirror its banner + grid + footer construction element-for-element** (button height `MIN_BUTTON_SIZE`, `SPACING`, `blue_selected_button`/`light_gray_button`, fill widths). The grid is `BuzzerSound::ALL` in 3 rows of 4; the footer is Cancel | TEST | Apply:
```rust
fn make_buzzer_select_page<'a>(
    snapshot: &GameSnapshot,
    settings: &EditableSettings,
    mode: Mode,
    clock_running: bool,
    page_entry_snapshot: Option<&PageEntrySnapshot>,
    portal_indicator: Option<PortalIndicatorState>,
) -> Element<'a, Message> {
    let selected = settings.sound.buzzer_sound;

    let cell = |s: BuzzerSound| -> Element<'a, Message> {
        let style = if s == selected { blue_selected_button } else { light_gray_button };
        button(centered_text(s.to_string().to_uppercase()))
            .padding(PADDING)
            .height(Length::Fixed(MIN_BUTTON_SIZE))
            .width(Length::Fill)
            .style(style)
            .on_press(Message::SelectBuzzer(s))
            .into()
    };

    let mut grid = column![make_game_time_button(
        snapshot, false, false, mode, clock_running, portal_indicator, None
    )]
    .spacing(SPACING)
    .height(Length::Fill);
    for chunk in BuzzerSound::ALL.chunks(4) {
        let mut r = Row::new().spacing(SPACING).height(Length::Fill);
        for &s in chunk { r = r.push(cell(s)); }
        for _ in chunk.len()..4 { r = r.push(horizontal_space()); }
        grid = grid.push(r);
    }
    // Spacer rows keep the footer pinned at the bottom (mirror the Language page's
    // trailing `row![horizontal_space()].height(Length::Fill)` count for 3 grid rows).
    grid = grid
        .push(row![horizontal_space()].height(Length::Fill))
        .push(row![horizontal_space()].height(Length::Fill));

    let has_changes = page_has_changes(ConfigPage::Buzzer, settings, page_entry_snapshot);
    let cancel = make_button(fl!("cancel"))
        .style(red_button).width(Length::Fill)
        .on_press(Message::CancelConfigPage(ConfigPage::Buzzer));
    let test = make_button(fl!("test"))
        .style(blue_button).width(Length::Fill)
        .on_press(Message::TestBuzzer);
    let apply = {
        let b = make_button(fl!("apply")).style(green_button).width(Length::Fill);
        if has_changes { b.on_press(Message::ApplyConfigPage(ConfigPage::Buzzer)) } else { b }
    };
    grid.push(row![cancel, test, apply].spacing(SPACING)).into()
}
```
Add the dispatch arm next to `ConfigPage::Language =>` (~347):
```rust
        ConfigPage::Buzzer => make_buzzer_select_page(
            snapshot, settings, mode, clock_running, page_entry_snapshot, portal_indicator,
        ),
```
(Match the exact argument set the sibling arms pass; adjust names if the local builder signature in this file differs.)

- [ ] **Step 5: Build + clippy**

Run: `cargo build -p refbox 2>&1 | tail -8 && cargo clippy -p refbox -- -D warnings 2>&1 | tail -8`
Expected: clean.

- [ ] **Step 6: Commit**

```bash
git add refbox/src/app/message.rs refbox/src/app/mod.rs refbox/src/app/view_builders/configuration.rs
git commit -m "feat(refbox): full-page buzzer picker on the Sound settings page"
```

---

## Task 6: Beep-test buzzer picker (mirrors the beep-test Language picker)

**Files:**
- Modify: `refbox/src/app/mod.rs` (`enum BeepTestConfigPage` ~327-333; beep-test view dispatch ~4908-4981; beep-test message handlers near 4359-4508)
- Modify: `refbox/src/app/message.rs` (new beep-test `Message` variants + their match arms)
- Modify: `refbox/src/app/view_builders/beep_test_settings.rs` (replace buzzer tile in `build_beep_test_sound_settings_page` ~232-241; add `build_beep_test_buzzer_picker`)

**Interfaces:**
- Consumes: `BuzzerSound::ALL`, `test_buzzer`, the beep-test footer pattern.
- Produces: `BeepTestConfigPage::Buzzer`; `Message::{BeepTestEditOpenBuzzer, BeepTestSelectBuzzer(BuzzerSound), BeepTestTestBuzzer, BeepTestBuzzerSave, BeepTestBuzzerCancel}`; `build_beep_test_buzzer_picker`.

- [ ] **Step 1: Add `BeepTestConfigPage::Buzzer` + the 5 `Message` variants**

`enum BeepTestConfigPage`: add `Buzzer`. `enum Message`: add the 5 variants above (doc each, mirroring the existing `BeepTest*` message docs ~176-239). Build to find every match site in `message.rs` (the eq/discriminant matches list each `BeepTest*` variant explicitly) and add the new ones the same way.

- [ ] **Step 2: Replace the beep-test buzzer tile with an entry button**

In `build_beep_test_sound_settings_page`, replace `buzzer_sound_btn` (~232-241) so it opens the picker (keep showing the current sound):
```rust
    let buzzer_sound_btn = make_value_button(
        fl!("buzzer-sound"),
        sound.buzzer_sound.to_string().to_uppercase(),
        (false, true),
        if sound_enabled { Some(Message::BeepTestEditOpenBuzzer) } else { None },
    );
```

- [ ] **Step 3: Add `build_beep_test_buzzer_picker`**

**Read `build_beep_test_language_picker` (beep_test_settings.rs ~664-884) and mirror its layout** (4-wide rows, blue-selected highlight, trailing filler rows, the bottom Cancel/Apply footer). It takes the staged sound from `edited.sound.buzzer_sound`, and the footer is Cancel | TEST | Apply with Apply enabled when the staged sound differs from the live `config.sound.buzzer_sound`:
```rust
pub(in super::super) fn build_beep_test_buzzer_picker<'a>(
    config: &Config,
    sound: &SoundSettings,
) -> Element<'a, Message> {
    let selected = sound.buzzer_sound;
    let has_changes = config.sound.buzzer_sound != selected;

    let cell = |s: BuzzerSound| -> Element<'a, Message> {
        let style = if s == selected { blue_selected_button } else { light_gray_button };
        button(centered_text(s.to_string().to_uppercase()))
            .padding(PADDING)
            .height(Length::Fixed(MIN_BUTTON_SIZE))
            .width(Length::Fill)
            .style(style)
            .on_press(Message::BeepTestSelectBuzzer(s))
            .into()
    };

    let mut col = Column::new().spacing(SPACING).height(Length::Fill);
    for chunk in BuzzerSound::ALL.chunks(4) {
        let mut r = Row::new().spacing(SPACING).height(Length::Fill);
        for &s in chunk { r = r.push(cell(s)); }
        for _ in chunk.len()..4 { r = r.push(horizontal_space()); }
        col = col.push(r);
    }
    col = col
        .push(row![horizontal_space()].height(Length::Fill))
        .push(row![horizontal_space()].height(Length::Fill));

    let cancel = make_button(fl!("cancel"))
        .style(red_button).width(Length::Fill)
        .on_press(Message::BeepTestBuzzerCancel);
    let test = make_button(fl!("test"))
        .style(blue_button).width(Length::Fill)
        .on_press(Message::BeepTestTestBuzzer);
    let apply = {
        let b = make_button(fl!("apply")).style(green_button).width(Length::Fill);
        if has_changes { b.on_press(Message::BeepTestBuzzerSave) } else { b }
    };
    col.push(row![cancel, test, apply].spacing(SPACING)).into()
}
```

- [ ] **Step 4: View dispatch + handlers**

In the `AppState::BeepTestSettings(page)` view match (~4908), add:
```rust
                BeepTestConfigPage::Buzzer => {
                    let edited = self.edited_settings.as_ref().expect(
                        "edited_settings must be Some when AppState is BeepTestSettings(Buzzer)",
                    );
                    build_beep_test_buzzer_picker(&self.config, &edited.sound)
                }
```
Add the handlers near the other `BeepTest*` handlers (~4359-4508). `Open` navigates from the Sound sub-page (edited_settings already seeded) to the Buzzer sub-page; `Select` stages; `Test` previews; `Save` returns to the Sound sub-page keeping the staged selection (the beep-test Sound Save persists it, exactly as the volumes work today); `Cancel` reverts the buzzer to the live value and returns:
```rust
            Message::BeepTestEditOpenBuzzer => {
                self.app_state = AppState::BeepTestSettings(BeepTestConfigPage::Buzzer);
                Task::none()
            }
            Message::BeepTestSelectBuzzer(sound) => {
                if let Some(edited) = self.edited_settings.as_mut() {
                    edited.sound.buzzer_sound = sound;
                }
                Task::none()
            }
            Message::BeepTestTestBuzzer => {
                if let Some(edited) = self.edited_settings.as_ref() {
                    self.sound.test_buzzer(edited.sound.buzzer_sound);
                }
                Task::none()
            }
            Message::BeepTestBuzzerSave => {
                self.app_state = AppState::BeepTestSettings(BeepTestConfigPage::Sound);
                Task::none()
            }
            Message::BeepTestBuzzerCancel => {
                if let Some(edited) = self.edited_settings.as_mut() {
                    edited.sound.buzzer_sound = self.config.sound.buzzer_sound;
                }
                self.app_state = AppState::BeepTestSettings(BeepTestConfigPage::Sound);
                Task::none()
            }
```

- [ ] **Step 5: Build + clippy**

Run: `cargo build -p refbox 2>&1 | tail -8 && cargo clippy -p refbox -- -D warnings 2>&1 | tail -8`
Expected: clean.

- [ ] **Step 6: Commit**

```bash
git add refbox/src/app/message.rs refbox/src/app/mod.rs refbox/src/app/view_builders/beep_test_settings.rs
git commit -m "feat(refbox): buzzer picker reachable from beep test settings"
```

---

## Task 7: Translations — `test` button label (15 locales)

**Files:**
- Modify: `refbox/translations/*/refbox.ftl` (all 15 locales)

**Interfaces:**
- Produces: `test` key, used by `fl!("test")` in both pickers.

- [ ] **Step 1: Add the key to en-US**

In `refbox/translations/en-US/refbox.ftl`, near `language`/`apply`:
```
test = TEST
```

- [ ] **Step 2: Add a best-guess translation to every other locale**

For each non-English locale dir, add `test = <translation>` (uppercase). Use the imperative "test/try" verb in each language (e.g. es: `PROBAR`, fr: `TESTER`, de: `TESTEN`, it: `PROVA`, pt: `TESTAR`, nl: `TEST`, tr: `TEST ET`, id: `UJI`, ms: `UJI`, tl: `SUBUKAN`, ja: `テスト`, ko: `테스트`, zh-Hans/zh-Hant: `测试`/`測試`, th: `ทดสอบ`). Mark for native review per project convention; do NOT leave English placeholders.

- [ ] **Step 3: Verify coverage**

Run:
```bash
for d in refbox/translations/*/; do grep -q "^test =" "$d/refbox.ftl" || echo "MISSING: $d"; done
```
Expected: no output (all locales have it).

- [ ] **Step 4: Commit**

```bash
git add refbox/translations
git commit -m "i18n(refbox): add TEST button label in all locales"
```

---

## Task 8: Full verification

**Files:** none (verification only)

- [x] **Step 1: Full check** — `just check` green (exit 0): clippy `-D warnings` clean on
  `--all` and `--all --no-default-features`; 349 tests + all suites pass (0 failed); audit only
  the 9 pre-existing allow-listed advisories. Re-run 2026-06-25 after the walkthrough edits.

- [x] **Step 2: Build the real binary** — `cargo build -p refbox` success (rebuilt after each
  walkthrough edit before relaunching the window).

- [x] **Step 3: In-app walkthrough (operator-observable)**

Launch (per project run convention, with sandbox disabled and Wayland forced off):
`WAYLAND_DISPLAY= cargo run -p refbox` (background, dangerouslyDisableSandbox).
Confirm:
- Settings → Sound: the BUZZER tile shows the current sound and opens the picker.
- Picker shows 12 sounds (3×4), selection highlighted; footer is Cancel | TEST | Apply; Apply gray until selection changes.
- Pick each NEW sound, press TEST → hear the full ~2.15s buzzer; Apply → returns to Sound page; trigger a real period-end buzzer and a held manual alarm → correct sound, seamless held loop.
- Cancel discards a change.
- Beep test → Settings → Sound: same BUZZER tile → identical picker; pick a sound, Apply, run a beep test → chosen sound plays at lap end.

- [x] **Step 4: Walkthrough result recorded (below).** All Step-3 items PASS — see the
  walkthrough-result deviation entry.

---

## Deviations

(Record any execution deviations here per the lean process — do not make standalone deviation commits.)

- **Tasks 2+3 merged** into one implementer dispatch (commit 01a40e11): adding `BuzzerSound`
  variants makes `SoundLibrary`'s `Index` match non-exhaustive, so the variants and the embeds
  must compile together.
- **Task 5 does NOT remove the old carousel infrastructure** (`impl Cyclable for BuzzerSound`,
  the `CyclingParameter::BuzzerSound` match arm at mod.rs:3550, or the `CyclingParameter::BuzzerSound`
  variant), contrary to Task 5 Steps 3–4. Reason: the beep-test Sound page tile
  (`beep_test_settings.rs:237`) still constructs `CyclingParameter::BuzzerSound` until Task 6
  migrates it, and the `.cycle()` handler at mod.rs:3550 keeps `impl Cyclable for BuzzerSound`
  live. Removing any of these in Task 5 breaks compilation. Task 5 only repoints the **main**
  Sound page tile to `ChangeConfigPage(ConfigPage::Buzzer)`. The carousel-infra removal moves to
  **Task 6**, after the beep-test tile is migrated (clippy `-D warnings` then forces the cleanup).

### Task 8 walkthrough result (2026-06-25) — operator-driven, all PASS

Walkthrough run live with the operator (WSL/X11; one audio hiccup mid-session from multiple
Ref Box instances colliding on the sim ports + audio device — resolved by running a single
instance, not a code issue). Three small in-scope changes were made on operator feedback during
the walkthrough; all re-verified with `just check` green afterward:

- **Empty filler row added to the main Sound buzzer picker** (`make_buzzer_select_page`,
  configuration.rs). The picker mirrored the real Language page, which has no trailing filler,
  so the footer sat directly under the sound grid. Added ONE `row![horizontal_space()]` of
  `Length::Fill` above the footer (operator chose one row — this page already carries the
  "next game" ribbon up top, which the beep-test picker lacks).
- **Beep-test buzzer picker bumped from 2 → 3 trailing filler rows**
  (`build_beep_test_buzzer_picker`, beep_test_settings.rs) on operator request — that page has
  no top ribbon, so it needed the extra filler to keep the footer off the grid.
- **Loudness re-level of the 3 quietest new sounds** (klaxon, airhorn, pipes) in
  `regen-buzzer-sounds.py`. Peak-normalization (`norm` → 0.95 peak) left honk-with-a-gap and
  decaying-clang sounds far quieter by ear than the old sounds (RMS 0.17–0.50 vs old band
  0.56–0.89). Added a per-sound `loud(x, drive)` finisher (tanh soft-saturate → peak-norm 0.97,
  memoryless so loop seams are preserved) with by-ear drives: klaxon 1.6 (RMS→0.62), airhorn 4.5
  (→0.59), pipes 5.5 (→0.49). pip/pulse/siren/trill already in-band → left on plain `norm`
  (byte-identical, unchanged). Operator ear-approved klaxon/airhorn/pipes against the old sounds.
  Siren/trill seams untouched per the standing "don't fix the trill/siren loop seams" instruction.

Walkthrough items confirmed PASS by the operator: picker layout + 12 sounds + selection
highlight + footer; Apply greys when unchanged, enables on change, commits + returns; Cancel
discards; TEST plays each sound; real period-end buzzer + held manual alarm (PIPES) with a
smooth held loop; beep-test picker reaches the same 12 sounds and the chosen sound (KLAXON)
fires at lap end.

Separate backlog idea surfaced (NOT this branch): "CANCEL → BACK on config-page footers when
the page has no pending changes" — captured in `docs/backlog/cancel-back-label-when-unchanged/`.
