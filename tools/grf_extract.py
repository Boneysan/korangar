#!/usr/bin/env python3
"""Extract a subtree of a GRF archive to a local folder.

Written for the asset-bundling work: pulling a curated slice of the 3 GB
`data.grf` into the loose `korangar/archive/` tree, which is already a
registered archive in `client/game_archives.ron`. `FolderArchive` lowercases
and swaps `/` for `\\` when it builds its lookup map, so extracting with
forward slashes round-trips correctly, Korean filenames included.

Usage:
  ./tools/grf_extract.py <archive.grf> <prefix> <outdir> [--dry-run]

<prefix> is matched case-insensitively against the GRF path.

  # size it first
  ./tools/grf_extract.py korangar/data.grf 'data\\sprite\\이팩트\\' \\
      korangar/archive --dry-run

  # the two effect families (see docs/plans/classic-effect-fidelity.md)
  ./tools/grf_extract.py korangar/data.grf 'data\\sprite\\이팩트\\' korangar/archive
  ./tools/grf_extract.py korangar/data.grf 'data\\texture\\effect\\' korangar/archive

!! LIMITATION — READ BEFORE RELYING ON THIS !!

This script does **not** implement GRF DES decryption, so it skips every
encrypted entry rather than writing it corrupt. That is not a rare edge case:
in `data.grf` roughly **45,000 of 151,000 entries are encrypted**
(flag 3 = ENCRYPT_MIXED: 34,228; flag 5 = ENCRYPT_HEADER: 11,150), including
most of `data\\sprite\\이팩트\\` — a dry run there reports 293 of 427 skipped.

Korangar itself handles these fine via
`korangar/src/loaders/archive/native/mixcrypt.rs` (`decrypt_file`). So for a
real extraction, **do not reimplement DES here** — drive Korangar's own
`GameFileLoader::get()`, which decrypts transparently, from a small Rust
binary or an `#[ignore]`d test. This script stays useful for sizing a subtree
and for extracting the plain (flag 1) majority.
"""
import os, struct, sys, zlib

FLAG_FILE = 0x01
FLAG_MIXED = 0x02
FLAG_DES_HEADER = 0x04


def read_table(path):
    with open(path, "rb") as fh:
        head = fh.read(0x2E)
        if not head[:15].startswith(b"Master of Magic"):
            raise SystemExit(f"{path}: not a GRF")
        offset, _seed, _cnt, version = struct.unpack("<IIII", head[0x1E:0x2E])
        if version != 0x200:
            raise SystemExit(f"{path}: unsupported version {version:#x}")
        fh.seek(0x2E + offset)
        comp_len, _uncomp_len = struct.unpack("<II", fh.read(8))
        return zlib.decompress(fh.read(comp_len))


def entries(table):
    pos, n = 0, len(table)
    while pos < n:
        end = table.find(b"\x00", pos)
        if end == -1:
            break
        name = table[pos:end].decode("cp949", errors="replace")
        pos = end + 1
        if pos + 17 > n:
            break
        comp_size, comp_aligned, real_size, flags, data_off = struct.unpack("<IIIBI", table[pos:pos + 17])
        pos += 17
        yield name, comp_size, comp_aligned, real_size, flags, data_off


def main():
    archive, prefix, outdir = sys.argv[1], sys.argv[2], sys.argv[3]
    dry = "--dry-run" in sys.argv
    prefix_l = prefix.lower()

    table = read_table(archive)
    selected = [e for e in entries(table) if e[0].lower().startswith(prefix_l)]

    total_real = sum(e[3] for e in selected)
    encrypted = [e for e in selected if e[4] & (FLAG_MIXED | FLAG_DES_HEADER)]
    print(f"{len(selected)} entries under {prefix!r}")
    print(f"uncompressed total: {total_real / 1024 / 1024:.1f} MiB")
    if encrypted:
        print(f"WARNING: {len(encrypted)} DES-encrypted entries will be skipped")
    if dry:
        return

    written = skipped = 0
    with open(archive, "rb") as fh:
        for name, comp_size, _aligned, real_size, flags, data_off in selected:
            if not flags & FLAG_FILE or flags & (FLAG_MIXED | FLAG_DES_HEADER):
                skipped += 1
                continue
            fh.seek(0x2E + data_off)
            blob = fh.read(comp_size)
            try:
                data = zlib.decompress(blob)
            except zlib.error:
                skipped += 1
                continue
            if len(data) != real_size:
                skipped += 1
                continue
            rel = name.replace("\\", "/")
            dest = os.path.join(outdir, rel)
            os.makedirs(os.path.dirname(dest), exist_ok=True)
            with open(dest, "wb") as out:
                out.write(data)
            written += 1

    print(f"wrote {written}, skipped {skipped}")


if __name__ == "__main__":
    main()
