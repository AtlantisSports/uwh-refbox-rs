# Translation consistency test — design

**Date:** 2026-08-12
**Status:** approved by Eric, not yet implemented
**Scope:** `refbox` only — one test-only module, one dev-dependency, one dead key deleted

## The problem

`refbox` ships 335 translation keys in 15 languages. Two failure modes silently produce wrong text
on a referee's screen at a tournament:

1. **A key missing from a locale.** Already covered — see below.
2. **A key whose `{ $variables }` do not match the English original.** Covered by nothing. A
   translation that drops `{ $game }` renders a sentence with a hole in it; one that misspells it
   renders the placeholder literally. Neither fails any build.

The second is not hypothetical for this codebase. The branch merged as #2219 removed `{ $portal }`
from two keys across 15 files; had one locale kept it, nothing would have said so.

## What already exists — and what it misses

**`refbox/build.rs` already checks key coverage.** It parses every `.ftl` with `fluent-syntax` (a
build-dependency, which is why it never appears in `src/`), extracts message IDs, and reports any
key missing from a file. **Do not claim the repo has no coverage check** — that error was made on
2026-08-12 by grepping `src/` for test functions instead of reading the build script.

Its three weaknesses, which this design works around rather than fixes:

- It compares each file against the **union of all files**, not against `en-US`, although
  `i18n.toml` declares `fallback_language = "en-US"`. One stray key in one locale reports the other
  fourteen as missing it.
- It checks only that keys **exist**, never their contents. Variables are invisible to it.
- In debug builds it emits `cargo:warning=`, which does not fail `just check` or CI — build-script
  warnings are unaffected by `-D warnings`.

**`build.rs` is deliberately left untouched.** Its per-file `cargo:rerun-if-changed` lines sit in
the same directory walk as the flawed comparison and may be what makes a release build notice an
edited translation. Removing a merely-redundant check is not worth disturbing that. Tidying it is a
separate, deliberate job.

## What the test asserts

All three against **`en-US` as the reference**, matching `i18n.toml`:

1. **Coverage** — every en-US message exists in all fourteen other locales.
2. **Variable consistency** — every message's set of `{ $variables }` matches en-US's exactly, in
   every locale. This is the gap that motivates the work.
3. **Usage** — every en-US message is actually reachable.

## How it parses

`fluent_syntax::parser::parse` — the same crate `build.rs` already uses, added under
`[dev-dependencies]` at the identical version, so **no new crate enters the dependency tree**.

Variables are collected by walking the AST **recursively**. This is not optional and a regex cannot
substitute: the translations contain nested select expressions. `penalty` is
`#{$player_number} - {$time -> … {$kind -> … } }`, with variables several levels down, and
`brightness` and `foul` are similar. The walk must descend into a select expression's **selector and
every variant's pattern**.

## What counts as "used"

Deliberately broader than `fl!()`, or the check produces false alarms:

- a **string-literal `fl!("key")`** anywhere under `refbox/src/**/*.rs`, **or**
- a **reference from another en-US message** — `penalty-kind` is referenced as `{ penalty-kind }`
  inside `penalty`, not from Rust.

Literal scanning is reliable here because **there are no dynamic `fl!()` call sites**: the only
non-literal occurrences are the macro's own definition in `main.rs` and a doc comment. This was
verified, not assumed. If a dynamic call site is ever introduced, this assertion becomes unsound and
must be revisited — worth a comment in the test saying so.

**Terms are excluded from assertion 3.** A term (`-dark-team-name`) exists to be referenced from
`.ftl` and never from code; requiring an `fl!()` call for one would be wrong. There are 2.

## Failure output

One test, reporting **every** problem at once, grouped by language. A translator fixing fourteen
locales should get one list, not fourteen consecutive build failures.

## Also in this branch

- **Delete the dead key `using-portal`** (`USING { $portal }PORTAL:`) from all 15 files. It is
  referenced only in historical spec and plan documents; its last real user was removed by
  `7923b316`, the game-source work merged as #2168. Assertion 3 would otherwise fail on day one.
- **Correct the plan document of the merged branch.** `docs/superpowers/plans/2026-08-12-source-neutral-health-wording.md`
  states the repository has no translation-coverage check. It is wrong, and it reached master —
  the correction commit missed the merge queue's snapshot for #2219. Fixing it here keeps the claim
  and its correction in one place.

## Explicitly not in scope

- **No change to `build.rs`.** Reasons above.
- **No change to any translation text.** The only translation edit is deleting one dead key.
- **No new crate.** `fluent-syntax` moves into `[dev-dependencies]` at the version already present.
- **No unused-key allowlist.** With exactly one dead key today, a suppression mechanism would be
  speculative machinery. Add it when a key genuinely needs to stay unreferenced.

## Where it lives

`refbox/src/translation_consistency.rs`, declared in `main.rs` as
`#[cfg(test)] mod translation_consistency;` so it compiles only under test and never enters the
shipped binary.

This follows the house pattern: all 532 existing refbox tests are `#[cfg(test)]` modules in `src/`.
`refbox/tests/` was considered and rejected — it currently holds only Gherkin `.feature` documents,
and refbox is bin-only (no `lib.rs`), so a `.rs` file there would be the crate's first compiled
integration test sitting oddly beside documentation.

Files are located from `env!("CARGO_MANIFEST_DIR")`, which resolves to `refbox/` under `cargo test`.

## Acceptance criteria

1. `just check` passes on the branch — the test goes green against the current translations, which
   were surveyed and are consistent today: 335 keys, no missing keys, no extras, no variable
   mismatches.
2. **The guard is proven to bite.** Each of the three assertions is deliberately broken locally, one
   at a time, and shown to fail with a readable message naming the language and key: remove a key
   from one locale; change a variable name in one locale; add an unreferenced key to en-US. A guard
   nobody has watched fail is not yet a guard.
3. `using-portal` is gone from all 15 files and the app still builds.
