#!/usr/bin/env python3
"""Find what the suite does *inconsistently*, across runs rather than within one.

WHY THIS EXISTS. On 2026-08-09 five skills across four jobs were found to go
silent intermittently — `MG_NAPALMBEAT`, `MG_SAFETYWALL`, `HT_SHOCKWAVE`,
`HP_BASILICA`, `SL_SMA`. Each looked like an isolated oddity in the run that
showed it. They were only visible as a *cluster* by comparing eight run logs,
which was done by hand with throwaway Python. That analysis was the single most
useful thing done to the suite that day, and it existed nowhere afterwards.

It also settled two arguments no single run could:

  * an allowlist cull based on ONE run would have removed three load-bearing
    entries and made the suite intermittently red;
  * a scenario that ran 4x slower than usual while still passing was variance,
    not the regression it looked like.

**A single run cannot tell you what is flaky, and flaky is where this suite's
remaining defects live.** Everything consistent is already asserted; what is
left is the stuff that only fails sometimes.

Usage:
  tools/audits/flaky.py <run-log> [<run-log> ...]
  tools/audits/flaky.py /path/to/logs/*.log --min-runs 3

Feed it whatever full-run logs you have. Two runs is enough to spot something;
more is better. Exits non-zero if anything is inconsistent, so it can gate a
release even when every individual run was green.
"""

import argparse
import collections
import pathlib
import re
import sys

ANSI = re.compile(r"\x1b\[[0-9;]*m")
# One row of a job sweep's per-skill table: id, name, type, level, outcome.
SKILL_ROW = re.compile(r"^\s+\d+\s+([A-Z][A-Z0-9_]+)\s+(\S+)\s+\S+\s+(.+)$", re.M)
# `[PASS] scenario-name (12.3s)`
SCENARIO = re.compile(r"\]\s*(\S+) \(([0-9.]+)s\)")
VERDICT = re.compile(r"\[(PASS|FAIL|SKIP|KNOWN-FAIL)\]\s*(\S+?):?\s", re.M)


def read(path: pathlib.Path) -> tuple[dict[str, str], dict[str, float], dict[str, str]]:
    text = ANSI.sub("", path.read_text(errors="replace"))
    skills = {name: outcome.strip() for name, _type, outcome in SKILL_ROW.findall(text)}
    durations = {name: float(seconds) for name, seconds in SCENARIO.findall(text)}
    verdicts = {name: verdict for verdict, name in VERDICT.findall(text)}
    return skills, durations, verdicts


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("logs", nargs="+", type=pathlib.Path)
    parser.add_argument("--min-runs", type=int, default=2, help="ignore anything seen in fewer runs than this")
    parser.add_argument("--slow-factor", type=float, default=2.5, help="flag a scenario whose slowest run exceeds its median by this")
    args = parser.parse_args()

    skill_runs = collections.defaultdict(dict)
    duration_runs = collections.defaultdict(dict)
    verdict_runs = collections.defaultdict(dict)
    usable = []
    for path in args.logs:
        if not path.is_file():
            continue
        skills, durations, verdicts = read(path)
        if not skills and not verdicts:
            continue  # not a run log
        usable.append(path.name)
        for name, outcome in skills.items():
            skill_runs[name][path.name] = outcome
        for name, seconds in durations.items():
            duration_runs[name][path.name] = seconds
        for name, verdict in verdicts.items():
            verdict_runs[name][path.name] = verdict

    if not usable:
        raise SystemExit("error: none of those files look like run logs")
    print(f"flaky audit — {len(usable)} run(s): {', '.join(sorted(usable))}\n")

    findings = 0

    unstable_verdict = {
        name: runs for name, runs in verdict_runs.items() if len(runs) >= args.min_runs and len(set(runs.values())) > 1
    }
    if unstable_verdict:
        findings += len(unstable_verdict)
        print(f"SCENARIOS THAT DID NOT AGREE WITH THEMSELVES ({len(unstable_verdict)}):")
        for name, runs in sorted(unstable_verdict.items()):
            tally = collections.Counter(runs.values())
            print(f"  {name:32} {dict(tally)}")
        print()

    unstable_skill = {
        name: runs for name, runs in skill_runs.items() if len(runs) >= args.min_runs and len(set(runs.values())) > 1
    }
    # Silence is the outcome that matters: everything else is a different shade
    # of "the server answered".
    silent = {name: runs for name, runs in unstable_skill.items() if any("silent" in v or "SILENT" in v for v in runs.values())}
    if silent:
        findings += len(silent)
        print(f"SKILLS THAT GO SILENT ONLY SOMETIMES ({len(silent)}) — the defects live here:")
        for name, runs in sorted(silent.items()):
            quiet = sorted(run for run, outcome in runs.items() if "ilent" in outcome or "SILENT" in outcome)
            other = sorted({outcome for outcome in runs.values() if "ilent" not in outcome and "SILENT" not in outcome})
            print(f"  {name:22} silent in {len(quiet)}/{len(runs)} runs, otherwise {other}")
        print()

    other_unstable = {name: runs for name, runs in unstable_skill.items() if name not in silent}
    if other_unstable:
        print(f"skills whose outcome merely varies ({len(other_unstable)}) — usually preconditions, not defects:")
        for name, runs in sorted(other_unstable.items())[:15]:
            print(f"  {name:22} {sorted(set(runs.values()))}")
        if len(other_unstable) > 15:
            print(f"  ... and {len(other_unstable) - 15} more")
        print()

    slow = []
    for name, runs in duration_runs.items():
        if len(runs) < max(args.min_runs, 3):
            continue
        values = sorted(runs.values())
        median = values[len(values) // 2]
        worst = max(values)
        if median > 0 and worst / median >= args.slow_factor:
            slow.append((name, median, worst, worst / median))
    if slow:
        findings += len(slow)
        print(f"SCENARIOS THAT SOMETIMES RUN FAR SLOWER ({len(slow)}) — passes hide these:")
        for name, median, worst, factor in sorted(slow, key=lambda row: -row[3]):
            print(f"  {name:32} median {median:6.1f}s   worst {worst:6.1f}s   {factor:.1f}x")
        print()

    if not findings:
        print("nothing inconsistent across these runs")
        return 0
    print(f"{findings} inconsistency finding(s). A single green run cannot see any of these.")
    return 1


if __name__ == "__main__":
    sys.exit(main())
