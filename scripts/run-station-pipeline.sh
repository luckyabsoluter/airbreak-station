#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

SOURCE_FIRMWARE="${AIRBREAK_SOURCE_FIRMWARE:-${ROOT_DIR}/firmware/resmed-air10.bin}"
PATCHED_FIRMWARE="${AIRBREAK_PATCHED_FIRMWARE:-${ROOT_DIR}/artifacts/firmware/stm32-ui-button.bin}"
BUILD_DIR="${AIRBREAK_PATCH_BUILD_DIR:-${ROOT_DIR}/artifacts/build}"
RUN_EMULATOR="${AIRBREAK_EMULATE:-1}"
EMULATOR_MODE="${AIRBREAK_EMULATOR_MODE:-gui}"
HEADLESS_MAX_INSTRUCTIONS="${AIRBREAK_RUST_MAX_INSTRUCTIONS:-500000000}"
CUSTOM_ABOUT_LABEL="${AIRBREAK_CUSTOM_ABOUT_LABEL:-Custom About}"
CUSTOM_ABOUT_DETAIL="${AIRBREAK_CUSTOM_ABOUT_DETAIL:-This is Custom About}"
CLINICAL_LABEL="${AIRBREAK_CLINICAL_LABEL:-Clinical Mode}"
BLOCK_BREAKER_LABEL="${AIRBREAK_BLOCK_BREAKER_LABEL:-Block Breaker}"

SRC="${ROOT_DIR}/patches/templates/my_options_essentials_mask_fit_patch.c"
LD="${ROOT_DIR}/patches/templates/my_options_essentials_mask_fit_patch.ld"
BUILD_SCRIPT="${ROOT_DIR}/patches/tools/build_function_patch.sh"
APPLY_SCRIPT="${ROOT_DIR}/patches/tools/apply_airbreak_ui_patch.py"
STUB_BIN="${BUILD_DIR}/my_options_essentials_mask_fit_patch.bin"
STUB_ELF="${BUILD_DIR}/my_options_essentials_mask_fit_patch.elf"

if [[ ! -f "${SOURCE_FIRMWARE}" ]]; then
  cat >&2 <<EOF
station_pipeline=fail stage=input reason=missing_source_firmware result=fail
Expected source firmware:
  ${SOURCE_FIRMWARE}

Set AIRBREAK_SOURCE_FIRMWARE=/path/to/resmed-air10.bin or place the file there.
Firmware binaries are private artifacts and are intentionally ignored by git.
EOF
  exit 1
fi

mkdir -p "${BUILD_DIR}" "$(dirname "${PATCHED_FIRMWARE}")"

"${BUILD_SCRIPT}" \
  --src "${SRC}" \
  --ld "${LD}" \
  --out-dir "${BUILD_DIR}"

CUSTOM_PAGE_HOOK_TARGET="$(
  arm-none-eabi-nm "${STUB_ELF}" |
    awk '$3 == "patch_custom_about_page_tail_hook" { print "0x" $1; found=1 } END { if (!found) exit 1 }'
)"

CUSTOM_PAGE_SEED_HOOK_TARGET="$(
  arm-none-eabi-nm "${STUB_ELF}" |
    awk '$3 == "patch_custom_about_page_seed_hook" { print "0x" $1; found=1 } END { if (!found) exit 1 }'
)"

BLOCK_BREAKER_MENU_RENDER_HOOK_TARGET="$(
  arm-none-eabi-nm "${STUB_ELF}" |
    awk '$3 == "patch_menu_render_entry_hook" { print "0x" $1; found=1 } END { if (!found) exit 1 }'
)"

BLOCK_BREAKER_POST_RENDER_HOOK_TARGET="$(
  arm-none-eabi-nm "${STUB_ELF}" |
    awk '$3 == "patch_block_breaker_post_render_wait_hook" { print "0x" $1; found=1 } END { if (!found) exit 1 }'
)"

