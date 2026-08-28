#!/usr/bin/env python3
"""
Regenerate the bundled Thai font subset.

Creates a merged font containing:
  - Thai glyphs from Noto Sans Thai (with full OpenType layout features preserved,
    since Thai requires GSUB/GPOS for correct vowel and tone-mark positioning)
  - Basic Latin glyphs from Roboto (digits, letters, punctuation needed for the
    game clock display and Latin-script UI elements)

Why merge rather than rely on font fallback?
  cosmic-text (the text renderer used by iced on Linux) does not reliably fall
  back from a Thai default font to Roboto for Basic Latin characters like digits.
  This was confirmed by testing: digits showed as boxes in Thai mode even though
  Roboto was loaded. The merged font sidesteps this by ensuring the "Noto Sans Thai"
  font family already contains every glyph needed, so no fallback is required.

Usage:
    python3 scripts/regen-thai-font.py

Requires:
    - fonttools  (sudo apt-get install python3-fonttools)
    - The Noto Sans Thai source font at /tmp/NotoSansThai-Regular.ttf
      (downloaded automatically if not present)
    - refbox/resources/Roboto-Medium.ttf  (already in the repo)

Output:
    refbox/resources/NotoSansThai-Subset.ttf  (~60-80 KB)
"""

import os
import re
import shutil
import sys
import urllib.request
from pathlib import Path

SOURCE_FONT_URL = (
    "https://github.com/googlefonts/noto-fonts/raw/main"
    "/hinted/ttf/NotoSansThai/NotoSansThai-Regular.ttf"
)
SOURCE_FONT_PATH = Path("/tmp/NotoSansThai-Regular.ttf")
ROBOTO_FONT_PATH = Path("refbox/resources/Roboto-Medium.ttf")
OUTPUT_FONT_PATH = Path("refbox/resources/NotoSansThai-Subset.ttf")
# Built here and moved into place only once verified, so a font known to be
# incomplete is never left in the working tree for someone to commit.
STAGING_FONT_PATH = Path("/tmp/noto-thai-subset-staging.ttf")
TRANSLATION_FILES = [
    Path("refbox/translations/th-TH/refbox.ftl"),
    # The fallback language (i18n.toml). A message missing from a translation is
    # shown in English but still drawn in the current language's font, so this
    # font has to be able to render it.
    Path("refbox/translations/en-US/refbox.ftl"),
]

# UI text written as Rust string literals rather than translation keys: the
# language picker's entries and the CANCEL/BACK/APPLY labels. Cutting from the
# .ftl files alone silently drops them. `refbox/build.rs` checks the same two
# files; adding UI literals to a third means adding it in both places.
UI_SOURCE_FILES = [
    Path("refbox/src/app/view_builders/shared_elements.rs"),
    Path("refbox/src/app/languages.rs"),
]

STRING_LITERAL = re.compile(r'"((?:[^"\\]|\\.)*)"')
# A `\u{65E5}` escape read literally becomes the ASCII "u{65E5}", so the real
# character would be neither cut into the subset nor reported as missing.
# `refbox/build.rs` decodes the same escape for the same reason.
UNICODE_ESCAPE = re.compile(r"\\u\{([0-9a-fA-F]+)\}")

# Printable Basic Latin (U+0020–U+007E) and Latin-1 Supplement (U+00A0–U+00FF).
# Included so the game clock digits, Latin-script language names, and other
# ASCII/Western text renders correctly when Thai is the default font — no
# fallback needed. Latin-1 Supplement covers the × sign (U+00D7) used in the
# schedule-spacing formula in the Thai translation file.
BASIC_LATIN = set(chr(c) for c in range(0x20, 0x7F)) | set(chr(c) for c in range(0xA0, 0x100))


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


def download_source_font():
    print("Downloading Noto Sans Thai from GitHub (~200 KB)...")
    urllib.request.urlretrieve(SOURCE_FONT_URL, SOURCE_FONT_PATH)
    print(f"Saved to {SOURCE_FONT_PATH}")


def collect_thai_characters():
    chars = set()
    for path in TRANSLATION_FILES:
        if not path.exists():
            print(f"Warning: translation file not found: {path}", file=sys.stderr)
            continue
        for ch in path.read_text(encoding="utf-8"):
            if ord(ch) > 127:
                chars.add(ch)
    return chars


