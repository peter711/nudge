#!/usr/bin/env python3
"""Draw the nudge app icon (1024x1024 RGBA PNG).

The macOS icon grid leaves a transparent margin around an 824x824 rounded square.
Run scripts/gen-app-icon.sh to build the .icns and the sized PNGs.
"""
import struct
import zlib
from pathlib import Path

OUT = Path(__file__).resolve().parent.parent / "src-tauri" / "icons" / "icon-1024.png"

SIZE = 1024
SS = 2  # render at 2048 and box-downsample; 4 samples per output pixel
W = SIZE * SS

MARGIN = 100
SQUARE = SIZE - 2 * MARGIN
RADIUS = 185

PILL_W = 520
PILL_H = 220
PILL_R = PILL_H / 2

TOP = (44, 44, 46)
BOTTOM = (18, 18, 20)
GREEN = (48, 209, 88)


def rounded_square(x, y):
    cx = cy = SIZE / 2
    half = SQUARE / 2
    r = RADIUS
    qx = abs(x - cx) - (half - r)
    qy = abs(y - cy) - (half - r)
    outside = (max(qx, 0.0) ** 2 + max(qy, 0.0) ** 2) ** 0.5
    return outside + min(max(qx, qy), 0.0) - r <= 0


def capsule(x, y):
    cx = cy = SIZE / 2
    half = PILL_W / 2 - PILL_R
    # distance to the horizontal segment from (cx-half) to (cx+half)
    dx = max(abs(x - cx) - half, 0.0)
    dy = y - cy
    return (dx * dx + dy * dy) ** 0.5 <= PILL_R


def main():
    # premultiplied RGBA at supersampled resolution
    buf = bytearray(W * W * 4)

    for py in range(W):
        y = (py + 0.5) / SS
        t = y / SIZE
        bg = tuple(round(TOP[i] + (BOTTOM[i] - TOP[i]) * t) for i in range(3))
        row = py * W * 4
        for px in range(W):
            x = (px + 0.5) / SS
            i = row + px * 4
            if capsule(x, y):
                r, g, b = GREEN
            elif rounded_square(x, y):
                r, g, b = bg
            else:
                continue
            buf[i] = r
            buf[i + 1] = g
            buf[i + 2] = b
            buf[i + 3] = 255

    # box downsample (premultiplied, so edges do not darken)
    rows = []
    for y in range(SIZE):
        row = bytearray((0,))
        for x in range(SIZE):
            r = g = b = a = 0
            for sy in range(SS):
                base = ((y * SS + sy) * W + x * SS) * 4
                for sx in range(SS):
                    i = base + sx * 4
                    r += buf[i]
                    g += buf[i + 1]
                    b += buf[i + 2]
                    a += buf[i + 3]
            samples = SS * SS
            opaque = a // 255
            if opaque == 0:
                row += bytes((0, 0, 0, 0))
            else:
                row += bytes(
                    (
                        min(255, r // opaque),
                        min(255, g // opaque),
                        min(255, b // opaque),
                        a // samples,
                    )
                )
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
