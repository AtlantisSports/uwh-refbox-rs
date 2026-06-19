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

# Face 0 of this collection is "WenQuanYi Zen Hei" (unitsPerEm 1024) -- the exact
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
    # Keep the PostScript name (nameID 6) as well as family/subfamily/full/unique
    # (1/2/4/3): fontdb (the font index iced/cosmic-text uses) skips any face that
    # has no PostScript name, which makes the whole "WenQuanYi Zen Hei" family
    # unavailable and blanks all CJK text.
    options.name_IDs = [1, 2, 3, 4, 6]
    options.drop_tables = ["DSIG"]

    tt = TTFont(str(SOURCE_FONT_PATH), fontNumber=SOURCE_FONT_NUMBER)
    subsetter = ftsubset.Subsetter(options=options)
    subsetter.populate(unicodes=sorted(ord(c) for c in chars))
    subsetter.subset(tt)

    # Strip vertical-writing metrics ("vhea"/"vmtx") and TrueType hinting tables
    # ("cvt "/"fpgm"/"prep"). refbox renders only horizontal, unhinted text, so
    # these are dead weight -- and the WenQuanYi Zen Hei collection face carries
    # them while the original working subset did not. Keeping the output's table
    # set identical to that proven-good subset avoids font-engine load failures
    # (iced/cosmic-text dropped the whole face, blanking all CJK glyphs).
    for tag in ("vhea", "vmtx", "cvt ", "fpgm", "prep"):
        if tag in tt:
            del tt[tag]

    # The WenQuanYi Zen Hei collection face declares usWeightClass=500 (Medium),
    # but iced requests this family at Weight::Normal (400), and the original
    # working subset declared 400. Normalize to 400 so the font's reported weight
    # matches what the app asks for (the glyph outlines are unchanged).
    tt["OS/2"].usWeightClass = 400

    tt.save(str(OUTPUT_FONT_PATH))

    size_kb = OUTPUT_FONT_PATH.stat().st_size // 1024
    print(f"Saved to {OUTPUT_FONT_PATH} ({size_kb} KB)")


def verify_subset(chars):
    from fontTools.ttLib import TTFont

    tt = TTFont(str(OUTPUT_FONT_PATH))
    cmap = tt.getBestCmap()

    # Hard gate: every printable-ASCII glyph must be present, or the app will
    # render blanks at runtime in CJK locales. This is the regression guard for
    # the 7/8/9 bug -- fail loudly instead of shipping a broken font.
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
