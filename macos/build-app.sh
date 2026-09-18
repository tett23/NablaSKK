#!/bin/sh
# Build NablaSKK.app (macOS input method).
#
#   sh macos/build-app.sh
#
# Output: macos/dist/NablaSKK.app
# Install: cp -r macos/dist/NablaSKK.app ~/Library/Input\ Methods/
#          then log out and back in (or kill any old instance), and add
#          "NablaSKK" in System Settings > Keyboard > Input Sources.
set -e
cd "$(dirname "$0")/.."

echo "==> building Rust engine"
cargo build --release -p nablaskk-ffi

APP=macos/dist/NablaSKK.app
CONTENTS="$APP/Contents"

rm -rf "$APP"
mkdir -p "$CONTENTS/MacOS" "$CONTENTS/Resources"

echo "==> building Swift input method"
swiftc -O \
    -import-objc-header crates/nablaskk-ffi/include/nablaskk.h \
    swift/SKKSession.swift \
    macos/Sources/SKKRustInputController.swift \
    macos/Sources/main.swift \
    target/release/libnablaskk_ffi.a \
    -framework Cocoa -framework InputMethodKit \
    -o "$CONTENTS/MacOS/NablaSKK"

cp macos/Info.plist "$CONTENTS/Info.plist"
printf 'APPL????' > "$CONTENTS/PkgInfo"

echo "==> ad-hoc code signing"
codesign --force --sign - "$APP"

echo "==> done: $APP"
