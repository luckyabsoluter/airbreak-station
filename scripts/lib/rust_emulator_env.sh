#!/usr/bin/env bash

# Rust STM32 emulator bootstrap for the GUI launcher. The emulator source is
# vendored in this repository so station runs do not clone or patch an external
# checkout at runtime.

if [[ -z "${ROOT_DIR:-}" ]]; then
  ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
fi

STM32_EMULATOR_DIR="${STM32_EMULATOR_DIR:-${ROOT_DIR}/rust/stm32-emulator}"
AIRBREAK_RUST_SVD_URL="${AIRBREAK_RUST_SVD_URL:-https://dl.antmicro.com/projects/renode/svd/STM32F40x.svd.gz}"
AIRBREAK_RUST_SVD_SHA256="${AIRBREAK_RUST_SVD_SHA256:-d71b3e11c870ec83346eb994e98a57c5eacdf0082d2949ed495d7a3828bf41ec}"
AIRBREAK_RUST_SVD_CACHE="${AIRBREAK_RUST_SVD_CACHE:-${ROOT_DIR}/.work/svd-cache/STM32F40x.svd.gz}"
AIRBREAK_RUST_ISOLATION="${AIRBREAK_RUST_ISOLATION:-1}"
AIRBREAK_RUST_CLEAN_RUN_ROOT="${AIRBREAK_RUST_CLEAN_RUN_ROOT:-1}"
AIRBREAK_RUST_RUN_ID="${AIRBREAK_RUST_RUN_ID:-$(date +%Y%m%d%H%M%S)-$$-${RANDOM:-0}}"
AIRBREAK_RUST_RUN_ROOT="${AIRBREAK_RUST_RUN_ROOT:-${ROOT_DIR}/.airbreak-rust-runs/${AIRBREAK_RUST_RUN_ID}}"
AIRBREAK_RUST_CONFIG="${AIRBREAK_RUST_CONFIG:-${AIRBREAK_RUST_RUN_ROOT}/airbreak-f405.yaml}"
AIRBREAK_RUST_SVD_SOURCE="${AIRBREAK_RUST_SVD_SOURCE:-${AIRBREAK_RUST_SVD_GZ:-}}"
AIRBREAK_RUST_SVD="${AIRBREAK_RUST_SVD:-${AIRBREAK_RUST_RUN_ROOT}/STM32F40x.svd}"
AIRBREAK_RUST_FIRMWARE="${AIRBREAK_RUST_FIRMWARE:-${ROOT_DIR}/stm32-patched.bin}"
AIRBREAK_RUST_FRAMEBUFFER_PNG="${AIRBREAK_RUST_FRAMEBUFFER_PNG:-${AIRBREAK_RUST_RUN_ROOT}/lcd-frame.png}"
AIRBREAK_RUST_CONFIG_TEMPLATE="${AIRBREAK_RUST_CONFIG_TEMPLATE:-${ROOT_DIR}/rust/airbreak-f405.yaml.in}"
AIRBREAK_RUST_USE_SDL="${AIRBREAK_RUST_USE_SDL:-auto}"
AIRBREAK_RUST_BUILD_MODE="${AIRBREAK_RUST_BUILD_MODE:-release}"
AIRBREAK_RUST_VERBOSE="${AIRBREAK_RUST_VERBOSE:-0}"
AIRBREAK_RUST_MAX_INSTRUCTIONS="${AIRBREAK_RUST_MAX_INSTRUCTIONS:-}"
AIRBREAK_RUST_RUN_FOR_MAX_INSTRUCTIONS="${AIRBREAK_RUST_RUN_FOR_MAX_INSTRUCTIONS:-2000000}"
AIRBREAK_RUST_INTERRUPT_PERIOD="${AIRBREAK_RUST_INTERRUPT_PERIOD:-1}"
AIRBREAK_RUST_BUSY_LOOP_STOP="${AIRBREAK_RUST_BUSY_LOOP_STOP:-0}"
AIRBREAK_RUST_STOP_ADDR="${AIRBREAK_RUST_STOP_ADDR:-}"
AIRBREAK_RUST_DUMP_STACK="${AIRBREAK_RUST_DUMP_STACK:-}"
AIRBREAK_RUST_DUMP_MEM32="${AIRBREAK_RUST_DUMP_MEM32:-}"
AIRBREAK_RUST_COUNT_ADDRS="${AIRBREAK_RUST_COUNT_ADDRS:-}"
AIRBREAK_RUST_TRACE_ADDRS="${AIRBREAK_RUST_TRACE_ADDRS:-}"
AIRBREAK_RUST_TRACE_ADDR_LIMIT="${AIRBREAK_RUST_TRACE_ADDR_LIMIT:-32}"
AIRBREAK_RUST_WATCH_WRITES="${AIRBREAK_RUST_WATCH_WRITES:-}"
AIRBREAK_RUST_WATCH_WRITE_LIMIT="${AIRBREAK_RUST_WATCH_WRITE_LIMIT:-64}"
AIRBREAK_RUST_FRONT_PANEL_SEQUENCE="${AIRBREAK_RUST_FRONT_PANEL_SEQUENCE:-}"
AIRBREAK_RUST_FRONT_PANEL_AT="${AIRBREAK_RUST_FRONT_PANEL_AT:-500000000}"
AIRBREAK_RUST_FRONT_PANEL_SETTLE="${AIRBREAK_RUST_FRONT_PANEL_SETTLE:-50000000}"
AIRBREAK_RUST_FRONT_PANEL_SNAPSHOT_DIR="${AIRBREAK_RUST_FRONT_PANEL_SNAPSHOT_DIR:-}"
AIRBREAK_RUST_FRONT_PANEL_REQUIRE_CHANGE="${AIRBREAK_RUST_FRONT_PANEL_REQUIRE_CHANGE:-1}"
AIRBREAK_RUST_REJECT_LOGO="${AIRBREAK_RUST_REJECT_LOGO:-1}"
AIRBREAK_RUST_MIN_NONZERO="${AIRBREAK_RUST_MIN_NONZERO:-512}"
AIRBREAK_RUST_MIN_BBOX_AREA="${AIRBREAK_RUST_MIN_BBOX_AREA:-2048}"

