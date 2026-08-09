#!/usr/bin/env python3
"""Catch the client registering a *stale variant* of a packet family.

THE FAILURE THIS EXISTS FOR IS INVISIBLE. Many `ZC_` families changed header
across packetvers — `idle_unitType` alone is 0x78 / 0x1d8 / 0x22a / 0x2ee /
0x7f9 / 0x857 / 0x915 / 0x9dd / 0x9ff depending on PACKETVER. If the client
registers the wrong one for the server's build, nothing errors: the real packet
is simply never matched, falls through `register_length_fallbacks`, is consumed
cleanly, and the feature goes quiet. No ledger entry, no deserialization
failure, no test failure — the packet is *framed* correctly, just never handled.

The audit was run by hand on 2026-08-08 (clean across 217 active headers) and
recorded as "worth re-running after any PACKETVER change or Hercules merge",
which is exactly the kind of instruction that stops being followed. This is that
check, mechanised.

METHOD, and the reason it is not a grep: the `#if` chains cannot be read by eye
without getting them wrong, so the **C preprocessor decides** which variant is
active — the same trick `tools/generate_packet_lengths.sh` uses. Reading the
`#if` branches manually is how a stale variant gets certified as correct.

  1. preprocess `packets_struct.h` at the server's PACKETVER -> the ACTIVE id
     for every family in `enum packet_headers`;
  2. parse the same header as raw text -> EVERY id each family can take;
  3. scrape `#[header(0x....)]` from the client's packet definitions;
  4. a family whose client registration is a *non-active* variant is a
     MISMATCH. Registering nothing at all is fine — that is an unmodelled
     packet, which the length fallback handles by design.

Usage:
  tools/audits/packetver-variants.py [--hercules DIR] [--packetver N] [--variant main|re|zero|sak|ad]

Exits non-zero if any mismatch is found, so it can gate a merge.
"""

import argparse
import pathlib
import re
import subprocess
import sys
import tempfile

ENUM_NAME = "packet_headers"
VARIANT_DEFINES = {"main": [], "re": ["-DPACKETVER_RE"], "zero": ["-DPACKETVER_ZERO"], "sak": ["-DPACKETVER_SAK"], "ad": ["-DPACKETVER_AD"]}
ASSIGN = re.compile(r"^\s*(\w+)\s*=\s*(0[xX][0-9a-fA-F]+|\d+)\s*,?\s*$")
ALIAS = re.compile(r"^\s*(\w+)\s*=\s*(\w+)\s*,?\s*$")


def enum_body(text: str) -> str:
    """The text between `enum packet_headers {` and its matching brace."""
    start = text.index(f"enum {ENUM_NAME}")
    open_brace = text.index("{", start)
    depth, index = 0, open_brace
    while index < len(text):
        if text[index] == "{":
            depth += 1
        elif text[index] == "}":
            depth -= 1
            if depth == 0:
                return text[open_brace + 1 : index]
        index += 1
    raise SystemExit(f"error: unterminated `enum {ENUM_NAME}`")


def active_headers(hercules: pathlib.Path, packetver: int, variant: str) -> dict[str, int]:
    """Ask the preprocessor which variant of each family is live."""
    with tempfile.NamedTemporaryFile("w", suffix=".c", delete=False) as stub:
        stub.write('#include "map/packets_struct.h"\n')
        stub_path = stub.name

    # `cc -E` rather than `cpp`: macOS's cpp is a wrapper that mangles arguments.
    # Unlike `generate_packet_lengths.sh` this must NOT pass `-nostdinc`:
    # `packets_struct.h` pulls in `cbasetypes.h`, which needs `<inttypes.h>`.
    result = subprocess.run(
        ["cc", "-E", "-x", "c", "-P", f"-DPACKETVER={packetver}", *VARIANT_DEFINES[variant], "-I", str(hercules / "src"), stub_path],
        capture_output=True,
        text=True,
    )
    pathlib.Path(stub_path).unlink(missing_ok=True)
    if result.returncode != 0:
        raise SystemExit(f"error: preprocessing failed:\n{result.stderr.strip()[:2000]}")

    headers: dict[str, int] = {}
    for line in enum_body(result.stdout).splitlines():
        match = ASSIGN.match(line)
        if match:
            headers[match.group(1)] = int(match.group(2), 0)
            continue
        alias = ALIAS.match(line)
        # `status_changeType = sc_notickType` — resolve against what we have.
        if alias and alias.group(2) in headers:
            headers[alias.group(1)] = headers[alias.group(2)]
    return headers


def variant_sets(hercules: pathlib.Path) -> dict[str, set[int]]:
    """Every id a family takes anywhere in the `#if` chain, guards ignored."""
    text = (hercules / "src/map/packets_struct.h").read_text(errors="replace")
    families: dict[str, set[int]] = {}
    for line in enum_body(text).splitlines():
        match = ASSIGN.match(line)
        if match:
            families.setdefault(match.group(1), set()).add(int(match.group(2), 0))
    return families


def client_headers(korangar: pathlib.Path) -> set[int]:
    found: set[int] = set()
    for path in (korangar / "ragnarok-packets/src").rglob("*.rs"):
        for match in re.finditer(r"#\[header\((0[xX][0-9a-fA-F]+)\)\]", path.read_text(errors="replace")):
            found.add(int(match.group(1), 0))
    if not found:
        raise SystemExit("error: no `#[header(0x....)]` found — has the packet macro changed shape?")
    return found


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    here = pathlib.Path(__file__).resolve().parent
    parser.add_argument("--hercules", type=pathlib.Path, default=here.parents[2] / "Hercules")
    parser.add_argument("--korangar", type=pathlib.Path, default=here.parents[1])
    parser.add_argument("--packetver", type=int, default=20220406)
    parser.add_argument("--variant", choices=sorted(VARIANT_DEFINES), default="main")
    args = parser.parse_args()

    if not (args.hercules / "src/map/packets_struct.h").is_file():
        raise SystemExit(f"error: {args.hercules} does not look like a Hercules checkout")

    active = active_headers(args.hercules, args.packetver, args.variant)
    families = variant_sets(args.hercules)
    registered = client_headers(args.korangar)

    mismatches, checked, unmodelled = [], 0, 0
    for family, ids in sorted(families.items()):
        if len(ids) < 2 or family not in active:
            continue  # single-variant families cannot go stale
        checked += 1
        live = active[family]
        if live in registered:
            continue
        stale = sorted(ids & registered - {live})
        if stale:
            mismatches.append((family, live, stale))
        else:
            unmodelled += 1

    print(f"packetver-variant audit — PACKETVER={args.packetver}, variant={args.variant}")
    print(f"  {checked} multi-variant families, {len(registered)} client headers, {unmodelled} not modelled (fine)")

    if not mismatches:
        print("  no stale variants")
        return 0

    print(f"\n  {len(mismatches)} STALE VARIANT(S) — these packets arrive and are silently consumed:")
    for family, live, stale in mismatches:
        stale_text = ", ".join(f"0x{value:04X}" for value in stale)
        print(f"    {family}: client registers {stale_text}, but 0x{live:04X} is active at this packetver")
    return 1


if __name__ == "__main__":
    sys.exit(main())
