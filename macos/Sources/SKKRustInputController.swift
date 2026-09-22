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
    static var supportDirectory: URL { DictionaryConfig.supportDirectory }

    static let session: SKKSession = {
        let userDictionary = supportDirectory.appendingPathComponent("skk-jisyo").path
        let session = SKKSession(userDictionaryPath: userDictionary)

        loadDictionaries(into: session)
        applyInputSettings(to: session)

        return session
    }()

    private static var loadedSettingsDate: Date?

    /// Apply settings.conf (punctuation sub-rules) when it changed on disk.
    static func reloadInputSettingsIfChanged() {
        guard InputSettings.modificationDate != loadedSettingsDate else { return }
        applyInputSettings(to: session)
    }

    private static func applyInputSettings(to session: SKKSession) {
        loadedSettingsDate = InputSettings.modificationDate
        session.resetKanaRules()
        let patch = InputSettings.load().kanaRulePatch
        if !patch.isEmpty {
            session.patchKanaRules(patch)
        }
    }

    private static var loadedConfigDate: Date?

    /// Reload the dictionaries when dictionaries.conf changed on disk
    /// (the preferences app saves it; the change takes effect the next
    /// time the input method becomes active).
    static func reloadDictionariesIfChanged() {
        guard DictionaryConfig.modificationDate != loadedConfigDate else { return }

        session.clearDictionaries()
        loadDictionaries(into: session)
    }

    private static func loadDictionaries(into session: SKKSession) {
        loadedConfigDate = DictionaryConfig.modificationDate

        for entry in DictionaryConfig.load() where entry.enabled {
            guard let type = SKKSession.DictionaryType(rawValue: entry.kind.rawValue) else { continue }
            session.addDictionary(type, location: entry.location)
        }

        loadedConfigDate = DictionaryConfig.modificationDate
    }

    /// The preferences app bundled inside the input method.
    static func openPreferences() {
        guard let url = Bundle.main.resourceURL?.appendingPathComponent("NablaSKK Preferences.app") else { return }

        NSWorkspace.shared.openApplication(at: url, configuration: NSWorkspace.OpenConfiguration())
    }
}

/// Opt-in diagnostics: create the file
/// `~/Library/Application Support/NablaSKK/debug-enabled` and key handling
/// metadata is appended to `debug.log` next to it. Plain printable keys are
/// never recorded.
enum DebugLog {
    private static let flag = Engine.supportDirectory.appendingPathComponent("debug-enabled")
    private static let file = Engine.supportDirectory.appendingPathComponent("debug.log")

    static func write(_ message: () -> String) {
        guard FileManager.default.fileExists(atPath: flag.path) else { return }

        let line = "\(Date()) \(message())\n"
        if let handle = try? FileHandle(forWritingTo: file) {
            handle.seekToEndOfFile()
            handle.write(Data(line.utf8))
            try? handle.close()
        } else {
            try? Data(line.utf8).write(to: file)
        }
    }
}

@objc(SKKRustInputController)
public class SKKRustInputController: IMKInputController {
    private static weak var activeController: SKKRustInputController?