export STM32_EMULATOR_DIR
export AIRBREAK_RUST_SVD_URL AIRBREAK_RUST_SVD_SHA256 AIRBREAK_RUST_SVD_CACHE
export AIRBREAK_RUST_ISOLATION AIRBREAK_RUST_CLEAN_RUN_ROOT AIRBREAK_RUST_RUN_ID AIRBREAK_RUST_RUN_ROOT
export AIRBREAK_RUST_CONFIG AIRBREAK_RUST_SVD_SOURCE AIRBREAK_RUST_SVD AIRBREAK_RUST_FIRMWARE AIRBREAK_RUST_FRAMEBUFFER_PNG AIRBREAK_RUST_CONFIG_TEMPLATE

airbreak_rust_front_panel_label() {
  python3 - "${1:-}" <<'PY'
import sys

def sanitize(value):
    out = []
    for ch in value.strip().lower():
        out.append(ch if ch.isalnum() and ch.isascii() else "-")
    text = "".join(out).strip("-")
    return text or "unnamed"

tokens = [token.strip() for token in sys.argv[1].split(",") if token.strip()]
if any("@" in token for token in tokens):
    actions = [sanitize(tokens[-1].split("@", 1)[0])]
else:
    actions = [sanitize(action) for action in tokens]
print("-".join(actions) if actions else "none")
PY
}

airbreak_rust_front_panel_snapshot_summary() {
  local sequence="$1"
  local snapshot_dir="$2"
  local require_change="$3"
  local label
  label="$(airbreak_rust_front_panel_label "${sequence}")" || return $?

  local before="${snapshot_dir}/before-${label}.png"
  local after="${snapshot_dir}/after-${label}.png"
  local sequence_field="${sequence//[[:space:]]/}"
  if [[ ! -f "${before}" || ! -f "${after}" ]]; then
    printf "front_panel_sequence=%s front_panel_snapshot=missing front_panel_before=%s front_panel_after=%s front_panel_changed=unknown" \
      "${sequence_field}" "${before}" "${after}"
    [[ "${require_change}" == "1" ]] && return 1
    return 0
  fi

  if cmp -s "${before}" "${after}"; then
    printf "front_panel_sequence=%s front_panel_snapshot=present front_panel_before=%s front_panel_after=%s front_panel_changed=0" \
      "${sequence_field}" "${before}" "${after}"
    [[ "${require_change}" == "1" ]] && return 1
    return 0
  fi

  printf "front_panel_sequence=%s front_panel_snapshot=present front_panel_before=%s front_panel_after=%s front_panel_changed=1" \
    "${sequence_field}" "${before}" "${after}"
  return 0
}

