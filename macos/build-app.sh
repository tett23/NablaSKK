#!/bin/sh
# Build AquaSKK-Rust.app (macOS input method).
#
#   sh macos/build-app.sh
#
# Output: macos/dist/AquaSKK-Rust.app
# Install: cp -r macos/dist/AquaSKK-Rust.app ~/Library/Input\ Methods/
#          then log out and back in (or kill any old instance), and add
#          "AquaSKK-Rust" in System Settings > Keyboard > Input Sources.
set -e
cd "$(dirname "$0")/.."

echo "==> building Rust engine"
cargo build --release -p aquaskk-ffi

APP=macos/dist/AquaSKK-Rust.app
CONTENTS="$APP/Contents"

rm -rf "$APP"
mkdir -p "$CONTENTS/MacOS" "$CONTENTS/Resources"

echo "==> building Swift input method"
swiftc -O \
    -import-objc-header crates/aquaskk-ffi/include/aquaskk.h \
    swift/SKKSession.swift \
    macos/Sources/SKKRustInputController.swift \
    macos/Sources/main.swift \
    target/release/libaquaskk_ffi.a \
    -framework Cocoa -framework InputMethodKit \
    -o "$CONTENTS/MacOS/AquaSKKRust"

cp macos/Info.plist "$CONTENTS/Info.plist"
printf 'APPL????' > "$CONTENTS/PkgInfo"

echo "==> ad-hoc code signing"
codesign --force --sign - "$APP"

echo "==> done: $APP"