    public override func activateServer(_ sender: Any!) {
        Engine.reloadDictionariesIfChanged()
        Engine.reloadInputSettingsIfChanged()
        Engine.session.reloadUserDictionaryIfChanged()

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

    // Items appended to the input source menu in the menu bar
    public override func menu() -> NSMenu! {
        let menu = NSMenu()
        let item = NSMenuItem(title: "辞書を管理...", action: #selector(openPreferences(_:)), keyEquivalent: "")
        item.target = self
        menu.addItem(item)
        return menu
    }

    @objc private func openPreferences(_ sender: Any?) {
        Engine.openPreferences()
    }

    public override func handle(_ event: NSEvent!, client sender: Any!) -> Bool {
        guard let event, event.type == .keyDown, let client = sender as? IMKTextInput else {
            return false
        }

        let key = KeyTranslator.translate(event)
        let (charcode, keycode, mods) = (key.charcode, key.keycode, key.mods)

        // Command shortcuts belong to the application, except Cmd-V while
        // composing: it pastes into the reading / registration word.
        let isPaste = (mods == [.meta] && charcode == UInt8(ascii: "v"))
            || (mods == [.ctrl] && charcode == UInt8(ascii: "y"))
        if mods.contains(.meta) && !(isPaste && !Engine.session.composing.isEmpty) {
            return false
        }

        if isPaste {
            // Single line only: a newline inside a dictionary word is invalid
            let text = NSPasteboard.general.string(forType: .string) ?? ""
            Engine.session.setClipboard(
                text.components(separatedBy: .newlines).joined())
        }

        let wasComposing = !Engine.session.composing.isEmpty
        let handled = Engine.session.handle(charcode: charcode, keycode: keycode, mods: mods)

        let produced = sync(to: client)

        DebugLog.write {
            // Metadata only: plain printable keys are not recorded
            let printable = mods.isEmpty && (0x20...0x7e).contains(charcode)
            let chars = printable ? "printable" : String(format: "0x%02x", charcode)
            let raw = (event.characters ?? "").unicodeScalars.map { String(format: "%02x", $0.value) }.joined(separator: ",")
            let rawIgnoring = (event.charactersIgnoringModifiers ?? "").unicodeScalars
                .map { String(format: "%02x", $0.value) }.joined(separator: ",")
            return "client=\(client.bundleIdentifier() ?? "?") key=\(chars) keycode=\(keycode) "
                + "mods=\(mods.rawValue) flags=\(String(event.modifierFlags.rawValue, radix: 16)) "
                + (printable ? "" : "characters=[\(raw)] ignoringModifiers=[\(rawIgnoring)] ")
                + "handled=\(handled) produced=\(produced) wasComposing=\(wasComposing) "
                + "mode=\(Engine.session.inputMode)"
        }

        // A key we consumed without touching any text (`l` switching to ASCII,
        // Ctrl-J switching back) is still acted on by some clients.
        let leak = ClientQuirks.keyLeak(for: client.bundleIdentifier())
        if ClientQuirks.shouldMask(leak, handled: handled, produced: produced,
                                   wasComposing: wasComposing, hasControl: mods.contains(.ctrl)) {
            maskKeyEvent(for: client)
        }

        return handled
    }

    /// Clients with a key leak (see `ClientQuirks`) only leave a consumed key
    /// alone when marked text exists before or after the key handler.
    /// AquaSKK's trick of marking 0x0c and clearing it inside the handler
    /// leaves no marked text at the end and does not help. Instead hold a
    /// zero-width space as marked text until the handler has returned, then
    /// cancel the composition.
    private func maskKeyEvent(for client: IMKTextInput) {
        let none = NSRange(location: NSNotFound, length: NSNotFound)
        client.setMarkedText("\u{200B}", selectionRange: NSRange(location: 0, length: 0), replacementRange: none)

        DispatchQueue.main.async {
            // A key typed in between already replaced the marked text
            guard Engine.session.composing.isEmpty else { return }
            client.setMarkedText("", selectionRange: NSRange(location: 0, length: 0), replacementRange: none)
        }
    }

    // ------------------------------------------------------------
    // Engine state -> client
    // ------------------------------------------------------------

    /// Push engine output to the client. Returns true when it committed
    /// text or left marked text behind.
    @discardableResult
    private func sync(to client: IMKTextInput) -> Bool {
        let fixed = Engine.session.takeFixed()
        if !fixed.isEmpty {
            insert(fixed, to: client)
        }

        var marked = Engine.session.composing

        // Caret position inside the marked text, in UTF-16 units. The engine
        // reports a character offset from the end of the composing string.
        let characters = Array(marked)
        let caretIndex = max(0, min(characters.count, characters.count + Engine.session.composingCursor))
        let caret = String(characters[..<caretIndex]).utf16.count

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

        setMarkedText(marked, caret: caret, to: client)

        return !fixed.isEmpty || !marked.isEmpty
    }

    private func insert(_ text: String, to client: IMKTextInput) {
        client.insertText(text, replacementRange: NSRange(location: NSNotFound, length: NSNotFound))
    }

    private func setMarkedText(_ text: String, caret: Int? = nil, to client: IMKTextInput) {
        let attributes: [NSAttributedString.Key: Any] = [
            .underlineStyle: NSUnderlineStyle.single.rawValue,
        ]
        let attributed = NSAttributedString(string: text, attributes: attributes)

        client.setMarkedText(
            attributed,
            selectionRange: NSRange(location: caret ?? text.utf16.count, length: 0),
            replacementRange: NSRange(location: NSNotFound, length: NSNotFound)
        )
    }
}
