#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "${ROOT_DIR}/scripts/lib/airbreak_ui_model.sh"

CASES_FILE="${AIRBREAK_PNG_REGRESSION_CASES_FILE:-${ROOT_DIR}/tests/png-regressions/cases.tsv}"
FIRMWARES_FILE="${AIRBREAK_PNG_REGRESSION_FIRMWARES_FILE:-${ROOT_DIR}/tests/png-regressions/firmwares.tsv}"
BASELINE_DIR="${AIRBREAK_PNG_REGRESSION_BASELINE_DIR:-${ROOT_DIR}/tests/png-regressions/baselines}"
OUT_DIR="${AIRBREAK_PNG_REGRESSION_OUT_DIR:-${ROOT_DIR}/artifacts/png-regressions}"
MODE="${AIRBREAK_PNG_REGRESSION_MODE:-test}"
if [[ "${AIRBREAK_PNG_REGRESSION_UPDATE:-0}" == "1" ]]; then
  MODE="update"
fi
SELECTED_FIRMWARES="${AIRBREAK_PNG_REGRESSION_FIRMWARES:-air10-vauto}"
SELECTED_CASES="${AIRBREAK_PNG_REGRESSION_CASES:-all}"
FAIL_FAST="${AIRBREAK_PNG_REGRESSION_FAIL_FAST:-0}"
REQUIRE_FIRMWARE="${AIRBREAK_PNG_REGRESSION_REQUIRE_FIRMWARE:-0}"
MAX_MISMATCH_PIXELS="${AIRBREAK_PNG_REGRESSION_MAX_MISMATCH_PIXELS:-0}"
MAX_MISMATCH_RATIO="${AIRBREAK_PNG_REGRESSION_MAX_MISMATCH_RATIO:-0}"
MAX_CHANNEL_DELTA="${AIRBREAK_PNG_REGRESSION_MAX_CHANNEL_DELTA:-0}"

PIPELINE="${ROOT_DIR}/scripts/run-station-pipeline.sh"
PNG_COMPARE="${ROOT_DIR}/scripts/lib/png_compare.py"

case "${MODE}" in
  test|update) ;;
  *)
    echo "png_regression=fail stage=config reason=invalid_mode mode=${MODE} result=fail" >&2
    echo "valid AIRBREAK_PNG_REGRESSION_MODE values: test, update" >&2
    exit 2
    ;;
esac

for required in "${CASES_FILE}" "${FIRMWARES_FILE}" "${PIPELINE}" "${PNG_COMPARE}"; do
  if [[ ! -f "${required}" ]]; then
    echo "png_regression=fail stage=config reason=missing_file path=${required} result=fail" >&2
    exit 2
  fi
done

declare -A FIRMWARE_PATHS=()
declare -a FIRMWARE_IDS=()
declare -A CASE_MAX_INSTRUCTIONS=()
declare -A CASE_FRAME=()
declare -A CASE_TARGET=()
declare -a CASE_IDS=()
RUN_ONE_OUTCOME=""

append_unique() {
  local value="$1"
  shift
  local existing
  for existing in "$@"; do
    [[ "${existing}" == "${value}" ]] && return 1
  done
  return 0
}

sanitize_id() {
  python3 - "$1" <<'PY'
import re
import sys

value = sys.argv[1].strip()
value = re.sub(r"\.bin$", "", value)
value = re.sub(r"[^A-Za-z0-9_.-]+", "-", value).strip("-")
print(value or "firmware")
PY
}

