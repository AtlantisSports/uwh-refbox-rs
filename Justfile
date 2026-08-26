# uwh-refbox-rs task runner
# Run `just` to see all available commands.
# Run `just check` before opening any pull request.

# Show all available commands
default:
    @just --list

# ── Validation ────────────────────────────────────────────────────────────────

# Run the full validation suite (same checks as CI) — use before any PR
check: fmt-check lint test test-vendor check-patch-applied audit

# ── Formatting ────────────────────────────────────────────────────────────────

# Format all code
fmt:
    cargo fmt --all

# Check formatting without modifying files (used by CI and pre-commit hook)
fmt-check:
    cargo fmt --all -- --check

# ── Linting ───────────────────────────────────────────────────────────────────

# Run clippy across the whole workspace (warnings are errors) — mirrors CI exactly
lint:
    cargo clippy --all -- -D warnings
    cargo clippy --all --no-default-features -- -D warnings

# ── Testing ───────────────────────────────────────────────────────────────────

# Run all workspace tests
test:
    cargo test --workspace

# Run the vendored iced_winit tests (excluded from the workspace, so `cargo test
# --workspace` does not reach them).
#
# `--features program` is load-bearing: without it the module those tests live in is
# never compiled, so `cargo test` reports "0 tests, ok" and PASSES with an empty gate.
# That is not hypothetical — it happened on this branch and went unnoticed for two
# tasks. So after running them, count them, and fail loudly if there were none.
test-vendor:
    #!/usr/bin/env bash
    set -euo pipefail
    cargo test --manifest-path vendor/iced_winit/Cargo.toml --locked --features program
    found=$(cargo test --manifest-path vendor/iced_winit/Cargo.toml --locked --features program -- --list | grep -c 'cursor::tests::' || true)
    if [ "$found" -eq 0 ]; then
        echo "ERROR: no cursor tests found in the vendored iced_winit." >&2
        echo "That run proved nothing. The touchscreen-tap regression gate is inert —" >&2
        echo "check that --features program is still being passed, and see" >&2
        echo "vendor/iced_winit/VENDORED.md." >&2
        exit 1
    fi
    echo "Vendored iced_winit: $found cursor regression tests ran."

# Fail if the workspace is no longer building iced_winit from vendor/iced_winit.
# The `[patch.crates-io]` redirect stops applying the moment nothing in the graph asks
# for iced_winit 0.13 any more (an iced 0.14 upgrade, say). Cargo only WARNS about an
# unused patch, and `test-vendor` keeps passing because it tests the vendored crate
# standalone — so the touchscreen-tap fix would silently drop out of the shipped binary
# while every gate stayed green.
check-patch-applied:
    #!/usr/bin/env bash
    set -uo pipefail
    out=$(cargo tree --locked --invert iced_winit 2>&1)
    if ! grep -q 'vendor/iced_winit' <<<"$out"; then
        echo "ERROR: the workspace is NOT building iced_winit from vendor/iced_winit." >&2
        echo "The [patch.crates-io] redirect in Cargo.toml has stopped applying, so the" >&2
        echo "touchscreen-tap fix is NOT in the binary — taps will be dropped on the Pi." >&2
        echo "Most likely cause: iced was upgraded, so nothing depends on iced_winit 0.13." >&2
        echo "See vendor/iced_winit/VENDORED.md before changing anything." >&2
        echo "" >&2
        echo "cargo tree --locked --invert iced_winit said:" >&2
        echo "$out" >&2
        exit 1
    fi
    echo "iced_winit is built from vendor/iced_winit — the touchscreen-tap fix is in the binary."

# ── Security ──────────────────────────────────────────────────────────────────

# Run security audit (matching CI ignore list)
# RUSTSEC-2024-0384: instant - unmaintained (no fix available)
# RUSTSEC-2024-0388: derivative - unmaintained (no fix available)
# RUSTSEC-2026-0009: time - fix (>=0.3.47) requires Rust 1.88+, above our MSRV of 1.85
#                    tracked in docs/decisions/002-time-cve-msrv.md
# RUSTSEC-2026-0194: quick-xml - reached only via build-time wayland-scanner XML parse (not exposed)
# RUSTSEC-2026-0195: quick-xml - same DoS vector, not exposed here
# RUSTSEC-2025-0052: async-std - unmaintained, transitively via iced_futures
#                    all three locked behind an out-of-scope iced/winit upgrade;
#                    tracked in docs/decisions/024-cargo-audit-iced-transitive-advisories.md
# Keep this ignore list in sync with the audit step in .github/workflows/rust.yml.
audit:
    cargo audit --ignore RUSTSEC-2024-0384 --ignore RUSTSEC-2024-0388 --ignore RUSTSEC-2026-0009 --ignore RUSTSEC-2026-0194 --ignore RUSTSEC-2026-0195 --ignore RUSTSEC-2025-0052

# ── Building ──────────────────────────────────────────────────────────────────

# Build the whole workspace in debug mode
build:
    cargo build --workspace

# Build the whole workspace in release mode
build-release:
    cargo build --workspace --release

# Cross-compile the refbox for Raspberry Pi 4/5 (requires `cross` and Docker)
build-rpi:
    cross build --release --target aarch64-unknown-linux-gnu -p refbox

# ── Embedded ──────────────────────────────────────────────────────────────────

# Check the wireless-remote embedded firmware (separate toolchain)
check-wireless:
    cd wireless-remote && cargo fmt -- --check && cargo clippy -- -D warnings

# ── Scoresheets ───────────────────────────────────────────────────────────────

# Verify the CMAS Official scoresheet still fits one A4 landscape page.
# Requires Chrome/Chromium; set SCORESHEET_BROWSER to point at a specific one.
check-cmas-sheet:
    cargo test -p schedule-processor cmas_official::tests::cmas_official_sheet_is_one_a4_landscape_page -- --ignored --nocapture

# ── Setup ─────────────────────────────────────────────────────────────────────

# Install the pre-commit hook (run once after cloning)
install-hooks:
    cp scripts/pre-commit .git/hooks/pre-commit
    chmod +x .git/hooks/pre-commit
    @echo "Pre-commit hook installed."

# Regenerate the bundled CJK font subset from the Japanese, Korean, and Chinese translation files.
# Run this any time those translations change. Requires: sudo apt-get install python3-fonttools fonts-wqy-zenhei
regen-cjk-font:
    python3 scripts/regen-cjk-font.py

# Regenerate the bundled Thai font subset from the Thai translation file.
# Run this any time the Thai translation changes. Requires: sudo apt-get install python3-fonttools
regen-thai-font:
    python3 scripts/regen-thai-font.py

# ── Layout previews ─────────────────────────────────────────────────────────────

# Regenerate the bundled front-display layout preview PNGs (shown on Display Options).
# Run this any time a layout's on-screen appearance changes, then commit the result.
# (WAYLAND_DISPLAY= forces X11 so the capture window renders correctly on WSLg.)
capture-previews:
    WAYLAND_DISPLAY= cargo run -p refbox -- --capture-previews refbox/resources/layout-previews

# Fail if the committed layout preview PNGs are out of date with the layout code.
# Regenerates into a temp dir and compares. CI runs this under a virtual display (xvfb).
check-previews:
    #!/usr/bin/env bash
    set -euo pipefail
    tmp=$(mktemp -d)
    WAYLAND_DISPLAY= cargo run -p refbox -- --capture-previews "$tmp"
    if ! diff -rq "$tmp" refbox/resources/layout-previews; then
        echo "Layout preview PNGs are stale. Run 'just capture-previews' and commit the result." >&2
        exit 1
    fi
    echo "Layout previews are up to date."
