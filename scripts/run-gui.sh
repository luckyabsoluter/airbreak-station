#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT_DIR}"

GUI_BACKEND="${GUI_BACKEND:-rust}"
if [[ "${GUI_BACKEND}" != "rust" ]]; then
  echo "run_gui=fail reason=unsupported_backend backend=${GUI_BACKEND} result=fail" >&2
  exit 2
fi

RUN_FOR="${RUN_FOR:-}"
HARD_TIMEOUT="${HARD_TIMEOUT:-}"
HARD_TIMEOUT_KILL_AFTER="${HARD_TIMEOUT_KILL_AFTER:-5s}"
FB_WIDTH="${FB_WIDTH:-240}"
FB_HEIGHT="${FB_HEIGHT:-320}"

source "${ROOT_DIR}/scripts/lib/rust_emulator_env.sh"

cleanup_and_exit() {
  local status="${1:-$?}"
  trap - EXIT TERM INT
  airbreak_rust_cleanup_run_root
  exit "${status}"
}

trap 'cleanup_and_exit $?' EXIT
trap 'cleanup_and_exit 143' TERM
trap 'cleanup_and_exit 130' INT

set +e
run_airbreak_rust_gui
STATUS=$?
set -e
cleanup_and_exit "${STATUS}"
