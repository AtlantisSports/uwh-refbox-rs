# Button Height By Role — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or
> superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`)
> syntax for tracking.

**Goal:** Make a button's height a consequence of its *role* rather than a per-call-site
decision, so a settings tile can never again render short next to its neighbours.

**Architecture:** Two roles, two helpers. A **tile** is a large content cell in the body of a
page and always fills its row. **Chrome** is the fixed-size furniture around the body — the
Cancel/Apply footer, Back buttons, keypad keys — and always keeps `MIN_BUTTON_SIZE`. After this
change no call site passes a height at all; the helper it picks decides.

**Tech Stack:** Rust 2024, iced 0.13, `refbox` crate only.

**Spec:** This plan is the spec — the design was settled in conversation on 2026-08-27. The
originating defect is the MANUAL GAMES tile rendering at 89 px beside two filling buttons on the
Game Options page, fixed separately on `fix/refbox/manual-games-button-height`.

## Global Constraints

- MSRV 1.85, edition 2024. Do not change either.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` must stay clean.
- Do not touch `uwh-common`, `overlay`, `wireless-remote`, or any wire format. This is a
  `refbox`-only UI change.
- Do not add dependencies.
- Do not introduce `unwrap()`/`expect()` in production code.
- `MIN_BUTTON_SIZE` stays at 89.0 and keeps its current meaning for chrome.
- **iced 0.13 has no `min_height`.** `Button` exposes only `height()`; `Container` exposes
  `max_height` but no minimum. A filling tile therefore has no floor. This was accepted
  knowingly: on the Pi the window is a fixed size and the Portal page's tiles already fill today
  without being too small to tap. Do not invent a floor mechanism to work around this.

---

## Scope boundary

**In scope — files that will change:**

- `refbox/src/app/view_builders/shared_elements.rs` — helper definitions and the rename
- `refbox/src/app/view_builders/configuration.rs` — settings pages (the bulk of the change)
- `refbox/src/app/view_builders/beep_test_settings.rs` — beep-test settings pages
- Every file that calls `make_button`, for the mechanical rename only (22 files, 120 sites)

**Explicitly out of scope — do not touch:**

- `make_small_button` (square keypad keys), `make_smaller_button` (XS controls) and
  `make_multi_label_button`. Their fixed sizes are deliberate and unrelated.
- The scroll-list arrows in `shared_elements.rs` (`up_btn` / `down_btn`, around line 172). They
  are *supposed* to be fixed squares with the scroll bar filling between them. This is the one
  legitimate fixed-next-to-filling pairing in the codebase.
- The main game screen, fouls, penalties, warnings, game-info and keypad pages. Their large
  buttons already fill; they need the rename and nothing else.
- Any layout, spacing, wording or behaviour change. Only tile *height* changes.

## Acceptance criteria

Observable by the human, without reading code:

1. On the Game Options page with **UWH PORTAL** selected, all six tiles (MANUAL GAMES,
   UWH PORTAL, CUSTOM, SITE/EVENT, ACCESS TOKEN, COURT) are the same height and fill the page
   evenly. This is the defect that started this work.
2. On the Game Options page with **MANUAL GAMES: YES**, all twelve tiles are the same height and
   fill the page evenly — matching the Portal view rather than sitting short with gaps beneath.
3. The Sound, App, Display and Remotes settings pages likewise show evenly-filled tiles.
4. The Cancel / TEST / Apply footers, Back buttons and keypad keys are **unchanged in size** on
   every page.
5. The scroll-list up/down arrows are unchanged — still square, with the bar filling between them.
6. The main game screen, fouls, penalties and warnings pages look exactly as they do today.
7. `just check` passes.

## Why there are no unit tests in this plan

There is no layout-test harness in this repo and iced 0.13 offers no way to assert a rendered
widget's height. `configuration.rs` has no test module and adding one would test nothing real.
Verification for this change is compilation, `just check`, and a human looking at the screen —
see "Verification" at the end. **Do not fabricate tests that assert on constructed widget values;
they would pass without proving anything.** This is a deliberate departure from the repo's
"write a test for every fix" rule, and it is the honest one here.

