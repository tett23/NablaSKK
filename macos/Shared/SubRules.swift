// NablaSKK: AquaSKK's sub-rules (AZIK, ACT, yakumono-ippai, old-kana),
// shared by the input method and the preferences app (ADR 0003).
//
// Written by tett23, 2026.
// License: GPL-2.0-or-later. See the LICENSE file for details.

import Foundation

/// Which of the bundled AquaSKK sub-rule files the settings select, and
/// their contents. The files are AquaSKK's own (data/, EUC-JP), copied
/// into the app bundles' Resources.
enum SubRules {
    /// Rule files in AquaSKK's order (sub-rule.desc): the input style,
    /// then yakumono-ippai, then old-kana; later rules win.
    static func ruleFiles(for settings: InputSettings) -> [String] {
        var files: [String] = []
        switch settings.inputStyle {
        case .standard: break
        case .azik: files.append("azik.rule")
        case .azikUS: files.append("azik_us.rule")
        case .act: files.append("act.rule")
        }
        if settings.yakumonoIppai { files.append("yakumono-ippai.rule") }
        if settings.oldKana { files.append("old-kana.rule") }
        return files
    }

    /// The keymap patch of the input style (both AZIK layouts share one).
    static func keymapFile(for settings: InputSettings) -> String? {
        switch settings.inputStyle {
        case .standard: return nil
        case .azik, .azikUS: return "azik.conf"
        case .act: return "act.conf"
        }
    }

    /// A bundled file as UTF-8 text (the rule files are EUC-JP).
    static func read(_ name: String, in directory: URL) -> String? {
        guard let data = try? Data(contentsOf: directory.appendingPathComponent(name)) else { return nil }
        return String(data: data, encoding: .japaneseEUC) ?? String(data: data, encoding: .utf8)
    }

    /// The selected rule files joined in order, or nil when none.
    static func ruleText(for settings: InputSettings, in directory: URL) -> String? {
        let texts = ruleFiles(for: settings).compactMap { read($0, in: directory) }
        return texts.isEmpty ? nil : texts.joined(separator: "\n")
    }

    static func keymapText(for settings: InputSettings, in directory: URL) -> String? {
        keymapFile(for: settings).flatMap { read($0, in: directory) }
    }

    /// "組み込み" plus the names of the selected sub-rules, for messages
    /// about the layered table.
    static func origin(for settings: InputSettings) -> String {
        var names = ["組み込み"]
        if settings.inputStyle != .standard { names.append(settings.inputStyle.label) }
        if settings.yakumonoIppai { names.append("記号入力") }
        if settings.oldKana { names.append("旧かな") }
        return names.joined(separator: "・")
    }

    /// The built-in rules with the selected sub-rules layered on top, as
    /// the engine ends up with them (a later rule for an input replaces
    /// the earlier one). Used for the preferences app's conflict checks
    /// and its list of symbol rules.
    static func layeredRules(base: [KanaRule], settings: InputSettings, in directory: URL) -> [KanaRule] {
        let layers = ruleFiles(for: settings).compactMap { read($0, in: directory) }
            .map(KanaRuleSettings.builtinRules(from:))
        return layers.reduce(base) { merged, layer in
            let replaced = Dictionary(layer.map { ($0.input, $0.output) }, uniquingKeysWith: { $1 })
            let kept = merged.map { rule -> KanaRule in
                guard let output = replaced[rule.input] else { return rule }
                return KanaRule(input: rule.input, output: output)
            }
            let known = Set(merged.map(\.input))
            return kept + layer.filter { !known.contains($0.input) }
        }
    }
}