resolve_path() {
  local path="$1"
  case "${path}" in
    /*) printf '%s\n' "${path}" ;;
    *) printf '%s\n' "${ROOT_DIR}/${path}" ;;
  esac
}

load_firmwares() {
  local id path
  while IFS=$'\t' read -r id path _; do
    [[ -z "${id}" || "${id}" == \#* ]] && continue
    if [[ -z "${path:-}" ]]; then
      echo "png_regression=fail stage=config reason=bad_firmware_manifest line_id=${id} result=fail" >&2
      exit 2
    fi
    FIRMWARE_IDS+=("${id}")
    FIRMWARE_PATHS["${id}"]="$(resolve_path "${path}")"
  done < "${FIRMWARES_FILE}"
}

load_cases() {
  local id target max_instructions frame
  while IFS=$'\t' read -r id target max_instructions frame _; do
    [[ -z "${id}" || "${id}" == \#* ]] && continue
    if [[ -z "${target:-}" || -z "${max_instructions:-}" || -z "${frame:-}" ]]; then
      echo "png_regression=fail stage=config reason=bad_case_manifest case=${id} result=fail" >&2
      exit 2
    fi
    airbreak_ui_screen_id "${target}" >/dev/null || exit $?
    CASE_IDS+=("${id}")
    CASE_TARGET["${id}"]="${target}"
    CASE_MAX_INSTRUCTIONS["${id}"]="${max_instructions}"
    CASE_FRAME["${id}"]="${frame}"
  done < "${CASES_FILE}"
}

select_firmwares() {
  local -n out_ref=$1
  local token id path base
  local -a tokens=()
  IFS=' ' read -r -a tokens <<< "$(printf '%s' "${SELECTED_FIRMWARES}" | tr ',' ' ')"
  for token in "${tokens[@]}"; do
    [[ -z "${token}" ]] && continue
    case "${token}" in
      *'*'*|*'?'*|'['*|*']'*)
        echo "png_regression=fail stage=config reason=wildcard_firmware_token token=${token} result=fail" >&2
        echo "Use a manifest id, 'all', 'firmware-list', or an explicit path." >&2
        exit 2
        ;;
      all)
        for id in "${FIRMWARE_IDS[@]}"; do
          append_unique "${id}" "${out_ref[@]}" && out_ref+=("${id}")
        done
        ;;
      firmware-list)
        for id in "${FIRMWARE_IDS[@]}"; do
          case "${FIRMWARE_PATHS[${id}]}" in
            "${ROOT_DIR}/firmware/firmware-list/"*)
              append_unique "${id}" "${out_ref[@]}" && out_ref+=("${id}")
              ;;
          esac
        done
        ;;
      *)
        if [[ -n "${FIRMWARE_PATHS[${token}]:-}" ]]; then
          append_unique "${token}" "${out_ref[@]}" && out_ref+=("${token}")
        elif [[ "${token}" == */* || "${token}" == *.bin ]]; then
          path="$(resolve_path "${token}")"
          base="$(basename "${path}")"
          id="$(sanitize_id "${base}")"
          FIRMWARE_PATHS["${id}"]="${path}"
          append_unique "${id}" "${out_ref[@]}" && out_ref+=("${id}")
        else
          echo "png_regression=fail stage=config reason=unknown_firmware token=${token} result=fail" >&2
          exit 2
        fi
        ;;
    esac
  done
}

select_cases() {
  local -n out_ref=$1
  local token id
  local -a tokens=()
  IFS=' ' read -r -a tokens <<< "$(printf '%s' "${SELECTED_CASES}" | tr ',' ' ')"
  for token in "${tokens[@]}"; do
    [[ -z "${token}" ]] && continue
    case "${token}" in
      all)
        for id in "${CASE_IDS[@]}"; do
          append_unique "${id}" "${out_ref[@]}" && out_ref+=("${id}")
        done
        ;;
      *)
        if [[ -z "${CASE_MAX_INSTRUCTIONS[${token}]:-}" ]]; then
          echo "png_regression=fail stage=config reason=unknown_case case=${token} result=fail" >&2
          exit 2
        fi
        append_unique "${token}" "${out_ref[@]}" && out_ref+=("${token}")
        ;;
    esac
  done
}

