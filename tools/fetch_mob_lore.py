#!/usr/bin/env python3
"""Fetch mob lore extracts from the Ragnarok Fandom wiki (CC-BY-SA).

Produces docs/mob_lore.json: mob_id -> { title, lore, url } for every
bestiary.json mob whose iRO display name (Hercules mob_db.conf Name field)
matches a wiki article. Community-authored CC-BY-SA text — raw DM reference
for the bestiary Scholar tier; campaign entries should be rewritten in
campaign voice (see docs/specs/bestiary-journal.md).

Usage: python3 tools/fetch_mob_lore.py [--limit N_BATCHES]
Re-run after mob DB changes, then rebuild (include_str! embeds at compile).
"""

import difflib
import json
import re
import subprocess
import sys
import time
import urllib.parse
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
MOB_DB = REPO.parent / "Hercules" / "db" / "re" / "mob_db.conf"
BESTIARY = REPO / "docs" / "bestiary.json"
OUT = REPO / "docs" / "mob_lore.json"

API = "https://ragnarok.fandom.com/api.php"
USER_AGENT = "SealCascadeDM/1.0 (private tabletop campaign tool; polite batch fetch)"
BATCH = 20          # MediaWiki extracts limit per query
SLEEP = 0.7         # politeness delay between requests

DISAMBIG_MARKERS = ("may refer to", "can refer to", "disambiguation")


def parse_mob_names():
    """mob_db.conf: Id -> iRO display Name."""
    text = MOB_DB.read_text(encoding="utf-8", errors="replace")
    names = {}
    for m in re.finditer(r"^\tId:\s*(\d+)\s*$.*?^\tName:\s*\"([^\"]+)\"", text, re.M | re.S):
        names[int(m.group(1))] = m.group(2)
    return names


def api_query(titles):
    params = {
        "action": "query",
        "prop": "extracts",
        "exintro": 1,
        "explaintext": 1,
        "redirects": 1,
        "format": "json",
        "titles": "|".join(titles),
    }
    url = API + "?" + urllib.parse.urlencode(params)
    # curl instead of urllib: system Python on macOS lacks CA certificates
    result = subprocess.run(
        ["curl", "-sf", "--max-time", "30", "-A", USER_AGENT, url],
        capture_output=True, text=True, check=True,
    )
    return json.loads(result.stdout)


def category_members(category):
    """All article titles in a wiki category (paged)."""
    titles, cont = [], {}
    while True:
        params = {"action": "query", "list": "categorymembers", "cmtitle": category,
                  "cmlimit": "500", "cmtype": "page", "format": "json", **cont}
        url = API + "?" + urllib.parse.urlencode(params)
        result = subprocess.run(["curl", "-sf", "--max-time", "30", "-A", USER_AGENT, url],
                                capture_output=True, text=True, check=True)
        data = json.loads(result.stdout)
        titles += [m["title"] for m in data["query"]["categorymembers"]]
        if "continue" not in data:
            return titles
        cont = data["continue"]
        time.sleep(SLEEP)


def normalize(name):
    return re.sub(r"[^a-z0-9]", "", name.lower())


