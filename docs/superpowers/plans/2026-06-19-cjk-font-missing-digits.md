# CJK Font Missing Digits (7/8/9) — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** In CJK locales (Korean/Japanese/Chinese), the refbox clock and scores must render the digits 7, 8, and 9 (and every other basic-Latin character) instead of blank.

**Architecture:** The bug is purely in the bundled CJK font, not in clock/score logic. The shipped `refbox/resources/WqyZenHei-Subset.ttf` is a character subset built from the literal characters in the CJK translation files; 7/8/9 never appear literally in those translations, so they were never subset in, and the scoreboard PC's software renderer does not fall back to Roboto. Fix = regenerate the subset from the exact same source typeface (WenQuanYi Zen Hei, face 0 of the system `wqy-zenhei.ttc`) but always include the full printable-ASCII range, and repair the regeneration script (which is currently out of sync with the shipped font) so this can't silently recur.

**Tech Stack:** Python 3 + `fonttools` (already a documented dependency of the regen scripts); `just`; Rust/iced 0.13 refbox (no Rust changes needed).

## Global Constraints

- Work against latest `origin/master` (commit `babb09d0`) in an isolated worktree.
- Branch/scope: `fix/refbox/cjk-font-missing-digits` — refbox crate only (resources + tooling).
- The rebuilt CJK font MUST be visually identical for CJK text: source = `WenQuanYi Zen Hei` face **0** of `/usr/share/fonts/truetype/wqy/wqy-zenhei.ttc` (verified: unitsPerEm 1024, glyph `'0'` bounds `(41,-10,573,713)` — identical to the current subset).
- Output file MUST stay `refbox/resources/WqyZenHei-Subset.ttf` (the exact name `main.rs` loads via `include_bytes!`).
- No new Rust dependencies (rust.md: deps need discussion). Regression guard lives in the Python verify step.
- No `unwrap`/`expect`/`unsafe` changes — no Rust code changes at all.
- Thai font (`NotoSansThai-Subset.ttf`) is NOT affected (already has all ten digits) — out of scope; `regen-thai-font.py` shares the same latent flaw and is noted as a follow-up only.

---

### Task 1: Rewrite `scripts/regen-cjk-font.py` to produce the real font with full Latin coverage

**Files:**
- Modify (rewrite): `scripts/regen-cjk-font.py`
- Modify (comment only): `Justfile:74-78` (add `fonts-wqy-zenhei` to the "Requires:" comment for `regen-cjk-font`)

**What & why:** The committed script downloads Noto CJK and writes `NotoSansCJK-Subset.otf`, which is NOT the file the app loads, and it filters out all ASCII (`ord(ch) > 127`). Replace it so it subsets the actual shipped typeface and always includes printable ASCII. The verify step becomes a hard gate.

- [ ] **Step 1: Replace the script body** with:

