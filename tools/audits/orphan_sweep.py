#!/usr/bin/env python3
"""Sweep for work that exists on one side of the project and not the other.

Born from 2026-10-02: Hercules generators kept expecting korangar files that
`main` never got, and a server packet (ZC_RECOVERY_STATE) was registered as a
no-op while the server sent it. Each is invisible to a check that only looks at
its own side. This lists four kinds of one-sided work. It reports; it never
writes or fixes.

  1. DATA      client data files (tsv/json/ron) that no Rust code or tool references
  2. PACKETS   fork-added server->client packets the client registers as no-ops
  3. GENERATORS Hercules `gen-*.py --check` results (never runs a writing mode)
  4. DOCS      file paths named in `docs/` that exist nowhere in either repo

Exit status is 0 whatever it finds: the point is the list, and several entries
are intentional (documented decisions). Triage them in the plan file.

    python3 tools/audits/orphan_sweep.py
"""

import glob
import os
import re
import subprocess
import sys

KORANGAR = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
WORKSPACE = os.path.dirname(KORANGAR)
HERCULES = os.path.join(WORKSPACE, 'Hercules')
SRC = os.path.join(KORANGAR, 'korangar', 'src')


def read(path):
    with open(path, encoding='utf-8', errors='replace') as handle:
        return handle.read()


def rust_sources():
    found = []
    for crate in glob.glob(os.path.join(KORANGAR, 'korangar*')) + glob.glob(os.path.join(KORANGAR, 'ragnarok-*')):
        for root, dirs, files in os.walk(crate):
            dirs[:] = [d for d in dirs if d != 'target']
            found += [os.path.join(root, f) for f in files if f.endswith('.rs')]
    return found


def tool_text():
    """Python/shell tools that read client data (e.g. a generator's input file)."""
    text = []
    for root in (os.path.join(KORANGAR, 'tools'), os.path.join(HERCULES, 'tools')):
        for dirpath, dirs, files in os.walk(root):
            dirs[:] = [d for d in dirs if d not in ('__pycache__', 'target')]
            # A generator that *writes* a file is not a reader of it, so gen-*/export_* do not count.
            text += [
                read(os.path.join(dirpath, f))
                for f in files
                if f.endswith(('.py', '.sh')) and not f.startswith(('gen-', 'gen_', 'export_'))
            ]
    return '\n'.join(text)


def data_files_without_a_reader(rust_text):
    rust_text = rust_text + '\n' + tool_text()
    candidates = []
    for directory in (os.path.join(SRC, 'world', 'library'), os.path.join(KORANGAR, 'korangar', 'data')):
        for name in sorted(os.listdir(directory)) if os.path.isdir(directory) else []:
            if name.endswith(('.tsv', '.json', '.ron', '.csv')) and name not in rust_text:
                candidates.append(os.path.relpath(os.path.join(directory, name), KORANGAR))
    return candidates


def noop_fork_packets():
    header = os.path.join(HERCULES, 'src', 'common', 'packets_len.h')
    handler = os.path.join(KORANGAR, 'korangar-networking', 'src', 'packet_versions', 'version_20220406.rs')
    if not (os.path.exists(header) and os.path.exists(handler)):
        return ['(skipped: packets_len.h or the packet handler is missing)']
    # "// 0x0EFD ZC_RECOVERY_STATE — ..." comments name the fork's own packets.
    fork = re.findall(r'//\s*0x[0-9A-Fa-f]{4}\s+(ZC_[A-Z_]+)', read(header))
    noops = set(re.findall(r'register_noop::<(\w+)>', read(handler)))
    out = []
    for wire_name in fork:
        # ZC_RECOVERY_STATE -> RecoveryStatePacket
        rust_name = ''.join(part.capitalize() for part in wire_name[3:].split('_')) + 'Packet'
        if rust_name in noops:
            out.append('%s is a server-sent fork packet the client registers as a no-op (%s)' % (wire_name, rust_name))
    return out


def generator_results():
    out = []
    tools = os.path.join(HERCULES, 'tools')
    for path in sorted(glob.glob(os.path.join(tools, 'gen-*.py'))):
        if '--check' not in read(path):
            out.append('%s: no --check mode, not run' % os.path.basename(path))
            continue
        run = subprocess.run([sys.executable, path, '--check'], capture_output=True, text=True, cwd=HERCULES)
        text = (run.stdout + run.stderr).strip().splitlines()
        if run.returncode != 0:
            out.append('%s: FAIL - %s' % (os.path.basename(path), text[-1][:150] if text else 'no output'))
    return out


PATH_RE = re.compile(r'`([A-Za-z0-9_./-]+\.(?:tsv|json|py|sh|rs|md|conf|txt|sql))`')
SKIP = ('*', '<', 'target/', '~', 'client/', 'client2/', 'passes/', 'xxx')


def doc_paths_that_do_not_exist():
    bases = [KORANGAR, os.path.join(KORANGAR, 'docs'), os.path.join(KORANGAR, 'docs', 'plans'), os.path.join(KORANGAR, 'docs', 'specs'),
             os.path.join(KORANGAR, 'korangar'), SRC, WORKSPACE, HERCULES, os.path.join(SRC, 'world', 'library'),
             os.path.join(KORANGAR, 'korangar-networking', 'examples', 'headless-tester'),
             os.path.join(KORANGAR, 'korangar-networking', 'examples', 'headless-tester', 'scenarios')]
    missing = {}
    docs = glob.glob(os.path.join(KORANGAR, 'docs', 'plans', '*.md')) + glob.glob(os.path.join(KORANGAR, 'docs', 'specs', '*.md'))
    docs.append(os.path.join(KORANGAR, 'docs', 'GDD.md'))
    # The sweep's own report quotes the stale paths it found; do not count those.
    docs = [doc for doc in docs if not os.path.basename(doc).startswith('missing-files-sweep')]
    for doc in docs:
        for number, line in enumerate(read(doc).splitlines(), 1):
            for path in PATH_RE.findall(line):
                if '/' not in path or any(s in path for s in SKIP):
                    continue
                if any(os.path.exists(os.path.join(base, path)) for base in bases):
                    continue
                missing.setdefault(path, []).append('%s:%d' % (os.path.basename(doc), number))
    return ['%s  (%s%s)' % (path, where[0], ' +%d' % (len(where) - 1) if len(where) > 1 else '') for path, where in sorted(missing.items())]


def main():
    rust_text = '\n'.join(read(path) for path in rust_sources())
    sections = [
        ('DATA: client data files no Rust code references', data_files_without_a_reader(rust_text)),
        ('PACKETS: fork packets the client ignores', noop_fork_packets()),
        ('GENERATORS: Hercules generators that fail --check', generator_results()),
        ('DOCS: paths named in docs/ that exist nowhere', doc_paths_that_do_not_exist()),
    ]
    for title, items in sections:
        print('## %s (%d)' % (title, len(items)))
        for item in items:
            print('  ' + item)
    return 0


if __name__ == '__main__':
    sys.exit(main())
