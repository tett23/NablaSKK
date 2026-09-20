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

if failures > 0 {
    print("\(failures) FAILED")
    exit(1)
}
print("PASS all key translation tests")
