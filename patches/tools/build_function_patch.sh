#!/usr/bin/env bash
set -euo pipefail

# Usage:
#   patches/tools/build_function_patch.sh \
#     --src patches/templates/function_patch_template.c \
#     --ld  patches/templates/function_patch_template.ld \
#     --out-dir patches/out

SRC=""
LD_SCRIPT=""
OUT_DIR=""
DEFINES=()

while [[ $# -gt 0 ]]; do
  case "$1" in
    --src)
      SRC="$2"
      shift 2
      ;;
    --ld)
      LD_SCRIPT="$2"
      shift 2
      ;;
    --out-dir)
      OUT_DIR="$2"
      shift 2
      ;;
    --define)
      DEFINES+=("-D$2")
      shift 2
      ;;
    *)
      echo "Unknown arg: $1" >&2
      exit 1
      ;;
  esac
done

if [[ -z "$SRC" || -z "$LD_SCRIPT" || -z "$OUT_DIR" ]]; then
  echo "Missing required args. Need --src, --ld, --out-dir" >&2
  exit 1
fi

if ! command -v arm-none-eabi-gcc >/dev/null 2>&1; then
  echo "arm-none-eabi-gcc not found in PATH" >&2
  exit 1
fi

mkdir -p "$OUT_DIR"

BASE_NAME="$(basename "$SRC" .c)"
OBJ="$OUT_DIR/$BASE_NAME.o"
ELF="$OUT_DIR/$BASE_NAME.elf"
BIN="$OUT_DIR/$BASE_NAME.bin"
MAP="$OUT_DIR/$BASE_NAME.map"
DIS="$OUT_DIR/$BASE_NAME.disasm.txt"
SIZE_TXT="$OUT_DIR/$BASE_NAME.size.txt"

arm-none-eabi-gcc \
  -mcpu=cortex-m4 \
  -mthumb \
  -ffreestanding \
  -fno-builtin \
  -fomit-frame-pointer \
  -fno-unwind-tables \
  -fno-asynchronous-unwind-tables \
  -fno-exceptions \
  -fdata-sections \
  -ffunction-sections \
  -Os \
  "${DEFINES[@]}" \
  -c "$SRC" -o "$OBJ"

arm-none-eabi-ld \
  -T "$LD_SCRIPT" \
  "$OBJ" \
  -Map "$MAP" \
  -o "$ELF"

arm-none-eabi-objcopy -O binary "$ELF" "$BIN"
arm-none-eabi-objdump -d "$ELF" > "$DIS"
arm-none-eabi-size -A "$ELF" > "$SIZE_TXT"

BIN_SIZE=$(wc -c < "$BIN" | tr -d '[:space:]')

echo "Build complete"
echo "  ELF : $ELF"
echo "  BIN : $BIN"
echo "  MAP : $MAP"
echo "  SIZE: ${BIN_SIZE} bytes"
echo "  DIS : $DIS"