run_one() {
  local firmware_id="$1"
  local case_id="$2"
  local firmware_path="${FIRMWARE_PATHS[${firmware_id}]}"
  local target="${CASE_TARGET[${case_id}]}"
  local max_instructions="${CASE_MAX_INSTRUCTIONS[${case_id}]}"
  local frame_relpath="${CASE_FRAME[${case_id}]}"
  local sequence=""
  local run_root="${OUT_DIR}/runs/${firmware_id}/${case_id}"
  local patched_fw="${OUT_DIR}/patched/${firmware_id}-${case_id}.bin"
  local log_path="${OUT_DIR}/logs/${firmware_id}-${case_id}.log"
  local actual_png="${run_root}/${frame_relpath}"
  local baseline_png="${BASELINE_DIR}/${firmware_id}/${case_id}.png"
  local diff_png="${OUT_DIR}/diffs/${firmware_id}-${case_id}.png"
  local status=0
  local compare_line=""
  RUN_ONE_OUTCOME="run"

  if [[ ! -f "${firmware_path}" ]]; then
    RUN_ONE_OUTCOME="skip"
    echo "png_regression=skip firmware=${firmware_id} case=${case_id} reason=missing_source path=${firmware_path} result=skip"
    [[ "${REQUIRE_FIRMWARE}" == "1" ]] && return 1
    return 0
  fi
  if ! sequence="$(airbreak_ui_front_panel_sequence_for_target "${target}")"; then
    RUN_ONE_OUTCOME="skip"
    echo "png_regression=skip firmware=${firmware_id} case=${case_id} reason=target_not_enabled target=${target} ui_screens=${AIRBREAK_UI_SCREENS} result=skip"
    return 0
  fi

  rm -rf -- "${run_root}"
  mkdir -p "$(dirname "${patched_fw}")" "$(dirname "${log_path}")" "$(dirname "${diff_png}")"

  set +e
  AIRBREAK_SOURCE_FIRMWARE="${firmware_path}" \
  AIRBREAK_PATCHED_FIRMWARE="${patched_fw}" \
  AIRBREAK_UI_SCREENS="${AIRBREAK_UI_SCREENS}" \
  AIRBREAK_EMULATOR_MODE=headless \
  AIRBREAK_RUST_RUN_ID="png-regression-${firmware_id}-${case_id}" \
  AIRBREAK_RUST_RUN_ROOT="${run_root}" \
  AIRBREAK_RUST_CLEAN_RUN_ROOT=0 \
  AIRBREAK_RUST_FRONT_PANEL_SEQUENCE="${sequence}" \
  AIRBREAK_RUST_FRONT_PANEL_REQUIRE_CHANGE=0 \
  AIRBREAK_RUST_MAX_INSTRUCTIONS="${max_instructions}" \
    "${PIPELINE}" > "${log_path}" 2>&1
  status=$?
  set -e

  if [[ "${status}" -ne 0 ]]; then
    echo "png_regression=fail firmware=${firmware_id} case=${case_id} stage=pipeline status=${status} log=${log_path} result=fail"
    return 1
  fi
  if [[ ! -f "${actual_png}" ]]; then
    echo "png_regression=fail firmware=${firmware_id} case=${case_id} stage=capture reason=missing_actual actual=${actual_png} log=${log_path} result=fail"
    return 1
  fi

  if [[ "${MODE}" == "update" ]]; then
    mkdir -p "$(dirname "${baseline_png}")"
    cp -- "${actual_png}" "${baseline_png}"
    echo "png_regression=pass firmware=${firmware_id} case=${case_id} mode=update baseline=${baseline_png} actual=${actual_png} result=pass"
    return 0
  fi

  if [[ ! -f "${baseline_png}" ]]; then
    echo "png_regression=fail firmware=${firmware_id} case=${case_id} stage=compare reason=missing_baseline baseline=${baseline_png} actual=${actual_png} log=${log_path} result=fail"
    return 1
  fi

  set +e
  compare_line="$(
    python3 "${PNG_COMPARE}" \
      --expected "${baseline_png}" \
      --actual "${actual_png}" \
      --diff "${diff_png}" \
      --max-mismatch-pixels "${MAX_MISMATCH_PIXELS}" \
      --max-mismatch-ratio "${MAX_MISMATCH_RATIO}" \
      --max-channel-delta "${MAX_CHANNEL_DELTA}" 2>&1
  )"
  status=$?
  set -e
  printf '%s\n' "${compare_line}"
  if [[ "${status}" -ne 0 ]]; then
    echo "png_regression=fail firmware=${firmware_id} case=${case_id} stage=compare status=${status} baseline=${baseline_png} actual=${actual_png} diff=${diff_png} log=${log_path} result=fail"
    return 1
  fi

  rm -f -- "${diff_png}"
  echo "png_regression=pass firmware=${firmware_id} case=${case_id} mode=test baseline=${baseline_png} actual=${actual_png} log=${log_path} result=pass"
  return 0
}

load_firmwares
load_cases
if ! airbreak_ui_configure "$(airbreak_ui_default_screens)"; then
  echo "png_regression=fail stage=config reason=invalid_ui_model result=fail" >&2
  exit 2
fi

declare -a SELECTED_FIRMWARE_IDS=()
declare -a SELECTED_CASE_IDS=()
select_firmwares SELECTED_FIRMWARE_IDS
select_cases SELECTED_CASE_IDS

if [[ "${#SELECTED_FIRMWARE_IDS[@]}" -eq 0 || "${#SELECTED_CASE_IDS[@]}" -eq 0 ]]; then
  echo "png_regression=fail stage=config reason=empty_selection result=fail" >&2
  exit 2
fi

mkdir -p "${OUT_DIR}"
FAILED=0
RUN=0
SKIPPED=0

for firmware_id in "${SELECTED_FIRMWARE_IDS[@]}"; do
  for case_id in "${SELECTED_CASE_IDS[@]}"; do
    if run_one "${firmware_id}" "${case_id}"; then
      if [[ "${RUN_ONE_OUTCOME}" == "skip" ]]; then
        SKIPPED=$((SKIPPED + 1))
      else
        RUN=$((RUN + 1))
      fi
    else
      FAILED=$((FAILED + 1))
      RUN=$((RUN + 1))
      if [[ "${FAIL_FAST}" == "1" ]]; then
        echo "png_regression=fail mode=${MODE} run=${RUN} skipped=${SKIPPED} failed=${FAILED} result=fail"
        exit 1
      fi
    fi
  done
done

if [[ "${FAILED}" -ne 0 ]]; then
  echo "png_regression=fail mode=${MODE} run=${RUN} skipped=${SKIPPED} failed=${FAILED} result=fail"
  exit 1
fi

echo "png_regression=pass mode=${MODE} run=${RUN} skipped=${SKIPPED} failed=0 result=pass"
