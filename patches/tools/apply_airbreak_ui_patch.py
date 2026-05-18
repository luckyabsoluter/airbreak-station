#!/usr/bin/env python3
"""
Static patch helper for AirBreak Station firmware UI experimentation.

Current behavior:
1) Apply startup cmp patch at 0xF0 by default (can be skipped for debugging).
2) Hook the rendered My Options append paths and add AirBreak-owned Block Breaker,
   Custom About, and Clinical Mode rows.
3) Repoint page index 14 into an AirBreak-owned custom page host.
4) Inject an externally built code-cave payload and custom AirBreak/Clinical strings.
"""

from __future__ import annotations

import argparse
import struct
from pathlib import Path


FLASH_BASE = 0x08000000
# Verified offsets from current reversing session.
OFF_MY_OPTIONS_EN = 0x00017588
OFF_BACK_EN = 0x00019194        # "Back"
OFF_TEXT_PTR_TABLE_MY_OPTIONS = 0x0001F9E0  # contains 0x08017588
OFF_TEXT_PTR_TABLE_BACK = 0x000203F0        # contains 0x08019194
OFF_STARTUP_CHECK_CMP = 0x000000F0          # cmp r4, r0 -> nop (0x46c0)
OFF_BUILD_ID = 0x00040000       # "SX567-0401"
MAX_CODE_CAVE_LEN = 0x000FFFFE - 0x000FF000

# Free region (0xFF bytes) confirmed near firmware tail.
OFF_CODE_CAVE = 0x000FF000
OFF_HOOK_MENU_APPEND_CALL = 0x0006194E      # rendered My Options final append
OFF_MY_OPTIONS_CAPACITY_IMM = 0x00061792    # rendered My Options movs r2, #11 in FUN_08061434 path
OFF_HOOK_EXPANDED_MENU_APPEND_CALL = 0x0006177E  # Plus-expanded rendered My Options final append
OFF_EXPANDED_MY_OPTIONS_CAPACITY_IMM = 0x0006153E  # Plus-expanded rendered My Options movs r2, #16
OFF_CUSTOM_ABOUT_LABEL_PTR = 0x0002078C      # English text pointer reached by blank label id 0xE2
OFF_CUSTOM_ABOUT_DETAIL_PTR = 0x000207A8     # English text pointer reached by blank label id 0xE3
ADDR_ORIG_BLANK_TEXT = 0x08019AF8            # original empty string pointer at the slot above
OFF_BLOCK_BREAKER_LABEL_PTR = 0x00020818      # blank label id 0xE7
OFF_CUSTOM_PAGE_TITLE_PTR = 0x00020834        # blank label id 0xE8, points to SRAM title buffer
OFF_BLOCK_BREAKER_STATUS_PTR = 0x00020850     # blank label id 0xE9
OFF_BLOCK_BREAKER_ROW0_PTR = 0x0002127C       # blank label id 0x146
OFF_BLOCK_BREAKER_ROW1_PTR = 0x00021298       # blank label id 0x147
OFF_BLOCK_BREAKER_ROW2_PTR = 0x000216A4       # blank label id 0x16C
OFF_BLOCK_BREAKER_LEFT_LABEL_PTR = 0x000216F8  # blank label id 0x16F
OFF_BLOCK_BREAKER_RIGHT_LABEL_PTR = 0x00021E4C  # blank label id 0x1B2
OFF_BLOCK_BREAKER_FIRE_LABEL_PTR = 0x00021E68  # blank label id 0x1B3
ADDR_CUSTOM_PAGE_TITLE_TEXT = 0x2001FCE0
ADDR_BLOCK_BREAKER_STATUS_TEXT = 0x2001FD00
ADDR_BLOCK_BREAKER_ROW0_TEXT = 0x2001FD20
ADDR_BLOCK_BREAKER_ROW1_TEXT = 0x2001FD40
ADDR_BLOCK_BREAKER_ROW2_TEXT = 0x2001FD60
OFF_CLINICAL_MODE_LABEL_PTR = 0x00020000     # English text pointer resolved by nav-row label id 0x3A
ADDR_ORIG_CLINICAL_MODE_LABEL = 0x0801869C   # original "View Oximeter" pointer at the slot above
OFF_BLOCK_BREAKER_PAGE_TITLE_IMM = 0x00063700  # page index 13 ctor: movs r3,#0x09 title id
OFF_BLOCK_BREAKER_PAGE_CAPACITY_IMM = 0x00063702  # page index 13 ctor: movs r2,#0x03 capacity
OFF_BLOCK_BREAKER_PAGE_SEED_APPEND_CALL = 0x00063734  # page index 13 Back append, seeds AirBreak rows
OFF_BLOCK_BREAKER_PAGE_TAIL_APPEND_CALL = 0x00063774  # page index 13 final append
OFF_BLOCK_BREAKER_MENU_RENDER_ENTRY_HOOK = 0x0006510C  # menu render entry, branched to a tail-overlay hook
OFF_BLOCK_BREAKER_POST_RENDER_WAIT_CALL = 0x0008DFE6  # delay predicate call used after page redraw work
OFF_EVENT_SET_ENTRY_HOOK = 0x00066E7E  # global event setter entry, gated while Block Breaker is active
ADDR_BLOCK_BREAKER_POST_RENDER_WAIT_ORIG = 0x0808EAB4
HW_BLOCK_BREAKER_PAGE_TITLE_ORIG = 0x2309
HW_BLOCK_BREAKER_PAGE_TITLE_PATCH = 0x23E8
HW_BLOCK_BREAKER_PAGE_CAPACITY_ORIG = 0x2203
HW_BLOCK_BREAKER_PAGE_CAPACITY_PATCH = 0x2214
OFF_CUSTOM_PAGE_TITLE_IMM = 0x00063786       # page index 14 ctor: movs r3,#0x48 title id
OFF_CUSTOM_PAGE_CAPACITY_IMM = 0x00063788    # page index 14 ctor: movs r2,#0x09 capacity
OFF_CUSTOM_PAGE_BACK_ROW_IMM = 0x000637A2    # page index 14 Back nav action: movs r2,#0x3c row
OFF_CUSTOM_PAGE_BACK_INDEX_IMM = 0x000637A4  # page index 14 Back nav action: movs r1,#6 index
OFF_CUSTOM_PAGE_SEED_APPEND_CALL = 0x000637D0  # page index 14 Back append, seeds AirBreak rows first
OFF_CUSTOM_PAGE_TAIL_APPEND_CALL = 0x000639D0  # page index 14 final append, replaced by AirBreak page tail
HW_CUSTOM_PAGE_TITLE_ORIG = 0x2348           # movs r3,#0x48
HW_CUSTOM_PAGE_TITLE_PATCH = 0x23E8          # movs r3,#0xE8 (AirBreak-owned dynamic title)
HW_CUSTOM_PAGE_TITLE_LEGACY_PATCH = 0x23E2   # old fixed Custom About title label
HW_CUSTOM_PAGE_CAPACITY_ORIG = 0x2209        # movs r2,#0x09
HW_CUSTOM_PAGE_CAPACITY_PATCH = 0x2214       # movs r2,#0x14
HW_CUSTOM_PAGE_BACK_ROW_ORIG = 0x223C        # movs r2,#0x3c
HW_CUSTOM_PAGE_BACK_ROW_LEGACY_PATCH = 0x2200  # old fixed Back patch to row 0
HW_CUSTOM_PAGE_BACK_INDEX_ORIG = 0x2106      # movs r1,#6
HW_CUSTOM_PAGE_BACK_INDEX_LEGACY_PATCH = 0x2102  # old fixed Back patch to Essentials=On My Options
BYTES_ORIG_STARTUP_CHECK = bytes.fromhex("844203d1")  # cmp r4,r0 ; bne +3
BYTES_PATCH_STARTUP_CHECK = b"\xC0\x46"     # nop
BYTES_ORIG_MENU_RENDER_ENTRY = bytes.fromhex("f8b50446")  # push {r3,r4,r5,r6,r7,lr}; mov r4,r0
BYTES_ORIG_EVENT_SET_ENTRY = bytes.fromhex("10b590b0")  # push {r4,lr}; sub sp,#0x40
ADDR_MENU_APPEND = 0x08064E8C

SUPPORTED_BUILD_IDS = {
    "SX567-0401": b"SX567-0401",
}

# Legacy built-in code-cave stub @ 0x080FF000.
# Current station pipeline uses an external stub; this remains only for manual
# experiments against older append-hook sites.
# 1) execute original append(menu, about_item)
# 2) allocate/create another about-style item
# 3) set item text object (item+0x18) to custom ASCII string via 0x0806C72A
# 4) append extra item to menu list
#
# NOTE:
# - hello-detail-text is staged in code cave data for dedicated handler flow.
# - Current constructor family is known to expose one text object path (+0x18).
STUB_CODE = bytes.fromhex(
    "30b5044665f742ff542064f7a7fe70b140f2fa1167f70efc05460849002205f118006df782fb2946204665f72fff30bd"
)
STUB_MENU_PTR_LIT_OFF = 0x3C
STUB_MENU_STR_OFF = 0x40

# 0401 계열에서 관찰된 CRC 구간/저장 위치
SEGMENTS = (
    (0x00000, 0x04000, 0x03FFE),
    (0x04000, 0x40000, 0x3FFFE),
    (0x40000, 0x100000, 0xFFFFE),
)


