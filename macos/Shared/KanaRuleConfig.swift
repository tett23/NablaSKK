// NablaSKK: user-defined romaji-kana rules (kana-rule.conf) shared by the
// input method and the preferences app.
//
// Written by tett23, 2026.
// License: GPL-2.0-or-later. See the LICENSE file for details.

import Foundation

/// One user rule: typing `input` in a kana mode produces `output`, like
/// the built-in "z." → "…". Katakana and half-width kana outputs are
/// derived from `output`.
struct KanaRule: Identifiable, Equatable {
    var id = UUID()
    var input: String
    var output: String

    static func == (lhs: KanaRule, rhs: KanaRule) -> Bool {
        lhs.input == rhs.input && lhs.output == rhs.output
    }

    /// True when the output is a symbol rather than kana (z. → …, [ → 「,
    /// , → 、): the built-in rules worth showing, since a → あ is obvious.
    /// Kana letters are hiragana U+3041–3096 plus the voiced sound marks,
    /// and katakana U+30A1–30FA; ー and ・ count as symbols.
    var isSymbolRule: Bool {
        !output.unicodeScalars.contains { scalar in
            (0x3041...0x3096).contains(scalar.value) || (0x309B...0x309C).contains(scalar.value)
                || (0x30A1...0x30FA).contains(scalar.value)
        }
    }

    /// Output for katakana mode (ひらがな → カタカナ; other characters as is).
    var katakana: String {
        output.applyingTransform(.hiraganaToKatakana, reverse: false) ?? output
    }

    /// Output for half-width kana mode (、→ ､, カ → ｶ).
    var jisx0201Kana: String {
        katakana.applyingTransform(.fullwidthToHalfwidth, reverse: false) ?? katakana
    }

    enum Problem: Equatable {
        case emptyInput
        case emptyOutput
        case invalidCharacters
        case uppercase
        case tooLong
        /// The first key switches modes before the rule can match.
        case modeKey(Character)
    }

    static let maxInputLength = 8
    /// Keys the engine treats as mode switches in kana mode when they do
    /// not continue a rule (keymap.conf: ToggleKana q, SwitchToAscii l,
    /// EnterAbbrev /). A rule starting with one can never be typed.
    static let modeKeys: [Character: String] = ["q": "ひらがな/カタカナ切替", "l": "英数モードへの切替", "/": "abbrev モードへの切替"]

    /// Why this rule cannot be used, or nil when it is fine.
    var problem: Problem? {
        if input.isEmpty { return .emptyInput }
        if output.isEmpty { return .emptyOutput }
        let bytes = Array(input.utf8)
        guard bytes.allSatisfy({ (0x21...0x7e).contains($0) }) else { return .invalidCharacters }
        if bytes.contains(where: { (0x41...0x5a).contains($0) }) { return .uppercase }
        if bytes.count > Self.maxInputLength { return .tooLong }
        if let first = input.first, Self.modeKeys[first] != nil { return .modeKey(first) }
        if output.contains(where: { $0.isNewline }) { return .invalidCharacters }
        return nil
    }

    var problemMessage: String? {
        switch problem {
        case .none: return nil
        case .emptyInput: return "入力キーを指定してください"
        case .emptyOutput: return "出力を指定してください"
        case .invalidCharacters: return "入力キーは空白を含まない半角英数記号にしてください"
        case .uppercase: return "大文字は▽変換の開始になるので使えません"
        case .tooLong: return "入力キーは \(Self.maxInputLength) 文字までです"
        case .modeKey(let key): return "\(key) は\(Self.modeKeys[key] ?? "モード切替")キーなので、このルールは入力できません"
        }
    }

    /// kana-rule.conf line: "input,ひらがな,カタカナ,半角カナ".
    var line: String {
        [Self.escape(input, isKey: true), Self.escape(output), Self.escape(katakana), Self.escape(jisx0201Kana)]
            .joined(separator: ",")
    }

    /// The engine splits on commas and spaces and treats a leading "#"
    /// as a comment, so those are written as entities.
    static func escape(_ text: String, isKey: Bool = false) -> String {
        var escaped = text.replacingOccurrences(of: ",", with: "&comma;")
            .replacingOccurrences(of: " ", with: "&space;")
        if isKey && escaped.hasPrefix("#") {
            escaped = "&sharp;" + escaped.dropFirst()
        }
        return escaped
    }

    static func unescape(_ text: String) -> String {
        text.replacingOccurrences(of: "&comma;", with: ",")
            .replacingOccurrences(of: "&space;", with: " ")
            .replacingOccurrences(of: "&sharp;", with: "#")
    }
}

/// The user's rules, stored in kana-rule.conf format in the support
/// directory. The input method applies them after the built-in rules and
/// the punctuation options, so they win over both.
struct KanaRuleSettings: Equatable {
    var rules: [KanaRule] = []

