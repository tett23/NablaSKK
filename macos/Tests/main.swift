// Key translation tests: builds real NSEvents the way different clients
// deliver them and checks the engine sees the binding keymap.conf expects.
// Run with: sh macos/run-tests.sh

import Cocoa

var failures = 0

func check(_ condition: Bool, _ label: String) {
    if condition {
        print("ok   \(label)")
    } else {
        print("FAIL \(label)")
        failures += 1
    }
}

func keyEvent(characters: String, ignoringModifiers: String, keyCode: UInt16,
              flags: NSEvent.ModifierFlags = []) -> NSEvent {
    NSEvent.keyEvent(with: .keyDown, location: .zero, modifierFlags: flags, timestamp: 0,
                     windowNumber: 0, context: nil, characters: characters,
                     charactersIgnoringModifiers: ignoringModifiers, isARepeat: false,
                     keyCode: keyCode)!
}

let j = UInt8(ascii: "j")

// Ctrl-J as AppKit normally reports it
var key = KeyTranslator.translate(
    keyEvent(characters: "\n", ignoringModifiers: "j", keyCode: 38, flags: .control))
check(key.charcode == j && key.mods == [.ctrl], "ctrl-j: letter + control flag")

// Ctrl-J delivered with the control character in both fields
key = KeyTranslator.translate(
    keyEvent(characters: "\n", ignoringModifiers: "\n", keyCode: 38, flags: .control))
check(key.charcode == j && key.mods == [.ctrl], "ctrl-j: control character in both fields")

// Ctrl-J delivered without the Control flag
key = KeyTranslator.translate(
    keyEvent(characters: "\n", ignoringModifiers: "j", keyCode: 38))
check(key.charcode == j && key.mods == [.ctrl], "ctrl-j: control flag missing")

// Dedicated keys keep their own codes
key = KeyTranslator.translate(keyEvent(characters: "\r", ignoringModifiers: "\r", keyCode: 36))
check(key.charcode == 0x0d && key.mods.isEmpty, "return stays return")
key = KeyTranslator.translate(keyEvent(characters: "\t", ignoringModifiers: "\t", keyCode: 48))
check(key.charcode == 0x09 && key.mods.isEmpty, "tab stays tab")
key = KeyTranslator.translate(keyEvent(characters: "\u{7f}", ignoringModifiers: "\u{7f}", keyCode: 51))
check(key.charcode == 0x08, "delete key is backspace")

// Shifted printable keys use the shifted character
key = KeyTranslator.translate(
    keyEvent(characters: "K", ignoringModifiers: "K", keyCode: 40, flags: .shift))
check(key.charcode == UInt8(ascii: "K") && key.mods == [.shift], "shift-k")

// End to end: every Ctrl-J shape switches ASCII -> hiragana and is consumed
for (label, event) in [
    ("normal", keyEvent(characters: "\n", ignoringModifiers: "j", keyCode: 38, flags: .control)),
    ("control char", keyEvent(characters: "\n", ignoringModifiers: "\n", keyCode: 38, flags: .control)),
    ("no flag", keyEvent(characters: "\n", ignoringModifiers: "j", keyCode: 38)),
] {
    let session = SKKSession(userDictionaryPath: NSTemporaryDirectory() + "nablaskk-key-test")
    session.handle(charcode: UInt8(ascii: "l"), keycode: 37)
    check(session.inputMode == .ascii, "\(label): in ascii mode")

    let translated = KeyTranslator.translate(event)
    let handled = session.handle(
        charcode: translated.charcode, keycode: translated.keycode, mods: translated.mods)
    check(handled && session.inputMode == .hirakana, "\(label): ctrl-j consumed, mode is hiragana")
}

// Client quirks: which consumed keys need masking
check(ClientQuirks.keyLeak(for: "com.mitchellh.ghostty") == .allKeys, "ghostty re-encodes every key")
check(ClientQuirks.keyLeak(for: "com.apple.TextEdit") == .none, "TextEdit honors the IME result")
check(ClientQuirks.keyLeak(for: nil) == .none, "unknown client")

// Ghostty: `l` (hiragana -> ASCII) produces nothing and must be masked
check(ClientQuirks.shouldMask(.allKeys, handled: true, produced: false, wasComposing: false, hasControl: false),
      "ghostty: silent plain key is masked")
check(!ClientQuirks.shouldMask(.allKeys, handled: true, produced: true, wasComposing: false, hasControl: false),
      "ghostty: keys that produce text are not masked")
