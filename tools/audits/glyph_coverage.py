#!/usr/bin/env python3
"""Fail when client text uses a character the game font cannot draw.

The interface draws every string with NotoSans from a pre-built glyph atlas
(`korangar/archive/data/font/`). A character the font does not map renders
as a missing-glyph box, and nothing else notices: on 2026-10-05 the map
window's visited marker (U+2713 check mark), the drop toasts' star, every
route arrow, and the party HP bar's block characters were all boxes on
screen while every test passed.

The audit reads the font's own character map and checks every non-ASCII
character inside a string literal in `korangar/src`.

Not flagged:
- Hangul syllables: they appear only in GRF asset paths (sprite and effect
  file names), which are looked up, never drawn.
- `graphics/primitives.rs`: upstream debug-inspector labels; its own TODO
  already notes the font lacks those arrows.

    python3 tools/audits/glyph_coverage.py
"""

from __future__ import annotations

import re
import struct
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
FONT = ROOT / "korangar/archive/data/font/NotoSans.ttf"
SOURCES = ROOT / "korangar/src"
EXCLUDED_FILES = {"graphics/primitives.rs"}
STRING = re.compile(r'"((?:[^"\\]|\\.)*)"')


def font_codepoints(path: Path) -> set[int]:
    data = path.read_bytes()
    tables = {}
    for index in range(struct.unpack(">H", data[4:6])[0]):
        tag, _, offset, length = struct.unpack(">4sIII", data[12 + 16 * index : 28 + 16 * index])
        tables[tag] = (offset, length)
    cmap, _ = tables[b"cmap"]
    codepoints: set[int] = set()
    for index in range(struct.unpack(">H", data[cmap + 2 : cmap + 4])[0]):
        _, _, sub = struct.unpack(">HHI", data[cmap + 4 + 8 * index : cmap + 12 + 8 * index])
        start = cmap + sub
        kind = struct.unpack(">H", data[start : start + 2])[0]
        if kind == 4:
            seg_x2 = struct.unpack(">H", data[start + 6 : start + 8])[0]
            segments = seg_x2 // 2
            ends = struct.unpack(f">{segments}H", data[start + 14 : start + 14 + seg_x2])
            starts = struct.unpack(f">{segments}H", data[start + 16 + seg_x2 : start + 16 + 2 * seg_x2])
            for low, high in zip(starts, ends):
                codepoints.update(range(low, high + 1))
        elif kind == 12:
            groups = struct.unpack(">I", data[start + 12 : start + 16])[0]
            for group in range(groups):
                low, high, _ = struct.unpack(">III", data[start + 16 + 12 * group : start + 28 + 12 * group])
                codepoints.update(range(low, high + 1))
    return codepoints


def is_hangul(character: str) -> bool:
    return 0xAC00 <= ord(character) <= 0xD7A3


def main() -> int:
    drawable = font_codepoints(FONT)
    problems = []
    for path in sorted(SOURCES.rglob("*.rs")):
        relative = path.relative_to(SOURCES).as_posix()
        if relative in EXCLUDED_FILES:
            continue
        for number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            code = line.split("//", 1)[0]
            for literal in STRING.finditer(code):
                for character in literal.group(1):
                    if ord(character) > 127 and ord(character) not in drawable and not is_hangul(character):
                        problems.append(f"{relative}:{number}: U+{ord(character):04X} {character!r}")
    if problems:
        print("Characters the game font (NotoSans) cannot draw; they render as boxes:")
        print("\n".join(f"  {problem}" for problem in problems))
        print("Use a character the font has (for example • › » – × …), or words.")
        return 1
    print(f"glyph coverage: every non-Hangul string character is in NotoSans ({len(drawable)} codepoints)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
