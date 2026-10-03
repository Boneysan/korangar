#!/usr/bin/env python3
"""Derive Kafra Teleport Service hops from Hercules' literal Kafra tables.

The teleport service is three pieces of loaded Hercules script, all read here
rather than retyped:

* ``F_KafSet`` (npc/kafras/functions_kafras.txt) sets each Kafra map's
  destination names and zeny fares from literal ``setarray`` lines;
* ``F_KafTele`` maps each destination name to a literal ``warp`` cell
  (Izlude's renewal branch is taken, matching ``npc/re/scripts_main.conf``);
* each Kafra NPC calls ``F_Kafra`` whose second argument selects its menu.
  Menus 1-3, 5-8 and 10 have no "Use Teleport Service" entry, menu 4 shows the
  Einbroch "no teleport" notice instead, and first argument 2 is a guild
  castle Kafra; every other Kafra that also calls ``F_KafSet`` can teleport.
  Cool Event Corp. staff call ``F_ZondaStaff`` instead, whose first argument 1
  is the menu with "Teleport Service" (npc/kafras/cool_event_corp.txt).
* ``duplicate(template)`` NPCs run the template's script on their own map, so
  a placed duplicate of a teleporting template teleports from that map.

The result replaces the ``service-kafra-*`` entries in the reviewed service
manifest and leaves every hand-reviewed entry untouched. A fare can be paid
with a Free Ticket for Kafra Transportation instead of zeny (``F_KafTele``).
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

from generate_navigation_graph import loaded_script_files, map_names

FUNCTIONS = "npc/kafras/functions_kafras.txt"
NO_TELEPORT_MENUS = {1, 2, 3, 4, 5, 6, 7, 8, 10}
GUILD_CASTLE_KAFRA = 2

NPC_HEADER = re.compile(r"^(-|[a-z0-9_]+(?:,(\d+),(\d+),\d+)?)\tscript\t([^\t]+?)\t[^\t]+,\{", re.M)
DUPLICATE = re.compile(r"^([a-z0-9_]+),(\d+),(\d+),\d+\tduplicate\(([^)]+)\)\t([^\t]+?)\t", re.M)
KAFSET_BRANCH = re.compile(r'strnpcinfo\(NPC_MAP\) == "([a-z0-9_]+)"\s*\)\s*\{(.*?)\n\t\}', re.S)
SETARRAY_DEST = re.compile(r"setarray @wrpD\$,\s*(.+?);")
SETARRAY_PRICE = re.compile(r"setarray @wrpP,\s*(.+?);")
TELE_WARP = re.compile(r'@wrpD\$\[\.@j\] == "([^"]+)"\)\s*(.+?)$', re.M)
WARP = re.compile(r'warp\s+"([a-z0-9_]+)",\s*(\d+),\s*(\d+);')
RENEWAL_WARP = re.compile(r'if \(RENEWAL\)\s*warp\s+"([a-z0-9_]+)",\s*(\d+),\s*(\d+);')
KAFRA_CALL = re.compile(r'callfunc "F_Kafra",\s*(\d+),\s*(\d+)')
ZONDA_CALL = re.compile(r'callfunc "F_ZondaStaff",\s*(\d+)')
ZONDA_TELEPORT_MENU = 1


def display_name(npc_name: str) -> str:
    """``Kafra Employee::kaf_alberta`` / ``Kafra Employee#iz`` -> ``Kafra Employee``."""
    return npc_name.split("::")[0].split("#")[0].strip()


def offers_teleport(body: str) -> bool:
    if 'callfunc "F_KafSet"' not in body:
        return False
    if (call := KAFRA_CALL.search(body)) is not None:
        welcome, menu = int(call.group(1)), int(call.group(2))
        return welcome != GUILD_CASTLE_KAFRA and menu not in NO_TELEPORT_MENUS
    if (call := ZONDA_CALL.search(body)) is not None:
        return int(call.group(1)) == ZONDA_TELEPORT_MENU
    return False


def function_body(text: str, name: str) -> tuple[str, int]:
    """Return a function's body and the 1-based line its header is on."""
    match = re.search(rf"^function\s+script\s+{name}\s*\{{", text, re.M)
    if match is None:
        raise ValueError(f"{FUNCTIONS}: function {name} not found")
    end = text.index("\n}\n", match.end())
    return text[match.end():end], text.count("\n", 0, match.start()) + 1


def parse_destinations(text: str) -> dict[str, tuple[str, int, int]]:
    body, _ = function_body(text, "F_KafTele")
    destinations: dict[str, tuple[str, int, int]] = {}
    for name, branch in TELE_WARP.findall(body):
        warp = RENEWAL_WARP.search(branch) or WARP.search(branch)
        if warp is None:
            raise ValueError(f"F_KafTele: no literal warp for destination {name!r}")
        destinations[name] = (warp.group(1), int(warp.group(2)), int(warp.group(3)))
    if not destinations:
        raise ValueError("F_KafTele: no destinations parsed")
    return destinations


def parse_fares(text: str) -> dict[str, list[tuple[str, int]]]:
    body, _ = function_body(text, "F_KafSet")
    fares: dict[str, list[tuple[str, int]]] = {}
    for kafra_map, branch in KAFSET_BRANCH.findall(body):
        names = SETARRAY_DEST.search(branch)
        prices = SETARRAY_PRICE.search(branch)
        if names is None or prices is None:
            raise ValueError(f"F_KafSet: {kafra_map} lacks a literal destination or price array")
        destination_names = [name.strip().strip('"') for name in names.group(1).split(",")]
        price_values = [int(price) for price in prices.group(1).split(",")]
        if len(destination_names) != len(price_values):
            raise ValueError(f"F_KafSet: {kafra_map} has {len(destination_names)} destinations but {len(price_values)} fares")
        fares[kafra_map] = list(zip(destination_names, price_values))
    if not fares:
        raise ValueError("F_KafSet: no map branches parsed")
    return fares