```python
#!/usr/bin/env python3
"""
Regenerate the bundled CJK font subset (refbox/resources/WqyZenHei-Subset.ttf).

Run this any time the Japanese, Korean, or Chinese translation files change, to
ensure the bundled font contains every character the UI can display.

The subset ALWAYS includes the full printable-ASCII range (U+0020..U+007E) in
addition to the CJK characters used in the translations. Digits, Latin letters,
and punctuation are generated at runtime (clock, scores, team/player numbers)
and frequently do NOT appear literally in the CJK translation text, so they must
be force-included or they render blank in CJK locales (the scoreboard PC's
software renderer does not fall back to Roboto for missing glyphs).

Usage:
    python3 scripts/regen-cjk-font.py      (or: just regen-cjk-font)

Requires:
    - fonttools                 (sudo apt-get install python3-fonttools)
    - WenQuanYi Zen Hei source  (sudo apt-get install fonts-wqy-zenhei)

Output:
    refbox/resources/WqyZenHei-Subset.ttf
"""

import os
import sys
from pathlib import Path

# Face 0 of this collection is "WenQuanYi Zen Hei" (unitsPerEm 1024) — the exact
# typeface the current bundled subset was cut from, so CJK glyphs are unchanged.
SOURCE_FONT_PATH = Path("/usr/share/fonts/truetype/wqy/wqy-zenhei.ttc")
SOURCE_FONT_NUMBER = 0
OUTPUT_FONT_PATH = Path("refbox/resources/WqyZenHei-Subset.ttf")
TRANSLATION_FILES = [
    Path("refbox/translations/ja-JP/refbox.ftl"),
    Path("refbox/translations/ko-KR/refbox.ftl"),
    Path("refbox/translations/zh-CN/refbox.ftl"),
]

# Every printable ASCII character. Force-included regardless of translations.
PRINTABLE_ASCII = {chr(cp) for cp in range(0x20, 0x7F)}


def collect_characters():
    chars = set(PRINTABLE_ASCII)
    for path in TRANSLATION_FILES:
        if not path.exists():
            print(f"Warning: translation file not found: {path}", file=sys.stderr)
            continue
        for ch in path.read_text(encoding="utf-8"):
            if ord(ch) > 127:  # CJK and other non-ASCII glyphs from translations
                chars.add(ch)
    return chars


def generate_subset(chars):
    try:
        from fontTools import subset as ftsubset
        from fontTools.ttLib import TTFont
    except ImportError:
        print(
            "Error: fonttools not installed. Run:\n"
            "  sudo apt-get install python3-fonttools",
            file=sys.stderr,
        )
        sys.exit(1)

    if not SOURCE_FONT_PATH.exists():
        print(
            f"Error: source font not found at {SOURCE_FONT_PATH}. Run:\n"
            "  sudo apt-get install fonts-wqy-zenhei",
            file=sys.stderr,
        )
        sys.exit(1)

    print(f"Subsetting {len(chars)} unique characters from {SOURCE_FONT_PATH}...")
    options = ftsubset.Options()
    options.layout_features = []
    options.name_IDs = [1, 2, 4]
    options.drop_tables = ["DSIG"]

    tt = TTFont(str(SOURCE_FONT_PATH), fontNumber=SOURCE_FONT_NUMBER)
    subsetter = ftsubset.Subsetter(options=options)
    subsetter.populate(unicodes=sorted(ord(c) for c in chars))
    subsetter.subset(tt)
    tt.save(str(OUTPUT_FONT_PATH))

    size_kb = OUTPUT_FONT_PATH.stat().st_size // 1024
    print(f"Saved to {OUTPUT_FONT_PATH} ({size_kb} KB)")


def verify_subset(chars):
    from fontTools.ttLib import TTFont

    tt = TTFont(str(OUTPUT_FONT_PATH))
    cmap = tt.getBestCmap()

    # Hard gate: every printable-ASCII glyph must be present, or the app will
    # render blanks at runtime in CJK locales. This is the regression guard for
    # the 7/8/9 bug — fail loudly instead of shipping a broken font.
    missing_ascii = sorted(c for c in PRINTABLE_ASCII if ord(c) not in cmap)
    if missing_ascii:
        print(
            "ERROR: printable-ASCII glyphs missing from subset: "
            + " ".join(f"U+{ord(c):04X}({c})" for c in missing_ascii),
            file=sys.stderr,
        )
        sys.exit(1)

    missing = sorted(c for c in chars if ord(c) not in cmap)
    if missing:
        print(f"ERROR: {len(missing)} requested characters missing:", file=sys.stderr)
        for c in missing[:20]:
            print(f"  U+{ord(c):04X} ({c})", file=sys.stderr)
        sys.exit(1)

    print(f"Verified: all {len(chars)} characters present (incl. full ASCII 0-9, A-Z, a-z).")


def main():
    repo_root = Path(__file__).parent.parent
    os.chdir(repo_root)

    chars = collect_characters()
    generate_subset(chars)
    verify_subset(chars)
    print("Done. Commit refbox/resources/WqyZenHei-Subset.ttf to apply the update.")


if __name__ == "__main__":
    main()
```

- [ ] **Step 2: Update the Justfile "Requires:" comment** for `regen-cjk-font` so it mentions the source font:

Change the comment line above `regen-cjk-font:` to:
```
# Run this any time those translations change. Requires: sudo apt-get install python3-fonttools fonts-wqy-zenhei
```

- [ ] **Step 3: Run the regen recipe**

