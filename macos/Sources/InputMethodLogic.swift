// NablaSKK: the input method's own decisions, kept free of InputMethodKit
// so macos/run-tests.sh can test them (ADR 0008). SKKRustInputController
// only calls these.
//
// Written by tett23, 2026.
// License: GPL-2.0-or-later. See the LICENSE file for details.

import Foundation

/// The input mode each application was last typing in, so an application
/// left in ASCII gets ASCII back when it is activated again (the engine
/// session is shared and resets to hiragana on every client switch).
///
/// Recorded after every key the application's client handles, not on
/// deactivation: the next client can be activated before the previous one
/// is deactivated, and by then the session has already been reset. Keyed
/// by bundle identifier: the system hands out a fresh controller and a
/// fresh client identifier on every activation, even for the same window.
struct ClientModeMemory {
    private var lastModes: [String: SKKSession.InputMode] = [:]

    mutating func recordKey(client: String, mode: SKKSession.InputMode) {
        lastModes[client] = mode
    }

    /// The mode to switch to when `client` is activated after another
    /// client: ASCII when it was last used in ASCII, otherwise nothing.
    func modeToRestore(client: String) -> SKKSession.InputMode? {
        lastModes[client] == .ascii ? .ascii : nil
    }
}

/// How the engine's romaji rules and keymap are stacked (ADR 0003).
enum EngineSetup {
    /// Built-in rules, then the sub-rules (input style, yakumono-ippai,
    /// old-kana in AquaSKK's order), then the punctuation options, then
    /// the user's kana-rule.conf: what the user set wins.
    static func applyKanaRules(to session: SKKSession, settings: InputSettings, userRules: KanaRuleSettings,
                               subRuleDirectory: URL) {
        session.resetKanaRules()
        if let subRules = SubRules.ruleText(for: settings, in: subRuleDirectory) {
            session.patchKanaRules(subRules)
        }
        if !settings.kanaRulePatch.isEmpty {
            session.patchKanaRules(settings.kanaRulePatch)
        }
        if !userRules.patchText.isEmpty {
            session.patchKanaRules(userRules.patchText)
        }
    }

    /// Built-in keymap, then the input style's keymap patch (merged, as
    /// AquaSKK does), then the user's keymap.conf overrides.
    static func applyKeymap(to session: SKKSession, settings: InputSettings, keymap: KeymapSettings,
                            subRuleDirectory: URL) {
        session.resetKeymap()
        if let patch = SubRules.keymapText(for: settings, in: subRuleDirectory) {
            session.patchKeymap(patch)
        }
        let overrides = keymap.overrideText
        if !overrides.isEmpty {
            session.overrideKeymap(overrides)
        }
    }
}

/// Which keys go to the application instead of the engine.
enum KeyRouting {
    /// Cmd-V, and Ctrl-Y (the engine's own paste key)
    static func isPaste(charcode: UInt8, mods: SKKSession.Modifiers) -> Bool {
        (mods == [.meta] && charcode == UInt8(ascii: "v")) || (mods == [.ctrl] && charcode == UInt8(ascii: "y"))
    }

    /// Command shortcuts belong to the application, except Cmd-V while
    /// composing: the engine pastes it into the word being registered
    /// (only a Cmd-V the application lets through reaches the IME; Chrome
    /// keeps it as a menu shortcut, Ctrl-Y always arrives).
    static func passesToApplication(charcode: UInt8, mods: SKKSession.Modifiers, composing: Bool) -> Bool {
        mods.contains(.meta) && !(isPaste(charcode: charcode, mods: mods) && composing)
    }
}

/// Marked text shown in the client.
enum MarkedText {
    /// The caret position in UTF-16 units. The engine reports a character
    /// offset from the end of the composing string.
    static func caret(composing: String, cursorFromEnd: Int) -> Int {
        let characters = Array(composing)
        let index = max(0, min(characters.count, characters.count + cursorFromEnd))
        return String(characters[..<index]).utf16.count
    }

    /// Candidate window contents rendered into the marked text: labels
    /// select directly, matching the engine's key handling.
    static func candidateLine(candidates: [String], cursor: Int, page: Int, pageCount: Int,
                              labels: String = "asdfjkl") -> String {
        var line = " 【"
        for (index, candidate) in candidates.enumerated() {
            let label = labels[labels.index(labels.startIndex, offsetBy: min(index, labels.count - 1))]
            line += index == cursor ? "▶\(label):\(candidate) " : " \(label):\(candidate) "
        }
        return line + "】(\(page)/\(pageCount))"
    }
}
