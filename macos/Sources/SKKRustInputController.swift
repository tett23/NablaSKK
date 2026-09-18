// NablaSKK: InputMethodKit controller bridging to the Rust engine.
//
// New Swift implementation, written with reference to AquaSKK's
// Objective-C++ sources (https://github.com/codefirst/aquaskk):
//   platform/mac/src/server/SKKPreProcessor.mm
//     Copyright (C) 2007 Tomotaka SUWA <t.suwa@mac.com>
//   platform/mac/src/server/SKKInputController.mm
//     Copyright (C) 2007-2013 Tomotaka SUWA <tomotaka.suwa@gmail.com>
// Written by tett23, 2026.
//
// License: GPL-2.0-or-later. See the LICENSE file for details.

import Cocoa
import InputMethodKit

/// One engine session shared by all clients (candidate dictionaries are
/// large; per-client sessions would duplicate them). Pending composition
/// is cleared when a different client activates.
enum Engine {
    static let supportDirectory: URL = {
        let url = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
            .appendingPathComponent("NablaSKK", isDirectory: true)
        try? FileManager.default.createDirectory(at: url, withIntermediateDirectories: true)
        return url
    }()

    static let session: SKKSession = {
        let userDictionary = supportDirectory.appendingPathComponent("skk-jisyo").path
        let session = SKKSession(userDictionaryPath: userDictionary)

        loadDictionaries(into: session)

        return session
    }()

    /// dictionaries.conf: one "type location" per line
    /// (0=SKK-JISYO with encoding auto-detection, 1=auto-update
    ///  "host url path", 2=skkserv host:port, 4=gadget, 5=UTF-8 forced).
    /// Created with defaults on first launch.
    private static func loadDictionaries(into session: SKKSession) {
        let config = supportDirectory.appendingPathComponent("dictionaries.conf")

        if !FileManager.default.fileExists(atPath: config.path) {
            var template = """
            # NablaSKK dictionaries: "type location" per line.
            #   0 = SKK-JISYO (encoding auto-detected)
            #   1 = auto-update "host url path"
            #   2 = skkserv host:port    4 = gadget (today/now/=expr)
            #   5 = SKK-JISYO (UTF-8 forced)
            # Add your main dictionary, e.g.:
            #   0 /usr/local/share/skk/SKK-JISYO.L
            4

            """

            // Reuse an existing AquaSKK dictionary when present
            let aquaskkDictionary = FileManager.default
                .urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
                .appendingPathComponent("AquaSKK/SKK-JISYO.L")
            if FileManager.default.fileExists(atPath: aquaskkDictionary.path) {
                template += "0 \(aquaskkDictionary.path)\n"
            }

            try? template.write(to: config, atomically: true, encoding: .utf8)
        }

        guard let text = try? String(contentsOf: config, encoding: .utf8) else { return }

        for line in text.split(separator: "\n") {
            let trimmed = line.trimmingCharacters(in: .whitespaces)
            if trimmed.isEmpty || trimmed.hasPrefix("#") { continue }

            let fields = trimmed.split(separator: " ", maxSplits: 1)
            guard let typeValue = Int32(fields[0]),
                  let type = SKKSession.DictionaryType(rawValue: typeValue)
            else { continue }

            let location = fields.count > 1 ? String(fields[1]) : ""
            session.addDictionary(type, location: location)
        }
    }
}

@objc(SKKRustInputController)
public class SKKRustInputController: IMKInputController {
    private static weak var activeController: SKKRustInputController?

    public override func activateServer(_ sender: Any!) {
        if Self.activeController !== self {
            // Another client had pending composition; drop it
            Engine.session.clear()
            _ = Engine.session.takeFixed()
            Self.activeController = self
        }
    }

    public override func deactivateServer(_ sender: Any!) {
        guard let client = client() else { return }

        // Commit pending text so nothing is lost on focus change
        Engine.session.commit()
        let fixed = Engine.session.takeFixed()
        if !fixed.isEmpty {
            insert(fixed, to: client)
        }
        setMarkedText("", to: client)
        Engine.session.save()
    }

    public override func commitComposition(_ sender: Any!) {
        guard let client = client() else { return }

        Engine.session.commit()
        let fixed = Engine.session.takeFixed()
        if !fixed.isEmpty {
            insert(fixed, to: client)
        }
        setMarkedText("", to: client)
    }

    public override func handle(_ event: NSEvent!, client sender: Any!) -> Bool {
        guard let event, event.type == .keyDown, let client = sender as? IMKTextInput else {
            return false
        }

        // Command shortcuts always belong to the application
        if event.modifierFlags.contains(.command) {
            return false
        }

        let (charcode, keycode, mods) = translate(event)
        let handled = Engine.session.handle(charcode: charcode, keycode: keycode, mods: mods)

        sync(to: client)

        return handled
    }

    // ------------------------------------------------------------
    // NSEvent -> engine event (port of SKKPreProcessor)
    // ------------------------------------------------------------

    private func translate(_ event: NSEvent) -> (UInt8, UInt8, SKKSession.Modifiers) {
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

        // The delete (⌫) key erases backwards; forward delete (⌦) forwards.
        switch keycode {
        case 51: charcode = 0x08
        case 117: charcode = 0x7f
        default: break
        }

        return (charcode, keycode, mods)
    }

    // ------------------------------------------------------------
    // Engine state -> client
    // ------------------------------------------------------------

    private func sync(to client: IMKTextInput) {
        let fixed = Engine.session.takeFixed()
        if !fixed.isEmpty {
            insert(fixed, to: client)
        }

        var marked = Engine.session.composing

        // Candidate window contents are rendered into the marked text
        // (labels select directly, matching the engine's key handling)
        if Engine.session.candidatesVisible {
            let candidates = Engine.session.candidates
            let cursor = Engine.session.candidateCursor
            let (page, pageCount) = Engine.session.candidatePage
            let labels = "asdfjkl"

            var line = " 【"
            for (index, candidate) in candidates.enumerated() {
                let label = labels[labels.index(labels.startIndex, offsetBy: min(index, labels.count - 1))]
                line += index == cursor ? "▶\(label):\(candidate) " : " \(label):\(candidate) "
            }
            line += "】(\(page)/\(pageCount))"
            marked += line
        }

        setMarkedText(marked, to: client)
    }

    private func insert(_ text: String, to client: IMKTextInput) {
        client.insertText(text, replacementRange: NSRange(location: NSNotFound, length: NSNotFound))
    }

    private func setMarkedText(_ text: String, to client: IMKTextInput) {
        let attributes: [NSAttributedString.Key: Any] = [
            .underlineStyle: NSUnderlineStyle.single.rawValue,
        ]
        let attributed = NSAttributedString(string: text, attributes: attributes)

        client.setMarkedText(
            attributed,
            selectionRange: NSRange(location: text.count, length: 0),
            replacementRange: NSRange(location: NSNotFound, length: NSNotFound)
        )
    }
}