def make_thai_subset(thai_chars):
    """Subset Noto Sans Thai to Thai characters only, preserving all layout features."""
    from fontTools import subset as ftsubset
    from fontTools.ttLib import TTFont

    thai_subset_path = Path("/tmp/thai-only.ttf")
    print(f"Subsetting {len(thai_chars)} Thai characters from source font...")

    options = ftsubset.Options()
    # Keep ALL layout features — Thai requires GSUB/GPOS for correct vowel
    # and tone-mark positioning. Do NOT restrict layout_features here.
    options.name_IDs = [1, 2, 4, 6]
    options.drop_tables = ["DSIG"]
    options.hinting = False
    options.desubroutinize = True

    tt = TTFont(str(SOURCE_FONT_PATH))
    subsetter = ftsubset.Subsetter(options=options)
    subsetter.populate(unicodes=sorted(ord(c) for c in thai_chars))
    subsetter.subset(tt)
    tt.save(str(thai_subset_path))

    size_kb = thai_subset_path.stat().st_size // 1024
    print(f"  Thai subset: {size_kb} KB at {thai_subset_path}")
    return thai_subset_path


def make_latin_subset(extra_chars=frozenset()):
    """Subset Roboto to Basic Latin, plus any character Noto Sans Thai lacks.

    Noto Sans Thai carries no general punctuation, so a Thai string using an em
    dash or an ellipsis would lose it silently. Those characters are taken from
    Roboto instead, which is already being merged in for the Latin range."""
    from fontTools import subset as ftsubset
    from fontTools.ttLib import TTFont

    latin_subset_path = Path("/tmp/latin-only.ttf")
    wanted = BASIC_LATIN | set(extra_chars)
    print(f"Subsetting Basic Latin ({len(wanted)} chars) from Roboto...")

    options = ftsubset.Options()
    options.name_IDs = [1, 2, 4, 6]
    options.drop_tables = ["DSIG"]
    options.hinting = False
    options.desubroutinize = True
    # Latin doesn't need complex shaping features
    options.layout_features = []

    tt = TTFont(str(ROBOTO_FONT_PATH))
    subsetter = ftsubset.Subsetter(options=options)
    subsetter.populate(unicodes=sorted(ord(c) for c in wanted))
    subsetter.subset(tt)
    tt.save(str(latin_subset_path))

    size_kb = latin_subset_path.stat().st_size // 1024
    print(f"  Latin subset: {size_kb} KB at {latin_subset_path}")
    return latin_subset_path


def merge_fonts(thai_path, latin_path, out_path):
    """Merge Thai and Latin subsets into a single font, naming it 'Noto Sans Thai'."""
    from fontTools.merge import Merger
    from fontTools.ttLib import TTFont
    from fontTools.ttLib.scaleUpem import scale_upem

    print("Merging Thai and Latin subsets...")

    # Both fonts must share the same units-per-em before merging.
    # Thai font uses 1000 UPM; Roboto uses 2048 UPM. Scale Latin down to 1000.
    tt_thai = TTFont(str(thai_path))
    thai_upem = tt_thai["head"].unitsPerEm

    scaled_latin_path = Path("/tmp/latin-scaled.ttf")
    tt_latin = TTFont(str(latin_path))
    if tt_latin["head"].unitsPerEm != thai_upem:
        print(
            f"  Scaling Latin subset from {tt_latin['head'].unitsPerEm} UPM "
            f"→ {thai_upem} UPM to match Thai font..."
        )
        scale_upem(tt_latin, thai_upem)
    tt_latin.save(str(scaled_latin_path))

    merger = Merger()
    # Thai font is listed first so its metrics (UPM, ascender, descender) dominate
    merged = merger.merge([str(thai_path), str(scaled_latin_path)])

    # Ensure the merged font's name table still says "Noto Sans Thai" so that
    # iced can locate it by Family::Name("Noto Sans Thai") at runtime.
    name_table = merged["name"]
    for record in name_table.names:
        if record.nameID == 1:  # Font Family name
            record.string = "Noto Sans Thai".encode("utf-16-be")
            record.platformID = 3
            record.platEncID = 1
            record.langID = 0x0409
        elif record.nameID == 4:  # Full name
            record.string = "Noto Sans Thai Regular".encode("utf-16-be")
            record.platformID = 3
            record.platEncID = 1
            record.langID = 0x0409

    # Rebuild the name table with only the corrected entries
    merged["name"].names = [
        r for r in merged["name"].names if r.platformID == 3
    ]

    merged.save(str(out_path))
    size_kb = out_path.stat().st_size // 1024
    print(f"  Merged font: {size_kb} KB, verifying before installing it...")


