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
import re
import shutil
import sys
from pathlib import Path

# Face 0 of this collection is "WenQuanYi Zen Hei" (unitsPerEm 1024) -- the exact
# typeface the current bundled subset was cut from, so CJK glyphs are unchanged.
SOURCE_FONT_PATH = Path("/usr/share/fonts/truetype/wqy/wqy-zenhei.ttc")
SOURCE_FONT_NUMBER = 0
OUTPUT_FONT_PATH = Path("refbox/resources/WqyZenHei-Subset.ttf")
# The subset is built here and only moved into place once it has been verified.
# Writing the tracked font first would leave a font known to be broken in the
# working tree, ready to be committed by anyone who missed the error.
STAGING_FONT_PATH = Path("/tmp/wqy-subset-staging.ttf")
TRANSLATION_FILES = [
    Path("refbox/translations/ja-JP/refbox.ftl"),
    Path("refbox/translations/ko-KR/refbox.ftl"),
    Path("refbox/translations/zh-CN/refbox.ftl"),
    # The fallback language (i18n.toml). A message missing from a translation is
    # shown in English but still drawn in the current language's font, so this
    # font has to be able to render it.
    Path("refbox/translations/en-US/refbox.ftl"),
]

# Every printable ASCII character. Force-included regardless of translations.
PRINTABLE_ASCII = {chr(cp) for cp in range(0x20, 0x7F)}

# Latin-1 Supplement, force-included for the same reason the Thai subset does
# it: refbox picks ONE font for the whole UI, so in a Japanese, Korean or
# Mandarin session this font also draws every Latin name arriving from the
# portal. Without it a team called "Cafe" with an acute accent, or "Munoz" with
# a tilde, renders with a hole in it -- and no translation file would ever have
# warned us. U+00A0 is dropped where the source font lacks it: it is a space,
# so a missing glyph is invisible anyway.
LATIN_1_SUPPLEMENT = {chr(cp) for cp in range(0xA0, 0x100)}

# UI text that is written as Rust string literals rather than translation keys,
# and so appears in no .ftl file: the language picker's entries (each language
# named in its own script, plus its "unverified" note) and the CANCEL/BACK/
# APPLY/restart labels. Cutting the subset from the .ftl files alone silently
# drops these -- 日本語 and 未验证 lost characters exactly that way.
#
# `refbox/build.rs` checks the same two files. Adding UI literals to a third
# file means adding it in both places.
UI_SOURCE_FILES = [
    Path("refbox/src/app/view_builders/shared_elements.rs"),
    Path("refbox/src/app/languages.rs"),
]

STRING_LITERAL = re.compile(r'"((?:[^"\\]|\\.)*)"')
# A `\u{65E5}` escape read literally becomes the ASCII "u{65E5}", so the real
# character would be neither cut into the subset nor reported as missing.
# `refbox/build.rs` decodes the same escape for the same reason.
UNICODE_ESCAPE = re.compile(r"\\u\{([0-9a-fA-F]+)\}")


def collect_characters():
    chars = set(PRINTABLE_ASCII) | set(LATIN_1_SUPPLEMENT)
    for path in TRANSLATION_FILES:
        if not path.exists():
            print(f"Warning: translation file not found: {path}", file=sys.stderr)
            continue
        for ch in path.read_text(encoding="utf-8"):
            if ord(ch) > 127:  # CJK and other non-ASCII glyphs from translations
                chars.add(ch)
    return chars


def collect_ui_source_characters():
    """Non-ASCII characters in string literals in the UI source files."""
    chars = set()
    for path in UI_SOURCE_FILES:
        if not path.exists():
            print(f"Warning: UI source file not found: {path}", file=sys.stderr)
            continue
        for literal in STRING_LITERAL.findall(path.read_text(encoding="utf-8")):
            decoded = UNICODE_ESCAPE.sub(lambda m: chr(int(m.group(1), 16)), literal)
            chars.update(ch for ch in decoded if ord(ch) > 127)
    return chars


def source_font_coverage():
    """Every code point the WenQuanYi source font can actually supply."""
    from fontTools.ttLib import TTFont

    return set(TTFont(str(SOURCE_FONT_PATH), fontNumber=SOURCE_FONT_NUMBER).getBestCmap())


def check_prerequisites():
    """Fail with install instructions, before anything else touches fontTools.

    These checks used to live in `generate_subset`, which no longer runs first:
    reading the source font's coverage does. On a machine missing either, that
    produced a raw traceback instead of the one line that says what to install.
    """
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


def generate_subset(chars):
    from fontTools import subset as ftsubset
    from fontTools.ttLib import TTFont

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

    tt.save(str(STAGING_FONT_PATH))

    size_kb = STAGING_FONT_PATH.stat().st_size // 1024
    print(f"Built {size_kb} KB subset, verifying before installing it...")


def verify_subset(chars):
    from fontTools.ttLib import TTFont

    tt = TTFont(str(STAGING_FONT_PATH))
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

    check_prerequisites()

    chars = collect_characters()

    # Text drawn from Rust literals rather than the .ftl files. Only what this
    # source font can supply: the picker also names Thai and Latin-script
    # languages, and those are the other bundled fonts' job.
    supported = source_font_coverage()

    # Everything in `chars` is required: build.rs errors if the shipped font
    # cannot draw it. Unlike the Thai script this one merges nothing, so a
    # character the source font lacks cannot be rescued from Roboto -- and
    # re-running, which is the remedy build.rs names, would fail the same way.
    # Say so here, with where it came from, instead of dead-ending.
    # Force-included characters the source font simply lacks are dropped rather
    # than treated as a failure: no translation ever asked for them.
    dropped = sorted(
        c for c in LATIN_1_SUPPLEMENT if ord(c) not in supported and c in chars
    )
    if dropped:
        print(
            "Force-included characters absent from the source font, left out: "
            + ", ".join(f"U+{ord(c):04X}" for c in dropped)
        )
        chars -= set(dropped)

    unsupplied = sorted(c for c in chars if ord(c) not in supported)
    if unsupplied:
        print(
            f"Error: {SOURCE_FONT_PATH.name} has no glyph for "
            + ", ".join(f"{c} (U+{ord(c):04X})" for c in unsupplied),
            file=sys.stderr,
        )
        for c in unsupplied:
            used_in = [
                str(path)
                for path in TRANSLATION_FILES
                if path.exists() and c in path.read_text(encoding="utf-8")
            ]
            print(f"  {c} is used by {', '.join(used_in) or 'the UI source'}", file=sys.stderr)
        print(
            "This script cannot supply them and re-running will not help: either the text has "
            "to change or a different source font is needed.",
            file=sys.stderr,
        )
        sys.exit(1)

    extra = collect_ui_source_characters()
    usable = {ch for ch in extra if ord(ch) in supported} - chars
    if usable:
        print(f"Adding {len(usable)} character(s) from UI source literals: {''.join(sorted(usable))}")
    for ch in sorted(extra - usable - chars):
        if ord(ch) not in supported:
            print(f"  {ch} (U+{ord(ch):04X}) is not in this source font -- another bundled font must carry it")
    chars |= usable

    generate_subset(chars)
    verify_subset(chars)
    shutil.move(str(STAGING_FONT_PATH), str(OUTPUT_FONT_PATH))
    print(f"Saved to {OUTPUT_FONT_PATH}")
    print("Done. Commit refbox/resources/WqyZenHei-Subset.ttf to apply the update.")


if __name__ == "__main__":
    main()
