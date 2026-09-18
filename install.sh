#!/bin/sh
# NablaSKK installer: downloads the latest release and installs the
# input method into ~/Library/Input Methods.
#
#   curl -fsSL https://raw.githubusercontent.com/tett23/NablaSKK/main/install.sh | sh
#
# curl does not set the quarantine attribute, so Gatekeeper does not
# block the app; any stray attribute is cleared defensively anyway.
set -e

REPO="tett23/NablaSKK"
ASSET="NablaSKK-macos-universal.app.zip"
DEST="$HOME/Library/Input Methods"

if [ "$(uname)" != "Darwin" ]; then
    echo "NablaSKK: macOS only" >&2
    exit 1
fi

TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT

echo "==> downloading latest release"
curl -fL --progress-bar \
    "https://github.com/$REPO/releases/latest/download/$ASSET" \
    -o "$TMP/$ASSET"

echo "==> installing to $DEST"
ditto -x -k "$TMP/$ASSET" "$TMP"
mkdir -p "$DEST"
rm -rf "$DEST/NablaSKK.app"
ditto "$TMP/NablaSKK.app" "$DEST/NablaSKK.app"
xattr -dr com.apple.quarantine "$DEST/NablaSKK.app" 2>/dev/null || true

# Restart a running instance so the new build is picked up
pkill -x NablaSKK 2>/dev/null || true

cat <<'MSG'
==> done

Next steps:
  1. Log out and back in (first install only)
  2. System Settings > Keyboard > Input Sources > Edit > "+"
     and add "NablaSKK" from Japanese
  3. Dictionaries: ~/Library/Application Support/NablaSKK/dictionaries.conf
MSG
