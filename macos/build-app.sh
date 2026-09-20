#!/bin/sh
# Build NablaSKK.app (macOS input method).
#
#   sh macos/build-app.sh                # host architecture, ad-hoc signed
#   UNIVERSAL=1 sh macos/build-app.sh   # arm64 + x86_64 universal binary
#   SIGN_IDENTITY="Developer ID Application: ..." sh macos/build-app.sh
#                                        # hardened-runtime Developer ID signing
#
# Output: macos/dist/NablaSKK.app
# Install: cp -r macos/dist/NablaSKK.app ~/Library/Input\ Methods/
#          then log out and back in (or kill any old instance), and add
#          "NablaSKK" in System Settings > Keyboard > Input Sources.
set -e
cd "$(dirname "$0")/.."

APP=macos/dist/NablaSKK.app
CONTENTS="$APP/Contents"
SWIFT_SOURCES="swift/SKKSession.swift \
    macos/Sources/KeyTranslator.swift \
    macos/Sources/SKKRustInputController.swift \
    macos/Sources/main.swift"
MACOS_TARGET=12

rm -rf "$APP"
mkdir -p "$CONTENTS/MacOS" "$CONTENTS/Resources"

build_swift() {
    # $1 = swift target triple, $2 = static library, $3 = output
    swiftc -O \
        -target "$1" \
        -import-objc-header crates/nablaskk-ffi/include/nablaskk.h \
        $SWIFT_SOURCES \
        "$2" \
        -framework Cocoa -framework InputMethodKit \
        -o "$3"
}

if [ "${UNIVERSAL:-0}" = "1" ]; then
    echo "==> building Rust engine (arm64 + x86_64)"
    rustup target add aarch64-apple-darwin x86_64-apple-darwin >/dev/null
    cargo build --release -p nablaskk-ffi --target aarch64-apple-darwin
    cargo build --release -p nablaskk-ffi --target x86_64-apple-darwin

    mkdir -p target/universal
    lipo -create \
        target/aarch64-apple-darwin/release/libnablaskk_ffi.a \
        target/x86_64-apple-darwin/release/libnablaskk_ffi.a \
        -output target/universal/libnablaskk_ffi.a

    echo "==> building Swift input method (universal)"
    build_swift "arm64-apple-macos$MACOS_TARGET" \
        target/universal/libnablaskk_ffi.a "$CONTENTS/MacOS/NablaSKK.arm64"
    build_swift "x86_64-apple-macos$MACOS_TARGET" \
        target/universal/libnablaskk_ffi.a "$CONTENTS/MacOS/NablaSKK.x86_64"
    lipo -create \
        "$CONTENTS/MacOS/NablaSKK.arm64" "$CONTENTS/MacOS/NablaSKK.x86_64" \
        -output "$CONTENTS/MacOS/NablaSKK"
    rm "$CONTENTS/MacOS/NablaSKK.arm64" "$CONTENTS/MacOS/NablaSKK.x86_64"
else
    echo "==> building Rust engine"
    cargo build --release -p nablaskk-ffi

    echo "==> building Swift input method"
    build_swift "$(uname -m | sed 's/^aarch64$/arm64/')-apple-macos$MACOS_TARGET" \
        target/release/libnablaskk_ffi.a "$CONTENTS/MacOS/NablaSKK"
fi

cp macos/Info.plist "$CONTENTS/Info.plist"
printf 'APPL????' > "$CONTENTS/PkgInfo"

# Developer ID signing when SIGN_IDENTITY is set; ad-hoc otherwise.
SIGN_IDENTITY="${SIGN_IDENTITY:--}"
if [ "$SIGN_IDENTITY" = "-" ]; then
    echo "==> ad-hoc code signing"
    codesign --force --sign - "$APP"
else
    echo "==> code signing as: $SIGN_IDENTITY"
    codesign --force --options runtime --timestamp --sign "$SIGN_IDENTITY" "$APP"
fi

echo "==> done: $APP"
lipo -info "$CONTENTS/MacOS/NablaSKK" 2>/dev/null || true
