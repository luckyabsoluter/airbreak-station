#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RUNNER="${ROOT_DIR}/scripts/run-gui.sh"

RUN_FOR="${RUN_FOR:-1}"
HARD_TIMEOUT="${HARD_TIMEOUT:-450s}"
AIRBREAK_RUST_MAX_INSTRUCTIONS="${AIRBREAK_RUST_MAX_INSTRUCTIONS:-${AIRBREAK_RUST_CHECK_MAX_INSTRUCTIONS:-500000000}}"
AIRBREAK_RUST_CLEAN_RUN_ROOT="${AIRBREAK_RUST_CLEAN_RUN_ROOT:-0}"

export RUN_FOR HARD_TIMEOUT AIRBREAK_RUST_MAX_INSTRUCTIONS AIRBREAK_RUST_CLEAN_RUN_ROOT

set +e
OUTPUT="$("${RUNNER}" 2>&1)"
STATUS=$?
set -e

printf '%s\n' "${OUTPUT}"

RESULT_LINE="$(printf '%s\n' "${OUTPUT}" | grep -E 'result=(pass|fail)' | tail -n 1 || true)"
if [[ ${STATUS} -eq 0 && "${RESULT_LINE}" == *"result=pass"* ]]; then
  echo "gui_bringup=pass status=${STATUS} result=pass"
  exit 0
fi

if [[ -z "${RESULT_LINE}" ]]; then
  echo "gui_bringup=fail status=${STATUS} reason=no_result_line result=fail"
else
  echo "gui_bringup=fail status=${STATUS} runner_summary=${RESULT_LINE// /,} result=fail"
fi
exit 1