---

## Task 1: Introduce the two roles in `shared_elements.rs`

**Files:**
- Modify: `refbox/src/app/view_builders/shared_elements.rs`

**Interfaces produced** (later tasks depend on these exact names):

- `make_chrome_button(label) -> Button` — fixed `MIN_BUTTON_SIZE`. This is today's
  `make_button`, renamed. Behaviour identical.
- `make_tile_button(label) -> Button` — same construction, but `Length::Fill`.
- `make_value_button(first_label, second_label, large_text, message) -> Button` — unchanged
  signature, but its height becomes `Length::Fill`.
- `make_long_value_button(label, value, message) -> Button` — unchanged signature, height
  becomes `Length::Fill`.

- [ ] **Step 1: Rename `make_button` to `make_chrome_button`**

Rename the function and update all 120 call sites across the 22 files that use it. This is a
pure rename — no behaviour change anywhere. Do it as its own commit so the behavioural commits
that follow are readable.

The point of the rename is that the role is now stated at every call site. A future author
placing `make_chrome_button` inside a filling row can see that they have chosen furniture, not
a tile.

- [ ] **Step 2: Add `make_tile_button` beside it**

Identical body to `make_chrome_button` except `.height(Length::Fill)`. Give it a doc comment
saying it is for large content cells in a page body and that it must not be used in a footer,
because a filling child in a shrink-height row collapses.

- [ ] **Step 3: Flip `make_value_button` and `make_long_value_button` to `Length::Fill`**

Change `.height(Length::Fixed(MIN_BUTTON_SIZE))` to `.height(Length::Fill)` in both.

Preserve the existing comment in `make_value_button` warning against pairing `align_y(Center)`
with `height(Fill)` on the *inner text widgets* — that warning is about an iced 0.13 paragraph
caching bug and still applies. It concerns the text inside the button, not the button itself.

- [ ] **Step 4: Build**

`cargo build -p refbox`. Expect it to compile. Expect pages to look wrong until Task 2 removes
the now-redundant overrides — that is fine and expected at this point.

- [ ] **Step 5: Commit**

```
refactor(refbox): split button helpers into tile and chrome roles
```

---

## Task 2: Remove the now-redundant `.height()` overrides

**Files:**
- Modify: `refbox/src/app/view_builders/configuration.rs`
- Modify: `refbox/src/app/view_builders/beep_test_settings.rs`

Every `.height(Length::Fill)` appended to a `make_value_button(...)` or
`make_long_value_button(...)` call is now redundant. Remove them all. The helper decides.

- [ ] **Step 1: Find them**

```
grep -n "make_value_button\|make_long_value_button" -A 12 refbox/src/app/view_builders/configuration.rs | grep -n "height(Length::Fill)"
```

Known sites include the EVENT tile, the COURT tile and the custom SITE tile in
`make_event_config_page`. Work through every match rather than the listed ones only.

- [ ] **Step 2: Remove each override, leaving the helper call bare**

- [ ] **Step 3: Build and eyeball the diff**

`cargo build -p refbox`, then `git diff` — the diff should be deletions only.

- [ ] **Step 4: Commit**

```
refactor(refbox): drop per-site tile height overrides
```

---

## Task 3: Convert the remaining tiles that were built as chrome

**Files:**
- Modify: `refbox/src/app/view_builders/configuration.rs`
- Modify: `refbox/src/app/view_builders/beep_test_settings.rs`

Some page tiles were built with the old `make_button` (now `make_chrome_button`) and are
therefore still fixed-height. They sit inside filling rows and must become
`make_tile_button`.

**The rule to apply:** a button is a tile if it sits in a container whose height is
`Length::Fill` and it is not a scroll arrow. Nothing else qualifies.

- [ ] **Step 1: Convert the known sites**

At minimum these, all currently fixed inside filling rows:

- `make_event_config_page`: the two source buttons (UWH PORTAL, CUSTOM) and the ACCESS TOKEN
  button are hand-built with `button(...)` plus an explicit `.height(Length::Fill)`. Leave their
  height behaviour as-is but note they are tiles; converting them to `make_tile_button` is only
  worthwhile if their inner layout allows it. **Do not force it** — a hand-built button with
  bespoke inner layout is not a regression.
