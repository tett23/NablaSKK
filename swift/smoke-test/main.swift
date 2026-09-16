// Smoke test: drives the Rust engine from Swift through the C ABI.
// Run with: sh swift/run-smoke-test.sh

import Foundation

func assertEqual(_ actual: String, _ expected: String, _ label: String) {
    guard actual == expected else {
        print("FAIL \(label): expected \"\(expected)\", got \"\(actual)\"")
        exit(1)
    }
    print("ok   \(label): \(actual.isEmpty ? "(empty)" : actual)")
}

let userDict = NSTemporaryDirectory() + "swift-smoke-jisyo"
try? FileManager.default.removeItem(atPath: userDict)

let session = SKKSession(userDictionaryPath: userDict)

// In-memory dictionaries are not exposed over FFI; write a small file.
let dictPath = NSTemporaryDirectory() + "swift-smoke-dict"
let dictBody = """
;; okuri-ari entries.
;; okuri-nasi entries.
かんじ /漢字/幹事/
"""
try! dictBody.write(toFile: dictPath, atomically: true, encoding: .utf8)
session.addDictionary(.commonUTF8, location: dictPath)

// かな入力
for c in "konnnitiha".utf8 { session.handle(charcode: c) }
assertEqual(session.takeFixed(), "こんにちは", "kana input")

// 変換
for c in "Kanji".utf8 { session.handle(charcode: c) }
assertEqual(session.composing, "▽かんじ", "composing")
session.handle(charcode: 0x20)
assertEqual(session.composing, "▼漢字", "conversion")
session.handle(charcode: 0x0d)
assertEqual(session.takeFixed(), "漢字", "commit")

// モード切替
session.handle(charcode: UInt8(ascii: "l"))
precondition(session.inputMode == .ascii, "ascii mode")
session.handle(charcode: UInt8(ascii: "j"), mods: .ctrl)
precondition(session.inputMode == .hirakana, "back to hirakana")
print("ok   input mode switching")

session.save()
print("PASS all Swift FFI smoke tests")
