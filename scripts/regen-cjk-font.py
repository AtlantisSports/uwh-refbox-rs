#!/usr/bin/env python3
"""
Regenerate the bundled CJK font subset.

Run this script any time the Japanese, Korean, or Chinese translation files
change, to ensure the bundled font contains all required characters.

Usage:
    python3 scripts/regen-cjk-font.py

Requires:
    - fonttools  (sudo apt-get install python3-fonttools)
    - The Noto Sans CJK KR source font at /tmp/NotoSansCJKkr-Regular.otf
      (downloaded automatically if not present)

Output:
    refbox/resources/NotoSansCJK-Subset.otf  (~150 KB)
"""

import os
import sys
import urllib.request
from pathlib import Path

SOURCE_FONT_URL = (
    "https://raw.githubusercontent.com/notofonts/noto-cjk/main"
    "/Sans/OTF/Korean/NotoSansCJKkr-Regular.otf"
)
SOURCE_FONT_PATH = Path("/tmp/NotoSansCJKkr-Regular.otf")
OUTPUT_FONT_PATH = Path("refbox/resources/NotoSansCJK-Subset.otf")
TRANSLATION_FILES = [
    Path("refbox/translations/ja-JP/refbox.ftl"),
    Path("refbox/translations/ko-KR/refbox.ftl"),
    Path("refbox/translations/zh-CN/refbox.ftl"),
]


def download_source_font():
    print(f"Downloading source font from GitHub (~16 MB)...")
    urllib.request.urlretrieve(SOURCE_FONT_URL, SOURCE_FONT_PATH)
    print(f"Saved to {SOURCE_FONT_PATH}")


def collect_characters():
    chars = set()
    for path in TRANSLATION_FILES:
        if not path.exists():
            print(f"Warning: translation file not found: {path}", file=sys.stderr)
            continue
        for ch in path.read_text(encoding="utf-8"):
            if ord(ch) > 127:
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

    print(f"Subsetting {len(chars)} unique characters from source font...")
    options = ftsubset.Options()
    options.layout_features = []
    options.name_IDs = [1, 2, 4]
    options.drop_tables = ["DSIG"]

    tt = TTFont(str(SOURCE_FONT_PATH))
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
    missing = [c for c in chars if ord(c) not in cmap]
    if missing:
        print(
            f"Warning: {len(missing)} characters missing from subset font:",
            file=sys.stderr,
        )
        for c in missing[:20]:
            print(f"  U+{ord(c):04X} ({c})", file=sys.stderr)
    else:
        print(f"Verified: all {len(chars)} characters present in subset font.")


def main():
    # Change to repo root if running from elsewhere
    repo_root = Path(__file__).parent.parent
    os.chdir(repo_root)

    if not SOURCE_FONT_PATH.exists():
        download_source_font()
    else:
        print(f"Using cached source font at {SOURCE_FONT_PATH}")

    chars = collect_characters()
    if not chars:
        print("No characters found — check that translation files exist.")
        sys.exit(1)

    generate_subset(chars)
    verify_subset(chars)
    print("Done. Commit refbox/resources/NotoSansCJK-Subset.otf to apply the update.")


if __name__ == "__main__":
    main()
