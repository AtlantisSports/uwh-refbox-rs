# uwh-refbox-rs task runner
# Run `just` to see all available commands.
# Run `just check` before opening any pull request.

# Show all available commands
default:
    @just --list

# ── Validation ────────────────────────────────────────────────────────────────

# Run the full validation suite (same checks as CI) — use before any PR
check: fmt-check lint test audit

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

# ── Security ──────────────────────────────────────────────────────────────────

# Run security audit (matching CI ignore list)
# RUSTSEC-2024-0384: instant - unmaintained (no fix available)
# RUSTSEC-2024-0388: derivative - unmaintained (no fix available)
# RUSTSEC-2026-0009: time - fix (>=0.3.47) requires Rust 1.88+, above our MSRV of 1.85
#                    tracked in docs/decisions/002-time-cve-msrv.md
audit:
    cargo audit --ignore RUSTSEC-2024-0384 --ignore RUSTSEC-2024-0388 --ignore RUSTSEC-2026-0009

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

# ── Setup ─────────────────────────────────────────────────────────────────────

# Install the pre-commit hook (run once after cloning)
install-hooks:
    cp scripts/pre-commit .git/hooks/pre-commit
    chmod +x .git/hooks/pre-commit
    @echo "Pre-commit hook installed."

# Regenerate the bundled CJK font subset from the Japanese, Korean, and Chinese translation files.
# Run this any time those translations change. Requires: sudo apt-get install python3-fonttools
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
