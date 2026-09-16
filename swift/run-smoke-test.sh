#!/bin/sh
# Build the Rust FFI library and run the Swift smoke test against it.
set -e
cd "$(dirname "$0")/.."

cargo build --release -p aquaskk-ffi

swiftc -import-objc-header crates/aquaskk-ffi/include/aquaskk.h \
    swift/SKKSession.swift swift/smoke-test/main.swift \
    target/release/libaquaskk_ffi.a \
    -o target/release/swift-smoke-test

./target/release/swift-smoke-test
