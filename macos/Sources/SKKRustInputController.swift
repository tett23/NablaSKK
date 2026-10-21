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
        applyKeymap(to: session)

        return session
    }()

    private static var loadedKeymapDate: Date?

    /// Apply keymap.conf overrides when the file changed on disk.
    static func reloadKeymapIfChanged() {
        guard KeymapSettings.modificationDate != loadedKeymapDate else { return }
        applyKeymap(to: session)
    }

    /// AquaSKK's sub-rule files (ADR 0003), bundled in Resources.
    static var subRuleDirectory: URL { Bundle.main.resourceURL ?? Bundle.main.bundleURL }

    /// The stacking order lives in EngineSetup (tested, ADR 0008).
    private static func applyKeymap(to session: SKKSession) {
        loadedKeymapDate = KeymapSettings.modificationDate
        EngineSetup.applyKeymap(to: session, settings: InputSettings.load(), keymap: KeymapSettings.load(),
                                subRuleDirectory: subRuleDirectory)
    }

    private static var loadedSettingsDate: Date?

    /// Apply settings.conf (punctuation sub-rules, skkserv) when it
    /// changed on disk. The skkserv entry lives in the dictionary list,
    /// so the dictionaries are rebuilt too.
    static func reloadInputSettingsIfChanged() {
        if KanaRuleSettings.modificationDate != loadedKanaRulesDate && InputSettings.modificationDate == loadedSettingsDate {
            applyKanaRules(to: session, settings: InputSettings.load())
        }
        guard InputSettings.modificationDate != loadedSettingsDate else { return }
        applyInputSettings(to: session)
        // The input style also patches the keymap
        applyKeymap(to: session)
        session.clearDictionaries()
        loadDictionaries(into: session)
    }

    private static var loadedKanaRulesDate: Date?

    /// The stacking order lives in EngineSetup (tested, ADR 0008).
    private static func applyKanaRules(to session: SKKSession, settings: InputSettings) {
        loadedKanaRulesDate = KanaRuleSettings.modificationDate
        EngineSetup.applyKanaRules(to: session, settings: settings, userRules: KanaRuleSettings.load(),
                                   subRuleDirectory: subRuleDirectory)
    }

    private static func applyInputSettings(to session: SKKSession) {
        loadedSettingsDate = InputSettings.modificationDate
        let settings = InputSettings.load()

        applyKanaRules(to: session, settings: settings)
        session.setOption(.enableDynamicCompletion, settings.suggestEnabled ? 1 : 0)
        session.setOption(.dynamicCompletionRange, Int32(settings.suggestCount))
        session.setOption(.enableExtendedCompletion, settings.completionExtended ? 1 : 0)
        session.setOption(.enableAnnotation, settings.annotationEnabled ? 1 : 0)
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

    /// dictionaries.conf in order, then the skkserv from settings.conf
    /// (if enabled) so local dictionaries are always consulted first.
    private static func loadDictionaries(into session: SKKSession) {
        loadedConfigDate = DictionaryConfig.modificationDate

        for entry in DictionaryConfig.load() where entry.enabled {
            guard let type = SKKSession.DictionaryType(rawValue: entry.kind.rawValue) else { continue }
            session.addDictionary(type, location: entry.location)
        }

        let settings = InputSettings.load()
        if settings.skkservIsConfigured,
           let type = SKKSession.DictionaryType(rawValue: settings.skkservEncoding.dictionaryKind.rawValue) {
            session.addDictionary(type, location: settings.skkservLocation)
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

    /// Per-application ASCII restore; see ClientModeMemory (tested, ADR 0008).
    private static var clientModes = ClientModeMemory()

    private var clientKey: String {
        client()?.bundleIdentifier() ?? "controller-\(ObjectIdentifier(self).hashValue)"
    }

    private func recordInputMode() {
        Self.clientModes.recordKey(client: clientKey, mode: Engine.session.inputMode)
    }

    public override func activateServer(_ sender: Any!) {
        Engine.reloadDictionariesIfChanged()
        Engine.reloadInputSettingsIfChanged()
        Engine.reloadKeymapIfChanged()
        Engine.session.reloadUserDictionaryIfChanged()

        let switched = Self.activeController !== self
        if switched {
            // Another client had pending composition; drop it
            Engine.session.clear()
            _ = Engine.session.takeFixed()
            Self.activeController = self

            if let mode = Self.clientModes.modeToRestore(client: clientKey) {
                Engine.session.inputMode = mode
            }
        }

        DebugLog.write {
            "activate client=\(clientKey) switched=\(switched) restore=\(Self.clientModes.modeToRestore(client: clientKey).map { "\($0)" } ?? "none") "
                + "mode=\(Engine.session.inputMode)"
        }
    }

    public override func deactivateServer(_ sender: Any!) {
        guard let client = client() else { return }

        // Commit pending text so nothing is lost on focus change
        flushPendingCommit(to: client)
        Engine.session.commit()
        let fixed = Engine.session.takeFixed()
        if !fixed.isEmpty {
            insert(fixed, to: client)
        }
        setMarkedText("", to: client)
        CompletionWindow.shared.hide()
        AnnotationWindow.shared.hide()
        Engine.session.save()

        DebugLog.write {
            "deactivate client=\(clientKey) active=\(Self.activeController === self) mode=\(Engine.session.inputMode)"
        }
    }

    public override func commitComposition(_ sender: Any!) {
        guard let client = client() else { return }

        flushPendingCommit(to: client)
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
        let item = NSMenuItem(title: "NablaSKK の設定...", action: #selector(openPreferences(_:)), keyEquivalent: "")
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

        // A commit held back by the previous key comes first (ADR 0010)
        let hadPending = flushPendingCommit(to: client)

        let key = KeyTranslator.translate(event)
        let (charcode, keycode, mods) = (key.charcode, key.keycode, key.mods)

        // Command shortcuts belong to the application, except Cmd-V while
        // composing (KeyRouting, tested, ADR 0008)
        let isPaste = KeyRouting.isPaste(charcode: charcode, mods: mods)
        if KeyRouting.passesToApplication(charcode: charcode, mods: mods, composing: !Engine.session.composing.isEmpty) {
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
        recordInputMode()

        // Some clients act on a key the input method handled unless marked
        // text exists before or after the handler (ClientQuirks).
        let fixed = Engine.session.takeFixed()
        let masking = ClientQuirks.masking(
            ClientQuirks.keyLeak(for: client.bundleIdentifier()), handled: handled,
            committed: !fixed.isEmpty, markedBefore: wasComposing || hadPending,
            markedAfter: !Engine.session.composing.isEmpty, hasControl: mods.contains(.ctrl))

        let produced = sync(to: client, fixed: masking == .markThenCommit ? "" : fixed) || !fixed.isEmpty

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
                + "mode=\(Engine.session.inputMode) masking=\(masking)"
        }

        switch masking {
        case .none: break
        case .zeroWidthSpace: maskKeyEvent(for: client, replacing: false)
        case .zeroWidthSpaceReplaced: maskKeyEvent(for: client, replacing: true)
        case .markThenCommit: holdCommit(fixed, for: client)
        }

        return handled
    }

    /// Text shown as marked text by `holdCommit`, committed once the key
    /// handler has returned or before the next key, whichever comes first.
    private var pendingCommit: String?

    /// Show committed text as marked text and commit it after the handler
    /// has returned, so the client sees marked text after the key (ADR 0010).
    private func holdCommit(_ text: String, for client: IMKTextInput) {
        setMarkedText(text, to: client)
        pendingCommit = text
        DispatchQueue.main.async { [weak self] in
            self?.flushPendingCommit(to: client)
        }
    }

    @discardableResult
    private func flushPendingCommit(to client: IMKTextInput) -> Bool {
        guard let text = pendingCommit else { return false }
        pendingCommit = nil
        insert(text, to: client)
        return true
    }

    /// Clients with a key leak (see `ClientQuirks`) only leave a consumed key
    /// alone when marked text exists before or after the key handler.
    /// AquaSKK's trick of marking 0x0c and clearing it inside the handler
    /// leaves no marked text at the end and does not help. Instead hold a
    /// zero-width space as marked text until the handler has returned, then
    /// cancel the composition: clear the marked text, or (`replacing`)
    /// replace it with empty text so editors end the composition with
    /// nothing left behind (ADR 0010).
    private func maskKeyEvent(for client: IMKTextInput, replacing: Bool) {
        let none = NSRange(location: NSNotFound, length: NSNotFound)
        let caret = NSRange(
            location: ClientQuirks.maskCaret(replacing ? .zeroWidthSpaceReplaced : .zeroWidthSpace), length: 0)
        client.setMarkedText("\u{200B}", selectionRange: caret, replacementRange: none)

        DispatchQueue.main.async { [weak self] in
            // A key typed in between already replaced the marked text
            guard Engine.session.composing.isEmpty, self?.pendingCommit == nil else { return }
            if replacing {
                client.insertText("", replacementRange: none)
            } else {
                client.setMarkedText("", selectionRange: NSRange(location: 0, length: 0), replacementRange: none)
            }
        }
    }

    // ------------------------------------------------------------
    // Engine state -> client
    // ------------------------------------------------------------

    /// Push engine output to the client. Returns true when it committed
    /// text or left marked text behind.
    @discardableResult
    private func sync(to client: IMKTextInput, fixed: String) -> Bool {
        if !fixed.isEmpty {
            insert(fixed, to: client)
        }

        var marked = Engine.session.composing

        // Caret and candidate line: MarkedText (tested, ADR 0008)
        let caret = MarkedText.caret(composing: marked, cursorFromEnd: Engine.session.composingCursor)
        if Engine.session.candidatesVisible {
            let (page, pageCount) = Engine.session.candidatePage
            marked += MarkedText.candidateLine(candidates: Engine.session.candidates,
                                               cursor: Engine.session.candidateCursor,
                                               page: page, pageCount: pageCount)
        }

        setMarkedText(marked, caret: caret, to: client)

        // Suggestions float under the caret once the marked text is placed
        // (so the client reports the right rectangle)
        if Engine.session.completionVisible {
            CompletionWindow.shared.show(
                completions: Engine.session.completions,
                prefixLength: Engine.session.completionPrefixLength,
                client: client)
        } else {
            CompletionWindow.shared.hide()
        }

        // The selected candidate's annotation, also under the caret (only
        // while selecting, so it never overlaps the suggestions)
        if let annotation = Engine.session.annotation {
            AnnotationWindow.shared.show(annotation, client: client)
        } else {
            AnnotationWindow.shared.hide()
        }

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
