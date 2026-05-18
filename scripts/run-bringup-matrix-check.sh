#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
GUI_CHECK="${ROOT_DIR}/scripts/run-gui-bringup-check.sh"

CASES="${CASES:-non_touch}"
FAIL_FAST="${FAIL_FAST:-0}"
FAILED=0
CHECKS_RUN=0
FAILED_CASES=()
SELECTED_CASES=()

append_case_unique() {
  local name="$1"
  local existing=""
  for existing in "${SELECTED_CASES[@]}"; do
    [[ "${existing}" == "${name}" ]] && return
  done
  SELECTED_CASES+=("${name}")
}

expand_case_token() {
  local token="$1"
  case "${token}" in
    all|default|non_touch|gui|gui_non_touch)
      append_case_unique "gui_non_touch"
      ;;
    *)
      echo "invalid CASES entry: ${token}" >&2
      echo "valid cases: all default non_touch gui gui_non_touch" >&2
      exit 2
      ;;
  esac
}

run_case() {
  local name="$1"
  local status=0
  local output=""
  local line=""

  set +e
  output="$("${GUI_CHECK}" 2>&1)"
  status=$?
  set -e

  printf '%s\n' "${output}"
  line="$(printf '%s\n' "${output}" | grep -E 'result=(pass|fail)' | tail -n 1 || true)"
  [[ -z "${line}" ]] && line="result=fail reason=no_result_line"
  printf 'check=%s status=%d %s\n' "${name}" "${status}" "${line}"

  if [[ ${status} -ne 0 ]]; then
    FAILED=$((FAILED + 1))
    FAILED_CASES+=("${name}")
    if [[ "${FAIL_FAST}" == "1" ]]; then
      echo "bringup_matrix=fail failed=${FAILED} failed_cases=$(IFS=,; echo "${FAILED_CASES[*]}") result=fail"
      exit 1
    fi
  fi
  CHECKS_RUN=$((CHECKS_RUN + 1))
}

IFS=' ' read -r -a selected_cases <<< "$(printf '%s' "${CASES}" | tr ',' ' ')"
if [[ ${#selected_cases[@]} -eq 0 ]]; then
  expand_case_token "non_touch"
else
  for name in "${selected_cases[@]}"; do
    [[ -z "${name}" ]] && continue
    expand_case_token "${name}"
  done
fi

for name in "${SELECTED_CASES[@]}"; do
  run_case "${name}"
done

if [[ ${FAILED} -ne 0 ]]; then
  echo "bringup_matrix=fail failed=${FAILED} failed_cases=$(IFS=,; echo "${FAILED_CASES[*]}") result=fail"
  exit 1
fi

echo "bringup_matrix=pass checks=${CHECKS_RUN} result=pass"