airbreak_rust_is_windows_mount_path() {
  case "${1:-}" in
    /mnt/*|/cygdrive/*)
      return 0
      ;;
    *)
      return 1
      ;;
  esac
}

airbreak_rust_require_tool() {
  local tool="$1"
  local hint="$2"
  if ! command -v "${tool}" >/dev/null 2>&1; then
    echo "[run-gui] rust backend unavailable: missing ${tool}. ${hint}" >&2
    return 127
  fi
}

airbreak_rust_ensure_emulator_source() {
  if [[ ! -f "${STM32_EMULATOR_DIR}/Cargo.toml" || ! -f "${STM32_EMULATOR_DIR}/src/main.rs" ]]; then
    echo "[run-gui] rust backend source not found: ${STM32_EMULATOR_DIR}" >&2
    echo "[run-gui] expected vendored source at rust/stm32-emulator or set STM32_EMULATOR_DIR to a source tree." >&2
    return 1
  fi
}

airbreak_rust_svd_sha256() {
  python3 - "${1}" <<'PY'
from pathlib import Path
import hashlib
import sys

h = hashlib.sha256()
with Path(sys.argv[1]).open("rb") as f:
    for chunk in iter(lambda: f.read(1024 * 1024), b""):
        h.update(chunk)
print(h.hexdigest())
PY
}

airbreak_rust_verify_svd_checksum() {
  local path="$1"
  local expected="${AIRBREAK_RUST_SVD_SHA256}"
  if [[ -z "${expected}" ]]; then
    return 0
  fi
  local actual
  actual="$(airbreak_rust_svd_sha256 "${path}")" || return $?
  if [[ "${actual}" != "${expected}" ]]; then
    echo "[run-gui] rust backend SVD checksum mismatch: path=${path} expected=${expected} actual=${actual}" >&2
    return 1
  fi
}

airbreak_rust_download_svd() {
  local url="${AIRBREAK_RUST_SVD_URL}"
  local output="${AIRBREAK_RUST_SVD_CACHE}"
  if [[ -z "${url}" ]]; then
    echo "[run-gui] rust backend SVD URL is empty and no AIRBREAK_RUST_SVD_SOURCE override was provided" >&2
    return 1
  fi

  mkdir -p "$(dirname "${output}")"
  python3 - "${url}" "${output}" <<'PY'
from pathlib import Path
import os
import sys
import urllib.request

url, output = sys.argv[1:]
target = Path(output)
tmp = target.with_name(target.name + ".tmp")
with urllib.request.urlopen(url, timeout=60) as response:
    data = response.read()
tmp.write_bytes(data)
os.replace(tmp, target)
PY
}

airbreak_rust_ensure_svd_cache() {
  airbreak_rust_require_tool python3 "The Rust backend uses python3 to download and verify the STM32 SVD." || return $?
  local cache="${AIRBREAK_RUST_SVD_CACHE}"
  if [[ -f "${cache}" ]]; then
    if airbreak_rust_verify_svd_checksum "${cache}"; then
      printf '%s\n' "${cache}"
      return 0
    fi
    rm -f -- "${cache}"
  fi

  echo "[run-gui] rust backend downloading SVD url=${AIRBREAK_RUST_SVD_URL} cache=${cache}" >&2
  airbreak_rust_download_svd || return $?
  airbreak_rust_verify_svd_checksum "${cache}" || return $?
  printf '%s\n' "${cache}"
}

airbreak_rust_generate_config() {
  airbreak_rust_require_tool python3 "The Rust backend uses python3 for safe config-template substitution." || return $?

  local svd_source="${AIRBREAK_RUST_SVD_SOURCE}"
  if [[ -z "${svd_source}" ]]; then
    svd_source="$(airbreak_rust_ensure_svd_cache)" || return $?
  fi
  if [[ ! -f "${AIRBREAK_RUST_FIRMWARE}" ]]; then
    echo "[run-gui] rust backend firmware not found: ${AIRBREAK_RUST_FIRMWARE}" >&2
    return 1
  fi
  if [[ ! -f "${svd_source}" ]]; then
    echo "[run-gui] rust backend SVD not found: ${svd_source}" >&2
    return 1
  fi
  if [[ ! -f "${AIRBREAK_RUST_CONFIG_TEMPLATE}" ]]; then
    echo "[run-gui] rust backend config template not found: ${AIRBREAK_RUST_CONFIG_TEMPLATE}" >&2
    return 1
  fi

  mkdir -p "${AIRBREAK_RUST_RUN_ROOT}"
  case "${svd_source}" in
    *.gz)
      airbreak_rust_require_tool gzip "The Rust backend needs gzip to expand compressed SVD input." || return $?
      gzip -dc "${svd_source}" > "${AIRBREAK_RUST_SVD}"
      ;;
    *)
      cp "${svd_source}" "${AIRBREAK_RUST_SVD}"
      ;;
  esac

  local use_sdl="${AIRBREAK_RUST_USE_SDL}"
  if [[ "${use_sdl}" == "auto" ]]; then
    use_sdl="1"
    if [[ "${AIRBREAK_RUST_DISABLE_GUI:-0}" == "1" || -n "${RUN_FOR:-}" || -n "${AIRBREAK_RUST_MAX_INSTRUCTIONS}" ]]; then
      use_sdl="0"
    fi
  fi

  local framebuffer_backend=""
  if [[ "${use_sdl}" == "1" ]]; then
    framebuffer_backend="    sdl: true"
  else
    mkdir -p "$(dirname "${AIRBREAK_RUST_FRAMEBUFFER_PNG}")"
    framebuffer_backend="    image:
      file: ${AIRBREAK_RUST_FRAMEBUFFER_PNG}"
  fi

  python3 - "${AIRBREAK_RUST_CONFIG_TEMPLATE}" "${AIRBREAK_RUST_CONFIG}" \
    "${AIRBREAK_RUST_SVD}" "${AIRBREAK_RUST_FIRMWARE}" "${FB_WIDTH:-240}" "${FB_HEIGHT:-320}" \
    "${framebuffer_backend}" <<'PY'
from pathlib import Path
import sys

template, output, svd, firmware, width, height, framebuffer_backend = sys.argv[1:]
text = Path(template).read_text(encoding="utf-8")
for key, value in {
    "@SVD@": svd,
    "@FIRMWARE@": firmware,
    "@FB_WIDTH@": width,
    "@FB_HEIGHT@": height,
    "@FRAMEBUFFER_BACKEND@": framebuffer_backend,
}.items():
    text = text.replace(key, value)
Path(output).write_text(text, encoding="utf-8")
PY

  if grep -Eq '^[[:space:]]*patches:' "${AIRBREAK_RUST_CONFIG}"; then
    echo "[run-gui] rust backend generated a patching config; refusing to run" >&2
    return 2
  fi
}

airbreak_rust_cleanup_run_root() {
  if [[ "${AIRBREAK_RUST_ISOLATION:-1}" != "1" ||
        "${AIRBREAK_RUST_CLEAN_RUN_ROOT:-1}" != "1" ||
        -z "${AIRBREAK_RUST_RUN_ROOT:-}" ]]; then
    return 0
  fi

  case "${AIRBREAK_RUST_RUN_ROOT}" in
    "${ROOT_DIR}/.airbreak-rust-runs/"*)
      rm -rf -- "${AIRBREAK_RUST_RUN_ROOT}"
      rmdir --ignore-fail-on-non-empty -- "${ROOT_DIR}/.airbreak-rust-runs" 2>/dev/null || true
      ;;
  esac
}

airbreak_rust_probe_frame_png() {
  local frame_path="$1"
  python3 - "${frame_path}" "${AIRBREAK_RUST_REJECT_LOGO}" "${AIRBREAK_RUST_MIN_NONZERO}" "${AIRBREAK_RUST_MIN_BBOX_AREA}" <<'PY'
from pathlib import Path
import struct
import sys
import zlib

path = Path(sys.argv[1])
reject_logo = sys.argv[2] != "0"
min_nonzero = int(sys.argv[3])
min_bbox_area = int(sys.argv[4])

def paeth(a, b, c):
    p = a + b - c
    pa = abs(p - a)
    pb = abs(p - b)
    pc = abs(p - c)
    if pa <= pb and pa <= pc:
        return a
    if pb <= pc:
        return b
    return c

def read_png(path):
    data = path.read_bytes()
    if not data.startswith(b"\x89PNG\r\n\x1a\n"):
        raise ValueError("not_png")
    pos = 8
    width = height = bit_depth = color_type = None
    idat = bytearray()
    while pos + 8 <= len(data):
        length = struct.unpack(">I", data[pos:pos + 4])[0]
        kind = data[pos + 4:pos + 8]
        payload = data[pos + 8:pos + 8 + length]
        pos += 12 + length
        if kind == b"IHDR":
            width, height, bit_depth, color_type, _, _, _ = struct.unpack(">IIBBBBB", payload)
        elif kind == b"IDAT":
            idat.extend(payload)
        elif kind == b"IEND":
            break
    if width is None or height is None:
        raise ValueError("missing_ihdr")
    if bit_depth != 8 or color_type not in (2, 6):
        raise ValueError(f"unsupported_png_color:{bit_depth}:{color_type}")
    channels = 3 if color_type == 2 else 4
    stride = width * channels
    raw = zlib.decompress(bytes(idat))
    rows = []
    prior = [0] * stride
    offset = 0
    for _ in range(height):
        filt = raw[offset]
        offset += 1
        row = list(raw[offset:offset + stride])
        offset += stride
        for i, value in enumerate(row):
            left = row[i - channels] if i >= channels else 0
            up = prior[i]
            up_left = prior[i - channels] if i >= channels else 0
            if filt == 1:
                row[i] = (value + left) & 0xff
            elif filt == 2:
                row[i] = (value + up) & 0xff
            elif filt == 3:
                row[i] = (value + ((left + up) // 2)) & 0xff
            elif filt == 4:
                row[i] = (value + paeth(left, up, up_left)) & 0xff
            elif filt != 0:
                raise ValueError(f"bad_png_filter:{filt}")
        rows.append(row)
        prior = row
    return width, height, channels, rows

def logo_like_geometry(bbox, nonzero, white=0):
    if bbox is None:
        return False
    x0, y0, x1, y1 = bbox
    return (
        (
            20 <= x0 <= 40
            and 85 <= y0 <= 105
            and 190 <= x1 <= 220
            and 140 <= y1 <= 230
            and nonzero < 6000
        )
        or (
            25 <= x0 <= 60
            and 85 <= y0 <= 110
            and 100 <= x1 <= 220
            and 105 <= y1 <= 160
            and nonzero < 1500
            and white < 100
        )
    )

width, height, channels, rows = read_png(path)
nonzero = 0
white = 0
bbox = None
for y, row in enumerate(rows):
    for x in range(width):
        base = x * channels
        rgb = tuple(row[base:base + 3])
        if rgb == (0, 0, 0):
            continue
        nonzero += 1
        if rgb == (255, 255, 255):
            white += 1
        if bbox is None:
            bbox = [x, y, x, y]
        else:
            bbox[0] = min(bbox[0], x)
            bbox[1] = min(bbox[1], y)
            bbox[2] = max(bbox[2], x)
            bbox[3] = max(bbox[3], y)

bbox_area = 0
if bbox is not None:
    bbox_area = (bbox[2] - bbox[0] + 1) * (bbox[3] - bbox[1] + 1)
logo_like = logo_like_geometry(tuple(bbox) if bbox else None, nonzero, white)
visual_ok = nonzero >= min_nonzero and bbox_area >= min_bbox_area and (not reject_logo or not logo_like)
if visual_ok:
    reason = "ok"
elif logo_like:
    reason = "logo_like_frame"
elif nonzero < min_nonzero:
    reason = "weak_frame_nonzero"
else:
    reason = "weak_frame_bbox"
bbox_text = "none" if bbox is None else ",".join(str(v) for v in bbox)
print(
    f"visual={'pass' if visual_ok else 'fail'} "
    f"visual_reason={reason} "
    f"visual_size={width}x{height} "
    f"visual_nonzero={nonzero} "
    f"visual_bbox={bbox_text} "
    f"visual_bbox_area={bbox_area} "
    f"visual_white={white} "
    f"visual_logo_like={1 if logo_like else 0}"
)
raise SystemExit(0 if visual_ok else 1)
PY
}

run_airbreak_rust_gui() {
  airbreak_rust_require_tool cargo "Install the Rust toolchain first." || return $?
  airbreak_rust_require_tool cmake "stm32-emulator builds bundled Unicorn and SDL2 through CMake; install cmake or provide it on PATH." || return $?
  airbreak_rust_ensure_emulator_source || return $?
  airbreak_rust_generate_config || return $?

  if airbreak_rust_is_windows_mount_path "${AIRBREAK_RUST_RUN_ROOT}"; then
    echo "[run-gui] rust backend run root must stay inside WSL/Linux paths: ${AIRBREAK_RUST_RUN_ROOT}" >&2
    return 2
  fi

  local -a args=()
  if [[ "${AIRBREAK_RUST_BUILD_MODE}" == "release" ]]; then
    args+=(--release)
  elif [[ "${AIRBREAK_RUST_BUILD_MODE}" != "debug" ]]; then
    echo "[run-gui] AIRBREAK_RUST_BUILD_MODE must be release or debug (got: ${AIRBREAK_RUST_BUILD_MODE})" >&2
    return 2
  fi

  args+=(-- "${AIRBREAK_RUST_CONFIG}")

  local verbose_count="${AIRBREAK_RUST_VERBOSE}"
  if ! [[ "${verbose_count}" =~ ^[0-9]+$ ]]; then
    echo "[run-gui] AIRBREAK_RUST_VERBOSE must be a non-negative integer (got: ${verbose_count})" >&2
    return 2
  fi
  local i=0
  while (( i < verbose_count )); do
    args+=("-v")
    i=$((i + 1))
  done

  local max_instructions="${AIRBREAK_RUST_MAX_INSTRUCTIONS}"
  if [[ -z "${max_instructions}" && -n "${RUN_FOR:-}" ]]; then
    max_instructions="${AIRBREAK_RUST_RUN_FOR_MAX_INSTRUCTIONS}"
  fi
  if [[ -n "${max_instructions}" ]]; then
    if ! [[ "${max_instructions}" =~ ^[0-9]+$ ]] || (( max_instructions <= 0 )); then
      echo "[run-gui] AIRBREAK_RUST_MAX_INSTRUCTIONS must be a positive integer (got: ${max_instructions})" >&2
      return 2
    fi
    args+=(--max-instructions "${max_instructions}")
  fi

  local front_panel_sequence="${AIRBREAK_RUST_FRONT_PANEL_SEQUENCE}"
  local front_panel_snapshot_dir="${AIRBREAK_RUST_FRONT_PANEL_SNAPSHOT_DIR}"
  if [[ -n "${front_panel_sequence}" ]]; then
    if ! [[ "${AIRBREAK_RUST_FRONT_PANEL_AT}" =~ ^[0-9]+$ ]]; then
      echo "[run-gui] AIRBREAK_RUST_FRONT_PANEL_AT must be a non-negative integer (got: ${AIRBREAK_RUST_FRONT_PANEL_AT})" >&2
      return 2
    fi
    if ! [[ "${AIRBREAK_RUST_FRONT_PANEL_SETTLE}" =~ ^[0-9]+$ ]]; then
      echo "[run-gui] AIRBREAK_RUST_FRONT_PANEL_SETTLE must be a non-negative integer (got: ${AIRBREAK_RUST_FRONT_PANEL_SETTLE})" >&2
      return 2
    fi
    if [[ "${AIRBREAK_RUST_FRONT_PANEL_REQUIRE_CHANGE}" != "0" && "${AIRBREAK_RUST_FRONT_PANEL_REQUIRE_CHANGE}" != "1" ]]; then
      echo "[run-gui] AIRBREAK_RUST_FRONT_PANEL_REQUIRE_CHANGE must be 0 or 1 (got: ${AIRBREAK_RUST_FRONT_PANEL_REQUIRE_CHANGE})" >&2
      return 2
    fi
    if [[ -z "${front_panel_snapshot_dir}" ]]; then
      front_panel_snapshot_dir="${AIRBREAK_RUST_RUN_ROOT}/front-panel-snapshots"
    fi
    local front_panel_end=$((AIRBREAK_RUST_FRONT_PANEL_AT + AIRBREAK_RUST_FRONT_PANEL_SETTLE))
    if [[ -n "${max_instructions}" ]] && (( max_instructions <= front_panel_end )); then
      echo "[run-gui] AIRBREAK_RUST_MAX_INSTRUCTIONS must exceed AIRBREAK_RUST_FRONT_PANEL_AT + AIRBREAK_RUST_FRONT_PANEL_SETTLE (${front_panel_end}) when AIRBREAK_RUST_FRONT_PANEL_SEQUENCE is set" >&2
      return 2
    fi
    args+=(--front-panel-sequence "${front_panel_sequence}")
    args+=(--front-panel-at "${AIRBREAK_RUST_FRONT_PANEL_AT}")
    args+=(--front-panel-settle "${AIRBREAK_RUST_FRONT_PANEL_SETTLE}")
    args+=(--front-panel-snapshot-dir "${front_panel_snapshot_dir}")
  fi

  if [[ "${AIRBREAK_RUST_BUSY_LOOP_STOP}" == "1" ]]; then
    args+=(--busy-loop-stop)
  elif [[ "${AIRBREAK_RUST_BUSY_LOOP_STOP}" != "0" ]]; then
    echo "[run-gui] AIRBREAK_RUST_BUSY_LOOP_STOP must be 0 or 1 (got: ${AIRBREAK_RUST_BUSY_LOOP_STOP})" >&2
    return 2
  fi

  if [[ -n "${AIRBREAK_RUST_STOP_ADDR}" ]]; then
    args+=(--stop-addr "${AIRBREAK_RUST_STOP_ADDR}")
  fi
  if [[ -n "${AIRBREAK_RUST_DUMP_STACK}" ]]; then
    args+=(--dump-stack "${AIRBREAK_RUST_DUMP_STACK}")
  fi
  if [[ -n "${AIRBREAK_RUST_DUMP_MEM32}" ]]; then
    local -a dump_addrs=()
    local dump_addr
    IFS=',' read -r -a dump_addrs <<< "${AIRBREAK_RUST_DUMP_MEM32}"
    for dump_addr in "${dump_addrs[@]}"; do
      [[ -z "${dump_addr}" ]] && continue
      args+=(--dump-mem32 "${dump_addr}")
    done
  fi
  if [[ -n "${AIRBREAK_RUST_COUNT_ADDRS}" ]]; then
    local -a count_addrs=()
    local count_addr
    IFS=',' read -r -a count_addrs <<< "${AIRBREAK_RUST_COUNT_ADDRS}"
    for count_addr in "${count_addrs[@]}"; do
      [[ -z "${count_addr}" ]] && continue
      args+=(--count-addr "${count_addr}")
    done
  fi
  if [[ -n "${AIRBREAK_RUST_TRACE_ADDRS}" ]]; then
    local -a trace_addrs=()
    local trace_addr
    IFS=',' read -r -a trace_addrs <<< "${AIRBREAK_RUST_TRACE_ADDRS}"
    for trace_addr in "${trace_addrs[@]}"; do
      [[ -z "${trace_addr}" ]] && continue
      args+=(--trace-addr "${trace_addr}")
    done
    args+=(--trace-addr-limit "${AIRBREAK_RUST_TRACE_ADDR_LIMIT}")
  fi
  if [[ -n "${AIRBREAK_RUST_WATCH_WRITES}" ]]; then
    local -a watch_addrs=()
    local watch_addr
    IFS=',' read -r -a watch_addrs <<< "${AIRBREAK_RUST_WATCH_WRITES}"
    for watch_addr in "${watch_addrs[@]}"; do
      [[ -z "${watch_addr}" ]] && continue
      args+=(--watch-write "${watch_addr}")
    done
    args+=(--watch-write-limit "${AIRBREAK_RUST_WATCH_WRITE_LIMIT}")
  fi
  if ! [[ "${AIRBREAK_RUST_INTERRUPT_PERIOD}" =~ ^[0-9]+$ ]] || (( AIRBREAK_RUST_INTERRUPT_PERIOD <= 0 )); then
    echo "[run-gui] AIRBREAK_RUST_INTERRUPT_PERIOD must be a positive integer (got: ${AIRBREAK_RUST_INTERRUPT_PERIOD})" >&2
    return 2
  fi
  args+=(--interrupt-period "${AIRBREAK_RUST_INTERRUPT_PERIOD}")

  printf "[run-gui] rust backend start source=%s config=%s framebuffer=%s\n" \
    "${STM32_EMULATOR_DIR}" "${AIRBREAK_RUST_CONFIG}" "${AIRBREAK_RUST_FRAMEBUFFER_PNG}" >&2

  local status=0
  set +e
  if [[ -n "${HARD_TIMEOUT:-}" ]]; then
    timeout --foreground -k "${HARD_TIMEOUT_KILL_AFTER:-5s}" "${HARD_TIMEOUT}" \
      cargo run --manifest-path "${STM32_EMULATOR_DIR}/Cargo.toml" "${args[@]}"
    status=$?
  else
    cargo run --manifest-path "${STM32_EMULATOR_DIR}/Cargo.toml" "${args[@]}"
    status=$?
  fi
  set -e

  if [[ "${status}" -eq 0 ]]; then
    local visual_summary=""
    local front_panel_summary=""
    local front_panel_status=0
    if [[ -n "${front_panel_sequence}" ]]; then
      if ! front_panel_summary="$(airbreak_rust_front_panel_snapshot_summary \
        "${front_panel_sequence}" "${front_panel_snapshot_dir}" "${AIRBREAK_RUST_FRONT_PANEL_REQUIRE_CHANGE}")"; then
        front_panel_status=1
      fi
    fi
    if [[ -f "${AIRBREAK_RUST_FRAMEBUFFER_PNG}" ]]; then
      if visual_summary="$(airbreak_rust_probe_frame_png "${AIRBREAK_RUST_FRAMEBUFFER_PNG}")"; then
        if [[ "${front_panel_status}" -eq 0 ]]; then
          printf "rust_gui=pass backend=rust max_instructions=%s framebuffer=%s %s %s result=pass\n" \
            "${max_instructions:-unbounded}" "${AIRBREAK_RUST_FRAMEBUFFER_PNG}" "${visual_summary}" "${front_panel_summary}"
          return 0
        fi
        printf "rust_gui=fail backend=rust max_instructions=%s framebuffer=%s %s %s result=fail\n" \
          "${max_instructions:-unbounded}" "${AIRBREAK_RUST_FRAMEBUFFER_PNG}" "${visual_summary}" "${front_panel_summary}"
        return 1
      fi
      printf "rust_gui=fail backend=rust max_instructions=%s framebuffer=%s %s %s result=fail\n" \
        "${max_instructions:-unbounded}" "${AIRBREAK_RUST_FRAMEBUFFER_PNG}" "${visual_summary}" "${front_panel_summary}"
      return 1
    fi
    if [[ "${front_panel_status}" -eq 0 ]]; then
      printf "rust_gui=pass backend=rust max_instructions=%s framebuffer=%s visual=unassessed visual_reason=no_framebuffer_png %s result=pass\n" \
        "${max_instructions:-unbounded}" "${AIRBREAK_RUST_FRAMEBUFFER_PNG}" "${front_panel_summary}"
      return 0
    fi
    printf "rust_gui=fail backend=rust max_instructions=%s framebuffer=%s visual=unassessed visual_reason=no_framebuffer_png %s result=fail\n" \
      "${max_instructions:-unbounded}" "${AIRBREAK_RUST_FRAMEBUFFER_PNG}" "${front_panel_summary}"
    return 1
  elif [[ "${status}" -eq 124 ]]; then
    printf "rust_gui=fail backend=rust reason=timeout status=%s result=fail\n" "${status}"
  else
    printf "rust_gui=fail backend=rust status=%s result=fail\n" "${status}"
  fi
  return "${status}"
}