def teleporting_kafras(root: Path, files: list[Path]) -> dict[str, list[tuple[int, int, str, str]]]:
    """Placed NPCs that set the destination table and offer teleport, by map."""
    kafras: dict[str, list[tuple[int, int, str, str]]] = {}
    templates: set[str] = set()
    texts = [(path, path.read_text(encoding="utf-8", errors="replace")) for path in files]
    for path, text in texts:
        headers = list(NPC_HEADER.finditer(text))
        for index, header in enumerate(headers):
            end = headers[index + 1].start() if index + 1 < len(headers) else len(text)
            if not offers_teleport(text[header.end():end]):
                continue
            npc_name = header.group(4).strip()
            if "::" in npc_name:
                templates.add(npc_name.split("::", 1)[1])
            if header.group(1) == "-":
                continue
            line = text.count("\n", 0, header.start()) + 1
            kafra_map = header.group(1).split(",")[0]
            source = f"{path.relative_to(root).as_posix()}:{line}"
            kafras.setdefault(kafra_map, []).append(
                (int(header.group(2)), int(header.group(3)), display_name(npc_name), source)
            )
    for path, text in texts:
        for duplicate in DUPLICATE.finditer(text):
            if duplicate.group(4) not in templates:
                continue
            line = text.count("\n", 0, duplicate.start()) + 1
            source = f"{path.relative_to(root).as_posix()}:{line} (duplicate of {duplicate.group(4)})"
            kafras.setdefault(duplicate.group(1), []).append(
                (int(duplicate.group(2)), int(duplicate.group(3)), display_name(duplicate.group(5)), source)
            )
    for entries in kafras.values():
        entries.sort()
    return kafras


def build(root: Path) -> list[dict]:
    functions_path = root / FUNCTIONS
    text = functions_path.read_text(encoding="utf-8", errors="replace")
    _, kafset_line = function_body(text, "F_KafSet")
    _, kafele_line = function_body(text, "F_KafTele")
    destinations = parse_destinations(text)
    fares = parse_fares(text)
    files = loaded_script_files(root, root / "npc/re/scripts_main.conf")
    if functions_path not in files:
        raise ValueError(f"{FUNCTIONS} is not loaded by npc/re/scripts_main.conf")
    kafras = teleporting_kafras(root, files)
    known_maps = map_names(root / "db/map_index.txt")

    services: list[dict] = []
    for kafra_map, routes in sorted(fares.items()):
        offering = kafras.get(kafra_map)
        if not offering:
            # A table branch with no loaded Kafra that can reach it adds nothing.
            continue
        x, y, npc_name, npc_source = offering[0]
        for destination_name, fare in routes:
            if destination_name not in destinations:
                raise ValueError(f"F_KafSet: {kafra_map} lists {destination_name!r}, which F_KafTele cannot warp to")
            to_map, to_x, to_y = destinations[destination_name]
            for map_name in (kafra_map, to_map):
                if map_name not in known_maps:
                    raise ValueError(f"unknown map {map_name!r} in the Kafra tables")
            services.append({
                "id": f"service-kafra-{kafra_map}-{to_map}",
                "availability": "conditional",
                "action": f"Talk to the {npc_name}, choose the teleport service, then {destination_name}.",
                "requirements": f"Costs {fare} zeny, or one Free Ticket for Kafra Transportation.",
                "fare_zeny": fare,
                "from": {"map": kafra_map, "x": x, "y": y},
                "to": {"map": to_map, "x": to_x, "y": to_y},
                "source": f"{npc_source}; {FUNCTIONS}:{kafset_line} (F_KafSet), {FUNCTIONS}:{kafele_line} (F_KafTele)",
            })
    ids = [service["id"] for service in services]
    if len(ids) != len(set(ids)):
        raise ValueError("duplicate Kafra service ids")
    return services


def merge(manifest: dict, kafra_services: list[dict]) -> dict:
    kept = [service for service in manifest.get("services", []) if not service["id"].startswith("service-kafra-")]
    return {**manifest, "services": sorted(kept + kafra_services, key=lambda service: service["id"])}


def main() -> int:
    client_root = Path(__file__).resolve().parents[1]
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--hercules-root", type=Path, default=client_root.parent / "Hercules")
    parser.add_argument("--services", type=Path, default=client_root / "korangar" / "data" / "navigation_services.json")
    parser.add_argument("--check", action="store_true", help="fail if the manifest's Kafra entries are stale")
    args = parser.parse_args()

    try:
        kafra_services = build(args.hercules_root.resolve())
    except (OSError, ValueError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 1
    manifest = json.loads(args.services.read_text(encoding="utf-8"))
    rendered = json.dumps(merge(manifest, kafra_services), ensure_ascii=False, indent=2) + "\n"
    if args.check:
        if args.services.read_text(encoding="utf-8") != rendered:
            print(f"stale Kafra services in {args.services} — re-run {Path(__file__).name}", file=sys.stderr)
            return 1
        print(f"Kafra services current: {len(kafra_services)} hops")
        return 0
    args.services.write_text(rendered, encoding="utf-8")
    print(f"wrote {len(kafra_services)} Kafra hops into {args.services}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