- `make_display_config_page`: `sides_btn`, `layout_btn`
- `make_sound_config_page`: `sound_button` and the volume/enable tiles
- `beep_test_settings.rs`: `app_mode_button`, `display_layout_button`, `edit_levels_button`,
  `language_button`, `sound_button`

- [ ] **Step 2: Sweep for any missed ones**

Re-run the rule over both files: for every `row!`/`column!` carrying `.height(Length::Fill)`,
confirm every button child is a tile helper. Any `make_chrome_button` still inside a filling row
is either a miss or a deliberate exception — if deliberate, write a comment saying why.

- [ ] **Step 3: Build and commit**

```
refactor(refbox): make page tiles fill their rows
```

---

## Task 4: The language pickers and buzzer grids

**Files:**
- Modify: `refbox/src/app/view_builders/configuration.rs`
- Modify: `refbox/src/app/view_builders/beep_test_settings.rs`
- Modify: `refbox/src/app/view_builders/shared_elements.rs` (`make_lang_button_with_note`)

These pages build their cells through local closures, so the helper swap does not reach them.
Both pages pair two cell shapes that **must stay the same height as each other**:

- The language pages use `lang_btn` (a hand-built `button(...)` with
  `.height(Length::Fixed(MIN_BUTTON_SIZE))`) alongside `lang_btn_note` (which wraps
  `make_lang_button_with_note`, also fixed). Change **both** to `Length::Fill` in the same
  commit. Changing one alone reintroduces exactly the defect this branch exists to remove.
- The two buzzer-sound pickers build 12 cells through a local `cell` closure with a fixed
  height, laid out in filling rows of four. Change the closure's height to `Length::Fill`.
  Because every cell comes from one closure they cannot disagree with each other, but they can
  disagree with the page around them.

- [ ] **Step 1: Change `lang_btn`, `lang_btn_note` and `make_lang_button_with_note` together**
- [ ] **Step 2: Change both `cell` closures**
- [ ] **Step 3: Build and commit**

```
refactor(refbox): fill language and buzzer picker cells
```

---

## Task 5: Verify and finish

- [ ] **Step 1: `just check`**

Run it **from inside this worktree**. A background task does not inherit a `cd`, so use
`cd <worktree> && just check` in one command. Confirm from the log that the compiled paths are
the worktree's, not the main checkout's — this mistake already happened once on the sibling
branch and produced a green result for code that was never tested.

- [ ] **Step 2: Confirm nothing outside scope moved**

`git diff --stat` against `origin/master`. Expect: `shared_elements.rs`, `configuration.rs`,
`beep_test_settings.rs` with real changes, and the remaining ~19 files with rename-only churn.
Any other file is scope creep — back it out.

- [ ] **Step 3: Record the deviations**

Append a "Deviations" section to this file describing anything that differed from the plan,
per `.claude/rules/plan-execution.md`. Do not create a standalone deviations commit — fold it
into the final code commit.

- [ ] **Step 4: Hand over for a human walkthrough**

This change cannot be verified without eyes on a screen. Write numbered steps covering every
acceptance criterion above and hand them to the human. Do not claim the change is verified on
the strength of `just check` alone.

---

## Risks

**The one that matters:** a filling tile has no minimum height (see Global Constraints). If the
refbox window is ever made much shorter than the Pi's screen, tiles will shrink below the 89 px
touch target. There is no guard against this in iced 0.13. If the human reports tiles looking
small on a resized desktop window, that is this trade-off surfacing — not a bug in the
implementation — and the fix would be a custom widget, which is out of scope here.

**Secondary:** a filling child inside a shrink-height row collapses. If a footer ever loses its
height, its buttons will vanish rather than render short. That is why the footer helpers stay
`make_chrome_button` and why Task 3's rule is scoped to filling containers only.

---

## Deviations

