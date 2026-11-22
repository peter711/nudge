#!/usr/bin/env bash
# nudge — curl installer for macOS.
#
#   curl -fsSL https://raw.githubusercontent.com/peter711/nudge/main/install.sh | bash
#   curl -fsSL ... | bash -s -- --system
#   curl -fsSL ... | bash -s -- --uninstall
#
# Flags:
#   --system     install into /Applications (needs sudo if not writable)
#   --no-launch  do not open the app after installing
#   --uninstall  remove the app (add --purge to delete saved state too)
#   --purge      with --uninstall: also remove ~/Library/Application Support/peter711.nudge
#
# Overrides (mostly for testing/forks):
#   NUDGE_REPO=peter711/nudge
#   NUDGE_INSTALL_DIR=$HOME/Applications
#   NUDGE_ZIP=/path/to/nudge-universal.zip   # skip the download
set -euo pipefail

REPO="${NUDGE_REPO:-peter711/nudge}"
APP_NAME="nudge"
ASSET="nudge-universal.zip"
STATE_DIR="$HOME/Library/Application Support/peter711.nudge"

INSTALL_DIR="${NUDGE_INSTALL_DIR:-$HOME/Applications}"
LAUNCH=1
UNINSTALL=0
PURGE=0

usage() {
  cat <<'EOF'
nudge installer

  curl -fsSL .../install.sh | bash
  curl -fsSL .../install.sh | bash -s -- --system
  curl -fsSL .../install.sh | bash -s -- --uninstall --purge

Flags:
  --system     install into /Applications (sudo if needed)
  --no-launch  do not open the app after installing
  --uninstall  remove nudge.app
  --purge      with --uninstall: also delete the saved task
EOF
}

for arg in "$@"; do
  case "$arg" in
    --system) INSTALL_DIR="/Applications" ;;
    --no-launch) LAUNCH=0 ;;
    --uninstall) UNINSTALL=1 ;;
    --purge) PURGE=1 ;;
    -h|--help) usage; exit 0 ;;
    *)
      echo "Unknown argument: $arg" >&2
      exit 2
      ;;
  esac
done

if [ "$(uname -s)" != "Darwin" ]; then
  echo "nudge is macOS-only." >&2
  exit 1
fi

if [ "$UNINSTALL" -eq 1 ]; then
  TARGET=""
  CANDIDATES=("/Applications/$APP_NAME.app" "$HOME/Applications/$APP_NAME.app")
  if [ -n "${NUDGE_INSTALL_DIR:-}" ]; then
    CANDIDATES=("$INSTALL_DIR/$APP_NAME.app")
  fi
  for candidate in "${CANDIDATES[@]}"; do
    if [ -d "$candidate" ]; then
      TARGET="$candidate"
      break
    fi
  done

  if [ -n "$TARGET" ]; then
    pkill -x "$APP_NAME" >/dev/null 2>&1 || true
    if [ -w "$(dirname "$TARGET")" ]; then
      rm -rf "$TARGET"
    else
      sudo rm -rf "$TARGET"
    fi
    echo "Removed $TARGET"
  else
    echo "nudge.app is not installed in /Applications or ~/Applications."
  fi

  if [ "$PURGE" -eq 1 ]; then
    rm -rf "$STATE_DIR"
    echo "Removed saved state: $STATE_DIR"
  else
    echo "Saved state kept in: $STATE_DIR (use --purge to remove it)."
  fi
  exit 0
fi

MACOS_MAJOR=$(sw_vers -productVersion | cut -d. -f1)
if [ "$MACOS_MAJOR" -lt 14 ]; then
  echo "nudge requires macOS 14 or newer (found $(sw_vers -productVersion))." >&2
  exit 1
fi

TMP_DIR=$(mktemp -d "${TMPDIR:-/tmp}/nudge-install.XXXXXX")
trap 'rm -rf "$TMP_DIR"' EXIT

if [ -n "${NUDGE_ZIP:-}" ]; then
  echo "Using local archive: $NUDGE_ZIP"
  cp "$NUDGE_ZIP" "$TMP_DIR/$ASSET"
else
  URL="https://github.com/$REPO/releases/latest/download/$ASSET"
  echo "Downloading nudge from ${REPO}…"
  if ! curl -fL --retry 3 --connect-timeout 15 -o "$TMP_DIR/$ASSET" "$URL"; then
    echo "Download failed. Is the repository public and does a release exist?" >&2
    echo "URL: $URL" >&2
    exit 1
  fi
fi

ditto -x -k "$TMP_DIR/$ASSET" "$TMP_DIR/unpacked"
APP_PATH=$(find "$TMP_DIR/unpacked" -maxdepth 3 -name "$APP_NAME.app" -print -quit)
if [ -z "$APP_PATH" ]; then
  echo "The archive does not contain $APP_NAME.app." >&2
  exit 1
fi

DEST="$INSTALL_DIR/$APP_NAME.app"
mkdir -p "$INSTALL_DIR"

if [ -w "$INSTALL_DIR" ]; then
  SUDO=""
else
  SUDO="sudo"
fi

pkill -x "$APP_NAME" >/dev/null 2>&1 || true
echo "Installing to ${DEST}…"
$SUDO rm -rf "$DEST"
$SUDO ditto "$APP_PATH" "$DEST"

# curl does not set quarantine, but this also covers archives fetched by hand.
$SUDO xattr -dr com.apple.quarantine "$DEST" >/dev/null 2>&1 || true

/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister \
  -f "$DEST" >/dev/null 2>&1 || true

echo "Installed: $DEST"

if [ "$LAUNCH" -eq 1 ]; then
  if [ "$(launchctl managername 2>/dev/null || true)" = "Aqua" ]; then
    open "$DEST"
  else
    echo "No GUI session detected; run: open \"$DEST\""
  fi
fi
