#!/usr/bin/env python3
"""Is the Guide's reference data still what the server would export today?

The Adventure Guide shows odds, rates and rules exported from Hercules at some
revision. If the server's config or scripts change and the export is not re-run,
the client confidently shows stale numbers. This answers, in one command:

  PROVENANCE  every docs/*.v1.json names the Hercules revision it came from;
              do they agree with each other and with Hercules HEAD, and was the
              tree dirty when they were exported
  CONTENT     every exporter's own `--check`, with a dirty working tree treated
              as clean (KORANGAR_EXPORT_IGNORE_DIRTY=1), so an unrelated
              uncommitted server change is not reported as drift but a changed
              exported value is

Exit status 1 only for CONTENT drift or exports from different revisions. A dirty
tree and a revision behind HEAD are reported, not failed: they are normal
mid-development and `--strict` makes them fail for a release.

    python3 tools/check_reference_drift.py [--strict]
"""

import argparse
import glob
import json
import os
import subprocess
import sys

TOOLS = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(TOOLS)
HERCULES = os.path.join(os.path.dirname(ROOT), 'Hercules')


def hercules_head():
    try:
        return subprocess.check_output(['git', '-C', HERCULES, 'rev-parse', 'HEAD'], text=True, stderr=subprocess.DEVNULL).strip()
    except (OSError, subprocess.CalledProcessError):
        return None


def provenance():
    """(file, revision, dirty) for every export that records one."""
    rows = []
    for path in sorted(glob.glob(os.path.join(ROOT, 'docs', '*.v1.json'))):
        try:
            with open(path, encoding='utf-8') as handle:
                data = json.load(handle)
        except (OSError, ValueError):
            continue
        if not (isinstance(data, dict) and 'source_revision' in data):
            continue
        revision = data['source_revision']
        # The loaded-script manifest records both repositories; its Hercules half is the one that matters here.
        if isinstance(revision, dict):
            revision = revision.get('hercules')
        if isinstance(revision, str):
            rows.append((os.path.basename(path), revision, bool(data.get('source_worktree_dirty', False))))
    return rows


def exporters():
    found = []
    for path in sorted(glob.glob(os.path.join(TOOLS, 'export_*.py'))):
        with open(path, encoding='utf-8', errors='replace') as handle:
            if '--check' in handle.read():
                found.append(path)
    return found


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--strict', action='store_true', help='also fail on a dirty export or a revision behind Hercules HEAD')
    args = parser.parse_args()

    failures, notes = [], []
    head = hercules_head()
    rows = provenance()

    revisions = sorted({revision for _, revision, _ in rows})
    print('PROVENANCE: %d exports record a revision; Hercules HEAD is %s' % (len(rows), head[:10] if head else 'unknown'))
    if len(revisions) > 1:
        failures.append('exports come from %d different Hercules revisions: %s' % (len(revisions), ', '.join(r[:10] for r in revisions)))
    behind = [name for name, revision, _ in rows if head and revision != head]
    if behind:
        notes.append('%d export(s) were read from a revision other than HEAD: %s' % (len(behind), ', '.join(behind[:6])))
    dirty = [name for name, _, was_dirty in rows if was_dirty]
    if dirty:
        notes.append('%d export(s) were exported from a dirty server tree: %s' % (len(dirty), ', '.join(dirty[:6])))

    env = dict(os.environ, KORANGAR_EXPORT_IGNORE_DIRTY='1')
    stale = []
    for path in exporters():
        run = subprocess.run([sys.executable, path, '--check'], capture_output=True, text=True, cwd=ROOT, env=env)
        if run.returncode != 0:
            message = (run.stderr + run.stdout).strip().splitlines()
            stale.append('%s: %s' % (os.path.basename(path), message[-1][:140] if message else 'failed'))
    print('CONTENT: %d exporters checked, %d stale' % (len(exporters()), len(stale)))
    failures.extend(stale)

    for note in notes:
        print('note: ' + note)
        if args.strict:
            failures.append(note)
    if failures:
        print('FAIL - %d problem(s):' % len(failures), file=sys.stderr)
        for failure in failures:
            print('  ' + failure, file=sys.stderr)
        return 1
    print('OK - the Guide data matches what the server exports today.')
    return 0


if __name__ == '__main__':
    sys.exit(main())