**1. `make_value_button` had the same dual role the plan only attributed to `make_button`.**
The plan assumed every label-and-value button was a tile. Two are furniture: the Game Options
footer's game-number picker (`game_picker_btn`, in the Cancel | picker | Apply row) and the
Display page's brightness button (stacked with OPEN NEW DISPLAY in a column that sizes to its
contents). Flipping the helper to `Length::Fill` made both *collapse*, not merely render short —
the failure mode named in the plan's Risks section, reached from a direction the plan did not
anticipate.

Resolved by adding `make_value_chrome_button` (fixed `MIN_BUTTON_SIZE`) and converting those two
sites, rather than renaming the 40 genuine tile sites. `make_value_button` keeps its name and now
means "tile"; its doc comment says so and points at the chrome variant.

**2. Task 4's closure flip was done by text pattern and over-reached.**
The language and buzzer cells are built inside local closures, so they were changed by matching
on `.padding(PADDING)` + `.height(Length::Fixed(MIN_BUTTON_SIZE))`. That pattern also matched
four buttons that are chrome, all in shrink-height rows:

- the main config page's blue power icon button (whose own comment says it is sized to line up
  with the Back button)
- the beep-test language picker's Cancel button
- the beep-test language picker's Apply button, both branches (restart / apply)

All four were reverted to fixed. They were caught by diffing every changed `height(...)` line
individually rather than trusting the replacement count — worth repeating on any future sweep of
this kind.

**3. Builds were not run per task.** One build after Tasks 1–4 rather than four cold builds in a
fresh worktree. No behaviour consequence.

**4. Verification is a static invariant check, not tests.** As the plan said no tests were
possible, the substitute is a script that reads every `row!`/`column!` in `view_builders/` and
resolves each child's height policy, asserting two invariants:

- **A:** no fixed-height button sits inside a filling container — *except* the scroll-list
  arrows, the one documented exception.
- **B:** no filling child sits inside a container that sizes to its contents, beyond the seven
  such pairings that already existed on `origin/master`.

Both hold. Invariant B is what caught deviation 1, and re-running it is what proved the fix. The
script lives in the session scratchpad, not the repo — it is a one-off audit tool, and checking
it in would imply a guarantee that nothing runs it.

**5. Only three redundant `.height(Length::Fill)` overrides existed** (the EVENT, COURT and
custom SITE tiles), not the larger number Task 2 implied.

**6. `sides_btn`, the `lang_btn` closures and the buzzer `cell` closures are hand-built buttons,
not helper calls.** Their heights were flipped in place rather than routed through
`make_tile_button`, because each has bespoke inner layout that the helper does not produce.

**7. The invariant in deviation 4 was stated wrongly, and the wrong form hid three real defects.**
"No fixed-height button inside a filling container" is too strong: a page of one-off actions
(the Updates page) legitimately keeps every button fixed inside filling rows, which is how it
renders today. The rule that actually describes the defect Eric reported is **no container may
*mix* fixed and filling button children**. Restated, and the code satisfies it — the only two
mixed containers left are the scroll-list arrows (documented) and a filling *text* beside two
fixed buttons in the remotes list, both pre-existing on `origin/master`.

**8. The invariant script could not see buttons bound as `Element`, and `code-review` caught
three violations because of it.** Its binding regex matched `let name = …` but not
`let name: Element<'a, Message> = …`, so `version_element`, `primary_element` and
`audio_output_slot` all resolved to UNRESOLVED and were skipped. Fixed the regex; the three
findings then reproduce. What the script missed:

- **Updates page:** CHECK FOR UPDATES stayed fixed beside a now-filling CURRENT VERSION tile.
  Resolved by treating the whole page as chrome — every button on it is a one-off action, not a
  content cell — so it renders exactly as it does today.
- **Sound page (non-Linux only):** UPDATE AUDIO OUTPUT stayed fixed beside a filling tile. Now a
  tile. Invisible on the Pi and in WSL, where that slot is a spacer — a Linux-only walkthrough
  would never have caught it.
- **Beep-test LANGUAGE:** made a tile inside `lower_left`, a column with no height.
  `iced_core::layout::flex` hard-codes `remaining = 0.0` when the container's main axis is
  `Shrink`, so the button would have collapsed to nothing. Reverted to chrome.

