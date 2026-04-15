#!/usr/bin/env python3
"""
Regenerate the bundled CJK and Thai font subsets.

Run this script any time the Japanese, Korean, Chinese, or Thai translation
files change, to ensure the bundled fonts contain all required characters.

Usage:
    python3 scripts/regen-cjk-font.py

Requires:
    - fonttools  (sudo apt-get install python3-fonttools)
    - Source fonts are downloaded automatically to /tmp if not already present.

Output:
    refbox/resources/NotoSansCJK-Subset.otf   (~150 KB)  Japanese, Korean, Chinese
    refbox/resources/NotoSansThai-Subset.ttf  (~15 KB)   Thai
"""

import os
import sys
import urllib.request
from pathlib import Path

CJK_SOURCE_URL = (
    "https://raw.githubusercontent.com/notofonts/noto-cjk/main"
    "/Sans/OTF/Korean/NotoSansCJKkr-Regular.otf"
)
CJK_SOURCE_PATH = Path("/tmp/NotoSansCJKkr-Regular.otf")
CJK_OUTPUT_PATH = Path("refbox/resources/NotoSansCJK-Subset.otf")
CJK_TRANSLATION_FILES = [
    Path("refbox/translations/ja-JP/refbox.ftl"),
    Path("refbox/translations/ko-KR/refbox.ftl"),
    Path("refbox/translations/zh-CN/refbox.ftl"),
]

THAI_SOURCE_URL = (
    "https://raw.githubusercontent.com/googlefonts/noto-fonts/main"
    "/hinted/ttf/NotoSansThai/NotoSansThai-Regular.ttf"
)
THAI_SOURCE_PATH = Path("/tmp/NotoSansThai-Regular.ttf")
THAI_OUTPUT_PATH = Path("refbox/resources/NotoSansThai-Subset.ttf")
THAI_TRANSLATION_FILES = [
    Path("refbox/translations/th-TH/refbox.ftl"),
]


def ensure_fonttools():
    try:
        from fontTools import subset as ftsubset  # noqa: F401
        from fontTools.ttLib import TTFont  # noqa: F401
    except ImportError:
        print(
            "Error: fonttools not installed. Run:\n"
            "  sudo apt-get install python3-fonttools",
            file=sys.stderr,
        )
        sys.exit(1)


def download_font(url, path, size_hint):
    print(f"Downloading source font from GitHub ({size_hint})...")
    urllib.request.urlretrieve(url, path)
    print(f"Saved to {path}")


def collect_characters(translation_files):
    chars = set()
    for path in translation_files:
        if not path.exists():
            print(f"Warning: translation file not found: {path}", file=sys.stderr)
            continue
        for ch in path.read_text(encoding="utf-8"):
            if ord(ch) > 127:
                chars.add(ch)
    return chars


def generate_subset(source_path, output_path, chars):
    from fontTools import subset as ftsubset
    from fontTools.ttLib import TTFont

    print(f"Subsetting {len(chars)} unique characters from {source_path.name}...")
    options = ftsubset.Options()
    options.layout_features = []
    options.name_IDs = [1, 2, 4]
    options.drop_tables = ["DSIG"]

    tt = TTFont(str(source_path))
    subsetter = ftsubset.Subsetter(options=options)
    subsetter.populate(unicodes=sorted(ord(c) for c in chars))
    subsetter.subset(tt)
    tt.save(str(output_path))

    size_kb = output_path.stat().st_size // 1024
    print(f"Saved to {output_path} ({size_kb} KB)")


def verify_subset(output_path, chars, latin_ok=True):
    from fontTools.ttLib import TTFont

    tt = TTFont(str(output_path))
    cmap = tt.getBestCmap()
    # U+00D7 (×) is Latin and covered by Roboto — ignore it here
    missing = [
        c for c in chars if ord(c) not in cmap and not (latin_ok and ord(c) < 0x0300)
    ]
    if missing:
        print(
            f"Warning: {len(missing)} characters missing from {output_path.name}:",
            file=sys.stderr,
        )
        for c in missing[:20]:
            print(f"  U+{ord(c):04X} ({c})", file=sys.stderr)
    else:
        print(f"Verified: all required characters present in {output_path.name}.")


def main():
    repo_root = Path(__file__).parent.parent
    os.chdir(repo_root)

    ensure_fonttools()

    # CJK subset (Japanese, Korean, Chinese)
    print("\n--- CJK (Japanese, Korean, Chinese) ---")
    if not CJK_SOURCE_PATH.exists():
        download_font(CJK_SOURCE_URL, CJK_SOURCE_PATH, "~16 MB")
    else:
        print(f"Using cached source font at {CJK_SOURCE_PATH}")
    cjk_chars = collect_characters(CJK_TRANSLATION_FILES)
    if cjk_chars:
        generate_subset(CJK_SOURCE_PATH, CJK_OUTPUT_PATH, cjk_chars)
        verify_subset(CJK_OUTPUT_PATH, cjk_chars)
    else:
        print("No CJK characters found — skipping.")

    # Thai subset
    print("\n--- Thai ---")
    if not THAI_SOURCE_PATH.exists():
        download_font(THAI_SOURCE_URL, THAI_SOURCE_PATH, "~37 KB")
    else:
        print(f"Using cached source font at {THAI_SOURCE_PATH}")
    thai_chars = collect_characters(THAI_TRANSLATION_FILES)
    if thai_chars:
        generate_subset(THAI_SOURCE_PATH, THAI_OUTPUT_PATH, thai_chars)
        verify_subset(THAI_OUTPUT_PATH, thai_chars)
    else:
        print("No Thai characters found — skipping.")

    print(
        "\nDone. Commit the updated font files in refbox/resources/ to apply the changes."
    )


if __name__ == "__main__":
    main()
