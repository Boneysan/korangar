#!/usr/bin/env python3
"""Export active static Hercules warp NPCs as deterministic navigation data."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import subprocess
import sys
from pathlib import Path


WARP = re.compile(
    r"^\s*([A-Za-z0-9_]+),(\d+),(\d+),(\d+)\s+warp\s+.+?\s+"
    r"(\d+),(\d+),([A-Za-z0-9_]+),(\d+),(\d+)\s*(?:$|//)"
)
INCLUDE = re.compile(r'^\s*@include\s+"([^"]+)"')
SCRIPT_HEADER = re.compile(r"^(-|[a-z0-9_]+,\d+,\d+,\d+)\t(script|duplicate\([^)]*\)|function)\t([^\t]+)", re.M)
SCRIPT_WARP = re.compile(r'\bwarp\s+"([a-z0-9_]+)",\s*(\d+),\s*(\d+)\s*;')
MAX_LISTED_ENTRANCES = 3
MAX_CONDITIONS = 5
MAX_SETTERS = 3
CONDITION = re.compile(r"\bif\s*\((.*)\)")
GATE_CALLS = re.compile(
    r"\b(BaseLevel|JobLevel|Upper|Class|BaseJob|Zeny|countitem|isequipped|getpartnerid|getgmlevel|F_GM_NPC|"
    r"questprogress|checkquest|isbegin_quest|getskilllv|checkhiding|rand)\b"
)
CHARACTER_VARIABLE = re.compile(r"(?<![.@$\w'])([A-Za-z][A-Za-z0-9_]*)\s*(?:==|!=|<=|>=|<|>|&)\s*-?\d")
COMPARISON = re.compile(r"(?<![.@$\w'])([A-Za-z][A-Za-z0-9_]*)\s*(==|!=|<=|>=|<|>|&)\s*(-?\d+)")
NOT_VARIABLES = {
    "BaseLevel", "JobLevel", "Upper", "Class", "BaseJob", "Zeny", "Sex", "Weight", "MaxWeight", "Hp", "MaxHp",
    "Sp", "MaxSp", "SkillPoint", "StatusPoint", "RENEWAL", "RENEWAL_EXP", "PACKETVER",
}
ASSIGNMENT = re.compile(r"(?:\bset\s+([A-Za-z][A-Za-z0-9_]*)\s*,\s*(-?\d+)|\b([A-Za-z][A-Za-z0-9_]*)\s*(\|=|\+=|=)\s*(-?\d+)\s*;)")
SCRIPT_FILE = re.compile(r'"([^"]+\.txt)"')


def source_revision(root: Path) -> tuple[str, bool]:
    try:
        revision = subprocess.check_output(
            ["git", "-C", str(root), "rev-parse", "HEAD"], text=True, stderr=subprocess.DEVNULL
        ).strip()
        dirty = bool(subprocess.check_output(["git", "-C", str(root), "status", "--porcelain"], text=True).strip())
        return revision, dirty
    except (OSError, subprocess.CalledProcessError):
        return "unknown", False


def loaded_script_files(root: Path, entry: Path) -> list[Path]:
    """Follow active @include manifests and collect their listed .txt files."""
    visited_configs: set[Path] = set()
    scripts: set[Path] = set()

    def visit(config: Path) -> None:
        config = config.resolve()
        if config in visited_configs or not config.is_file():
            return
        visited_configs.add(config)
        for raw in config.read_text(encoding="utf-8", errors="replace").splitlines():
            line = raw.split("//", 1)[0].strip()
            if not line:
                continue
            include = INCLUDE.match(line)
            if include:
                visit(root / include.group(1))
                continue
            for match in SCRIPT_FILE.finditer(line):
                path = (root / match.group(1)).resolve()
                if path.is_file():
                    scripts.add(path)

    visit(entry)
    return sorted(scripts)


def map_names(map_index: Path) -> set[str]:
    names: set[str] = set()
    for raw in map_index.read_text(encoding="utf-8", errors="replace").splitlines():
        line = raw.strip()
        if line and not line.startswith("//"):
            names.add(line.split()[0])
    return names


def parse_warps(root: Path, files: list[Path], known_maps: set[str]) -> tuple[list[dict], list[str], list[str]]:
    edges: list[dict] = []
    unsupported: list[str] = []
    errors: list[str] = []

    for path in files:
        text = path.read_text(encoding="utf-8", errors="replace")
        in_block_comment = False
        for line_number, raw in enumerate(text.splitlines(), start=1):
            line = raw
            if in_block_comment:
                if "*/" in line:
                    line = line.split("*/", 1)[1]
                    in_block_comment = False
                else:
                    continue
            while "/*" in line:
                before, after = line.split("/*", 1)
                if "*/" not in after:
                    line = before
                    in_block_comment = True
                    break
                line = before + after.split("*/", 1)[1]
            line = line.split("//", 1)[0]
            if not line.strip():
                continue

            match = WARP.match(line)
            if not match:
                if re.match(r"^\s*[A-Za-z0-9_]+,\d+,\d+,\d+\s+warp\b", line):
                    unsupported.append(f"{path.relative_to(root)}:{line_number}")
                continue

            source_map, sx, sy, _direction, width, height, target_map, tx, ty = match.groups()
            if source_map not in known_maps or target_map not in known_maps:
                errors.append(f"{path.relative_to(root)}:{line_number}: unknown map {source_map!r} -> {target_map!r}")
                continue
            provenance = f"{path.relative_to(root).as_posix()}:{line_number}"
            identity = f"{provenance}|{source_map},{sx},{sy}|{target_map},{tx},{ty}"
            edges.append(
                {
                    "id": "warp-" + hashlib.sha1(identity.encode()).hexdigest()[:16],
                    "kind": "walk_warp",
                    "availability": "always",
                    "from": {"map": source_map, "x": int(sx), "y": int(sy), "width": int(width), "height": int(height)},
                    "to": {"map": target_map, "x": int(tx), "y": int(ty)},
                    "source": provenance,
                }
            )

    edges.sort(key=lambda edge: edge["id"])
    return edges, unsupported, errors


def parse_authored_services(path: Path, known_maps: set[str]) -> tuple[list[dict], list[str]]:
    """Load reviewed NPC/service hops that cannot be derived from static warp lines."""
    artifact = json.loads(path.read_text(encoding="utf-8"))
    services = artifact.get("services")
    if not isinstance(services, list):
        return [], [f"{path}: expected a 'services' array"]

    edges: list[dict] = []
    errors: list[str] = []
    seen_ids: set[str] = set()
    for index, service in enumerate(services):
        label = f"{path}:services[{index}]"
        if not isinstance(service, dict):
            errors.append(f"{label}: expected an object")
            continue
        required = {"id", "availability", "action", "from", "to", "source"}
        missing = required - service.keys()
        if missing:
            errors.append(f"{label}: missing {', '.join(sorted(missing))}")
            continue
        edge_id = service["id"]
        if not isinstance(edge_id, str) or not edge_id.startswith("service-") or edge_id in seen_ids:
            errors.append(f"{label}: service id must be unique and begin with 'service-'")
            continue
        seen_ids.add(edge_id)
        locations_valid = True
        for side in ("from", "to"):
            position = service[side]
            if not isinstance(position, dict) or not {"map", "x", "y"} <= position.keys():
                errors.append(f"{label}: {side} must include map, x, and y")
                locations_valid = False
                continue
            if position["map"] not in known_maps:
                errors.append(f"{label}: unknown {side} map {position['map']!r}")
                locations_valid = False
            if any(not isinstance(position[key], int) or not 0 <= position[key] <= 65535 for key in ("x", "y")):
                errors.append(f"{label}: {side} coordinates must be unsigned 16-bit integers")
                locations_valid = False
        if not locations_valid:
            continue
        if not isinstance(service["availability"], str) or service["availability"] not in {"always", "conditional", "unknown"}:
            errors.append(f"{label}: availability must be always, conditional, or unknown")
            continue
        if (
            not isinstance(service["action"], str)
            or not service["action"].strip()
            or not isinstance(service["source"], str)
            or not service["source"].strip()
        ):
            errors.append(f"{label}: action and source must be non-empty strings")
            continue
        fare = service.get("fare_zeny", 0)
        if not isinstance(fare, int) or isinstance(fare, bool) or fare < 0:
            errors.append(f"{label}: fare_zeny must be a non-negative integer")
            continue
        has_lock = "locked_by" in service or "unlock_steps" in service
        if has_lock and not (
            isinstance(service.get("locked_by"), str)
            and service["locked_by"].strip()
            and isinstance(service.get("unlock_steps"), list)
            and service["unlock_steps"]
            and all(isinstance(step, str) and step.strip() for step in service["unlock_steps"])
        ):
            errors.append(f"{label}: a lock needs both a non-empty locked_by and a non-empty unlock_steps list")
            continue
        edge = dict(service)
        edge["kind"] = "npc_service"
        edge.setdefault("requirements", None)
        edges.append(edge)
    return edges, errors


def script_entrances(root: Path, files: list[Path]) -> dict[str, list[dict]]:
    """Literal ``warp "map",x,y;`` calls in loaded NPC scripts, by destination.

    These are clues, not routes: each records the gate-looking ``if``
    conditions that precede the warp inside its NPC, but the control flow
    that decides who is actually sent is not interpreted here.
    """
    entrances: dict[str, list[dict]] = {}
    for path in files:
        text = path.read_text(encoding="utf-8", errors="replace")
        lines = text.split("\n")
        headers = list(SCRIPT_HEADER.finditer(text))
        for warp in SCRIPT_WARP.finditer(text):
            preceding = [header for header in headers if header.start() < warp.start()]
            if not preceding:
                continue
            header = preceding[-1]
            location = header.group(1)
            npc_map = None if location == "-" or header.group(2) == "function" else location.split(",")[0]
            header_line = text.count("\n", 0, header.start()) + 1
            line = text.count("\n", 0, warp.start()) + 1
            conditions = []
            for raw in lines[header_line:line - 1]:
                code = raw.split("//", 1)[0]
                match = CONDITION.search(code)
                if match and (GATE_CALLS.search(match.group(1)) or CHARACTER_VARIABLE.search(match.group(1))):
                    condition = " ".join(match.group(1).split())
                    if "checkhiding" not in condition and condition not in conditions:
                        conditions.append(condition[:120])
            entrances.setdefault(warp.group(1), []).append({
                "npc": header.group(3).split("::")[0].split("#")[0].strip(),
                "npc_map": npc_map,
                "source": f"{path.relative_to(root).as_posix()}:{line}",
                "conditions": conditions[-MAX_CONDITIONS:],
            })
    return entrances


def variable_setters(root: Path, files: list[Path]) -> dict[str, list[dict]]:
    """Placed NPCs that assign a literal number to a character variable."""
    setters: dict[str, list[dict]] = {}
    for path in files:
        text = path.read_text(encoding="utf-8", errors="replace")
        header = None
        for line_number, raw in enumerate(text.split("\n"), start=1):
            code = raw.split("//", 1)[0]
            if match := SCRIPT_HEADER.match(raw):
                location = match.group(1)
                header = None
                if location != "-" and match.group(2) != "function":
                    map_name, x, y, _ = location.split(",")
                    header = {
                        "npc": match.group(3).split("::")[0].split("#")[0].strip(),
                        "map": map_name,
                        "x": int(x),
                        "y": int(y),
                    }
                continue
            if header is None:
                continue
            for assignment in ASSIGNMENT.finditer(code):
                name = assignment.group(1) or assignment.group(3)
                value = assignment.group(2) or assignment.group(5)
                operator = "=" if assignment.group(1) else assignment.group(4)
                if name in NOT_VARIABLES:
                    continue
                setters.setdefault(name, []).append({
                    **header,
                    "sets": f"{operator} {value}",
                    "source": f"{path.relative_to(root).as_posix()}:{line_number}",
                })
    return setters


def setter_rank(setter: dict, operator: str, threshold: int) -> tuple[int, int]:
    """Lower is a better answer to "who makes this condition pass?"."""
    setter_operator, value = setter["sets"].split()
    value = int(value)
    if operator == "&":
        return (0 if setter_operator == "|=" and value & threshold else 2, value)
    if operator == "==":
        return (0 if setter_operator == "=" and value == threshold else 2, abs(value - threshold))
    if operator in (">=", ">"):
        passes = value >= threshold if operator == ">=" else value > threshold
        return (0 if setter_operator == "=" and passes else 2, value)
    return (1, value)


def note_variables(entrances: list[dict], setters: dict[str, list[dict]]) -> dict[str, list[dict]]:
    """For each character variable an entrance checks, the NPCs most likely to
    assign the value that passes the check (exact value, then the smallest
    passing value, then the matching bit flag)."""
    found: dict[str, list[dict]] = {}
    for entrance in entrances:
        for condition in entrance.get("conditions", []):
            for name, operator, number in COMPARISON.findall(condition):
                if name in NOT_VARIABLES or name in found or name not in setters:
                    continue
                ranked = sorted(setters[name], key=lambda setter: (*setter_rank(setter, operator, int(number)), setter["source"]))
                distinct = []
                for setter in ranked:
                    if all(setter["npc"] != kept["npc"] or setter["sets"] != kept["sets"] for kept in distinct):
                        distinct.append(setter)
                found[name] = distinct[:MAX_SETTERS]
    return found


def access_notes(
    edges: list[dict],
    entrances: dict[str, list[dict]],
    setters: dict[str, list[dict]] | None = None,
    origin: str = "prontera",
) -> dict[str, dict]:
    """Why each graph map cannot be reached from ``origin``, for player text."""
    outgoing: dict[str, set[str]] = {}
    maps: set[str] = set()
    for edge in edges:
        outgoing.setdefault(edge["from"]["map"], set()).add(edge["to"]["map"])
        maps |= {edge["from"]["map"], edge["to"]["map"]}
    reachable = {origin}
    pending = [origin]
    while pending:
        for neighbor in outgoing.get(pending.pop(), ()):
            if neighbor not in reachable:
                reachable.add(neighbor)
                pending.append(neighbor)

    incoming: dict[str, set[str]] = {}
    for source, targets in outgoing.items():
        for target in targets:
            incoming.setdefault(target, set()).add(source)

    def listed(map_name: str) -> list[dict]:
        return sorted(entrances.get(map_name, []), key=lambda entrance: entrance["source"])

    def gateway(map_name: str) -> str | None:
        """Nearest unreachable map with a script entrance that walks here."""
        seen = {map_name}
        frontier = [map_name]
        while frontier:
            next_frontier = []
            for current in sorted(frontier):
                for previous in sorted(incoming.get(current, ())):
                    if previous in seen or previous in reachable:
                        continue
                    if listed(previous):
                        return previous
                    seen.add(previous)
                    next_frontier.append(previous)
            frontier = next_frontier
        return None

    notes: dict[str, dict] = {}
    for map_name in sorted((maps | set(entrances)) - reachable):
        known = listed(map_name)
        if known:
            notes[map_name] = {
                "kind": "unreviewed_script_entrance",
                "entrances": known[:MAX_LISTED_ENTRANCES],
                "entrance_count": len(known),
                "variables": note_variables(known[:MAX_LISTED_ENTRANCES], setters or {}),
            }
        elif map_name in maps:
            via = gateway(map_name)
            if via is None:
                notes[map_name] = {"kind": "no_known_entrance"}
            else:
                notes[map_name] = {
                    "kind": "behind_unreviewed_entrance",
                    "via": via,
                    "entrances": listed(via)[:MAX_LISTED_ENTRANCES],
                    "entrance_count": len(listed(via)),
                    "variables": note_variables(listed(via)[:MAX_LISTED_ENTRANCES], setters or {}),
                }
    return notes


def coverage(edges: list[dict]) -> dict:
    maps = {edge["from"]["map"] for edge in edges} | {edge["to"]["map"] for edge in edges}
    neighbors = {name: set() for name in maps}
    for edge in edges:
        source, target = edge["from"]["map"], edge["to"]["map"]
        neighbors[source].add(target)
        neighbors[target].add(source)
    components = 0
    unseen = set(maps)
    while unseen:
        components += 1
        pending = [unseen.pop()]
        while pending:
            current = pending.pop()
            for neighbor in neighbors[current] & unseen:
                unseen.remove(neighbor)
                pending.append(neighbor)
    return {"maps_with_edges": len(maps), "directed_edges": len(edges), "weakly_connected_components": components}


def main() -> int:
    client_root = Path(__file__).resolve().parents[1]
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--hercules-root", type=Path, default=client_root.parent / "Hercules")
    parser.add_argument("--output", type=Path, default=client_root / "korangar" / "data" / "navigation_graph.json")
    parser.add_argument(
        "--services",
        type=Path,
        default=client_root / "korangar" / "data" / "navigation_services.json",
        help="reviewed NPC/service edges authored outside static warp scripts",
    )
    parser.add_argument("--check", action="store_true", help="fail if the generated artifact differs from the existing file")
    args = parser.parse_args()

    root = args.hercules_root.resolve()
    entry = root / "npc/re/scripts_main.conf"
    index = root / "db/map_index.txt"
    if not entry.is_file() or not index.is_file():
        parser.error(f"Hercules root must contain {entry.relative_to(root)} and db/map_index.txt")

    files = loaded_script_files(root, entry)
    known_maps = map_names(index)
    edges, unsupported, errors = parse_warps(root, files, known_maps)
    services, service_errors = parse_authored_services(args.services, known_maps)
    errors.extend(service_errors)
    if errors:
        print("\n".join(errors), file=sys.stderr)
        return 1
    edges.extend(services)
    edges.sort(key=lambda edge: edge["id"])
    entrances = {
        map_name: found for map_name, found in script_entrances(root, files).items() if map_name in known_maps
    }
    used_maps = sorted({edge["from"]["map"] for edge in edges} | {edge["to"]["map"] for edge in edges})
    revision, dirty = source_revision(root)
    artifact = {
        "schema_version": 1,
        "hercules_revision": revision,
        "hercules_worktree_dirty": dirty,
        "maps": used_maps,
        "edges": edges,
        "access_notes": access_notes(edges, entrances, variable_setters(root, files)),
        "coverage": {
            **coverage(edges),
            "active_script_files_scanned": len(files),
            "unsupported_static_warps": unsupported,
            "authored_services": len(services),
            "coordinate_validation": "map names validated against db/map_index.txt; map cache unavailable to this generator",
        },
    }
    output = (json.dumps(artifact, ensure_ascii=False, indent=2, sort_keys=True) + "\n").encode()
    if args.check:
        if not args.output.is_file() or args.output.read_bytes() != output:
            print(f"stale navigation graph: {args.output}", file=sys.stderr)
            return 1
        print(f"navigation graph current: {args.output} ({len(edges)} edges)")
        return 0

    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_bytes(output)
    print(f"wrote {args.output}: {len(used_maps)} maps, {len(edges)} directed edges, {len(unsupported)} unsupported static warps")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
