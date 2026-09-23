// NablaSKK: key binding overrides (keymap.conf) shared by the input
// method and the preferences app.
//
// Written by tett23, 2026.
// License: GPL-2.0-or-later. See the LICENSE file for details.

import Foundation

/// One rebindable action from data/keymap.conf.
struct KeymapAction: Identifiable, Equatable {
    /// Symbol name as keymap.conf spells it (SKK_JMODE, ToggleKana, ...).
    let symbol: String
    let label: String
    /// Key spec from the built-in keymap.conf; shown as the default.
    let defaultKeys: String

    var id: String { symbol }
}

/// Per-action key overrides. Stored as `keymap.conf` in the support
/// directory using the built-in file's syntax, one `Symbol keys` line per
/// customised action. The engine loads the built-in keymap and then
/// applies these lines, each replacing every key of its symbol.
struct KeymapSettings: Equatable {
    var overrides: [String: String] = [:]

    static let fileName = "keymap.conf"
    static var fileURL: URL { DictionaryConfig.supportDirectory.appendingPathComponent(fileName) }

    /// Actions offered by the preferences app. Defaults must match
    /// data/keymap.conf (checked by the macOS tests). SKK_PASTE is left
    /// out: the input method recognises its keys before the engine does.
    static let actions: [KeymapAction] = [
        KeymapAction(symbol: "SKK_JMODE", label: "かなモードへ", defaultKeys: "ctrl::j"),
        KeymapAction(symbol: "SKK_ENTER", label: "確定", defaultKeys: "group::hex::0x03,0x0a,0x0d||ctrl::m"),
        KeymapAction(symbol: "SKK_CANCEL", label: "取り消し", defaultKeys: "ctrl::g||hex::0x1b"),
        KeymapAction(symbol: "SKK_BACKSPACE", label: "1 文字削除 (後退)", defaultKeys: "hex::0x08||ctrl::h"),
        KeymapAction(symbol: "SKK_DELETE", label: "1 文字削除 (前方)", defaultKeys: "hex::0x7f||ctrl::d"),
        KeymapAction(symbol: "SKK_TAB", label: "補完", defaultKeys: "hex::0x09||ctrl::i"),
        KeymapAction(symbol: "SKK_LEFT", label: "カーソル左", defaultKeys: "hex::0x1c||ctrl::b||keycode::7b"),
        KeymapAction(symbol: "SKK_RIGHT", label: "カーソル右", defaultKeys: "hex::0x1d||ctrl::f||keycode::7c"),
        KeymapAction(symbol: "SKK_UP", label: "カーソル先頭", defaultKeys: "hex::0x1e||ctrl::a||keycode::7e"),
        KeymapAction(symbol: "SKK_DOWN", label: "カーソル末尾", defaultKeys: "hex::0x1f||ctrl::e||keycode::7d"),
        KeymapAction(symbol: "SKK_PING", label: "モード表示 (ping)", defaultKeys: "ctrl::l"),
        KeymapAction(symbol: "SKK_UNDO", label: "確定の取り消し", defaultKeys: "ctrl::/"),
        KeymapAction(symbol: "ToggleKana", label: "ひらがな / カタカナ切替", defaultKeys: "q"),
        KeymapAction(symbol: "ToggleJisx0201Kana", label: "半角カナ切替", defaultKeys: "ctrl::q"),
        KeymapAction(symbol: "SwitchToAscii", label: "英数モードへ", defaultKeys: "l"),
        KeymapAction(symbol: "SwitchToJisx0208Latin", label: "全角英数モードへ", defaultKeys: "L"),
        KeymapAction(symbol: "EnterAbbrev", label: "abbrev モードへ", defaultKeys: "/"),
        KeymapAction(symbol: "EnterJapanese", label: "日本語入力開始 (Q)", defaultKeys: "Q"),
        KeymapAction(symbol: "NextCompletion", label: "次の補完候補", defaultKeys: "."),
        KeymapAction(symbol: "PrevCompletion", label: "前の補完候補", defaultKeys: ","),
        KeymapAction(symbol: "NextCandidate", label: "次の変換候補", defaultKeys: "hex::0x20||ctrl::n"),
        KeymapAction(symbol: "PrevCandidate", label: "前の変換候補", defaultKeys: "x||ctrl::p"),
        KeymapAction(symbol: "RemoveTrigger", label: "候補の削除", defaultKeys: "X"),
        KeymapAction(symbol: "CompConversion", label: "補完候補で変換", defaultKeys: "alt::hex::0x20||shift::hex::0x20"),
    ]

