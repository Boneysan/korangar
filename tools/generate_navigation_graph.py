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
    if errors:
        print("\n".join(errors), file=sys.stderr)
        return 1
    used_maps = sorted({edge["from"]["map"] for edge in edges} | {edge["to"]["map"] for edge in edges})
    revision, dirty = source_revision(root)
    artifact = {
        "schema_version": 1,
        "hercules_revision": revision,
        "hercules_worktree_dirty": dirty,
        "maps": used_maps,
        "edges": edges,
        "coverage": {
            **coverage(edges),
            "active_script_files_scanned": len(files),
            "unsupported_static_warps": unsupported,
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
