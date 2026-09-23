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
    macos/Shared/DictionaryConfig.swift \
    macos/Shared/UserDictionaryModel.swift \
    macos/Shared/InputSettings.swift \
    macos/Shared/KeymapConfig.swift \
    macos/Sources/KeyTranslator.swift \
    macos/Sources/ClientQuirks.swift \
    macos/Sources/SKKRustInputController.swift \
    macos/Sources/main.swift"
PREFS_SOURCES="macos/Shared/DictionaryConfig.swift \
    macos/Shared/UserDictionaryModel.swift \
    macos/Shared/InputSettings.swift \
    macos/Preferences/PreferencesApp.swift \
    macos/Shared/KeymapConfig.swift \
    macos/Preferences/UserDictionaryView.swift \
    macos/Preferences/InputSettingsView.swift \
    macos/Preferences/KeymapSettingsView.swift"
PREFS_APP="$CONTENTS/Resources/NablaSKK Preferences.app"
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

build_prefs() {
    # $1 = swift target triple, $2 = output
    swiftc -O -parse-as-library \
        -target "$1" \
        $PREFS_SOURCES \
        -framework SwiftUI -framework Cocoa \
        -o "$2"
}

# The preferences app lives inside the input method bundle so a single
# copy installs both.
prepare_prefs_bundle() {
    mkdir -p "$PREFS_APP/Contents/MacOS"
    cp macos/Preferences/Info.plist "$PREFS_APP/Contents/Info.plist"
    printf 'APPL????' > "$PREFS_APP/Contents/PkgInfo"
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

    echo "==> building preferences app (universal)"
    prepare_prefs_bundle
    build_prefs "arm64-apple-macos$MACOS_TARGET" "$PREFS_APP/Contents/MacOS/prefs.arm64"
    build_prefs "x86_64-apple-macos$MACOS_TARGET" "$PREFS_APP/Contents/MacOS/prefs.x86_64"
    lipo -create "$PREFS_APP/Contents/MacOS/prefs.arm64" "$PREFS_APP/Contents/MacOS/prefs.x86_64" \
        -output "$PREFS_APP/Contents/MacOS/NablaSKK Preferences"
    rm "$PREFS_APP/Contents/MacOS/prefs.arm64" "$PREFS_APP/Contents/MacOS/prefs.x86_64"
else
    echo "==> building Rust engine"
    cargo build --release -p nablaskk-ffi

    echo "==> building Swift input method"
    build_swift "$(uname -m | sed 's/^aarch64$/arm64/')-apple-macos$MACOS_TARGET" \
        target/release/libnablaskk_ffi.a "$CONTENTS/MacOS/NablaSKK"

    echo "==> building preferences app"
    prepare_prefs_bundle
    build_prefs "$(uname -m | sed 's/^aarch64$/arm64/')-apple-macos$MACOS_TARGET" \
        "$PREFS_APP/Contents/MacOS/NablaSKK Preferences"
fi

cp macos/Info.plist "$CONTENTS/Info.plist"
printf 'APPL????' > "$CONTENTS/PkgInfo"

# Developer ID signing when SIGN_IDENTITY is set; ad-hoc otherwise.
SIGN_IDENTITY="${SIGN_IDENTITY:--}"
if [ "$SIGN_IDENTITY" = "-" ]; then
    echo "==> ad-hoc code signing"
    codesign --force --sign - "$PREFS_APP"
    codesign --force --sign - "$APP"
else
    echo "==> code signing as: $SIGN_IDENTITY"
    codesign --force --options runtime --timestamp --sign "$SIGN_IDENTITY" "$PREFS_APP"
    codesign --force --options runtime --timestamp --sign "$SIGN_IDENTITY" "$APP"
fi

echo "==> done: $APP"
lipo -info "$CONTENTS/MacOS/NablaSKK" 2>/dev/null || true