**9. `code-review` also caught defects no invariant would find**, all fixed: the filling
STARTING SIDES button left its caption pinned above the team labels (the row now centres its
children); `make_value_chrome_button` carried a copy-pasted doc block describing a different
function and duplicated ~55 lines verbatim (now a two-line delegate); the beep-test layout
comment claimed tiles do not stretch, when after this branch they do; two *live* backlog notes
told readers to grep for `make_button`, which no longer exists.

**10. The lesson for the next sweep of this kind.** Every defect in deviations 8 and 9 was legal
Rust that passed `cargo check`, `clippy` and `fmt`. The static script found the ones it could
see and silently skipped the rest — a clean run from it means "found nothing", never "there is
nothing". It is a filter, not a gate; the gate is review plus a human looking at the screen.

**11. The second review found that deviation 8's beep-test fix was itself wrong, and the reason
matters more than the fix.** I reverted the beep-test LANGUAGE tile to chrome believing
`lower_left` was a `Shrink` column where a filling child would collapse. It is not.
`Row::push`/`Column::push` run `Length::enclose`, which upgrades a container from `Shrink` to
`Fill` on its first filling child, and the `row!`/`column!` macros fold through `push`. Only a
*filling* child upgrades a container: every Cancel/Apply footer is
`row![cancel, horizontal_space(), apply]` where the buttons are `Fixed` and `horizontal_space()`
is `Fill`-wide but `Shrink`-tall, so those rows stay `Shrink` — which is precisely why putting a
tile in one would make it grow.

So the revert *created* the defect this branch exists to remove: LANGUAGE would have rendered at
89 px, top-aligned, beside four filling tiles. Restored to a tile.

**The claim was load-bearing in three places and wrong in all of them.** "A filling child in a
shrink row collapses to nothing" appeared in `make_tile_button`'s doc and in the comments on both
`make_value_chrome_button` call sites. The real consequence of a tile in a footer is the
opposite: the footer *becomes* `Fill`, claims a share of the page, and **grows** at the body's
expense. All three comments now say that.

The two chrome conversions from deviation 1 were still right — a growing footer is as wrong as a
collapsing one — but they were right for a reason I had not established. Recorded in memory as
[[reference_iced_container_height_enclose]].

**12. Everything else the second review raised, fixed:** the stray doc block above
`make_value_chrome_button` (a duplicate of `make_long_value_button`'s, which deviation 9 claimed
to have removed and had not); the two helper docs disagreeing on how many furniture sites exist;
`make_tile_button` duplicating `make_chrome_button`'s body instead of delegating; a
`make_chrome_button` call site in `list_selector.rs` restating the helper's own height; the
STARTING SIDES caption keeping a now-dead `align_y(Center)` that is also the stale-paragraph
antipattern this repo removed from its helpers; the Updates-page comment claiming the page
renders unchanged when its labels shift like every other value button; and a *live*
regression-coverage feature file naming `make_button`.

**13. What three review passes cost, and what they bought.** Pass 1 found a real defect and one
false one. Pass 2 found three real violations my script structurally could not see. Pass 3 found
that pass 1's beep-test finding was wrong and I had acted on it. Every defect in all three passes
was legal Rust that compiled clean, linted clean and passed 717 tests. The reviews were the only
thing that found any of it — and the third pass was necessary precisely because acting on a
review is itself a change that needs reviewing.

**14. The scope boundary is breached, deliberately, and this is the record of it.** The boundary
above says "Only tile *height* changes." It is not true: the inner row of `make_value_button`
gained `.height(Length::Fill)`, which moves the **label** down a few pixels in *every*
label-and-value button in the app — including the three deliberately kept at
`MIN_BUTTON_SIZE`. Without it, a filling tile strands its label at the top, so the change is
required; but `.claude/rules/scope.md` says an out-of-scope change is surfaced, not folded in.
The walkthrough must therefore include the three chrome sites — the Game Options footer's game
picker, the Display page's brightness button, and the Updates page's version row — where the
button does *not* change size but its label moves.

