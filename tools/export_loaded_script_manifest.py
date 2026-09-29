#!/usr/bin/env python3
"""Export Hercules NPC scripts manifest with sha256 hashes for Adventure Guide.

Purpose: Create a deterministic manifest of all loaded Hercules NPC scripts,
including their sha256 hashes, plus dirty file tracking for both korangar/
and Hercules/ repositories. Used as baseline data for the E0 package and
encyclopedia coverage denominators.

Manifest contents:
- schema_version: 1
- source_revision: git HEAD hash from each repo (korangar, hercules)
- source_worktree_dirty: boolean indicating if either repo has uncommitted changes
- mode: "renewal" (the active server configuration)
- loaded_scripts: list of all .txt files actually loaded via scripts.conf
- dirty_files: list of paths in both repos that have uncommitted changes

Usage:
    tools/export_loaded_script_manifest.py [--check]

    --check  exit 1 if docs/loaded-script-manifest.v1.json is stale (for CI)

Source: Hercules npc/scripts.conf and the npc/scripts_*.conf includes actually
        loaded by the active server (renewal mode).
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import subprocess
import sys
from pathlib import Path


REPO = Path(__file__).resolve().parent.parent
HERCULES = REPO.parent / "Hercules"
OUTPUT = REPO / "docs" / "loaded-script-manifest.v1.json"

# Regex patterns for parsing scripts.conf format
INCLUDE = re.compile(r'^\s*@include\s+"([^"]+)"')
SCRIPT_FILE = re.compile(r'"([^"]+\.txt)"')


def source_revision(root: Path) -> tuple[str, bool]:
    """Get git HEAD and dirty status for a repository."""
    try:
        revision = subprocess.check_output(
            ["git", "-C", str(root), "rev-parse", "HEAD"],
            text=True,
            stderr=subprocess.DEVNULL
        ).strip()
        dirty_output = subprocess.check_output(
            ["git", "-C", str(root), "status", "--porcelain"],
            text=True
        ).strip()
        dirty = bool(dirty_output)
        return revision, dirty
    except (OSError, subprocess.CalledProcessError):
        return "unknown", False


def sha256_of_file(path: Path) -> str:
    """Compute sha256 hash of a file."""
    hasher = hashlib.sha256()
    with open(path, 'rb') as f:
        for chunk in iter(lambda: f.read(8192), b''):
            hasher.update(chunk)
    return hasher.hexdigest()


def ordered_script_files(root: Path, entry: Path) -> list[Path]:
    """Follow @include manifests and collect all loaded .txt files.

    This follows the same pattern as generate_navigation_graph.py and
    export_map_flags.py to ensure consistency.
    """
    visited_configs: set[Path] = set()
    scripts: list[Path] = []

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
                if path.is_file() and path not in scripts:
                    scripts.append(path)

    visit(entry)
    return sorted(scripts)


def build_manifest(korangar: Path, hercules: Path) -> dict:
    """Build the complete manifest dictionary."""
    # Get source revisions for both repos
    korangar_rev, korangar_dirty = source_revision(korangar)
    hercules_rev, hercules_dirty = source_revision(hercules)
    any_dirty = korangar_dirty or hercules_dirty

    # Collect loaded script files from Hercules
    entry_config = hercules / "npc/re/scripts_main.conf"
    if not entry_config.is_file():
        raise SystemExit(f"Hercules scripts_main.conf not found: {entry_config}")

    loaded_scripts = ordered_script_files(hercules, entry_config)
    loaded_script_list = [
        {
            "path": str(path.relative_to(hercules).as_posix()),
            "sha256": sha256_of_file(path),
        }
        for path in loaded_scripts
    ]

    # Collect dirty files from both repos
    dirty_files = []

    if korangar_dirty:
        korangar_status = subprocess.check_output(
            ["git", "-C", str(korangar), "status", "--porcelain"],
            text=True
        ).strip()
        for line in korangar_status.splitlines():
            # Format: "XY path" where X=Y status chars (M=modified, A=added, D=deleted, etc.)
            # First char is index status, second is working tree status
            parts = line.split(maxsplit=1)
            if len(parts) == 2:
                status_code = parts[0]
                file_status = status_code[1] if len(status_code) > 1 else status_code[0]
                dirty_files.append({
                    "repo": "korangar",
                    "path": parts[1],
                    "sha256": sha256_of_file(korangar / parts[1]) if file_status != 'D' else "N/A",
                })

    if hercules_dirty:
        hercules_status = subprocess.check_output(
            ["git", "-C", str(hercules), "status", "--porcelain"],
            text=True
        ).strip()
        for line in hercules_status.splitlines():
            parts = line.split(maxsplit=1)
            if len(parts) == 2:
                status_code = parts[0]
                file_status = status_code[1] if len(status_code) > 1 else status_code[0]
                # Skip files that don't exist (deleted)
                file_path = hercules / parts[1]
                dirty_files.append({
                    "repo": "hercules",
                    "path": parts[1],
                    "sha256": sha256_of_file(file_path) if file_status != 'D' else "N/A",
                })

    return {
        "schema_version": 1,
        "source_revision": {
            "korangar": korangar_rev,
            "hercules": hercules_rev,
        },
        "source_worktree_dirty": any_dirty,
        "mode": "renewal",
        "loaded_scripts": loaded_script_list,
        "dirty_files": dirty_files,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="exit 1 if the export is stale")
    args = parser.parse_args()

    manifest = build_manifest(REPO, HERCULES)
    rendered = json.dumps(manifest, indent=2, ensure_ascii=False) + "\n"

    if args.check:
        current = OUTPUT.read_text(encoding="utf-8") if OUTPUT.exists() else ""
        if current != rendered:
            print(f"stale manifest: {OUTPUT}", file=sys.stderr)
            return 1
        loaded_count = len(manifest["loaded_scripts"])
        dirty_count = len(manifest["dirty_files"])
        print(f"manifest current: {OUTPUT} ({loaded_count} scripts, {dirty_count} dirty files)")
        return 0

    OUTPUT.write_text(rendered, encoding="utf-8")
    loaded_count = len(manifest["loaded_scripts"])
    dirty_count = len(manifest["dirty_files"])
    korangar_dirty = manifest["source_worktree_dirty"]
    print(f"wrote {OUTPUT}: {loaded_count} loaded scripts, {dirty_count} dirty files ({'korangar dirty' if korangar_dirty else 'both repos clean'})")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
