#!/usr/bin/env python3
"""Export encyclopedia unit dispositions with strict source-anchor validation.

Each disposition cites exact loaded Hercules lines and quotes every cited line.
Counts are reported per package and result so coverage can say how much of a
queue is closed and how, without counting a clue as a disposition.
"""
from __future__ import annotations

import argparse
import json
import sys
from collections import Counter
from typing import Any

from encyclopedia_checks import check_date, check_disposition
from encyclopedia_common import DISPOSITIONS, ROOT
from export_item_grant_reference import source_revision

OUTPUT = ROOT / "docs" / "encyclopedia-dispositions.v1.json"


def build() -> dict[str, Any]:
    data = json.loads(DISPOSITIONS.read_text(encoding="utf-8"))
    if data.get("schema_version") != 1 or not isinstance(data.get("entries"), list):
        raise ValueError("tools/encyclopedia_dispositions.json must have schema_version 1 and an entries list")
    seen: set[str] = set()
    for record in data["entries"]:
        label = f"disposition {record.get('id')}"
        if record.get("id") in seen:
            raise ValueError(f"{label}: duplicate id")
        seen.add(record.get("id"))
        errors = check_disposition(label, record, None)
        if not str(record.get("unit", "")).startswith(f"{record.get('package')}-"):
            errors.append(f"{label}: unit must start with its package")
        if not record.get("reviewed_by") or not check_date(record.get("reviewed_on")):
            errors.append(f"{label}: needs reviewed_by and a YYYY-MM-DD reviewed_on")
        if errors:
            raise ValueError("; ".join(errors))
    entries = sorted(data["entries"], key=lambda record: record["id"])
    counts = Counter(f"{record['package']}_{record['result']}" for record in entries)
    revision, dirty = source_revision()
    return {
        "schema_version": 1,
        "source_revision": revision,
        "source_worktree_dirty": dirty,
        "mode": "renewal",
        "rule": data["rule"],
        "counts": dict(sorted(counts.items())),
        "entries": entries,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="check for stale output without writing")
    args = parser.parse_args()
    try:
        rendered = json.dumps(build(), indent=2, ensure_ascii=False) + "\n"
    except (OSError, ValueError, KeyError, json.JSONDecodeError) as error:
        print(f"encyclopedia disposition export failed: {error}", file=sys.stderr)
        return 1
    if args.check:
        if not OUTPUT.is_file() or OUTPUT.read_text(encoding="utf-8") != rendered:
            print(f"stale or missing: {OUTPUT.relative_to(ROOT)}", file=sys.stderr)
            return 1
        print(f"up to date: {len(json.loads(rendered)['entries'])} dispositions")
        return 0
    OUTPUT.write_text(rendered, encoding="utf-8")
    print(f"wrote {OUTPUT.relative_to(ROOT)} ({len(json.loads(rendered)['entries'])} dispositions)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
