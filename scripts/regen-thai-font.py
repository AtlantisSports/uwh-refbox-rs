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
TRANSLATION_FILES = [
    Path("refbox/translations/th-TH/refbox.ftl"),
]

# Printable Basic Latin (U+0020–U+007E) and Latin-1 Supplement (U+00A0–U+00FF).
# Included so the game clock digits, Latin-script language names, and other
# ASCII/Western text renders correctly when Thai is the default font — no
# fallback needed. Latin-1 Supplement covers the × sign (U+00D7) used in the
# schedule-spacing formula in the Thai translation file.
BASIC_LATIN = set(chr(c) for c in range(0x20, 0x7F)) | set(chr(c) for c in range(0xA0, 0x100))


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


def make_latin_subset():
    """Subset Roboto to Basic Latin characters."""
    from fontTools import subset as ftsubset
    from fontTools.ttLib import TTFont

    latin_subset_path = Path("/tmp/latin-only.ttf")
    print(f"Subsetting Basic Latin ({len(BASIC_LATIN)} chars) from Roboto...")

    options = ftsubset.Options()
    options.name_IDs = [1, 2, 4, 6]
    options.drop_tables = ["DSIG"]
    options.hinting = False
    options.desubroutinize = True
    # Latin doesn't need complex shaping features
    options.layout_features = []

    tt = TTFont(str(ROBOTO_FONT_PATH))
    subsetter = ftsubset.Subsetter(options=options)
    subsetter.populate(unicodes=sorted(ord(c) for c in BASIC_LATIN))
    subsetter.subset(tt)
    tt.save(str(latin_subset_path))

    size_kb = latin_subset_path.stat().st_size // 1024
    print(f"  Latin subset: {size_kb} KB at {latin_subset_path}")
    return latin_subset_path


def merge_fonts(thai_path, latin_path):
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

    merged.save(str(OUTPUT_FONT_PATH))
    size_kb = OUTPUT_FONT_PATH.stat().st_size // 1024
    print(f"  Merged font: {size_kb} KB at {OUTPUT_FONT_PATH}")


def verify(thai_chars):
    from fontTools.ttLib import TTFont

    tt = TTFont(str(OUTPUT_FONT_PATH))
    cmap = tt.getBestCmap()

    # Check Thai characters
    missing_thai = [c for c in thai_chars if ord(c) not in cmap]
    if missing_thai:
        print(
            f"Warning: {len(missing_thai)} Thai characters missing from output font:",
            file=sys.stderr,
        )
        for c in missing_thai[:20]:
            print(f"  U+{ord(c):04X} ({c})", file=sys.stderr)
    else:
        print(f"Verified: all {len(thai_chars)} Thai characters present.")

    # Check digits
    missing_digits = [c for c in "0123456789" if ord(c) not in cmap]
    if missing_digits:
        print(
            f"Warning: digits missing from output font: {''.join(missing_digits)}",
            file=sys.stderr,
        )
    else:
        print("Verified: all digits 0–9 present.")

    # Report font name as registered
    for record in tt["name"].names:
        if record.nameID == 1 and record.platformID == 3:
            print(f"Font family name: {record.toUnicode()!r}")
            break


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

    thai_path = make_thai_subset(thai_chars)
    latin_path = make_latin_subset()
    merge_fonts(thai_path, latin_path)
    verify(thai_chars)
    print("Done. Commit refbox/resources/NotoSansThai-Subset.ttf to apply the update.")


if __name__ == "__main__":
    main()
