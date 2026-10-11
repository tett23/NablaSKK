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

// 単語登録中のカーソル移動 (Ctrl-B で確定済み文字列上を左へ)
for c in "Mikan ".utf8 { session.handle(charcode: c) }
for c in "aiu".utf8 { session.handle(charcode: c) }
assertEqual(session.composing, "[登録：みかん]あいう", "registration word")
session.handle(charcode: UInt8(ascii: "b"), mods: .ctrl)
precondition(session.composingCursor == -1, "cursor moved left")
for c in "e".utf8 { session.handle(charcode: c) }
assertEqual(session.composing, "[登録：みかん]あいえう", "insert at cursor")
session.clear()
_ = session.takeFixed()

// 単語登録中のペースト (Cmd-V)
for c in "Mikan ".utf8 { session.handle(charcode: c) }
session.setClipboard("蜜柑")
let pasteHandled = session.handle(charcode: UInt8(ascii: "v"), mods: .meta)
precondition(pasteHandled, "paste consumed while registering")
assertEqual(session.composing, "[登録：みかん]蜜柑", "paste into registration")
session.handle(charcode: 0x0d)
assertEqual(session.takeFixed(), "蜜柑", "registered pasted word")
for c in "Mikan ".utf8 { session.handle(charcode: c) }
assertEqual(session.composing, "▼蜜柑", "pasted word was learned")
session.clear()
_ = session.takeFixed()

// モード切替
session.handle(charcode: UInt8(ascii: "l"))
precondition(session.inputMode == .ascii, "ascii mode")
session.handle(charcode: UInt8(ascii: "j"), mods: .ctrl)
precondition(session.inputMode == .hirakana, "back to hirakana")
print("ok   input mode switching")

session.save()
// The header's constants and the Swift wrapper's enums must agree
// (ADR 0002: SKK_DICT_PROXY_UTF8 was missing from the header)
func assertSame(_ header: some BinaryInteger, _ swift: Int32, _ label: String) {
    guard Int64(header) == Int64(swift) else {
        print("FAIL \(label): header \(header), Swift \(swift)")
        exit(1)
    }
}
let dictionaryTypes: [(Int64, SKKSession.DictionaryType, String)] = [
    (Int64(SKK_DICT_COMMON), .common, "SKK_DICT_COMMON"),
    (Int64(SKK_DICT_AUTO_UPDATE), .autoUpdate, "SKK_DICT_AUTO_UPDATE"),
    (Int64(SKK_DICT_PROXY), .proxy, "SKK_DICT_PROXY"),
    (Int64(SKK_DICT_GADGET), .gadget, "SKK_DICT_GADGET"),
    (Int64(SKK_DICT_COMMON_UTF8), .commonUTF8, "SKK_DICT_COMMON_UTF8"),
    (Int64(SKK_DICT_PROXY_UTF8), .proxyUTF8, "SKK_DICT_PROXY_UTF8"),
]
for (header, type, label) in dictionaryTypes { assertSame(header, type.rawValue, label) }
let inputModes: [(Int64, SKKSession.InputMode, String)] = [
    (Int64(SKK_MODE_HIRAKANA), .hirakana, "SKK_MODE_HIRAKANA"),
    (Int64(SKK_MODE_KATAKANA), .katakana, "SKK_MODE_KATAKANA"),
    (Int64(SKK_MODE_JISX0201KANA), .jisx0201kana, "SKK_MODE_JISX0201KANA"),
    (Int64(SKK_MODE_ASCII), .ascii, "SKK_MODE_ASCII"),
    (Int64(SKK_MODE_JISX0208LATIN), .jisx0208latin, "SKK_MODE_JISX0208LATIN"),
]
for (header, mode, label) in inputModes { assertSame(header, mode.rawValue, label) }
let options: [(Int64, SKKSession.Option, String)] = [
    (Int64(SKK_OPTION_SUPPRESS_NEWLINE_ON_COMMIT), .suppressNewlineOnCommit, "SKK_OPTION_SUPPRESS_NEWLINE_ON_COMMIT"),
    (Int64(SKK_OPTION_INLINE_BACKSPACE_IMPLIES_COMMIT), .inlineBackspaceImpliesCommit, "SKK_OPTION_INLINE_BACKSPACE_IMPLIES_COMMIT"),
    (Int64(SKK_OPTION_DELETE_OKURI_WHEN_QUIT), .deleteOkuriWhenQuit, "SKK_OPTION_DELETE_OKURI_WHEN_QUIT"),
    (Int64(SKK_OPTION_HANDLE_RECURSIVE_ENTRY_AS_OKURI), .handleRecursiveEntryAsOkuri, "SKK_OPTION_HANDLE_RECURSIVE_ENTRY_AS_OKURI"),
    (Int64(SKK_OPTION_FIX_INTERMEDIATE_CONVERSION), .fixIntermediateConversion, "SKK_OPTION_FIX_INTERMEDIATE_CONVERSION"),
    (Int64(SKK_OPTION_DISPLAY_SHORTEST_MATCH), .displayShortestMatch, "SKK_OPTION_DISPLAY_SHORTEST_MATCH"),
    (Int64(SKK_OPTION_USE_NUMERIC_CONVERSION), .useNumericConversion, "SKK_OPTION_USE_NUMERIC_CONVERSION"),
    (Int64(SKK_OPTION_MAX_INLINE_CANDIDATES), .maxInlineCandidates, "SKK_OPTION_MAX_INLINE_CANDIDATES"),
    (Int64(SKK_OPTION_ENABLE_DYNAMIC_COMPLETION), .enableDynamicCompletion, "SKK_OPTION_ENABLE_DYNAMIC_COMPLETION"),
    (Int64(SKK_OPTION_DYNAMIC_COMPLETION_RANGE), .dynamicCompletionRange, "SKK_OPTION_DYNAMIC_COMPLETION_RANGE"),
    (Int64(SKK_OPTION_ENABLE_EXTENDED_COMPLETION), .enableExtendedCompletion, "SKK_OPTION_ENABLE_EXTENDED_COMPLETION"),
    (Int64(SKK_OPTION_ENABLE_ANNOTATION), .enableAnnotation, "SKK_OPTION_ENABLE_ANNOTATION"),
]
for (header, option, label) in options { assertSame(header, option.rawValue, label) }
print("ok   header constants match the Swift wrapper (\(dictionaryTypes.count + inputModes.count + options.count) values)")

// An skkserv speaking UTF-8 is added even when nothing listens there
let proxySession = SKKSession(userDictionaryPath: NSTemporaryDirectory() + "swift-smoke-proxy-jisyo")
guard proxySession.addDictionary(.proxyUTF8, location: "127.0.0.1:1") else {
    print("FAIL addDictionary(.proxyUTF8) was rejected")
    exit(1)
}
print("ok   addDictionary(.proxyUTF8) accepted")

print("PASS all Swift FFI smoke tests")