def assert_dword(buf: bytes, off: int, expected: int, name: str) -> None:
    got = struct.unpack_from("<I", buf, off)[0]
    if got != expected:
        raise RuntimeError(
            f"{name} mismatch at 0x{off:08X}: got 0x{got:08X}, expected 0x{expected:08X}"
        )


def assert_bytes(buf: bytes, off: int, expected: bytes, name: str) -> None:
    got = bytes(buf[off : off + len(expected)])
    if got != expected:
        raise RuntimeError(
            f"{name} mismatch at 0x{off:08X}: got {got.hex()}, expected {expected.hex()}"
        )


def align_up(val: int, align: int) -> int:
    return (val + align - 1) & ~(align - 1)


def encode_thumb_bl(src_addr: int, dst_addr: int) -> bytes:
    """
    Encode Thumb-2 BL immediate from src_addr to dst_addr.
    src_addr is the virtual address of the BL instruction (first halfword).
    """
    off = dst_addr - (src_addr + 4)
    if (off & 1) != 0:
        raise RuntimeError(f"BL target must be halfword-aligned: src=0x{src_addr:08X}, dst=0x{dst_addr:08X}")
    if off < -(1 << 24) or off > ((1 << 24) - 2):
        raise RuntimeError(f"BL target out of range: src=0x{src_addr:08X}, dst=0x{dst_addr:08X}")

    s = (off >> 24) & 1
    i1 = (off >> 23) & 1
    i2 = (off >> 22) & 1
    imm10 = (off >> 12) & 0x3FF
    imm11 = (off >> 1) & 0x7FF
    j1 = ((~i1) & 1) ^ s
    j2 = ((~i2) & 1) ^ s

    hw1 = 0xF000 | (s << 10) | imm10
    hw2 = 0xD000 | (j1 << 13) | (1 << 12) | (j2 << 11) | imm11
    return struct.pack("<HH", hw1, hw2)


def encode_thumb_b_w(src_addr: int, dst_addr: int) -> bytes:
    """
    Encode Thumb-2 B.W immediate from src_addr to dst_addr.
    src_addr is the virtual address of the B.W instruction (first halfword).
    """
    off = dst_addr - (src_addr + 4)
    if (off & 1) != 0:
        raise RuntimeError(f"B.W target must be halfword-aligned: src=0x{src_addr:08X}, dst=0x{dst_addr:08X}")
    if off < -(1 << 24) or off > ((1 << 24) - 2):
        raise RuntimeError(f"B.W target out of range: src=0x{src_addr:08X}, dst=0x{dst_addr:08X}")

    s = (off >> 24) & 1
    i1 = (off >> 23) & 1
    i2 = (off >> 22) & 1
    imm10 = (off >> 12) & 0x3FF
    imm11 = (off >> 1) & 0x7FF
    j1 = ((~i1) & 1) ^ s
    j2 = ((~i2) & 1) ^ s

    hw1 = 0xF000 | (s << 10) | imm10
    hw2 = 0x9000 | (j1 << 13) | (1 << 12) | (j2 << 11) | imm11
    return struct.pack("<HH", hw1, hw2)


def patch_capacity_halfword(
    fw: bytearray,
    off: int,
    orig_hword: int,
    new_hword: int,
    name: str,
) -> None:
    if off < 0 or off + 2 > len(fw):
        raise RuntimeError(f"{name} capacity offset out of range: 0x{off:08X}")
    cur_hw = struct.unpack_from("<H", fw, off)[0]
    valid_hws = (orig_hword & 0xFFFF, new_hword & 0xFFFF)
    if cur_hw not in valid_hws:
        raise RuntimeError(
            f"unexpected halfword at {name} capacity site 0x{off:08X}: "
            f"0x{cur_hw:04X} (expected 0x{valid_hws[0]:04X} or 0x{valid_hws[1]:04X})"
        )
    struct.pack_into("<H", fw, off, valid_hws[1])


def expected_bl_pair(src_off: int, orig_target: int, patched_target: int) -> tuple[int, bytes, bytes]:
    src_va = FLASH_BASE + src_off
    return (
        src_va,
        encode_thumb_bl(src_va, orig_target),
        encode_thumb_bl(src_va, patched_target),
    )


def assert_patchable_bl(
    fw: bytearray,
    off: int,
    orig_call: bytes,
    patched_call: bytes,
    name: str,
) -> None:
    if off < 0 or off + 4 > len(fw):
        raise RuntimeError(f"{name} hook offset out of range: 0x{off:08X}")
    cur_call = bytes(fw[off : off + 4])
    if cur_call not in (orig_call, patched_call):
        raise RuntimeError(
            f"unexpected bytes at {name} hook site 0x{off:08X}: "
            f"{cur_call.hex()} (expected {orig_call.hex()} or {patched_call.hex()})"
        )


def encode_ascii_field(text: str, field_name: str) -> bytes:
    try:
        out = text.encode("ascii")
    except UnicodeEncodeError as exc:
        raise RuntimeError(
            f"{field_name} must be ASCII for this sample patch (got non-ASCII text)"
        ) from exc
    if not out:
        raise RuntimeError(f"{field_name} must not be empty")
    if b"\x00" in out:
        raise RuntimeError(f"{field_name} must not include NUL bytes")
    return out


def detect_build_id(fw: bytes) -> str:
    for build_name, sig in SUPPORTED_BUILD_IDS.items():
        if fw[OFF_BUILD_ID : OFF_BUILD_ID + len(sig)] == sig:
            return build_name
    raw = fw[OFF_BUILD_ID : OFF_BUILD_ID + 12]
    return raw.split(b"\x00", 1)[0].decode("ascii", errors="replace")


def build_stub_blob(hello_text: str, hello_detail_text: str) -> tuple[bytes, int, int]:
    menu_ascii = encode_ascii_field(hello_text, "--hello-text") + b"\x00"
    detail_ascii = encode_ascii_field(hello_detail_text, "--hello-detail-text") + b"\x00"

    blob = bytearray(STUB_CODE)
    if len(blob) > STUB_MENU_PTR_LIT_OFF:
        raise RuntimeError("internal stub layout error: code overlaps literal slot")

    # keep fixed literal location used by ldr r1, [pc, #32]
    blob.extend(b"\x00" * (STUB_MENU_PTR_LIT_OFF - len(blob)))
    menu_str_va = FLASH_BASE + OFF_CODE_CAVE + STUB_MENU_STR_OFF
    blob.extend(struct.pack("<I", menu_str_va))

    if len(blob) < STUB_MENU_STR_OFF:
        blob.extend(b"\x00" * (STUB_MENU_STR_OFF - len(blob)))

    menu_off = len(blob)
    blob.extend(menu_ascii)
    blob.extend(b"\x00" * (align_up(len(blob), 4) - len(blob)))

    detail_off = len(blob)
    blob.extend(detail_ascii)
    blob.extend(b"\x00" * (align_up(len(blob), 4) - len(blob)))

    if len(blob) > MAX_CODE_CAVE_LEN:
        raise RuntimeError(
            f"stub+data too large for code cave: {len(blob)} > {MAX_CODE_CAVE_LEN}"
        )
    return bytes(blob), menu_off, detail_off


def append_custom_texts(
    stub_blob: bytes,
    label_text: str,
    detail_text: str,
    clinical_text: str,
    block_breaker_text: str,
    block_left_text: str,
    block_right_text: str,
    block_fire_text: str,
) -> tuple[bytes, int, int, int, int, int, int, int]:
    label_ascii = encode_ascii_field(label_text, "--custom-about-label") + b"\x00"
    detail_ascii = encode_ascii_field(detail_text, "--custom-about-detail") + b"\x00"
    clinical_ascii = encode_ascii_field(clinical_text, "--clinical-label") + b"\x00"
    block_breaker_ascii = encode_ascii_field(block_breaker_text, "--block-breaker-label") + b"\x00"
    block_left_ascii = encode_ascii_field(block_left_text, "--block-breaker-left-label") + b"\x00"
    block_right_ascii = encode_ascii_field(block_right_text, "--block-breaker-right-label") + b"\x00"
    block_fire_ascii = encode_ascii_field(block_fire_text, "--block-breaker-fire-label") + b"\x00"

    blob = bytearray(stub_blob)
    blob.extend(b"\x00" * (align_up(len(blob), 4) - len(blob)))
    label_off = len(blob)
    blob.extend(label_ascii)
    blob.extend(b"\x00" * (align_up(len(blob), 4) - len(blob)))
    detail_off = len(blob)
    blob.extend(detail_ascii)
    blob.extend(b"\x00" * (align_up(len(blob), 4) - len(blob)))
    clinical_off = len(blob)
    blob.extend(clinical_ascii)
    blob.extend(b"\x00" * (align_up(len(blob), 4) - len(blob)))
    block_breaker_off = len(blob)
    blob.extend(block_breaker_ascii)
    blob.extend(b"\x00" * (align_up(len(blob), 4) - len(blob)))
    block_left_off = len(blob)
    blob.extend(block_left_ascii)
    blob.extend(b"\x00" * (align_up(len(blob), 4) - len(blob)))
    block_right_off = len(blob)
    blob.extend(block_right_ascii)
    blob.extend(b"\x00" * (align_up(len(blob), 4) - len(blob)))
    block_fire_off = len(blob)
    blob.extend(block_fire_ascii)
    blob.extend(b"\x00" * (align_up(len(blob), 4) - len(blob)))

    if len(blob) > MAX_CODE_CAVE_LEN:
        raise RuntimeError(
            f"stub+custom text too large for code cave: {len(blob)} > {MAX_CODE_CAVE_LEN}"
        )
    return (
        bytes(blob),
        label_off,
        detail_off,
        clinical_off,
        block_breaker_off,
        block_left_off,
        block_right_off,
        block_fire_off,
    )


