#!/usr/bin/env python3
"""Compare two RGB/RGBA PNG files without external image dependencies."""

from __future__ import annotations

import argparse
from pathlib import Path
import struct
import sys
import zlib


PNG_SIGNATURE = b"\x89PNG\r\n\x1a\n"


def paeth(a: int, b: int, c: int) -> int:
    p = a + b - c
    pa = abs(p - a)
    pb = abs(p - b)
    pc = abs(p - c)
    if pa <= pb and pa <= pc:
        return a
    if pb <= pc:
        return b
    return c


def read_png(path: Path) -> tuple[int, int, list[tuple[int, int, int]]]:
    data = path.read_bytes()
    if not data.startswith(PNG_SIGNATURE):
        raise ValueError("not_png")

    pos = len(PNG_SIGNATURE)
    width = height = bit_depth = color_type = None
    idat = bytearray()
    while pos + 8 <= len(data):
        length = struct.unpack(">I", data[pos : pos + 4])[0]
        kind = data[pos + 4 : pos + 8]
        payload = data[pos + 8 : pos + 8 + length]
        if pos + 12 + length > len(data):
            raise ValueError("truncated_chunk")
        pos += 12 + length
        if kind == b"IHDR":
            width, height, bit_depth, color_type, _, _, _ = struct.unpack(">IIBBBBB", payload)
        elif kind == b"IDAT":
            idat.extend(payload)
        elif kind == b"IEND":
            break

    if width is None or height is None or bit_depth is None or color_type is None:
        raise ValueError("missing_ihdr")
    if bit_depth != 8 or color_type not in (2, 6):
        raise ValueError(f"unsupported_png_color:{bit_depth}:{color_type}")

    channels = 3 if color_type == 2 else 4
    stride = width * channels
    raw = zlib.decompress(bytes(idat))
    prior = [0] * stride
    offset = 0
    pixels: list[tuple[int, int, int]] = []
    for _ in range(height):
        if offset >= len(raw):
            raise ValueError("truncated_scanline")
        filt = raw[offset]
        offset += 1
        row = list(raw[offset : offset + stride])
        if len(row) != stride:
            raise ValueError("truncated_scanline")
        offset += stride
        for i, value in enumerate(row):
            left = row[i - channels] if i >= channels else 0
            up = prior[i]
            up_left = prior[i - channels] if i >= channels else 0
            if filt == 1:
                row[i] = (value + left) & 0xFF
            elif filt == 2:
                row[i] = (value + up) & 0xFF
            elif filt == 3:
                row[i] = (value + ((left + up) // 2)) & 0xFF
            elif filt == 4:
                row[i] = (value + paeth(left, up, up_left)) & 0xFF
            elif filt != 0:
                raise ValueError(f"bad_png_filter:{filt}")
        for x in range(width):
            base = x * channels
            pixels.append((row[base], row[base + 1], row[base + 2]))
        prior = row

    return width, height, pixels


def png_chunk(kind: bytes, payload: bytes) -> bytes:
    return (
        struct.pack(">I", len(payload))
        + kind
        + payload
        + struct.pack(">I", zlib.crc32(kind + payload) & 0xFFFFFFFF)
    )


def write_rgb_png(path: Path, width: int, height: int, pixels: list[tuple[int, int, int]]) -> None:
    raw = bytearray()
    row_len = width * 3
    for y in range(height):
        raw.append(0)
        start = y * width
        for r, g, b in pixels[start : start + width]:
            raw.extend((r, g, b))
        if len(raw) < (y + 1) * (row_len + 1):
            raise ValueError("bad_pixel_count")

    path.parent.mkdir(parents=True, exist_ok=True)
    payload = (
        PNG_SIGNATURE
        + png_chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, 2, 0, 0, 0))
        + png_chunk(b"IDAT", zlib.compress(bytes(raw), level=9))
        + png_chunk(b"IEND", b"")
    )
    path.write_bytes(payload)


def build_diff_pixels(
    expected: list[tuple[int, int, int]], actual: list[tuple[int, int, int]]
) -> list[tuple[int, int, int]]:
    out: list[tuple[int, int, int]] = []
    for lhs, rhs in zip(expected, actual):
        if lhs == rhs:
            r, g, b = rhs
            out.append((r // 4, g // 4, b // 4))
        else:
            out.append((255, 0, 255))
    return out


def compare(args: argparse.Namespace) -> int:
    expected_path = Path(args.expected)
    actual_path = Path(args.actual)

    if not expected_path.is_file():
        print(f"png_compare=fail reason=missing_expected expected={expected_path} result=fail")
        return 2
    if not actual_path.is_file():
        print(f"png_compare=fail reason=missing_actual actual={actual_path} result=fail")
        return 2

    try:
        ew, eh, expected = read_png(expected_path)
        aw, ah, actual = read_png(actual_path)
    except Exception as exc:
        print(f"png_compare=fail reason=decode_error error={exc} result=fail")
        return 2

    if (ew, eh) != (aw, ah):
        print(
            "png_compare=fail "
            f"reason=dimension_mismatch expected_size={ew}x{eh} actual_size={aw}x{ah} result=fail"
        )
        return 1

    mismatched = 0
    max_channel_delta = 0
    for lhs, rhs in zip(expected, actual):
        delta = max(abs(lhs[i] - rhs[i]) for i in range(3))
        if delta > args.max_channel_delta:
            mismatched += 1
        max_channel_delta = max(max_channel_delta, delta)

    total = ew * eh
    allowed = args.max_mismatch_pixels
    if args.max_mismatch_ratio > 0:
        allowed = max(allowed, int(total * args.max_mismatch_ratio))

    passed = mismatched <= allowed
    if not passed and args.diff:
        write_rgb_png(Path(args.diff), ew, eh, build_diff_pixels(expected, actual))

    print(
        f"png_compare={'pass' if passed else 'fail'} "
        f"size={ew}x{eh} pixels={total} mismatched_pixels={mismatched} "
        f"allowed_mismatched_pixels={allowed} max_channel_delta={max_channel_delta} "
        f"result={'pass' if passed else 'fail'}"
    )
    return 0 if passed else 1


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--expected", required=True)
    parser.add_argument("--actual", required=True)
    parser.add_argument("--diff", default="")
    parser.add_argument("--max-mismatch-pixels", type=int, default=0)
    parser.add_argument("--max-mismatch-ratio", type=float, default=0.0)
    parser.add_argument("--max-channel-delta", type=int, default=0)
    args = parser.parse_args()
    if args.max_mismatch_pixels < 0:
        parser.error("--max-mismatch-pixels must be non-negative")
    if args.max_mismatch_ratio < 0:
        parser.error("--max-mismatch-ratio must be non-negative")
    if args.max_channel_delta < 0:
        parser.error("--max-channel-delta must be non-negative")
    return compare(args)


if __name__ == "__main__":
    raise SystemExit(main())
