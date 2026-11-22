#!/usr/bin/env python3
"""Build a .icns from an .iconset directory without iconutil.

iconutil is unreliable in sandboxed environments; the ICNS container is simple:
a header, then PNG-coded chunks (type, big-endian length, bytes).
"""
import struct
import sys
from pathlib import Path

# (pixel size, ICNS type, preferred file name inside the iconset)
CHUNKS = [
    (16, b"icp4", "icon_16x16.png"),
    (32, b"icp5", "icon_16x16@2x.png"),
    (64, b"icp6", "icon_32x32@2x.png"),
    (128, b"ic07", "icon_128x128.png"),
    (256, b"ic08", "icon_128x128@2x.png"),
    (512, b"ic09", "icon_256x256@2x.png"),
    (1024, b"ic10", "icon_512x512@2x.png"),
]


def main(iconset: Path, out: Path):
    body = bytearray()
    for _size, kind, name in CHUNKS:
        path = iconset / name
        data = path.read_bytes()
        if not data.startswith(b"\x89PNG\r\n\x1a\n"):
            raise SystemExit(f"{path} is not a PNG")
        body += kind + struct.pack(">I", len(data) + 8) + data

    icns = b"icns" + struct.pack(">I", len(body) + 8) + bytes(body)
    out.write_bytes(icns)
    print(f"wrote {out} ({len(icns)} bytes)")


if __name__ == "__main__":
    if len(sys.argv) != 3:
        raise SystemExit("usage: make-icns.py <iconset-dir> <out.icns>")
    main(Path(sys.argv[1]), Path(sys.argv[2]))