    static let fileName = "kana-rule.conf"
    static var fileURL: URL { DictionaryConfig.supportDirectory.appendingPathComponent(fileName) }

    static let header = """
    # NablaSKK user romaji-kana rules (UTF-8), applied over the built-in
    # kana-rule.conf: "input,hiragana,katakana,half-width katakana" per line.
    # Edit with the NablaSKK preferences app (ローマ字 page) or by hand.
    """

    /// Rule text for the engine: usable rules only, in order (a later
    /// rule for the same input wins).
    var patchText: String {
        rules.filter { $0.problem == nil }.map { $0.line + "\n" }.joined()
    }

    static func parse(_ text: String) -> KanaRuleSettings {
        var settings = KanaRuleSettings()
        for rawLine in text.split(separator: "\n") {
            let line = String(rawLine)
            if line.trimmingCharacters(in: .whitespaces).isEmpty || line.hasPrefix("#") { continue }
            let fields = line.split(separator: ",", omittingEmptySubsequences: false).map(String.init)
            guard fields.count >= 2 else { continue }
            settings.rules.append(KanaRule(input: KanaRule.unescape(fields[0]), output: KanaRule.unescape(fields[1])))
        }
        return settings
    }

    func serialize() -> String {
        ([Self.header] + rules.map(\.line)).joined(separator: "\n") + "\n"
    }

    static func load() -> KanaRuleSettings {
        guard let text = try? String(contentsOf: fileURL, encoding: .utf8) else { return KanaRuleSettings() }
        return parse(text)
    }

    func save() throws {
        try serialize().write(to: Self.fileURL, atomically: true, encoding: .utf8)
    }

    static var modificationDate: Date? {
        (try? FileManager.default.attributesOfItem(atPath: fileURL.path))?[.modificationDate] as? Date
    }

    /// Why each rule conflicts with the built-in table or with a rule
    /// earlier in the list, keyed by rule id. Rules that are invalid on
    /// their own (`KanaRule.problem`) are not checked. A rule conflicts
    /// when it
    /// - repeats an existing rule (same input and output),
    /// - reuses an existing input with a different output, or
    /// - extends an existing rule that currently completes on its last key
    ///   (adding "ka." would make "ka" wait for another key).
    /// Being a prefix of an existing rule ("f" of "fa") changes nothing
    /// else and is allowed.
    func conflicts(builtin: [KanaRule]) -> [UUID: String] {
        var result: [UUID: String] = [:]
        var accepted = builtin.map { (rule: $0, isBuiltin: true) }

        for rule in rules where rule.problem == nil {
            if let same = accepted.last(where: { $0.rule.input == rule.input }) {
                let origin = same.isBuiltin ? "組み込みのルール" : "追加したルール"
                result[rule.id] = same.rule.output == rule.output
                    ? "同じルールが\(origin)にあります"
                    : "\(origin)「\(same.rule.input) → \(same.rule.output)」と入力キーが衝突します"
                continue
            }

            let inputs = accepted.map(\.rule.input)
            if let blocked = accepted.first(where: { candidate in
                let prefix = candidate.rule.input
                guard rule.input.count > prefix.count, rule.input.hasPrefix(prefix) else { return false }
                // Completes on its last key today: nothing else continues it
                return !inputs.contains { $0 != prefix && $0.hasPrefix(prefix) }
            }) {
                result[rule.id] = "「\(blocked.rule.input) → \(blocked.rule.output)」がすぐに確定しなくなるため使えません"
                continue
            }

            accepted.append((rule: rule, isBuiltin: false))
        }
        return result
    }

    /// The rules that are saved and handed to the engine.
    func acceptedRules(builtin: [KanaRule]) -> [KanaRule] {
        let conflicts = conflicts(builtin: builtin)
        return rules.filter { $0.problem == nil && conflicts[$0.id] == nil }
    }

    /// Built-in rules (input → hiragana) from data/kana-rule.utf8.conf in
    /// file order, shown read-only in the preferences app. A later line for
    /// the same input replaces an earlier one, as in the engine.
    static func builtinRules(from text: String) -> [KanaRule] {
        var rules: [KanaRule] = []
        var index: [String: Int] = [:]
        for rawLine in text.split(separator: "\n") {
            let line = String(rawLine)
            if line.isEmpty || line.hasPrefix("#") { continue }
            let fields = line.split(separator: ",", omittingEmptySubsequences: false).map(String.init)
            guard fields.count >= 4 else { continue }
            let rule = KanaRule(input: KanaRule.unescape(fields[0]), output: KanaRule.unescape(fields[1]))
            if let existing = index[rule.input] {
                rules[existing].output = rule.output
            } else {
                index[rule.input] = rules.count
                rules.append(rule)
            }
        }
        return rules
    }
}