def patch_pointer_slot(
    fw: bytearray,
    off: int,
    expected_original: int,
    patched_value: int,
    name: str,
) -> None:
    cur = struct.unpack_from("<I", fw, off)[0]
    valid_ptrs = (expected_original, patched_value)
    if cur not in valid_ptrs:
        raise RuntimeError(
            f"unexpected {name} pointer at 0x{off:08X}: "
            f"0x{cur:08X} (expected 0x{valid_ptrs[0]:08X} or 0x{valid_ptrs[1]:08X})"
        )
    struct.pack_into("<I", fw, off, patched_value)


def crc16_ccitt_false(data: bytes, init: int = 0xFFFF, poly: int = 0x1021) -> int:
    crc = init
    for b in data:
        crc ^= (b << 8)
        for _ in range(8):
            if crc & 0x8000:
                crc = ((crc << 1) ^ poly) & 0xFFFF
            else:
                crc = (crc << 1) & 0xFFFF
    return crc


def validate_crc_zero(fw: bytes) -> tuple[int, int, int]:
    vals = []
    for start, end, _ in SEGMENTS:
        vals.append(crc16_ccitt_false(fw[start:end]))
    return tuple(vals)


def fix_segment_crc(fw: bytearray, start: int, end: int, crc_off: int) -> None:
    new_crc = crc16_ccitt_false(bytes(fw[start : end - 2]))
    fw[crc_off] = (new_crc >> 8) & 0xFF
    fw[crc_off + 1] = new_crc & 0xFF