**15. A fourth review pass confirmed all of pass 3's changes were functionally correct and found
no new defect — but found the same class of error again in the prose.** The rationale I wrote in
deviation 11 ("the container grows") is true only for containers that declare no height. The
Display side column and the Updates version row both set their own height, so neither can grow;
they stay fixed because each has a fixed *sibling* to match. Three passes in a row I stated a
layout mechanism confidently and got it wrong — first "collapses", then "grows", then
"grows everywhere" — while the actual code changes were right each time. The corrected rule now
lives in `make_tile_button`'s doc with all three limits on `Length::enclose` spelled out.

Pass 4 also caught an integrity error, not a layout one: the rename sweep had edited
`button-damage-tracking.feature` — a dated record carrying `@user_verified @tested_pass` and a
2026-05-15 walkthrough note — to list `make_tile_button` among the helpers that sweep covered.
That helper did not exist then and was never in that walkthrough, so the edit asserted human
sign-off that was never given. Reverted to naming the old symbol with the rename noted.
`docs/decisions/023-language-ui-chrome.md` still says `make_button` and is **left alone** on
purpose, per `.claude/rules/plan-execution.md`: an ADR stays accurate as of its approval.

## Known follow-ups, not done here

- `lang_btn` and the buzzer `cell` closure are each duplicated across `configuration.rs` and
  `beep_test_settings.rs` — four hand-maintained copies whose heights this branch flipped. A doc
  comment warns about the drift; extracting shared helpers would remove it.
- `centered_text` pairs `align_y(Center)` with `height(Fill)`, the stale-paragraph pattern this
  repo removed from its button helpers. Pre-existing, no live regression (the labels are static),
  but this branch enlarged the four widgets that use it.
- `beep_test.rs`'s `top_row_tile` lays its labels out at zero height and paints them across the
  tile's top edge — the same class as the originating defect, visible during a beep test.
- The PORTAL and CUSTOM source buttons use plain `text` rather than `fit_text`, so they cannot
  shrink for a long translation the way every tile beside them does.

## Walkthrough — done

Walked by Eric on 2026-08-28, on this machine (945×691) against the dev portal, on the build from
this branch. All nine steps confirmed, including the four that were most at risk: the Game
Options footer, the Display page's paired buttons, the Updates page's version row, and the
beep-test LANGUAGE tile that had been wrongly reverted to chrome and restored.

**One question raised and settled during the walk.** The Cancel/Apply action row stays at
`MIN_BUTTON_SIZE` while the content rows above it fill, which reads as an odd short row on a
four-row page. Eric asked whether it should match. It should not, and the reason is not
aesthetic: Manage Remotes and Check Version have a single content row each, so a filling footer
would be *half the page* on both. The tempting middle rule — fill on pages with three or more
rows — puts the height decision back at the call site, which is the defect class this branch
exists to remove. Left fixed; revisit only if it reads wrong on the Pi, where the rows are
nearer 106px than the 118px seen here.

## Re-walked after the rebase

Rebased onto `b481ff6d` ("refuse zero time values in the game settings"), which touches the same
file. Clean rebase — and worth noting *why* that needed checking rather than trusting: the
incoming commit references `make_button`, which commit 1 renames away. Git resolved it by
applying the rename; had it resolved the other way the branch would have looked perfectly
rebased and not compiled. Verified by grep (no `make_button` survives) and
`cargo check --all-targets`.

Eric re-walked the intersection on 2026-08-28 and confirmed both: a zero Half Length is still
refused with its red text, and the page still reads correctly while showing it. Test count went
717 → 741, the extra 24 being the incoming commit's guard suite passing against this refactor.

**One process failure worth keeping.** The first re-walk attempt tested nothing, twice over: the
running app was the *other session's* worktree build (the shared config had been claimed again
after I closed mine), and my own binary was stale anyway — `cargo check` and `just check` do not
refresh `target/debug/refbox`, because `just check` builds test binaries. It looked exactly like
the zero-value fix had been lost in the rebase. Before any walkthrough: rebuild, then confirm
`/proc/<pid>/exe` points at the worktree you mean.
