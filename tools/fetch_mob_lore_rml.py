#!/usr/bin/env python3
"""Fetch the Ragnarok Monster Lore [RML] encyclopedia from WarpPortal forums.

Mitten's fan-made encyclopedia (forums.warpportal.com topic 106898) indexes
~70 per-monster lore threads mixing official kRO source material, real-world
mythology notes, and fan fiction. This harvests each thread's first post into
docs/mob_lore_rml.json keyed by mob id.

These are fan-authored forum posts (no explicit license, unlike the
CC-BY-SA Fandom text in mob_lore.json). Shipped as bestiary Scholar-tier
game data for this private, non-commercial campaign client with per-entry
author/source attribution retained; hand-written campaign entries override
where they exist.

Usage: python3 tools/fetch_mob_lore_rml.py
"""

import difflib
import json
import re
import subprocess
import sys
import time
import urllib.parse
from html import unescape
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
MOB_DB = REPO.parent / "Hercules" / "db" / "re" / "mob_db.conf"
OUT = REPO / "docs" / "mob_lore_rml.json"

HUB = "https://forums.warpportal.com/index.php?/topic/106898-encyclopaedia-ragnarok-monster-lore-mitten/"

# RML thread title -> Hercules mob_db Name, where iRO-community naming diverges
ALIASES = {
    "abysmal knight": "Knight of Abyss",
    "false angel": "Fake Angel",
    "female thiefbug": "Thief Bug Female",
    "male thiefbug": "Thief Bug Male",
    "pitmen": "Pitman",
    "miyabi doll": "Miyabi Ningyo",
}
UA = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (SealCascadeDM lore reference fetch)"
SLEEP = 1.0
EXCERPT_CHARS = 500  # ship a short representative excerpt, not the full forum post; url stays for the rest


def excerpt(text, limit=EXCERPT_CHARS):
    if len(text) <= limit:
        return text
    cut = text[:limit]
    end = max(cut.rfind(". "), cut.rfind(".\n"), cut.rfind("! "), cut.rfind("? "))
    if end > limit * 0.4:
        cut = cut[:end + 1]
    return cut.strip() + "…"


def get(url):
    # -k: forums.warpportal.com serves an incomplete certificate chain
    result = subprocess.run(["curl", "-skL", "--max-time", "40", "-A", UA, url],
                            capture_output=True, text=True, check=True)
    return result.stdout


def parse_mob_names():
    text = MOB_DB.read_text(encoding="utf-8", errors="replace")
    names = {}
    for m in re.finditer(r"^\tId:\s*(\d+)\s*$.*?^\tName:\s*\"([^\"]+)\"", text, re.M | re.S):
        names.setdefault(m.group(2), []).append(int(m.group(1)))
    return names


def normalize(name):
    return re.sub(r"[^a-z0-9]", "", name.lower())


def first_post_text(page_html):
    m = re.search(r"<div[^>]*class=['\"]post entry-content[^'\"]*['\"][^>]*>(.*?)</div>\s*<(?:div|/div)",
                  page_html, re.S)
    if not m:
        m = re.search(r"<div[^>]*entry-content[^>]*>(.*)", page_html, re.S)
    if not m:
        return ""
    body = m.group(1)
    body = re.sub(r"<(script|style).*?</\1>", "", body, flags=re.S)
    body = re.sub(r"<img[^>]*>", "", body)
    body = re.sub(r"<br[^>]*>|</p>", "\n", body)
    body = re.sub(r"<[^>]+>", " ", body)
    text = unescape(body)
    text = re.sub(r"[ \t]+", " ", text)
    lines = [l.strip() for l in text.splitlines()]
    # strip forum boilerplate so the text is game-display clean
    drop = re.compile(
        r"^(this thread is listed under|redirect to original thread|"
        r"ragnarok monster lore\s*:|\[rml\]|source\s*:?\s*http|http\S+$|"
        r"click (here|on)|credits? (to|:)|screenshots? (by|:))", re.I)
    lines = [l for l in lines if l and not drop.match(l)]
    text = "\n".join(lines)
    text = re.sub(r"https?://\S+", "", text)  # inline link remnants
    text = re.sub(r"\n{3,}", "\n\n", text)
    # posts end with edit notes / signatures; trim the standard footer
    text = re.split(r"Edited by \w+,", text)[0]
    return text.strip()


def main():
    hub_html = get(HUB)
    links = {}
    for href in re.findall(r"href=['\"]([^'\"]*?/topic/\d+-rml-[^'\"]*)['\"]", hub_html):
        href = unescape(href)
        slug = re.search(r"/topic/(\d+-rml-[a-z0-9-]+)", href).group(1)
        links[slug] = "https://forums.warpportal.com/index.php?/topic/" + slug + "/"
    print(f"{len(links)} RML entry threads linked from the hub")

    mob_names = parse_mob_names()
    by_norm = {normalize(n): n for n in mob_names}
    norm_keys = list(by_norm)

    entries, unmatched = {}, []
    for n, (slug, url) in enumerate(sorted(links.items()), 1):
        html_page = get(url)
        title_m = re.search(r"\[RML\]\s*(.*?)\s*\[", html_page)
        title = unescape(title_m.group(1)).strip() if title_m else slug
        text = first_post_text(html_page)
        if len(text) < 80:
            print(f"  {n}/{len(links)} {title}: extraction too short, skipped", file=sys.stderr)
            time.sleep(SLEEP)
            continue

        # a thread may cover aliases: "Lord of Death / Lord of the Dead"
        mob_ids = []
        for cand in re.split(r"[/,]| aka ", title, flags=re.I):
            key = normalize(cand)
            alias = ALIASES.get(cand.strip().lower())
            hit = alias if alias in mob_names else by_norm.get(key)
            if not hit:
                close = difflib.get_close_matches(key, norm_keys, n=2, cutoff=0.9)
                hit = by_norm[close[0]] if len(close) == 1 else None
            if hit:
                mob_ids += mob_names[hit]
        record = {"title": title, "lore": excerpt(text), "url": url, "author": "Mitten (RML project)"}
        if mob_ids:
            for mid in sorted(set(mob_ids)):
                entries[str(mid)] = record
        else:
            unmatched.append(title)
        print(f"  {n}/{len(links)} {title}: {len(text)} chars, ids {sorted(set(mob_ids)) or 'UNMATCHED'}")
        time.sleep(SLEEP)

    out = {
        "_meta": {
            "source": HUB,
            "license": "NONE — fan-authored forum posts, individual author copyright. "
                       "Private DM reference only; rewrite before player-facing use.",
            "fetched": time.strftime("%Y-%m-%d"),
            "entries": len(set(id(v) for v in entries.values())),
            "unmatched_titles": unmatched,
        },
        "mobs": dict(sorted(entries.items(), key=lambda kv: int(kv[0]))),
    }
    OUT.write_text(json.dumps(out, indent=1, ensure_ascii=False) + "\n")
    print(f"wrote {OUT}: {len(entries)} mob ids, {len(unmatched)} unmatched titles: {unmatched}")


if __name__ == "__main__":
    main()
