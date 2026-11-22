#!/usr/bin/env bash
# Build the macOS app icon from scripts/gen-app-icon.py.
#   scripts/gen-app-icon.sh
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
ICONS="$ROOT/src-tauri/icons"
SOURCE="$ICONS/icon-1024.png"
ICONSET="$(mktemp -d)/nudge.iconset"

python3 "$ROOT/scripts/gen-app-icon.py"

mkdir -p "$ICONSET"
sips -z 16 16     "$SOURCE" --out "$ICONSET/icon_16x16.png"      >/dev/null
sips -z 32 32     "$SOURCE" --out "$ICONSET/icon_16x16@2x.png"   >/dev/null
sips -z 32 32     "$SOURCE" --out "$ICONSET/icon_32x32.png"      >/dev/null
sips -z 64 64     "$SOURCE" --out "$ICONSET/icon_32x32@2x.png"   >/dev/null
sips -z 128 128   "$SOURCE" --out "$ICONSET/icon_128x128.png"    >/dev/null
sips -z 256 256   "$SOURCE" --out "$ICONSET/icon_128x128@2x.png" >/dev/null
sips -z 256 256   "$SOURCE" --out "$ICONSET/icon_256x256.png"    >/dev/null
sips -z 512 512   "$SOURCE" --out "$ICONSET/icon_256x256@2x.png" >/dev/null
sips -z 512 512   "$SOURCE" --out "$ICONSET/icon_512x512.png"    >/dev/null
cp "$SOURCE" "$ICONSET/icon_512x512@2x.png"

python3 "$ROOT/scripts/make-icns.py" "$ICONSET" "$ICONS/icon.icns"
cp "$ICONSET/icon_32x32.png"     "$ICONS/32x32.png"
cp "$ICONSET/icon_128x128.png"   "$ICONS/128x128.png"
cp "$ICONSET/icon_128x128@2x.png" "$ICONS/128x128@2x.png"
cp "$SOURCE" "$ICONS/icon.png"

rm -rf "$(dirname "$ICONSET")"
rm -f "$SOURCE"

echo "icon.icns, 32x32.png, 128x128.png, 128x128@2x.png and icon.png updated"