check(!ClientQuirks.shouldMask(.allKeys, handled: true, produced: false, wasComposing: true, hasControl: false),
      "ghostty: marked text before the key already hides it")
check(!ClientQuirks.shouldMask(.allKeys, handled: false, produced: false, wasComposing: false, hasControl: false),
      "ghostty: pass-through keys are not masked")
// Chromium: only Control combinations leak
check(ClientQuirks.shouldMask(.controlKeys, handled: true, produced: false, wasComposing: false, hasControl: true),
      "chromium: silent ctrl key is masked")
check(!ClientQuirks.shouldMask(.controlKeys, handled: true, produced: false, wasComposing: false, hasControl: false),
      "chromium: silent plain key is left alone")
check(!ClientQuirks.shouldMask(.none, handled: true, produced: false, wasComposing: false, hasControl: true),
      "native clients are never masked")

// The engine side of the Ghostty case: `l` is consumed and produces nothing
do {
    let session = SKKSession(userDictionaryPath: NSTemporaryDirectory() + "nablaskk-key-test")
    let handled = session.handle(charcode: UInt8(ascii: "l"), keycode: 37)
    check(handled && session.inputMode == .ascii && session.composing.isEmpty && session.takeFixed().isEmpty,
          "l: consumed, ascii mode, no text and no marked text")
}

// dictionaries.conf: parse / serialize round trip, disabled entries, legacy lines
do {
    let text = """
    # comment
    0 /usr/share/skk/SKK-JISYO.L
    - 5 /Users/me/My Dictionary.utf8
    4
    2 localhost:1178
    garbage line
    """
    let entries = DictionaryConfig.parse(text)
    check(entries.count == 4, "config: parses four entries, skipping comments and garbage")
    check(entries[0].enabled && entries[0].kind == .common && entries[0].location == "/usr/share/skk/SKK-JISYO.L",
          "config: enabled common dictionary")
    check(!entries[1].enabled && entries[1].kind == .commonUTF8 && entries[1].location == "/Users/me/My Dictionary.utf8",
          "config: disabled entry keeps its type and a location with spaces")
    check(entries[2].kind == .gadget && entries[2].location.isEmpty, "config: gadget without location")
    check(entries[3].kind == .proxy && entries[3].location == "localhost:1178", "config: skkserv")

    let reparsed = DictionaryConfig.parse(DictionaryConfig.serialize(entries))
    check(reparsed.map { ($0.enabled, $0.kind, $0.location) }.elementsEqual(
            entries.map { ($0.enabled, $0.kind, $0.location) }, by: { $0 == $1 }),
          "config: serialize/parse round trip")
    check(DictionaryConfig.serialize(entries).contains("\n- 5 /Users/me/My Dictionary.utf8\n"),
          "config: disabled entries are written with a leading dash")
}

// Importing dictionaries: copy into the support directory, reuse identical
// copies, keep distinct files with the same name apart
do {
    let scratch = URL(fileURLWithPath: NSTemporaryDirectory()).appendingPathComponent("nablaskk-import-test")
    try? FileManager.default.removeItem(at: scratch)
    try! FileManager.default.createDirectory(at: scratch, withIntermediateDirectories: true)
    let a = scratch.appendingPathComponent("SKK-JISYO.test")
    let b = scratch.appendingPathComponent("other/SKK-JISYO.test")
    try! FileManager.default.createDirectory(at: b.deletingLastPathComponent(), withIntermediateDirectories: true)
    try! "a".write(to: a, atomically: true, encoding: .utf8)
    try! "b".write(to: b, atomically: true, encoding: .utf8)

    let imported = try! DictionaryConfig.importDictionary(at: a)
    check(imported.hasPrefix(DictionaryConfig.dictionariesDirectory.path)
          && FileManager.default.contentsEqual(atPath: a.path, andPath: imported),
          "import: copied into the dictionaries directory")
    check(try! DictionaryConfig.importDictionary(at: a) == imported, "import: identical file reuses the copy")
    let second = try! DictionaryConfig.importDictionary(at: b)
    check(second != imported && second.hasSuffix("SKK-JISYO-1.test"), "import: different file with the same name gets a suffix")
    check(try! DictionaryConfig.importDictionary(at: URL(fileURLWithPath: imported)) == imported,
          "import: a file already in the support directory is used in place")

    try? FileManager.default.removeItem(atPath: imported)
    try? FileManager.default.removeItem(atPath: second)
    try? FileManager.default.removeItem(at: scratch)
}

if failures > 0 {
    print("\(failures) FAILED")
    exit(1)
}
print("PASS all key translation tests")
