#!/usr/bin/env python3
"""Dump the file table of a GRF archive (v2xx). Filenames are CP949.

Use this instead of Korangar's `GameFileLoader::get_files_with_extension`,
which **under-reports the GRFs badly** — it listed 26 root-level `.str` files
where `data.grf` actually holds 311, and omitted files the client provably
loads (`stormgust.str`, `sanctuary.str`). For a yes/no on a single path,
probing with `file_exists` is reliable; for inventory, use this.

    ./tools/grf_list.py korangar/data.grf                      # everything
    ./tools/grf_list.py korangar/data.grf 'texture\\effect'      # filtered

data.grf holds ~151k entries, ~2,070 of them `.str`.
See docs/plans/classic-effect-fidelity.md.
"""
import struct, sys, zlib

def entries(path):
    with open(path, "rb") as fh:
        data = fh.read(0x2E)
        sig = data[:15]
        if not sig.startswith(b"Master of Magic"):
            raise SystemExit(f"{path}: not a GRF ({sig!r})")
        offset, seed, filecount_raw, version = struct.unpack("<IIII", data[0x1E:0x2E])
        if version not in (0x200, 0x103, 0x102):
            raise SystemExit(f"{path}: unsupported GRF version {version:#x}")
        fh.seek(0x2E + offset)
        comp_len, uncomp_len = struct.unpack("<II", fh.read(8))
        table = zlib.decompress(fh.read(comp_len))

    pos = 0
    n = len(table)
    while pos < n:
        end = table.find(b"\x00", pos)
        if end == -1:
            break
        name = table[pos:end]
        pos = end + 1
        if pos + 17 > n:
            break
        pos += 17
        yield name.decode("cp949", errors="replace")

if __name__ == "__main__":
    archive = sys.argv[1]
    needle = sys.argv[2].lower() if len(sys.argv) > 2 else ""
    count = 0
    for name in entries(archive):
        if needle in name.lower():
            print(name)
            count += 1
    print(f"--- {count} matching entries", file=sys.stderr)
