#!/bin/sh
# Build the Rust FFI library and run the Swift smoke test against it.
set -e
cd "$(dirname "$0")/.."

cargo build --release -p nablaskk-ffi

swiftc -import-objc-header crates/nablaskk-ffi/include/nablaskk.h \
    swift/SKKSession.swift swift/smoke-test/main.swift \
    target/release/libnablaskk_ffi.a \
    -o target/release/swift-smoke-test

./target/release/swift-smoke-test