def fix_all_crc(fw: bytearray) -> tuple[int, int, int]:
    for start, end, crc_off in SEGMENTS:
        fix_segment_crc(fw, start, end, crc_off)
    return validate_crc_zero(bytes(fw))


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("input_bin", type=Path)
    ap.add_argument("output_bin", type=Path)
    ap.add_argument(
        "--target-build",
        choices=sorted(SUPPORTED_BUILD_IDS),
        default="SX567-0401",
        help="firmware build guard (default: SX567-0401)",
    )
    ap.add_argument(
        "--skip-crc-fix",
        action="store_true",
        help="do not repair firmware CRC fields after patch (debug only; usually unbootable)",
    )
    ap.add_argument(
        "--skip-startup-check",
        action="store_true",
        help=(
            "skip startup cmp patch at 0xF0 (debug only; observed symptom: black screen + "
            "power LED blink + reboot loop)"
        ),
    )
    ap.add_argument(
        "--patch-menu-hook",
        action=argparse.BooleanOptionalAction,
        default=True,
        help=f"patch My Options builder hook at 0x{OFF_HOOK_MENU_APPEND_CALL:06X} (default: enabled)",
    )
    ap.add_argument(
        "--hook-off",
        type=lambda x: int(x, 0),
        default=OFF_HOOK_MENU_APPEND_CALL,
        help=f"hook call-site file offset (default: 0x{OFF_HOOK_MENU_APPEND_CALL:06X})",
    )
    ap.add_argument(
        "--hook-orig-target",
        type=lambda x: int(x, 0),
        default=ADDR_MENU_APPEND,
        help=f"expected original BL target virtual address (default: 0x{ADDR_MENU_APPEND:08X})",
    )
    ap.add_argument(
        "--patch-capacity-imm",
        action=argparse.BooleanOptionalAction,
        default=True,
        help="patch My Options ctor capacity immediate (default: enabled)",
    )
    ap.add_argument(
        "--capacity-imm-off",
        type=lambda x: int(x, 0),
        default=OFF_MY_OPTIONS_CAPACITY_IMM,
        help=f"file offset for capacity immediate patch (default: 0x{OFF_MY_OPTIONS_CAPACITY_IMM:06X})",
    )
    ap.add_argument(
        "--capacity-imm-orig-hword",
        type=lambda x: int(x, 0),
        default=0x220B,
        help="expected original Thumb halfword at capacity site (default: 0x220b, movs r2,#11)",
    )
    ap.add_argument(
        "--capacity-imm-new-hword",
        type=lambda x: int(x, 0),
        default=0x220E,
        help="replacement Thumb halfword at capacity site (default: 0x220e, movs r2,#14)",
    )
    ap.add_argument(
        "--patch-expanded-menu-hook",
        action=argparse.BooleanOptionalAction,
        default=True,
        help=(
            f"patch Plus-expanded rendered My Options hook at 0x{OFF_HOOK_EXPANDED_MENU_APPEND_CALL:06X} "
            "(default: enabled)"
        ),
    )
    ap.add_argument(
        "--expanded-hook-off",
        type=lambda x: int(x, 0),
        default=OFF_HOOK_EXPANDED_MENU_APPEND_CALL,
        help=f"Plus-expanded hook call-site file offset (default: 0x{OFF_HOOK_EXPANDED_MENU_APPEND_CALL:06X})",
    )
    ap.add_argument(
        "--expanded-hook-orig-target",
        type=lambda x: int(x, 0),
        default=ADDR_MENU_APPEND,
        help=f"expected Plus-expanded original BL target virtual address (default: 0x{ADDR_MENU_APPEND:08X})",
    )
    ap.add_argument(
        "--patch-expanded-capacity-imm",
        action=argparse.BooleanOptionalAction,
        default=True,
        help="patch Plus-expanded rendered My Options ctor capacity immediate (default: enabled)",
    )
    ap.add_argument(
        "--expanded-capacity-imm-off",
        type=lambda x: int(x, 0),
        default=OFF_EXPANDED_MY_OPTIONS_CAPACITY_IMM,
        help=(
            "file offset for Plus-expanded capacity immediate patch "
            f"(default: 0x{OFF_EXPANDED_MY_OPTIONS_CAPACITY_IMM:06X})"
        ),
    )
    ap.add_argument(
        "--expanded-capacity-imm-orig-hword",
        type=lambda x: int(x, 0),
        default=0x2210,
        help="expected original Plus-expanded Thumb halfword (default: 0x2210, movs r2,#16)",
    )
    ap.add_argument(
        "--expanded-capacity-imm-new-hword",
        type=lambda x: int(x, 0),
        default=0x2213,
        help="replacement Plus-expanded Thumb halfword (default: 0x2213, movs r2,#19)",
    )
    ap.add_argument(
        "--hello-text",
        default="Hello World",
        help="ASCII text for appended menu item label (default: Hello World)",
    )
    ap.add_argument(
        "--hello-detail-text",
        default="Hello World",
        help="ASCII text staged for appended-item detail flow (default: Hello World)",
    )
    ap.add_argument(
        "--custom-about-label",
        default="Custom About",
        help="ASCII label for the injected Custom About row (default: Custom About)",
    )
    ap.add_argument(
        "--custom-about-detail",
        default="This is Custom About",
        help="ASCII body text for the injected Custom About page (default: This is Custom About)",
    )
    ap.add_argument(
        "--clinical-label",
        default="Clinical Mode",
        help="ASCII label for the injected Clinical Mode row (default: Clinical Mode)",
    )
    ap.add_argument(
        "--block-breaker-label",
        default="Block Breaker",
        help="ASCII label/title for the injected Block Breaker game (default: Block Breaker)",
    )
    ap.add_argument(
        "--block-breaker-left-label",
        default="Move Left",
        help="ASCII text for the retained legacy Block Breaker left label slot (default: Move Left)",
    )
    ap.add_argument(
        "--block-breaker-right-label",
        default="Move Right",
        help="ASCII text for the retained legacy Block Breaker right label slot (default: Move Right)",
    )
    ap.add_argument(
        "--block-breaker-fire-label",
        default="Fire",
        help="ASCII text for the retained legacy Block Breaker fire label slot (default: Fire)",
    )
    ap.add_argument(
        "--patch-custom-about-label",
        action=argparse.BooleanOptionalAction,
        default=True,
        help="patch the AirBreak-owned Custom About label slot to the code-cave string (default: enabled)",
    )
    ap.add_argument(
        "--patch-custom-about-detail",
        action=argparse.BooleanOptionalAction,
        default=True,
        help="patch the AirBreak-owned Custom About page body slot to the code-cave string (default: enabled)",
    )
    ap.add_argument(
        "--patch-clinical-label",
        action=argparse.BooleanOptionalAction,
        default=True,
        help="patch the Clinical Mode row label text pointer to the code-cave string (default: enabled)",
    )
    ap.add_argument(
        "--patch-block-breaker-labels",
        action=argparse.BooleanOptionalAction,
        default=True,
        help="patch the Block Breaker menu label and retained legacy label slots (default: enabled)",
    )
    ap.add_argument(
        "--patch-block-breaker-dynamic-text",
        action=argparse.BooleanOptionalAction,
        default=True,
        help="patch the Block Breaker board text slots to SRAM render buffers (default: enabled)",
    )
    ap.add_argument(
        "--patch-block-breaker-page",
        action=argparse.BooleanOptionalAction,
        default=False,
        help="patch page index 13 as the alternate AirBreak page host (default: disabled)",
    )
    ap.add_argument(
        "--block-breaker-page-seed-hook-off",
        type=lambda x: int(x, 0),
        default=OFF_BLOCK_BREAKER_PAGE_SEED_APPEND_CALL,
        help=(
            "file offset for alternate AirBreak page seed hook "
            f"(default: 0x{OFF_BLOCK_BREAKER_PAGE_SEED_APPEND_CALL:06X})"
        ),
    )
    ap.add_argument(
        "--block-breaker-page-seed-hook-target",
        type=lambda x: int(x, 0),
        help="virtual address of patch_custom_about_page_seed_hook inside the injected stub",
    )
    ap.add_argument(
        "--block-breaker-page-hook-off",
        type=lambda x: int(x, 0),
        default=OFF_BLOCK_BREAKER_PAGE_TAIL_APPEND_CALL,
        help=(
            "file offset for alternate AirBreak page tail hook "
            f"(default: 0x{OFF_BLOCK_BREAKER_PAGE_TAIL_APPEND_CALL:06X})"
        ),
    )
    ap.add_argument(
        "--block-breaker-page-hook-target",
        type=lambda x: int(x, 0),
        help="virtual address of patch_custom_about_page_tail_hook inside the injected stub",
    )
    ap.add_argument(
        "--patch-block-breaker-menu-render-hook",
        action=argparse.BooleanOptionalAction,
        default=False,
        help="patch the menu render entry so Block Breaker can overlay the full LCD after stock rows render",
    )
    ap.add_argument(
        "--block-breaker-menu-render-hook-off",
        type=lambda x: int(x, 0),
        default=OFF_BLOCK_BREAKER_MENU_RENDER_ENTRY_HOOK,
        help=(
            "file offset for Block Breaker menu render entry hook "
            f"(default: 0x{OFF_BLOCK_BREAKER_MENU_RENDER_ENTRY_HOOK:06X})"
        ),
    )
    ap.add_argument(
        "--block-breaker-menu-render-hook-target",
        type=lambda x: int(x, 0),
        help="virtual address of patch_menu_render_entry_hook inside the injected stub",
    )
    ap.add_argument(
        "--patch-block-breaker-post-render-hook",
        action=argparse.BooleanOptionalAction,
        default=False,
        help="patch a wait-loop call so Block Breaker can redraw after stock page rendering",
    )
    ap.add_argument(
        "--block-breaker-post-render-hook-off",
        type=lambda x: int(x, 0),
        default=OFF_BLOCK_BREAKER_POST_RENDER_WAIT_CALL,
        help=(
            "file offset for Block Breaker post-render wait hook "
            f"(default: 0x{OFF_BLOCK_BREAKER_POST_RENDER_WAIT_CALL:06X})"
        ),
    )
    ap.add_argument(
        "--block-breaker-post-render-hook-target",
        type=lambda x: int(x, 0),
        help="virtual address of patch_block_breaker_post_render_wait_hook inside the injected stub",
    )
    ap.add_argument(
        "--patch-block-breaker-event-set-hook",
        action=argparse.BooleanOptionalAction,
        default=False,
        help="patch the global event setter so stock UI page/row changes are ignored during Block Breaker",
    )
    ap.add_argument(
        "--block-breaker-event-set-hook-off",
        type=lambda x: int(x, 0),
        default=OFF_EVENT_SET_ENTRY_HOOK,
        help=(
            "file offset for Block Breaker event setter entry hook "
            f"(default: 0x{OFF_EVENT_SET_ENTRY_HOOK:06X})"
        ),
    )
    ap.add_argument(
        "--block-breaker-event-set-hook-target",
        type=lambda x: int(x, 0),
        help="virtual address of patch_event_set_hook inside the injected stub",
    )
    ap.add_argument(
        "--patch-custom-about-page",
        action=argparse.BooleanOptionalAction,
        default=True,
        help="patch page index 14 into the AirBreak Custom About detail page (default: enabled)",
    )
    ap.add_argument(
        "--custom-page-hook-off",
        type=lambda x: int(x, 0),
        default=OFF_CUSTOM_PAGE_TAIL_APPEND_CALL,
        help=f"file offset for Custom About page tail hook (default: 0x{OFF_CUSTOM_PAGE_TAIL_APPEND_CALL:06X})",
    )
    ap.add_argument(
        "--custom-page-hook-target",
        type=lambda x: int(x, 0),
        help="virtual address of patch_custom_about_page_tail_hook inside the injected stub",
    )
    ap.add_argument(
        "--custom-page-seed-hook-off",
        type=lambda x: int(x, 0),
        default=OFF_CUSTOM_PAGE_SEED_APPEND_CALL,
        help=f"file offset for Custom About page seed hook (default: 0x{OFF_CUSTOM_PAGE_SEED_APPEND_CALL:06X})",
    )
    ap.add_argument(
        "--custom-page-seed-hook-target",
        type=lambda x: int(x, 0),
        help="virtual address of patch_custom_about_page_seed_hook inside the injected stub",
    )
    ap.add_argument(
        "--custom-page-title-imm-off",
        type=lambda x: int(x, 0),
        default=OFF_CUSTOM_PAGE_TITLE_IMM,
        help=f"file offset for Custom About page title immediate (default: 0x{OFF_CUSTOM_PAGE_TITLE_IMM:06X})",
    )
    ap.add_argument(
        "--stub-bin",
        type=Path,
        help=(
            "optional external code-cave payload binary. "
            "when set, built-in stub/text generation is skipped and this blob is injected as-is"
        ),
    )
    args = ap.parse_args()

    src = args.input_bin.read_bytes()
    fw = bytearray(src)

    pre_crc = validate_crc_zero(bytes(fw))
    if pre_crc != (0, 0, 0):
        raise RuntimeError(
            f"Input CRC check failed: seg1=0x{pre_crc[0]:04x} seg2=0x{pre_crc[1]:04x} seg3=0x{pre_crc[2]:04x}"
        )

    # Guard checks to avoid patching wrong firmware revision/build.
    detected_build = detect_build_id(bytes(fw))
    expected_sig = SUPPORTED_BUILD_IDS[args.target_build]
    assert_bytes(fw, OFF_BUILD_ID, expected_sig, "target_build")
    assert_dword(fw, OFF_TEXT_PTR_TABLE_MY_OPTIONS, FLASH_BASE + OFF_MY_OPTIONS_EN, "my_options_ptr")
    assert_dword(fw, OFF_TEXT_PTR_TABLE_BACK, FLASH_BASE + OFF_BACK_EN, "back_ptr")

    startup_window = bytes(fw[OFF_STARTUP_CHECK_CMP : OFF_STARTUP_CHECK_CMP + 4])
    startup_is_orig = startup_window == BYTES_ORIG_STARTUP_CHECK
    startup_is_patched = (
        startup_window[:2] == BYTES_PATCH_STARTUP_CHECK
        and startup_window[2:] == BYTES_ORIG_STARTUP_CHECK[2:]
    )
    if not (startup_is_orig or startup_is_patched):
        raise RuntimeError(
            f"unexpected startup bytes at 0x{OFF_STARTUP_CHECK_CMP:08X}: "
            f"{startup_window.hex()} (expected {BYTES_ORIG_STARTUP_CHECK.hex()} or {BYTES_PATCH_STARTUP_CHECK.hex() + BYTES_ORIG_STARTUP_CHECK[2:].hex()})"
        )
    startup_patch_applied = not args.skip_startup_check
    if startup_patch_applied:
        fw[OFF_STARTUP_CHECK_CMP : OFF_STARTUP_CHECK_CMP + 2] = BYTES_PATCH_STARTUP_CHECK
    else:
        print(
            "WARN: startup cmp patch skipped (--skip-startup-check). "
            "Observed hardware symptom: black screen + power LED blink + reboot loop."
        )
        if startup_is_patched:
            print(
                "WARN: input firmware already has startup cmp patch bytes; "
                "effective behavior may still be startup-patched."
            )

    if args.patch_expanded_menu_hook and not args.patch_menu_hook:
        raise RuntimeError("--patch-menu-hook must be enabled when --patch-expanded-menu-hook is enabled")

    capacity_patch_applied = args.patch_capacity_imm
    capacity_off = args.capacity_imm_off
    if capacity_patch_applied:
        patch_capacity_halfword(
            fw,
            capacity_off,
            args.capacity_imm_orig_hword,
            args.capacity_imm_new_hword,
            "Essentials=On My Options",
        )

    expanded_capacity_patch_applied = args.patch_expanded_capacity_imm
    expanded_capacity_off = args.expanded_capacity_imm_off
    if expanded_capacity_patch_applied:
        patch_capacity_halfword(
            fw,
            expanded_capacity_off,
            args.expanded_capacity_imm_orig_hword,
            args.expanded_capacity_imm_new_hword,
            "Plus-expanded rendered My Options",
        )

    hook_off = args.hook_off
    hook_dst = FLASH_BASE + OFF_CODE_CAVE
    hook_va, expected_orig_call, expected_patched_call = expected_bl_pair(
        hook_off,
        args.hook_orig_target,
        hook_dst,
    )

    # Verify expected call bytes, then patch hook based on option.
    assert_patchable_bl(fw, hook_off, expected_orig_call, expected_patched_call, "Essentials=On My Options")

    expanded_hook_patch_applied = args.patch_expanded_menu_hook
    expanded_hook_off = args.expanded_hook_off
    expanded_hook_va = FLASH_BASE + expanded_hook_off
    expected_expanded_orig_call = b""
    expected_expanded_patched_call = b""
    if expanded_hook_patch_applied:
        expanded_hook_va, expected_expanded_orig_call, expected_expanded_patched_call = expected_bl_pair(
            expanded_hook_off,
            args.expanded_hook_orig_target,
            hook_dst,
        )
        assert_patchable_bl(
            fw,
            expanded_hook_off,
            expected_expanded_orig_call,
            expected_expanded_patched_call,
            "expanded/Plus My Options",
        )

    stub_blob = b""
    menu_text_off = 0
    detail_text_off = 0
    custom_label_off = 0
    custom_detail_off = 0
    clinical_label_off = 0
    block_breaker_label_off = 0
    block_breaker_left_label_off = 0
    block_breaker_right_label_off = 0
    block_breaker_fire_label_off = 0
    custom_label_va = 0
    custom_detail_va = 0
    clinical_label_va = 0
    block_breaker_label_va = 0
    block_breaker_left_label_va = 0
    block_breaker_right_label_va = 0
    block_breaker_fire_label_va = 0
    custom_label_patch_applied = False
    custom_detail_patch_applied = False
    clinical_label_patch_applied = False
    block_breaker_label_patch_applied = False
    block_breaker_dynamic_text_patch_applied = False
    block_breaker_page_patch_applied = False
    block_breaker_menu_render_patch_applied = False
    block_breaker_post_render_patch_applied = False
    block_breaker_event_set_patch_applied = False
    custom_page_patch_applied = False
    stub_mode = "disabled"
    if args.patch_menu_hook:
        if args.stub_bin is not None:
            stub_blob = args.stub_bin.read_bytes()
            if not stub_blob:
                raise RuntimeError(f"--stub-bin is empty: {args.stub_bin}")
            if len(stub_blob) > MAX_CODE_CAVE_LEN:
                raise RuntimeError(
                    f"--stub-bin too large for code cave: {len(stub_blob)} > {MAX_CODE_CAVE_LEN}"
                )
            stub_mode = f"external:{args.stub_bin}"
        else:
            if args.hook_off == OFF_HOOK_MENU_APPEND_CALL:
                raise RuntimeError(
                    "--stub-bin is required for the default Essentials=On append hook. "
                    "The built-in legacy stub only matches the older append-call hook."
                )
            stub_blob, menu_text_off, detail_text_off = build_stub_blob(
                args.hello_text,
                args.hello_detail_text,
            )
            stub_mode = "builtin"

        if (
            args.patch_custom_about_label
            or args.patch_custom_about_detail
            or args.patch_clinical_label
            or args.patch_block_breaker_labels
        ):
            (
                stub_blob,
                custom_label_off,
                custom_detail_off,
                clinical_label_off,
                block_breaker_label_off,
                block_breaker_left_label_off,
                block_breaker_right_label_off,
                block_breaker_fire_label_off,
            ) = append_custom_texts(
                stub_blob,
                args.custom_about_label,
                args.custom_about_detail,
                args.clinical_label,
                args.block_breaker_label,
                args.block_breaker_left_label,
                args.block_breaker_right_label,
                args.block_breaker_fire_label,
            )
            custom_label_va = FLASH_BASE + OFF_CODE_CAVE + custom_label_off
            custom_detail_va = FLASH_BASE + OFF_CODE_CAVE + custom_detail_off
            clinical_label_va = FLASH_BASE + OFF_CODE_CAVE + clinical_label_off
            block_breaker_label_va = FLASH_BASE + OFF_CODE_CAVE + block_breaker_label_off
            block_breaker_left_label_va = FLASH_BASE + OFF_CODE_CAVE + block_breaker_left_label_off
            block_breaker_right_label_va = FLASH_BASE + OFF_CODE_CAVE + block_breaker_right_label_off
            block_breaker_fire_label_va = FLASH_BASE + OFF_CODE_CAVE + block_breaker_fire_label_off

        if args.patch_custom_about_label:
            cur_label_ptr = struct.unpack_from("<I", fw, OFF_CUSTOM_ABOUT_LABEL_PTR)[0]
            valid_label_ptrs = (ADDR_ORIG_BLANK_TEXT, custom_label_va)
            if cur_label_ptr not in valid_label_ptrs:
                raise RuntimeError(
                    f"unexpected AirBreak Custom About label pointer at 0x{OFF_CUSTOM_ABOUT_LABEL_PTR:08X}: "
                    f"0x{cur_label_ptr:08X} (expected 0x{valid_label_ptrs[0]:08X} or "
                    f"0x{valid_label_ptrs[1]:08X})"
                )
            struct.pack_into("<I", fw, OFF_CUSTOM_ABOUT_LABEL_PTR, custom_label_va)
            custom_label_patch_applied = True

        if args.patch_custom_about_detail:
            cur_detail_ptr = struct.unpack_from("<I", fw, OFF_CUSTOM_ABOUT_DETAIL_PTR)[0]
            valid_detail_ptrs = (ADDR_ORIG_BLANK_TEXT, custom_detail_va)
            if cur_detail_ptr not in valid_detail_ptrs:
                raise RuntimeError(
                    f"unexpected AirBreak Custom About detail pointer at 0x{OFF_CUSTOM_ABOUT_DETAIL_PTR:08X}: "
                    f"0x{cur_detail_ptr:08X} (expected 0x{valid_detail_ptrs[0]:08X} or "
                    f"0x{valid_detail_ptrs[1]:08X})"
                )
            struct.pack_into("<I", fw, OFF_CUSTOM_ABOUT_DETAIL_PTR, custom_detail_va)
            custom_detail_patch_applied = True

        if args.patch_clinical_label:
            cur_clinical_ptr = struct.unpack_from("<I", fw, OFF_CLINICAL_MODE_LABEL_PTR)[0]
            valid_clinical_ptrs = (ADDR_ORIG_CLINICAL_MODE_LABEL, clinical_label_va)
            if cur_clinical_ptr not in valid_clinical_ptrs:
                raise RuntimeError(
                    f"unexpected Clinical Mode label pointer at 0x{OFF_CLINICAL_MODE_LABEL_PTR:08X}: "
                    f"0x{cur_clinical_ptr:08X} (expected 0x{valid_clinical_ptrs[0]:08X} or "
                    f"0x{valid_clinical_ptrs[1]:08X})"
                )
            struct.pack_into("<I", fw, OFF_CLINICAL_MODE_LABEL_PTR, clinical_label_va)
            clinical_label_patch_applied = True

        if args.patch_block_breaker_labels:
            patch_pointer_slot(
                fw,
                OFF_BLOCK_BREAKER_LABEL_PTR,
                ADDR_ORIG_BLANK_TEXT,
                block_breaker_label_va,
                "Block Breaker label",
            )
            patch_pointer_slot(
                fw,
                OFF_BLOCK_BREAKER_LEFT_LABEL_PTR,
                ADDR_ORIG_BLANK_TEXT,
                block_breaker_left_label_va,
                "Block Breaker left label",
            )
            patch_pointer_slot(
                fw,
                OFF_BLOCK_BREAKER_RIGHT_LABEL_PTR,
                ADDR_ORIG_BLANK_TEXT,
                block_breaker_right_label_va,
                "Block Breaker right label",
            )
            patch_pointer_slot(
                fw,
                OFF_BLOCK_BREAKER_FIRE_LABEL_PTR,
                ADDR_ORIG_BLANK_TEXT,
                block_breaker_fire_label_va,
                "Block Breaker fire label",
            )
            block_breaker_label_patch_applied = True

        if args.patch_block_breaker_dynamic_text:
            patch_pointer_slot(
                fw,
                OFF_CUSTOM_PAGE_TITLE_PTR,
                ADDR_ORIG_BLANK_TEXT,
                ADDR_CUSTOM_PAGE_TITLE_TEXT,
                "AirBreak custom page title text",
            )
            patch_pointer_slot(
                fw,
                OFF_BLOCK_BREAKER_STATUS_PTR,
                ADDR_ORIG_BLANK_TEXT,
                ADDR_BLOCK_BREAKER_STATUS_TEXT,
                "Block Breaker status text",
            )
            patch_pointer_slot(
                fw,
                OFF_BLOCK_BREAKER_ROW0_PTR,
                ADDR_ORIG_BLANK_TEXT,
                ADDR_BLOCK_BREAKER_ROW0_TEXT,
                "Block Breaker row 0 text",
            )
            patch_pointer_slot(
                fw,
                OFF_BLOCK_BREAKER_ROW1_PTR,
                ADDR_ORIG_BLANK_TEXT,
                ADDR_BLOCK_BREAKER_ROW1_TEXT,
                "Block Breaker row 1 text",
            )
            patch_pointer_slot(
                fw,
                OFF_BLOCK_BREAKER_ROW2_PTR,
                ADDR_ORIG_BLANK_TEXT,
                ADDR_BLOCK_BREAKER_ROW2_TEXT,
                "Block Breaker row 2 text",
            )
            block_breaker_dynamic_text_patch_applied = True

        fw[hook_off : hook_off + 4] = expected_patched_call
        if expanded_hook_patch_applied:
            fw[expanded_hook_off : expanded_hook_off + 4] = expected_expanded_patched_call

        if args.patch_block_breaker_page:
            if args.block_breaker_page_hook_target is None:
                raise RuntimeError(
                    "--block-breaker-page-hook-target is required when --patch-block-breaker-page is enabled"
                )
            if args.block_breaker_page_seed_hook_target is None:
                raise RuntimeError(
                    "--block-breaker-page-seed-hook-target is required when --patch-block-breaker-page is enabled"
                )

            block_title_hw = struct.unpack_from("<H", fw, OFF_BLOCK_BREAKER_PAGE_TITLE_IMM)[0]
            valid_block_title_hws = (HW_BLOCK_BREAKER_PAGE_TITLE_ORIG, HW_BLOCK_BREAKER_PAGE_TITLE_PATCH)
            if block_title_hw not in valid_block_title_hws:
                raise RuntimeError(
                    f"unexpected Block Breaker page title halfword at 0x{OFF_BLOCK_BREAKER_PAGE_TITLE_IMM:08X}: "
                    f"0x{block_title_hw:04X} (expected 0x{valid_block_title_hws[0]:04X} or "
                    f"0x{valid_block_title_hws[1]:04X})"
                )
            struct.pack_into("<H", fw, OFF_BLOCK_BREAKER_PAGE_TITLE_IMM, HW_BLOCK_BREAKER_PAGE_TITLE_PATCH)

            block_capacity_hw = struct.unpack_from("<H", fw, OFF_BLOCK_BREAKER_PAGE_CAPACITY_IMM)[0]
            valid_block_capacity_hws = (
                HW_BLOCK_BREAKER_PAGE_CAPACITY_ORIG,
                HW_BLOCK_BREAKER_PAGE_CAPACITY_PATCH,
            )
            if block_capacity_hw not in valid_block_capacity_hws:
                raise RuntimeError(
                    f"unexpected Block Breaker page capacity halfword at 0x{OFF_BLOCK_BREAKER_PAGE_CAPACITY_IMM:08X}: "
                    f"0x{block_capacity_hw:04X} (expected 0x{valid_block_capacity_hws[0]:04X} or "
                    f"0x{valid_block_capacity_hws[1]:04X})"
                )
            struct.pack_into(
                "<H",
                fw,
                OFF_BLOCK_BREAKER_PAGE_CAPACITY_IMM,
                HW_BLOCK_BREAKER_PAGE_CAPACITY_PATCH,
            )

            block_seed_hook_off = args.block_breaker_page_seed_hook_off
            if block_seed_hook_off < 0 or block_seed_hook_off + 4 > len(fw):
                raise RuntimeError(
                    f"--block-breaker-page-seed-hook-off out of range: 0x{block_seed_hook_off:08X}"
                )
            block_seed_hook_va = FLASH_BASE + block_seed_hook_off
            expected_block_seed_orig_call = encode_thumb_bl(block_seed_hook_va, args.hook_orig_target)
            expected_block_seed_patched_call = encode_thumb_bl(
                block_seed_hook_va,
                args.block_breaker_page_seed_hook_target,
            )
            cur_block_seed_call = bytes(fw[block_seed_hook_off : block_seed_hook_off + 4])
            if cur_block_seed_call not in (
                expected_block_seed_orig_call,
                expected_block_seed_patched_call,
            ):
                raise RuntimeError(
                    f"unexpected bytes at alternate AirBreak page seed hook site 0x{block_seed_hook_off:08X}: "
                    f"{cur_block_seed_call.hex()} (expected {expected_block_seed_orig_call.hex()} or "
                    f"{expected_block_seed_patched_call.hex()})"
                )
            fw[block_seed_hook_off : block_seed_hook_off + 4] = expected_block_seed_patched_call

            block_hook_off = args.block_breaker_page_hook_off
            if block_hook_off < 0 or block_hook_off + 4 > len(fw):
                raise RuntimeError(f"--block-breaker-page-hook-off out of range: 0x{block_hook_off:08X}")
            block_hook_va = FLASH_BASE + block_hook_off
            expected_block_orig_call = encode_thumb_bl(block_hook_va, args.hook_orig_target)
            expected_block_patched_call = encode_thumb_bl(
                block_hook_va,
                args.block_breaker_page_hook_target,
            )
            cur_block_call = bytes(fw[block_hook_off : block_hook_off + 4])
            if cur_block_call not in (expected_block_orig_call, expected_block_patched_call):
                raise RuntimeError(
                    f"unexpected bytes at Block Breaker page hook site 0x{block_hook_off:08X}: "
                    f"{cur_block_call.hex()} (expected {expected_block_orig_call.hex()} or "
                    f"{expected_block_patched_call.hex()})"
                )
            fw[block_hook_off : block_hook_off + 4] = expected_block_patched_call
            block_breaker_page_patch_applied = True

        if args.patch_block_breaker_menu_render_hook:
            if args.block_breaker_menu_render_hook_target is None:
                raise RuntimeError(
                    "--block-breaker-menu-render-hook-target is required when "
                    "--patch-block-breaker-menu-render-hook is enabled"
                )

            menu_render_hook_off = args.block_breaker_menu_render_hook_off
            if menu_render_hook_off < 0 or menu_render_hook_off + 4 > len(fw):
                raise RuntimeError(
                    f"--block-breaker-menu-render-hook-off out of range: 0x{menu_render_hook_off:08X}"
                )
            menu_render_hook_va = FLASH_BASE + menu_render_hook_off
            expected_menu_render_patched_branch = encode_thumb_b_w(
                menu_render_hook_va,
                args.block_breaker_menu_render_hook_target,
            )
            cur_menu_render = bytes(fw[menu_render_hook_off : menu_render_hook_off + 4])
            if cur_menu_render not in (
                BYTES_ORIG_MENU_RENDER_ENTRY,
                expected_menu_render_patched_branch,
            ):
                raise RuntimeError(
                    f"unexpected bytes at Block Breaker menu render hook site 0x{menu_render_hook_off:08X}: "
                    f"{cur_menu_render.hex()} (expected {BYTES_ORIG_MENU_RENDER_ENTRY.hex()} or "
                    f"{expected_menu_render_patched_branch.hex()})"
                )
            fw[menu_render_hook_off : menu_render_hook_off + 4] = expected_menu_render_patched_branch
            block_breaker_menu_render_patch_applied = True

        if args.patch_block_breaker_post_render_hook:
            if args.block_breaker_post_render_hook_target is None:
                raise RuntimeError(
                    "--block-breaker-post-render-hook-target is required when "
                    "--patch-block-breaker-post-render-hook is enabled"
                )

            post_render_hook_off = args.block_breaker_post_render_hook_off
            if post_render_hook_off < 0 or post_render_hook_off + 4 > len(fw):
                raise RuntimeError(
                    f"--block-breaker-post-render-hook-off out of range: 0x{post_render_hook_off:08X}"
                )
            post_render_hook_va = FLASH_BASE + post_render_hook_off
            expected_post_render_orig_call = encode_thumb_bl(
                post_render_hook_va,
                ADDR_BLOCK_BREAKER_POST_RENDER_WAIT_ORIG,
            )
            expected_post_render_patched_call = encode_thumb_bl(
                post_render_hook_va,
                args.block_breaker_post_render_hook_target,
            )
            cur_post_render_call = bytes(fw[post_render_hook_off : post_render_hook_off + 4])
            if cur_post_render_call not in (
                expected_post_render_orig_call,
                expected_post_render_patched_call,
            ):
                raise RuntimeError(
                    f"unexpected bytes at Block Breaker post-render hook site 0x{post_render_hook_off:08X}: "
                    f"{cur_post_render_call.hex()} (expected {expected_post_render_orig_call.hex()} or "
                    f"{expected_post_render_patched_call.hex()})"
                )
            fw[post_render_hook_off : post_render_hook_off + 4] = expected_post_render_patched_call
            block_breaker_post_render_patch_applied = True

        if args.patch_block_breaker_event_set_hook:
            if args.block_breaker_event_set_hook_target is None:
                raise RuntimeError(
                    "--block-breaker-event-set-hook-target is required when "
                    "--patch-block-breaker-event-set-hook is enabled"
                )

            event_set_hook_off = args.block_breaker_event_set_hook_off
            if event_set_hook_off < 0 or event_set_hook_off + 4 > len(fw):
                raise RuntimeError(
                    f"--block-breaker-event-set-hook-off out of range: 0x{event_set_hook_off:08X}"
                )
            event_set_hook_va = FLASH_BASE + event_set_hook_off
            expected_event_set_patched_branch = encode_thumb_b_w(
                event_set_hook_va,
                args.block_breaker_event_set_hook_target,
            )
            cur_event_set = bytes(fw[event_set_hook_off : event_set_hook_off + 4])
            if cur_event_set not in (
                BYTES_ORIG_EVENT_SET_ENTRY,
                expected_event_set_patched_branch,
            ):
                raise RuntimeError(
                    f"unexpected bytes at Block Breaker event setter hook site 0x{event_set_hook_off:08X}: "
                    f"{cur_event_set.hex()} (expected {BYTES_ORIG_EVENT_SET_ENTRY.hex()} or "
                    f"{expected_event_set_patched_branch.hex()})"
                )
            fw[event_set_hook_off : event_set_hook_off + 4] = expected_event_set_patched_branch
            block_breaker_event_set_patch_applied = True

        if args.patch_custom_about_page:
            if args.custom_page_hook_target is None:
                raise RuntimeError("--custom-page-hook-target is required when --patch-custom-about-page is enabled")
            if args.custom_page_seed_hook_target is None:
                raise RuntimeError(
                    "--custom-page-seed-hook-target is required when --patch-custom-about-page is enabled"
                )

            title_imm_off = args.custom_page_title_imm_off
            if title_imm_off < 0 or title_imm_off + 2 > len(fw):
                raise RuntimeError(f"--custom-page-title-imm-off out of range: 0x{title_imm_off:08X}")
            cur_title_hw = struct.unpack_from("<H", fw, title_imm_off)[0]
            valid_title_hws = (
                HW_CUSTOM_PAGE_TITLE_ORIG,
                HW_CUSTOM_PAGE_TITLE_PATCH,
                HW_CUSTOM_PAGE_TITLE_LEGACY_PATCH,
            )
            if cur_title_hw not in valid_title_hws:
                raise RuntimeError(
                    f"unexpected Custom About page title halfword at 0x{title_imm_off:08X}: "
                    f"0x{cur_title_hw:04X} (expected 0x{valid_title_hws[0]:04X} or "
                    f"0x{valid_title_hws[1]:04X})"
                )
            struct.pack_into("<H", fw, title_imm_off, HW_CUSTOM_PAGE_TITLE_PATCH)

            custom_capacity_hw = struct.unpack_from("<H", fw, OFF_CUSTOM_PAGE_CAPACITY_IMM)[0]
            valid_custom_capacity_hws = (HW_CUSTOM_PAGE_CAPACITY_ORIG, HW_CUSTOM_PAGE_CAPACITY_PATCH)
            if custom_capacity_hw not in valid_custom_capacity_hws:
                raise RuntimeError(
                    f"unexpected Custom About page capacity halfword at 0x{OFF_CUSTOM_PAGE_CAPACITY_IMM:08X}: "
                    f"0x{custom_capacity_hw:04X} (expected 0x{valid_custom_capacity_hws[0]:04X} or "
                    f"0x{valid_custom_capacity_hws[1]:04X})"
                )
            struct.pack_into("<H", fw, OFF_CUSTOM_PAGE_CAPACITY_IMM, HW_CUSTOM_PAGE_CAPACITY_PATCH)

            back_row_hw = struct.unpack_from("<H", fw, OFF_CUSTOM_PAGE_BACK_ROW_IMM)[0]
            valid_back_row_hws = (HW_CUSTOM_PAGE_BACK_ROW_ORIG, HW_CUSTOM_PAGE_BACK_ROW_LEGACY_PATCH)
            if back_row_hw not in valid_back_row_hws:
                raise RuntimeError(
                    f"unexpected Custom About Back row halfword at 0x{OFF_CUSTOM_PAGE_BACK_ROW_IMM:08X}: "
                    f"0x{back_row_hw:04X} (expected 0x{valid_back_row_hws[0]:04X} or "
                    f"0x{valid_back_row_hws[1]:04X})"
                )
            struct.pack_into("<H", fw, OFF_CUSTOM_PAGE_BACK_ROW_IMM, HW_CUSTOM_PAGE_BACK_ROW_ORIG)

            back_index_hw = struct.unpack_from("<H", fw, OFF_CUSTOM_PAGE_BACK_INDEX_IMM)[0]
            valid_back_index_hws = (HW_CUSTOM_PAGE_BACK_INDEX_ORIG, HW_CUSTOM_PAGE_BACK_INDEX_LEGACY_PATCH)
            if back_index_hw not in valid_back_index_hws:
                raise RuntimeError(
                    f"unexpected Custom About Back index halfword at 0x{OFF_CUSTOM_PAGE_BACK_INDEX_IMM:08X}: "
                    f"0x{back_index_hw:04X} (expected 0x{valid_back_index_hws[0]:04X} or "
                    f"0x{valid_back_index_hws[1]:04X})"
                )
            struct.pack_into("<H", fw, OFF_CUSTOM_PAGE_BACK_INDEX_IMM, HW_CUSTOM_PAGE_BACK_INDEX_ORIG)

            custom_seed_hook_off = args.custom_page_seed_hook_off
            if custom_seed_hook_off < 0 or custom_seed_hook_off + 4 > len(fw):
                raise RuntimeError(f"--custom-page-seed-hook-off out of range: 0x{custom_seed_hook_off:08X}")
            custom_seed_hook_va = FLASH_BASE + custom_seed_hook_off
            expected_custom_seed_orig_call = encode_thumb_bl(custom_seed_hook_va, args.hook_orig_target)
            expected_custom_seed_patched_call = encode_thumb_bl(
                custom_seed_hook_va,
                args.custom_page_seed_hook_target,
            )
            cur_custom_seed_call = bytes(fw[custom_seed_hook_off : custom_seed_hook_off + 4])
            if cur_custom_seed_call not in (expected_custom_seed_orig_call, expected_custom_seed_patched_call):
                raise RuntimeError(
                    f"unexpected bytes at Custom About page seed hook site 0x{custom_seed_hook_off:08X}: "
                    f"{cur_custom_seed_call.hex()} (expected {expected_custom_seed_orig_call.hex()} or "
                    f"{expected_custom_seed_patched_call.hex()})"
                )
            fw[custom_seed_hook_off : custom_seed_hook_off + 4] = expected_custom_seed_patched_call

            custom_hook_off = args.custom_page_hook_off
            if custom_hook_off < 0 or custom_hook_off + 4 > len(fw):
                raise RuntimeError(f"--custom-page-hook-off out of range: 0x{custom_hook_off:08X}")
            custom_hook_va = FLASH_BASE + custom_hook_off
            expected_custom_orig_call = encode_thumb_bl(custom_hook_va, args.hook_orig_target)
            expected_custom_patched_call = encode_thumb_bl(custom_hook_va, args.custom_page_hook_target)
            cur_custom_call = bytes(fw[custom_hook_off : custom_hook_off + 4])
            if cur_custom_call not in (expected_custom_orig_call, expected_custom_patched_call):
                raise RuntimeError(
                    f"unexpected bytes at Custom About page hook site 0x{custom_hook_off:08X}: "
                    f"{cur_custom_call.hex()} (expected {expected_custom_orig_call.hex()} or "
                    f"{expected_custom_patched_call.hex()})"
                )
            fw[custom_hook_off : custom_hook_off + 4] = expected_custom_patched_call
            custom_page_patch_applied = True

        # Install stub in code cave (idempotent for same text/build settings).
        cur_stub = bytes(fw[OFF_CODE_CAVE : OFF_CODE_CAVE + len(stub_blob)])
        if cur_stub != stub_blob and any(b != 0xFF for b in cur_stub):
            raise RuntimeError(
                f"code cave not empty at 0x{OFF_CODE_CAVE:08X}; found non-FF bytes "
                "and they do not match generated stub/data layout"
            )
        fw[OFF_CODE_CAVE : OFF_CODE_CAVE + len(stub_blob)] = stub_blob

    crc_after_patch = validate_crc_zero(bytes(fw))
    if crc_after_patch != (0, 0, 0):
        if args.skip_crc_fix:
            print(
                "WARN: CRC mismatch kept (--skip-crc-fix):",
                f"seg1=0x{crc_after_patch[0]:04x} seg2=0x{crc_after_patch[1]:04x} seg3=0x{crc_after_patch[2]:04x}",
            )
        else:
            crc_after_patch = fix_all_crc(fw)
            if crc_after_patch != (0, 0, 0):
                raise RuntimeError(
                    f"CRC repair failed: seg1=0x{crc_after_patch[0]:04x} seg2=0x{crc_after_patch[1]:04x} seg3=0x{crc_after_patch[2]:04x}"
                )
    final_crc = validate_crc_zero(bytes(fw))

    args.output_bin.write_bytes(fw)

    print("Patched:", args.output_bin)
    print("  mode               :", "append-new-item")
    print("  build guard        :", f"target={args.target_build}, detected={detected_build}")
    print("  startup-check patch:", "enabled" if startup_patch_applied else "skipped (--skip-startup-check)")
    print("  capacity imm patch :", "enabled" if capacity_patch_applied else "disabled")
    if capacity_patch_applied:
        print(
            "  capacity bytes     :",
            f"off=0x{capacity_off:08X}",
            f"hw=0x{struct.unpack_from('<H', src, capacity_off)[0]:04X}",
            "->",
            f"0x{struct.unpack_from('<H', fw, capacity_off)[0]:04X}",
        )
    print("  expanded capacity  :", "enabled" if expanded_capacity_patch_applied else "disabled")
    if expanded_capacity_patch_applied:
        print(
            "  expanded cap bytes :",
            f"off=0x{expanded_capacity_off:08X}",
            f"hw=0x{struct.unpack_from('<H', src, expanded_capacity_off)[0]:04X}",
            "->",
            f"0x{struct.unpack_from('<H', fw, expanded_capacity_off)[0]:04X}",
        )
    print(
        "  startup bytes      :",
        bytes(src[OFF_STARTUP_CHECK_CMP : OFF_STARTUP_CHECK_CMP + 4]).hex(),
        "->",
        bytes(fw[OFF_STARTUP_CHECK_CMP : OFF_STARTUP_CHECK_CMP + 4]).hex(),
    )
    print("  menu-hook patch    :", "enabled" if args.patch_menu_hook else "disabled")
    if args.patch_menu_hook:
        print("  stub mode          :", stub_mode)
        print(f"  hook site          : 0x{hook_off:08X} (VA 0x{hook_va:08X}) -> bl 0x{hook_dst:08X}")
        print(f"  hook orig target   : 0x{args.hook_orig_target:08X}")
        print("  expanded hook patch:", "enabled" if expanded_hook_patch_applied else "disabled")
        if expanded_hook_patch_applied:
            print(
                f"  expanded hook site : 0x{expanded_hook_off:08X} "
                f"(VA 0x{expanded_hook_va:08X}) -> bl 0x{hook_dst:08X}"
            )
            print(f"  expanded orig target: 0x{args.expanded_hook_orig_target:08X}")
        print(f"  stub base          : 0x{OFF_CODE_CAVE:08X}")
        print(f"  stub size          : {len(stub_blob)} bytes")
        print("  block label patch  :", "enabled" if block_breaker_label_patch_applied else "disabled")
        if block_breaker_label_patch_applied:
            print(
                f"  block label        : '{args.block_breaker_label}' "
                f"ptr@0x{OFF_BLOCK_BREAKER_LABEL_PTR:08X} -> 0x{block_breaker_label_va:08X}"
            )
            print(
                "  block legacy labels:",
                f"left@0x{OFF_BLOCK_BREAKER_LEFT_LABEL_PTR:08X}->0x{block_breaker_left_label_va:08X}",
                f"right@0x{OFF_BLOCK_BREAKER_RIGHT_LABEL_PTR:08X}->0x{block_breaker_right_label_va:08X}",
                f"fire@0x{OFF_BLOCK_BREAKER_FIRE_LABEL_PTR:08X}->0x{block_breaker_fire_label_va:08X}",
            )
        print(
            "  block dynamic text :",
            "enabled" if block_breaker_dynamic_text_patch_applied else "disabled",
        )
        if block_breaker_dynamic_text_patch_applied:
            print(
                "  block text SRAM    :",
                f"title=0x{ADDR_CUSTOM_PAGE_TITLE_TEXT:08X}",
                f"status=0x{ADDR_BLOCK_BREAKER_STATUS_TEXT:08X}",
                f"rows=0x{ADDR_BLOCK_BREAKER_ROW0_TEXT:08X},0x{ADDR_BLOCK_BREAKER_ROW1_TEXT:08X},"
                f"0x{ADDR_BLOCK_BREAKER_ROW2_TEXT:08X}",
            )
        print("  block page patch   :", "enabled" if block_breaker_page_patch_applied else "disabled")
        if block_breaker_page_patch_applied:
            print(
                f"  block page hook    : 0x{args.block_breaker_page_hook_off:08X} "
                f"(VA 0x{FLASH_BASE + args.block_breaker_page_hook_off:08X}) "
                f"-> bl 0x{args.block_breaker_page_hook_target:08X}"
            )
            print(
                f"  block page seed    : 0x{args.block_breaker_page_seed_hook_off:08X} "
                f"(VA 0x{FLASH_BASE + args.block_breaker_page_seed_hook_off:08X}) "
                f"-> bl 0x{args.block_breaker_page_seed_hook_target:08X}"
            )
            print(
                f"  block page title   : off=0x{OFF_BLOCK_BREAKER_PAGE_TITLE_IMM:08X} "
                f"hw=0x{HW_BLOCK_BREAKER_PAGE_TITLE_ORIG:04X} -> 0x{HW_BLOCK_BREAKER_PAGE_TITLE_PATCH:04X}"
            )
            print(
                f"  block page capacity: off=0x{OFF_BLOCK_BREAKER_PAGE_CAPACITY_IMM:08X} "
                f"hw=0x{HW_BLOCK_BREAKER_PAGE_CAPACITY_ORIG:04X} -> 0x{HW_BLOCK_BREAKER_PAGE_CAPACITY_PATCH:04X}"
            )
        print(
            "  block render hook  :",
            "enabled" if block_breaker_menu_render_patch_applied else "disabled",
        )
        if block_breaker_menu_render_patch_applied:
            print(
                f"  block render branch: 0x{args.block_breaker_menu_render_hook_off:08X} "
                f"(VA 0x{FLASH_BASE + args.block_breaker_menu_render_hook_off:08X}) "
                f"-> b.w 0x{args.block_breaker_menu_render_hook_target:08X}"
            )
        print(
            "  block post-render :",
            "enabled" if block_breaker_post_render_patch_applied else "disabled",
        )
        if block_breaker_post_render_patch_applied:
            print(
                f"  block post hook    : 0x{args.block_breaker_post_render_hook_off:08X} "
                f"(VA 0x{FLASH_BASE + args.block_breaker_post_render_hook_off:08X}) "
                f"-> bl 0x{args.block_breaker_post_render_hook_target:08X}"
            )
        print(
            "  block event gate  :",
            "enabled" if block_breaker_event_set_patch_applied else "disabled",
        )
        if block_breaker_event_set_patch_applied:
            print(
                f"  block event branch: 0x{args.block_breaker_event_set_hook_off:08X} "
                f"(VA 0x{FLASH_BASE + args.block_breaker_event_set_hook_off:08X}) "
                f"-> b.w 0x{args.block_breaker_event_set_hook_target:08X}"
            )
        print("  custom label patch :", "enabled" if custom_label_patch_applied else "disabled")
        if custom_label_patch_applied:
            print(
                f"  custom label       : '{args.custom_about_label}' "
                f"ptr@0x{OFF_CUSTOM_ABOUT_LABEL_PTR:08X} -> 0x{custom_label_va:08X}"
            )
        print("  custom detail patch:", "enabled" if custom_detail_patch_applied else "disabled")
        if custom_detail_patch_applied:
            print(
                f"  custom detail      : '{args.custom_about_detail}' "
                f"ptr@0x{OFF_CUSTOM_ABOUT_DETAIL_PTR:08X} -> 0x{custom_detail_va:08X}"
            )
        print("  custom page patch  :", "enabled" if custom_page_patch_applied else "disabled")
        if custom_page_patch_applied:
            print(
                f"  custom page seed   : 0x{args.custom_page_seed_hook_off:08X} "
                f"(VA 0x{FLASH_BASE + args.custom_page_seed_hook_off:08X}) "
                f"-> bl 0x{args.custom_page_seed_hook_target:08X}"
            )
            print(
                f"  custom page hook   : 0x{args.custom_page_hook_off:08X} "
                f"(VA 0x{FLASH_BASE + args.custom_page_hook_off:08X}) -> bl 0x{args.custom_page_hook_target:08X}"
            )
            print(
                f"  custom page title  : off=0x{args.custom_page_title_imm_off:08X} "
                f"hw=0x{HW_CUSTOM_PAGE_TITLE_ORIG:04X} -> 0x{HW_CUSTOM_PAGE_TITLE_PATCH:04X}"
            )
            print(
                f"  custom page cap    : off=0x{OFF_CUSTOM_PAGE_CAPACITY_IMM:08X} "
                f"hw=0x{HW_CUSTOM_PAGE_CAPACITY_ORIG:04X} -> 0x{HW_CUSTOM_PAGE_CAPACITY_PATCH:04X}"
            )
            print(
                "  custom page back   : stock immediates preserved; "
                "runtime Back action returns to the Custom About entry origin"
            )
        print("  clinical label patch:", "enabled" if clinical_label_patch_applied else "disabled")
        if clinical_label_patch_applied:
            print(
                f"  clinical label     : '{args.clinical_label}' "
                f"ptr@0x{OFF_CLINICAL_MODE_LABEL_PTR:08X} -> 0x{clinical_label_va:08X}"
            )
        if args.stub_bin is None:
            menu_text_va = FLASH_BASE + OFF_CODE_CAVE + menu_text_off
            detail_text_va = FLASH_BASE + OFF_CODE_CAVE + detail_text_off
            print(f"  menu text          : '{args.hello_text}' @ 0x{menu_text_va:08X}")
            print(f"  detail text        : '{args.hello_detail_text}' @ 0x{detail_text_va:08X}")
    print("  region class       :", "main-app")
    print(
        "  crc pre            :",
        f"seg1=0x{pre_crc[0]:04x} seg2=0x{pre_crc[1]:04x} seg3=0x{pre_crc[2]:04x}",
    )
    print(
        "  crc final          :",
        f"seg1=0x{final_crc[0]:04x} seg2=0x{final_crc[1]:04x} seg3=0x{final_crc[2]:04x}",
    )
    print(
        "  crc status         :",
        "kept mismatched (debug)" if args.skip_crc_fix else "recomputed and verified",
    )
    print("NOTE: input original firmware is never modified; output file only.")


if __name__ == "__main__":
    main()
