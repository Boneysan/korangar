#!/usr/bin/env python3
"""Generate `hercules_messages.tsv` from Hercules' own `messages_*.h`.

Why: `ZC_MSG` / `ZC_MSG_VALUE` / `ZC_MSG_SKILL` carry only a **message id**, and
the client resolves it through `msgstringtable.txt`. That file ships with the
*client* data, and ours is from a different build than the server: measured
2026-08-07, 1614 of the 4006 documented ids disagree with what Hercules means by
them and 433 are missing outright (the table is 3577 lines; the ids run past
4000).

Most disagreements are harmless rewording, but the drift is not uniform — some
regions are off by a line, which produces text that is not merely different but
*wrong*. The worst case found: a **successful** production skill (`MSG_SKILL_SUCCESS`,
0x626) resolved to "Item does not exist.", and a **failed** one resolved to
"Successful." — an inverted report.

Hercules documents the English gloss for every id in a comment right above it,
and those ids are exactly what the server sends, so the server's own header is
authoritative for this pairing. Same principle as
`tools/generate_packet_lengths.sh`: derive it from the server rather than hoping
the client data matches.

Korean-only glosses are skipped so the bundled msgstringtable — which is at least
in the player's language — stays the fallback for them.

Usage:
    tools/generate_message_table.py [HERCULES_DIR] [VARIANT]
"""

import re
import sys
from pathlib import Path

HANGUL = re.compile(r"[가-힯]")
# /*<version note>
#  <korean>
#  <english>
#  */
#  MSG_NAME = 0xNNN,
BLOCK = re.compile(
    r"/\*[^\n]*\n(?P<body>.*?)\*/\s*\n\s*(?P<name>MSG_\w+)\s*=\s*(?P<id>0x[0-9a-fA-F]+|\d+)\s*,",
    re.S,
)


def main() -> int:
    hercules = Path(sys.argv[1] if len(sys.argv) > 1 else "../Hercules")
    variant = sys.argv[2] if len(sys.argv) > 2 else "main"
    source = hercules / "src" / "map" / f"messages_{variant}.h"
    if not source.is_file():
        print(f"error: {source} not found", file=sys.stderr)
        return 1

    text = source.read_text(encoding="utf-8", errors="replace")
    rows: dict[int, str] = {}
    for match in BLOCK.finditer(text):
        body = [line.strip() for line in match.group("body").strip().splitlines() if line.strip()]
        if not body:
            continue
        gloss = body[-1]
        # A Korean-only entry has no English line to take; leave it to the
        # client's own table rather than showing Korean to an English player.
        if HANGUL.search(gloss) or not gloss:
            continue
        raw = match.group("id")
        message_id = int(raw, 16) if raw.startswith("0x") else int(raw)
        if message_id > 0xFFFF:
            continue
        rows.setdefault(message_id, gloss.replace("\t", " "))

    out = Path(__file__).parent.parent / "korangar" / "src" / "world" / "library" / "hercules_messages.tsv"
    out.write_text("".join(f"{i}\t{t}\n" for i, t in sorted(rows.items())), encoding="utf-8")
    print(f"wrote {len(rows)} messages to {out.relative_to(out.parents[5])}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
