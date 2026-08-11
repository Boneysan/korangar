#!/usr/bin/env bash
#
# Run the headless suite and KEEP THE LOG.
#
# WHY THIS EXISTS. `tools/audits/flaky.py` is the only thing in this repo that
# can answer "what does the suite do inconsistently", and flaky is where the
# remaining defects live — everything consistent is already asserted. It needs
# two or more full-run logs, and nothing was archiving any. The audit existed
# for a day before anyone noticed it had nothing to read.
#
# So: the same tester command, with timestamped console and JSON evidence under
# runs/. The wrapper also makes red-but-complete runs archival and keeps
# interrupted or targeted runs out of flaky.py's full-suite comparison glob.
#
#   tools/testing/run-suite.sh                 # --scenario all
#   tools/testing/run-suite.sh --shuffle 20260810
#   tools/testing/run-suite.sh --scenario skills-mage
#
# Then, once you have two or more:
#
#   tools/audits/flaky.py tools/testing/runs/*.log
#
# The servers must already be up (`Hercules/dev.sh start && dev.sh wait`).
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo="$(cd "$here/../.." && pwd)"
runs="${HEADLESS_RUNS_DIR:-$here/runs}"
mkdir -p "$runs"

# `--scenario all` unless the caller said otherwise. Checked on the whole
# argument list, not just $1, so `--shuffle <seed>` alone still runs everything.
#
# `${array[@]+"${array[@]}"}` rather than `"${array[@]}"`: macOS ships bash 3.2,
# where expanding an empty array under `set -u` is an error, and the obvious
# `"${array[@]-}"` fix expands to one EMPTY argument instead of none — which the
# tester then rejects as an unparseable scenario name.
arguments=(${@+"$@"})
case " ${arguments[*]-} " in
*" --scenario "*) ;;
*) arguments=(--scenario all ${arguments[@]+"${arguments[@]}"}) ;;
esac

# **Written as `.partial` and only renamed to `.log` when the run finishes.**
#
# `flaky.py` globs `*.log` and treats each as a complete run. An interrupted run
# — Ctrl-C, a killed background job, a server restart — otherwise leaves a file
# that looks exactly like a short one, and the audit silently compares a full run
# against a third of a run. That is the same silent-failure shape this whole
# directory exists to prevent, so the script must not depend on whoever killed it
# remembering to clean up.
log="$runs/$(date +%Y%m%d-%H%M%S)"
echo "logging to $log.log (kept as .partial until it completes)"

# Keep structured evidence next to every archived console log. A caller may
# supply its own destination (CI does), otherwise this wrapper owns the partial
# file and promotes it with the log only after a complete summary appears.
owns_results_json=true
case " ${arguments[*]-} " in
*" --results-json "*) owns_results_json=false ;;
*) arguments=(${arguments[@]+"${arguments[@]}"} --results-json "$log.results.partial.json") ;;
esac

cd "$repo"
# `tee` so a long run is still watchable, and the pipeline's exit status is the
# tester's. Temporarily disable `errexit`: otherwise a red-but-complete run exits
# from the pipeline before its log can be promoted into the evidence archive.
set +e
cargo run --release --example headless-tester -p korangar-networking -- "${arguments[@]}" 2>&1 | tee "$log.partial"
pipeline_status=("${PIPESTATUS[@]}")
set -e
runner_status=${pipeline_status[0]}

# A RED run is still a complete run, and flaky.py wants it — that is where the
# inconsistency lives. Only an *unfinished* one is disqualified, which is what
# the missing summary line means.
#
# A TARGETED run (`--scenario skills-mage`) is complete but is not a *run*: it
# covers one scenario, and letting it into the `*.log` archive means flaky.py
# compares a full sweep against a single job and calls the difference
# inconsistency. Kept as `.scoped` — the evidence is still on disk, it just is
# not cross-run material.
case " ${arguments[*]} " in
*" --scenario all "*) scope="log" ;;
*) scope="scoped" ;;
esac

if grep -q "=== Summary" "$log.partial"; then
    mv "$log.partial" "$log.$scope"
    if $owns_results_json && [ -f "$log.results.partial.json" ]; then
        mv "$log.results.partial.json" "$log.$scope.json"
    fi
    [ "$scope" = "scoped" ] && echo "targeted run — kept as $log.scoped, outside the cross-run archive"
else
    echo "run did not finish — left as $log.partial, which flaky.py will not read"
fi
exit "$runner_status"