BLOCK_BREAKER_EVENT_SET_HOOK_TARGET="$(
  arm-none-eabi-nm "${STUB_ELF}" |
    awk '$3 == "patch_event_set_hook" { print "0x" $1; found=1 } END { if (!found) exit 1 }'
)"

python3 "${APPLY_SCRIPT}" \
  --stub-bin "${STUB_BIN}" \
  --target-build SX567-0401 \
  --patch-capacity-imm \
  --capacity-imm-off 0x61792 \
  --capacity-imm-orig-hword 0x220b \
  --capacity-imm-new-hword 0x220e \
  --patch-menu-hook \
  --hook-off 0x6194e \
  --hook-orig-target 0x08064e8c \
  --patch-expanded-capacity-imm \
  --expanded-capacity-imm-off 0x6153e \
  --expanded-capacity-imm-orig-hword 0x2210 \
  --expanded-capacity-imm-new-hword 0x2213 \
  --patch-expanded-menu-hook \
  --expanded-hook-off 0x6177e \
  --expanded-hook-orig-target 0x08064e8c \
  --custom-about-label "${CUSTOM_ABOUT_LABEL}" \
  --custom-about-detail "${CUSTOM_ABOUT_DETAIL}" \
  --block-breaker-label "${BLOCK_BREAKER_LABEL}" \
  --patch-block-breaker-page \
  --block-breaker-page-seed-hook-target "${CUSTOM_PAGE_SEED_HOOK_TARGET}" \
  --block-breaker-page-hook-target "${CUSTOM_PAGE_HOOK_TARGET}" \
  --patch-block-breaker-menu-render-hook \
  --block-breaker-menu-render-hook-off 0x64fbe \
  --block-breaker-menu-render-hook-target "${BLOCK_BREAKER_MENU_RENDER_HOOK_TARGET}" \
  --patch-block-breaker-post-render-hook \
  --block-breaker-post-render-hook-target "${BLOCK_BREAKER_POST_RENDER_HOOK_TARGET}" \
  --patch-block-breaker-event-set-hook \
  --block-breaker-event-set-hook-target "${BLOCK_BREAKER_EVENT_SET_HOOK_TARGET}" \
  --custom-page-hook-target "${CUSTOM_PAGE_HOOK_TARGET}" \
  --custom-page-seed-hook-target "${CUSTOM_PAGE_SEED_HOOK_TARGET}" \
  --clinical-label "${CLINICAL_LABEL}" \
  "${SOURCE_FIRMWARE}" \
  "${PATCHED_FIRMWARE}"

if [[ "${RUN_EMULATOR}" != "1" ]]; then
  echo "station_pipeline=pass stage=patch firmware=${PATCHED_FIRMWARE} result=pass"
  exit 0
fi

export AIRBREAK_RUST_FIRMWARE="${PATCHED_FIRMWARE}"

case "${EMULATOR_MODE}" in
  gui|interactive)
    export AIRBREAK_RUST_USE_SDL="${AIRBREAK_RUST_USE_SDL:-1}"
    "${ROOT_DIR}/scripts/run-gui.sh"
    echo "station_pipeline=pass stage=emulate mode=gui firmware=${PATCHED_FIRMWARE} result=pass"
    ;;
  headless|check|ci)
    export AIRBREAK_RUST_CLEAN_RUN_ROOT="${AIRBREAK_RUST_CLEAN_RUN_ROOT:-0}"
    export AIRBREAK_RUST_MAX_INSTRUCTIONS="${HEADLESS_MAX_INSTRUCTIONS}"
    "${ROOT_DIR}/scripts/run-gui-bringup-check.sh"
    echo "station_pipeline=pass stage=emulate mode=headless firmware=${PATCHED_FIRMWARE} result=pass"
    ;;
  *)
    echo "station_pipeline=fail stage=emulate reason=invalid_emulator_mode mode=${EMULATOR_MODE} result=fail" >&2
    echo "valid AIRBREAK_EMULATOR_MODE values: gui, headless" >&2
    exit 2
    ;;
esac
