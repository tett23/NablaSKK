// NablaSKK: NSEvent -> engine key translation.
//
// New Swift implementation, written with reference to AquaSKK's
// Objective-C++ sources (https://github.com/codefirst/aquaskk):
//   platform/mac/src/server/SKKPreProcessor.mm
//     Copyright (C) 2007 Tomotaka SUWA <t.suwa@mac.com>
// Written by tett23, 2026.
//
// License: GPL-2.0-or-later. See the LICENSE file for details.

import Cocoa

enum KeyTranslator {
    struct Key: Equatable {
        var charcode: UInt8
        var keycode: UInt8
        var mods: SKKSession.Modifiers
    }

    /// Keys whose own character is a control code (Return, Tab, Escape,
    /// Enter, Delete); their characters must not be read as Ctrl+letter.
    private static let dedicatedControlKeys: Set<UInt16> = [36, 48, 51, 53, 76, 117]

    static func translate(_ event: NSEvent) -> Key {
        let keycode = UInt8(truncatingIfNeeded: event.keyCode)

        var charcode: UInt8 = 0
        if let scalar = event.charactersIgnoringModifiers?.unicodeScalars.first, scalar.value < 0x80 {
            charcode = UInt8(scalar.value)
        }

        var mods: SKKSession.Modifiers = []

        if event.modifierFlags.contains(.shift) {
            // Use the shifted character for printable keys (SKKPreProcessor)
            if let scalar = event.characters?.unicodeScalars.first,
               scalar.value > 0x20, scalar.value < 0x7f {
                charcode = UInt8(scalar.value)
            }
            mods.insert(.shift)
        }
        if event.modifierFlags.contains(.control) { mods.insert(.ctrl) }
        if event.modifierFlags.contains(.option) { mods.insert(.alt) }
        if event.modifierFlags.contains(.command) { mods.insert(.meta) }

        // Normalize Control combinations to "letter + ctrl", which is what
        // keymap.conf binds (ctrl::j). Clients do not agree on the event
        // they hand to the input method: some deliver the control character
        // itself (Ctrl-J as 0x0a) in charactersIgnoringModifiers, and some
        // drop the Control flag while `characters` still carries 0x0a. Left
        // alone, Ctrl-J reads as an unbound key: the engine passes it through
        // and a terminal receives a newline instead of switching modes.
        if !dedicatedControlKeys.contains(event.keyCode) {
            let control = [event.charactersIgnoringModifiers, event.characters]
                .compactMap { $0?.unicodeScalars.first?.value }
                .first { (0x01...0x1a).contains($0) }

            if let control {
                charcode = UInt8(control) | 0x60
                mods.insert(.ctrl)
            }
        }

        // The delete (⌫) key erases backwards; forward delete (⌦) forwards.
        switch event.keyCode {
        case 51: charcode = 0x08
        case 117: charcode = 0x7f
        default: break
        }

        return Key(charcode: charcode, keycode: keycode, mods: mods)
    }
}
