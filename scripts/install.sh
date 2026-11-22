#!/usr/bin/env bash
# Install nudge.app into /Applications from a local build or a .dmg.
#
#   scripts/install.sh                                  # use the local release build
#   scripts/install.sh ~/Downloads/nudge_0.1.0_aarch64.dmg
#   scripts/install.sh path/to/nudge.app --no-launch
#
# Works over SSH/scp too: files copied with scp/rsync/curl carry no quarantine
# attribute, so Gatekeeper stays quiet. For browser-downloaded DMGs the script
# strips the quarantine flag after copying.
set -euo pipefail

LAUNCH=1
SOURCE=""

for arg in "$@"; do
  case "$arg" in
    --no-launch) LAUNCH=0 ;;
    -h|--help)
      sed -n '2,10p' "$0" | sed 's/^# \{0,1\}//'
      exit 0
      ;;
    *)
      if [ -n "$SOURCE" ]; then
        echo "Unexpected argument: $arg" >&2
        exit 2
      fi
      SOURCE="$arg"
      ;;
  esac
done

if [ -z "$SOURCE" ]; then
  SOURCE="src-tauri/target/release/bundle/macos/nudge.app"
fi

if [ ! -e "$SOURCE" ]; then
  echo "Not found: $SOURCE" >&2
  echo "Build it first with: npm run tauri build" >&2
  exit 1
fi

MOUNT=""
cleanup() {
  if [ -n "$MOUNT" ]; then
    hdiutil detach "$MOUNT" -quiet >/dev/null 2>&1 || true
  fi
}
trap cleanup EXIT

if [ "${SOURCE##*.}" = "dmg" ]; then
  echo "Mounting ${SOURCE}…"
  MOUNT=$(hdiutil attach -nobrowse -readonly "$SOURCE" | awk -F '\t' 'END {print $NF}')
  SOURCE=$(find "$MOUNT" -maxdepth 2 -name '*.app' -print -quit)
  if [ -z "$SOURCE" ]; then
    echo "No .app inside the image." >&2
    exit 1
  fi
fi

if [ ! -d "$SOURCE" ]; then
  echo "Not an .app bundle: $SOURCE" >&2
  exit 1
fi

APP_NAME=$(basename "$SOURCE")
DEST="/Applications/$APP_NAME"

if [ -w /Applications ]; then
  SUDO=""
else
  SUDO="sudo"
fi

# Quit a running copy first so the bundle can be replaced cleanly.
pkill -x nudge >/dev/null 2>&1 || true

echo "Installing to ${DEST}…"
$SUDO rm -rf "$DEST"
$SUDO ditto "$SOURCE" "$DEST"

# Browser downloads get com.apple.quarantine; an unsigned app would be blocked.
$SUDO xattr -dr com.apple.quarantine "$DEST" >/dev/null 2>&1 || true

# Make sure LaunchServices notices the new bundle right away.
/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister \
  -f "$DEST" >/dev/null 2>&1 || true

echo "Installed: $DEST"

if [ "$LAUNCH" -eq 1 ]; then
  open "$DEST"
fi