def verify(thai_chars, font_path):
    from fontTools.ttLib import TTFont

    tt = TTFont(str(font_path))
    cmap = tt.getBestCmap()

    ok = True

    # Check Thai characters
    missing_thai = [c for c in thai_chars if ord(c) not in cmap]
    if missing_thai:
        print(
            f"Warning: {len(missing_thai)} Thai characters missing from output font:",
            file=sys.stderr,
        )
        for c in missing_thai[:20]:
            print(f"  U+{ord(c):04X} ({c})", file=sys.stderr)
        ok = False
    else:
        print(f"Verified: all {len(thai_chars)} Thai characters present.")

    # Check digits
    missing_digits = [c for c in "0123456789" if ord(c) not in cmap]
    if missing_digits:
        print(
            f"Warning: digits missing from output font: {''.join(missing_digits)}",
            file=sys.stderr,
        )
        ok = False
    else:
        print("Verified: all digits 0–9 present.")

    # Report font name as registered
    for record in tt["name"].names:
        if record.nameID == 1 and record.platformID == 3:
            print(f"Font family name: {record.toUnicode()!r}")
            break

    return ok


def main():
    repo_root = Path(__file__).parent.parent
    os.chdir(repo_root)

    try:
        from fontTools import subset as _  # noqa: F401
    except ImportError:
        print(
            "Error: fonttools not installed. Run:\n"
            "  sudo apt-get install python3-fonttools",
            file=sys.stderr,
        )
        sys.exit(1)

    if not SOURCE_FONT_PATH.exists():
        download_source_font()
    else:
        print(f"Using cached source font at {SOURCE_FONT_PATH}")

    if not ROBOTO_FONT_PATH.exists():
        print(f"Error: Roboto font not found at {ROBOTO_FONT_PATH}", file=sys.stderr)
        sys.exit(1)

    thai_chars = collect_thai_characters()
    if not thai_chars:
        print("No Thai characters found — check that translation files exist.")
        sys.exit(1)

    from fontTools.ttLib import TTFont

    source_cmap = TTFont(str(SOURCE_FONT_PATH)).getBestCmap()
    roboto_cmap = TTFont(str(ROBOTO_FONT_PATH)).getBestCmap()

    # Text drawn from Rust literals rather than the .ftl files. Only what one of
    # these two source fonts can supply: the picker also names the CJK
    # languages, and those are the CJK subset's job. Best-effort, so these are
    # not part of what `verify` insists on.
    extra = collect_ui_source_characters()
    usable = {
        c for c in extra if ord(c) in source_cmap or ord(c) in roboto_cmap
    } - thai_chars
    if usable:
        print(
            f"Adding {len(usable)} character(s) from UI source literals: "
            f"{''.join(sorted(usable))}"
        )
    # Neither source font has these. Saying so matters: build.rs may require one
    # of them, and its remedy is to re-run this script -- which would drop it
    # again, silently, forever. The CJK script guards the same case.
    unsuppliable = sorted(extra - usable - thai_chars)
    if unsuppliable:
        print(
            f"  {len(unsuppliable)} character(s) are in neither Noto Sans Thai nor Roboto and "
            f"were left out: {''.join(unsuppliable)}. Another bundled font must carry them; if "
            f"this font is asked for one, the text or a source font has to change.",
            file=sys.stderr,
        )

    wanted = thai_chars | usable
    from_thai = {c for c in wanted if ord(c) in source_cmap}
    from_roboto = wanted - from_thai
    if from_roboto:
        print(
            f"  {len(from_roboto)} character(s) absent from Noto Sans Thai, "
            f"taking them from Roboto: {''.join(sorted(from_roboto))}"
        )

    thai_path = make_thai_subset(from_thai)
    latin_path = make_latin_subset(from_roboto)
    merge_fonts(thai_path, latin_path, STAGING_FONT_PATH)
    # Verified against everything we believed a source font could supply, not
    # just the .ftl characters: a UI literal silently lost in the merge would
    # otherwise install clean and leave build.rs demanding a character while
    # telling you to re-run this script, which would lose it again.
    if not verify(thai_chars | usable, STAGING_FONT_PATH):
        print(
            "Font NOT updated cleanly: the output is missing characters the "
            "translation uses. See the warnings above.",
            file=sys.stderr,
        )
        sys.exit(1)
    shutil.move(str(STAGING_FONT_PATH), str(OUTPUT_FONT_PATH))
    print(f"Saved to {OUTPUT_FONT_PATH}")
    print("Done. Commit refbox/resources/NotoSansThai-Subset.ttf to apply the update.")


if __name__ == "__main__":
    main()
