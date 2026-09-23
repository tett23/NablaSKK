#!/bin/sh
# Build and run the macOS key translation tests.
set -e
cd "$(dirname "$0")/.."

cargo build --release -p nablaskk-ffi

swiftc -import-objc-header crates/nablaskk-ffi/include/nablaskk.h \
    swift/SKKSession.swift macos/Shared/DictionaryConfig.swift macos/Shared/UserDictionaryModel.swift macos/Shared/InputSettings.swift macos/Shared/KeymapConfig.swift macos/Sources/KeyTranslator.swift macos/Sources/ClientQuirks.swift macos/Tests/main.swift \
    target/release/libnablaskk_ffi.a \
    -framework Cocoa \
    -o target/release/macos-key-tests

./target/release/macos-key-tests