Run: `just regen-cjk-font`
Expected output ends with: `Verified: all <N> characters present (incl. full ASCII 0-9, A-Z, a-z).` and `Done.`

- [ ] **Step 4: Confirm the rebuilt font has 7/8/9 and unchanged CJK glyph `'0'`**

Run:
```bash
python3 - <<'PY'
from fontTools.ttLib import TTFont
from fontTools.pens.boundsPen import BoundsPen
tt = TTFont("refbox/resources/WqyZenHei-Subset.ttf"); cmap = tt.getBestCmap(); gs = tt.getGlyphSet()
print("7/8/9 present:", all(cp in cmap for cp in (0x37,0x38,0x39)))
print("A-Z present:", all(cp in cmap for cp in range(0x41,0x5B)))
bp = BoundsPen(gs); gs[cmap[0x30]].draw(bp)
print("'0' bounds:", bp.bounds, "(expect (41, -10, 573, 713))")
print("CJK sample 검 present:", ord("검") in cmap, " 다 present:", ord("다") in cmap)
PY
```
Expected: `7/8/9 present: True`, `A-Z present: True`, `'0' bounds: (41, -10, 573, 713)`, CJK samples `True`.

- [ ] **Step 5: Commit**

```bash
git add scripts/regen-cjk-font.py Justfile refbox/resources/WqyZenHei-Subset.ttf
git commit -m "fix(refbox): include full ASCII in CJK font subset so 7/8/9 render"
```

---

### Task 2: In-app + suite verification

**Files:** none (verification only).

- [ ] **Step 1: Build refbox** (memory: clippy builds a *test* binary, not `target/debug/refbox`)

Run: `cargo build -p refbox`
Expected: builds clean.

- [ ] **Step 2: Run the full suite**

Run: `just check`
Expected: fmt, lint, tests, audit all clean (no Rust changed, so this should pass as on master).

- [ ] **Step 3: On-screen verification (operator-visible)**

Launch refbox in Korean (config language = Korean), with `WAYLAND_DISPLAY=` and sandbox disabled. Observe:
- "다음 경기" countdown showing a value containing 7/8/9 (e.g. `14:58`, `14:37`) renders all digits.
- A score containing 7/8/9 renders fully.
- CJK labels (e.g. "다음 경기", "심판 타임아웃") look unchanged.
Compare against the LED Panel Simulator (already correct) — they should now match.

---

## Out of scope / follow-ups

- `scripts/regen-thai-font.py` has the same latent flaw (works only because the Thai translations happen to include all digits). Thai is currently correct, so not touched here — recommend a separate follow-up to apply the same full-ASCII guard.
- Portal-supplied team/player names containing non-ASCII non-CJK characters (e.g. accented Latin) are a pre-existing limitation unrelated to this bug.

## Deviations

The plan's "regenerate with full ASCII" step was necessary but **not sufficient** — it
fixed 7/8/9 but introduced a regression where ALL CJK rendered as tofu. Root-causing that
(via a throwaway fontdb-0.16.2 + ttf-parser-0.20 harness that replicates iced's exact font
loader) revealed two further requirements the generated subset must satisfy:

1. **Keep the PostScript name (nameID 6).** `fontdb` (the index iced/cosmic-text uses) skips
   any face with no PostScript name, so the whole "WenQuanYi Zen Hei" family became
   unavailable. `ttf-parser` parsed the font fine, which is why ordinary tools didn't flag it.
   Fix: `options.name_IDs = [1, 2, 3, 4, 6]` (was `[1, 2, 4]`).
2. **Normalize usWeightClass to 400.** The WenQuanYi *collection* face declares 500 (Medium);
   iced requests `Weight::Normal` (400) and the original working subset declared 400. Set
   `tt["OS/2"].usWeightClass = 400` after subsetting.

Also: stripped vertical-writing (`vhea`/`vmtx`) and hinting (`cvt `/`fpgm`/`prep`) tables so the
output's table set matches the proven-good original (refbox renders horizontal, unhinted text).

The script now produces a font that registers in fontdb identically to the original
(`families=["WenQuanYi Zen Hei"] post="WenQuanYiZenHei" weight=400`, query MATCHED), verified
before rebuilding refbox. On-screen verification in Korean confirmed CJK + 7/8/9 both render.
