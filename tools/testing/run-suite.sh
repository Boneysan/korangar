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
# So: same command as before, plus a timestamped log under runs/. That is the
# whole feature. It is a directory and a redirect, deliberately — a wrapper that
# grows options becomes a thing to learn instead of a thing to use.
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
runs="$here/runs"
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

log="$runs/$(date +%Y%m%d-%H%M%S).log"
echo "logging to $log"

cd "$repo"
# `tee` so a long run is still watchable, and the pipeline's exit status is the
# tester's — `set -o pipefail` above, or a red run would exit 0 through tee.
cargo run --release --example headless-tester -p korangar-networking -- "${arguments[@]}" 2>&1 | tee "$log"