    static let header = """
    # NablaSKK key bindings: "Symbol keys" per line, keymap.conf syntax.
    # Each line replaces every key of its symbol in the built-in keymap.
    # Edit with the NablaSKK preferences app (キー tab) or by hand.
    """

    /// Effective key spec for an action.
    func keys(for action: KeymapAction) -> String {
        overrides[action.symbol] ?? action.defaultKeys
    }

    /// Store a spec; a value equal to the default (or empty) clears the
    /// override.
    mutating func set(_ keys: String, for action: KeymapAction) {
        let trimmed = keys.trimmingCharacters(in: .whitespaces)
        if trimmed.isEmpty || trimmed == action.defaultKeys {
            overrides.removeValue(forKey: action.symbol)
        } else {
            overrides[action.symbol] = trimmed
        }
    }

    /// The lines handed to the engine (only valid overrides).
    var overrideText: String {
        Self.actions.compactMap { action in
            guard let keys = overrides[action.symbol], Self.isValid(keys) else { return nil }
            return "\(action.symbol)\t\(keys)"
        }.joined(separator: "\n")
    }

    /// Syntax check mirroring the engine's KeymapEntry parser:
    /// `spec||spec`, each spec `[modifier::]*[hex::|keycode::]key` or
    /// `group::[hex::|keycode::]item,item-item`.
    static func isValid(_ keys: String) -> Bool {
        let trimmed = keys.trimmingCharacters(in: .whitespaces)
        guard !trimmed.isEmpty, !trimmed.contains(where: { $0 == " " || $0 == "\t" }) else { return false }

        for spec in trimmed.components(separatedBy: "||") {
            var group = false, hex = false, keycode = false
            var key: String?
            for token in spec.components(separatedBy: "::") {
                switch token {
                case "group": group = true
                case "hex": hex = true
                case "keycode": keycode = true
                case "shift", "ctrl", "alt", "meta": break
                default:
                    guard key == nil else { return false }
                    key = token
                }
            }
            guard let key, !key.isEmpty else { return false }

            let items = group ? key.split(separator: ",", omittingEmptySubsequences: false).map(String.init) : [key]
            for item in items {
                let codes = group && item.contains("-") && (hex || keycode || item.count > 1)
                    ? item.split(separator: "-", omittingEmptySubsequences: false).map(String.init)
                    : [item]
                guard codes.count <= 2 else { return false }
                for code in codes {
                    if hex || keycode {
                        let digits = code.hasPrefix("0x") ? String(code.dropFirst(2)) : code
                        guard !digits.isEmpty, digits.count <= 2,
                              digits.allSatisfy({ $0.isHexDigit }) else { return false }
                    } else {
                        guard code.utf8.count == 1 else { return false }
                    }
                }
            }
        }
        return true
    }

    static func parse(_ text: String) -> KeymapSettings {
        var settings = KeymapSettings()
        let known = Dictionary(uniqueKeysWithValues: actions.map { ($0.symbol, $0) })
        for rawLine in text.split(separator: "\n") {
            let line = rawLine.trimmingCharacters(in: .whitespaces)
            if line.isEmpty || line.hasPrefix("#") { continue }
            let fields = line.split(maxSplits: 1, whereSeparator: { $0 == " " || $0 == "\t" })
            guard fields.count == 2, let action = known[String(fields[0])] else { continue }
            settings.set(String(fields[1]), for: action)
        }
        return settings
    }

    func serialize() -> String {
        var lines = [Self.header]
        for action in Self.actions {
            if let keys = overrides[action.symbol] {
                lines.append("\(action.symbol)\t\(keys)")
            }
        }
        return lines.joined(separator: "\n") + "\n"
    }

    static func load() -> KeymapSettings {
        guard let text = try? String(contentsOf: fileURL, encoding: .utf8) else { return KeymapSettings() }
        return parse(text)
    }

    func save() throws {
        try serialize().write(to: Self.fileURL, atomically: true, encoding: .utf8)
    }

    static var modificationDate: Date? {
        (try? FileManager.default.attributesOfItem(atPath: fileURL.path))?[.modificationDate] as? Date
    }
}
