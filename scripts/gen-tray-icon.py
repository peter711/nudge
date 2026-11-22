#!/usr/bin/env python3
"""Generate the macOS template tray icon (black + alpha, no colour).

Run: python3 scripts/gen-tray-icon.py
"""
import struct
import zlib
from pathlib import Path

SIZE = 32
SS = 8  # supersampling factor
OUT = Path(__file__).resolve().parent.parent / "src-tauri" / "icons" / "tray.png"


def rounded_rect_coverage(x, y):
    """Return coverage 0..1 for a capsule inside the 32x32 canvas."""
    left, right = 5.0, 27.0
    top, bottom = 10.0, 22.0
    r = (bottom - top) / 2.0
    # capsule: union of the inner rectangle and two end circles
    if left + r <= x <= right - r:
        inside = top <= y <= bottom
    else:
        cx = left + r if x < left + r else right - r
        cy = (top + bottom) / 2.0
        inside = (x - cx) ** 2 + (y - cy) ** 2 <= r * r
    return 255 if inside else 0


def main():
    w = h = SIZE * SS
    alpha = bytearray(w * h)
    for py in range(h):
        y = (py + 0.5) / SS
        for px in range(w):
            x = (px + 0.5) / SS
            alpha[py * w + px] = rounded_rect_coverage(x, y)

    # box-filter downsample to SIZE x SIZE
    rows = []
    for y in range(SIZE):
        row = bytearray()
        row.append(0)  # filter: none
        for x in range(SIZE):
            total = 0
            for sy in range(SS):
                base = (y * SS + sy) * w + x * SS
                total += sum(alpha[base:base + SS])
            a = total // (SS * SS)
            row += bytes((0, 0, 0, a))  # black with alpha -> template image
        rows.append(bytes(row))

    raw = b"".join(rows)

    def chunk(tag, data):
        return (
            struct.pack(">I", len(data))
            + tag
            + data
            + struct.pack(">I", zlib.crc32(tag + data) & 0xFFFFFFFF)
        )

    png = b"\x89PNG\r\n\x1a\n"
    png += chunk(b"IHDR", struct.pack(">IIBBBBB", SIZE, SIZE, 8, 6, 0, 0, 0))
    png += chunk(b"IDAT", zlib.compress(raw, 9))
    png += chunk(b"IEND", b"")

    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_bytes(png)
    print(f"wrote {OUT} ({len(png)} bytes)")


if __name__ == "__main__":
    main()