def recovery_pass(lore, name_to_ids):
    """Second pass: match Category:Monsters titles to still-loreless mobs.

    Catches case mismatches (MediaWiki titles are case-sensitive past the
    first letter) and spelling variants (Baphomet Junior vs Baphomet Jr.).
    """
    unmatched = {n: ids for n, ids in name_to_ids.items()
                 if not any(str(i) in lore for i in ids)}
    by_norm = {}
    for n in unmatched:
        by_norm.setdefault(normalize(n), n)

    used_titles = {v["title"] for v in lore.values()}
    candidates = [t for t in category_members("Category:Monsters") if t not in used_titles]

    resolved = {}  # wiki title -> our db name
    norm_keys = list(by_norm)
    for title in candidates:
        key = normalize(title)
        if key in by_norm:
            resolved[title] = by_norm[key]
            continue
        close = difflib.get_close_matches(key, norm_keys, n=2, cutoff=0.88)
        if len(close) == 1:  # unambiguous fuzzy match only
            resolved[title] = by_norm[close[0]]

    print(f"recovery: {len(resolved)} category titles matched to loreless mobs")
    batch_titles = sorted(resolved)
    for i in range(0, len(batch_titles), BATCH):
        batch = batch_titles[i:i + BATCH]
        try:
            data = api_query(batch)
        except Exception as exc:
            print(f"recovery batch FAILED: {exc}", file=sys.stderr)
            continue
        for page in data.get("query", {}).get("pages", {}).values():
            title = page.get("title")
            extract = (page.get("extract") or "").strip()
            if title not in resolved or "missing" in page or not extract:
                continue
            if any(m in extract.lower()[:120] for m in DISAMBIG_MARKERS):
                continue
            url = "https://ragnarok.fandom.com/wiki/" + urllib.parse.quote(title.replace(" ", "_"))
            for mid in name_to_ids[resolved[title]]:
                lore.setdefault(str(mid), {"title": title, "lore": extract, "url": url})
        time.sleep(SLEEP)


def main():
    limit = None
    if "--limit" in sys.argv:
        limit = int(sys.argv[sys.argv.index("--limit") + 1])

    mob_names = parse_mob_names()
    bestiary_ids = {m["Id"] for m in json.loads(BESTIARY.read_text())}
    wanted = {mid: name for mid, name in mob_names.items() if mid in bestiary_ids}

    name_to_ids = {}
    for mid, name in wanted.items():
        name_to_ids.setdefault(name, []).append(mid)
    unique_names = sorted(name_to_ids)
    print(f"{len(wanted)} bestiary mobs, {len(unique_names)} unique names")

    lore = {}
    batches = [unique_names[i:i + BATCH] for i in range(0, len(unique_names), BATCH)]
    if limit:
        batches = batches[:limit]

    for n, batch in enumerate(batches, 1):
        try:
            data = api_query(batch)
        except Exception as exc:  # keep partial results on network hiccups
            print(f"batch {n}/{len(batches)} FAILED: {exc}", file=sys.stderr)
            time.sleep(SLEEP * 4)
            continue

        query = data.get("query", {})
        # map queried title -> final title through normalization + redirects
        resolved = {t: t for t in batch}
        for step in ("normalized", "redirects"):
            for entry in query.get(step, []):
                for src, dst in list(resolved.items()):
                    if dst == entry["from"]:
                        resolved[src] = entry["to"]

        by_title = {p.get("title"): p for p in query.get("pages", {}).values()}
        for queried in batch:
            page = by_title.get(resolved[queried])
            if not page or "missing" in page:
                continue
            extract = (page.get("extract") or "").strip()
            if not extract or any(m in extract.lower()[:120] for m in DISAMBIG_MARKERS):
                continue
            title = page["title"]
            url = "https://ragnarok.fandom.com/wiki/" + urllib.parse.quote(title.replace(" ", "_"))
            for mid in name_to_ids[queried]:
                lore[str(mid)] = {"title": title, "lore": extract, "url": url}

        print(f"batch {n}/{len(batches)}: {len(lore)} mobs matched so far")
        time.sleep(SLEEP)

    if not limit:
        recovery_pass(lore, name_to_ids)

    out = {
        "_meta": {
            "source": "https://ragnarok.fandom.com (MediaWiki API, intro extracts)",
            "license": "CC-BY-SA — attribution required; raw reference text, "
                       "rewrite in campaign voice before player-facing use",
            "fetched": time.strftime("%Y-%m-%d"),
            "coverage": f"{len(lore)}/{len(wanted)} bestiary mobs",
        },
        "mobs": dict(sorted(lore.items(), key=lambda kv: int(kv[0]))),
    }
    OUT.write_text(json.dumps(out, indent=1, ensure_ascii=False) + "\n")
    print(f"wrote {OUT} — coverage {len(lore)}/{len(wanted)}")


if __name__ == "__main__":
    main()
