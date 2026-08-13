# An unwritable config dir yields a green portal indicator and memory-only results

**Found:** 2026-08-13, reviewing PR #2367 (degraded mode no write target).
**Status:** pre-existing on master, NOT introduced by that PR, and not fixed by it.
**Reachability:** plausible on the field Pi, which runs a read-only overlayfs.

## What happens

`RefBoxApp::new` creates the config dir best-effort and discards the result:

```rust
std::fs::create_dir_all(&config_dir).ok();   // app/mod.rs, ~2392
```

Then `PortalManager::new` → `QueueStore::open(config_dir)` → `queue::load_or_empty(dir)`, which
returns **success without touching the disk** when the queue file does not exist yet:

```rust
if !path.exists() {
    return Ok(QueueFile::empty());          // queue.rs:88
}
```

So on a first run in a directory that cannot be written:

1. `open` succeeds and hands back a write target.
2. `PortalManager::new` returns `Ok`, so `RefBoxApp::new` never tries its second-choice directory
   (`std::env::temp_dir()`) and never reaches degraded mode.
3. `startup_problem` stays `false`, so the portal indicator is **green** and every control is live.
4. Every `enqueue_game_end` then fails inside `write_atomic` at `fs::File::create(tmp)`, logging one
   `error!` line and leaving the result in memory only.

Result: a whole tournament's results are memory-only and lost on restart, with no visible signal —
the operator sees a healthy green portal the entire time.

## Why PR #2367 does not fix it

That PR's guarantee is narrow and exact: *never write where a read failed*. Holding a `QueueStore`
proves the directory was read, not that it can be written. When there is no queue file there is
nothing to read, so nothing is proven at all. The doc comment on `QueueStore::open` says so
explicitly rather than overstating it.

## Possible fixes, none chosen

- Have `open` verify writability — e.g. write and remove a probe file, or attempt the tmp-file
  create that `write_atomic` uses. Turns this into an `Err`, so `RefBoxApp::new` falls back to the
  temp dir exactly as it does for an unreadable queue. Cost: an extra file create per launch, and it
  must not itself destroy anything.
- Stop discarding `create_dir_all`'s result at `app/mod.rs:2392` and treat failure as a reason to
  use the second-choice directory.
- Surface a repeated write failure in the indicator instead of a single `error!` line, so the
  operator is not shown green while nothing is being saved.

The first two are the honest fixes; the third is worth having regardless, since a write that starts
failing mid-tournament has the same invisible outcome.

## Related

- `docs/superpowers/specs/2026-08-13-degraded-no-write-target-design.md`
- `reference_pi_deployment_launch` in memory — the field Pi's read-only overlayfs
