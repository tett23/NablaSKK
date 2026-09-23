// NablaSKK: input settings (settings.conf) shared by the input method
// and the preferences app.
//
// Written by tett23, 2026.
// License: GPL-2.0-or-later. See the LICENSE file for details.

import Foundation

/// User-adjustable input options, stored as `key=value` lines. Each
/// option is independent; the input method re-reads the file when it next
/// becomes active.
struct InputSettings: Equatable {
    /// Type "," as "，" instead of "、".
    var fullWidthComma = false
    /// Type "." as "．" instead of "。".
    var fullWidthPeriod = false

    /// Wire encoding of an external skkserv.
    enum SkkservEncoding: String, CaseIterable, Identifiable {
        case eucJP = "euc-jp"
        case utf8 = "utf-8"

        var id: String { rawValue }

        var label: String {
            switch self {
            case .eucJP: return "EUC-JP"
            case .utf8: return "UTF-8"
            }
        }

        /// Dictionary type id for the engine (2 = skkserv EUC-JP,
        /// 6 = skkserv UTF-8), matching `DictionaryEntry.Kind`.
        var dictionaryKind: DictionaryEntry.Kind {
            switch self {
            case .eucJP: return .proxy
            case .utf8: return .proxyUTF8
            }
        }
    }

    /// Dynamic completion ("suggest"): while a reading is typed, list
    /// dictionary entries that extend it (AquaSKK's enable_dynamic_completion).
    var suggestEnabled = false
    /// How many entries to list (dynamic_completion_range).
    var suggestCount = 5
    /// Complete (TAB and suggest) from every dictionary rather than the
    /// user dictionary alone (enable_extended_completion, on in AquaSKK).
    var completionExtended = true

    static let suggestCountRange = 1...20

    /// Query an external skkserv after every local dictionary.
    var skkservEnabled = false
    var skkservHost = "localhost"
    var skkservPort = 1178
    var skkservEncoding = SkkservEncoding.eucJP

    /// "host:port" as the engine's proxy dictionary expects it.
    var skkservLocation: String {
        "\(skkservHost.trimmingCharacters(in: .whitespaces)):\(skkservPort)"
    }

    /// Whether the skkserv settings describe a usable server.
    var skkservIsConfigured: Bool {
        skkservEnabled && !skkservHost.trimmingCharacters(in: .whitespaces).isEmpty
            && (1...65535).contains(skkservPort)
    }

    static let fileName = "settings.conf"
    static var fileURL: URL { DictionaryConfig.supportDirectory.appendingPathComponent(fileName) }

    static let header = """
    # NablaSKK input settings: key=value per line.
    #   comma=fullwidth   type , as ， (default: 、)
    #   period=fullwidth  type . as ． (default: 。)
    #   suggest=on        list dictionary entries extending the reading being typed
    #   suggest_count=5   how many to list
    #   completion_extended=on|off  complete from all dictionaries (off: user dictionary only)
    #   skkserv=on        also query an skkserv, after the local dictionaries
    #   skkserv_host=localhost
    #   skkserv_port=1178
    #   skkserv_encoding=euc-jp | utf-8
    """

    // Sub-rules from AquaSKK's data/comma.rule and data/period.rule
    static let commaRule = "&comma;,，,，,&comma;\n"
    static let periodRule = ".,．,．,.\n"

    /// kana-rule patch text implementing the enabled options.
    var kanaRulePatch: String {
        (fullWidthComma ? Self.commaRule : "") + (fullWidthPeriod ? Self.periodRule : "")
    }

    static func parse(_ text: String) -> InputSettings {
        var settings = InputSettings()
        for rawLine in text.split(separator: "\n") {
            let line = rawLine.trimmingCharacters(in: .whitespaces)
            if line.isEmpty || line.hasPrefix("#") { continue }
            guard let equals = line.firstIndex(of: "=") else { continue }
            let key = line[..<equals].trimmingCharacters(in: .whitespaces)
            let value = line[line.index(after: equals)...].trimmingCharacters(in: .whitespaces)
            switch key {
            case "comma": settings.fullWidthComma = value == "fullwidth"
            case "period": settings.fullWidthPeriod = value == "fullwidth"
            case "suggest": settings.suggestEnabled = value == "on"
            case "suggest_count":
                settings.suggestCount = min(max(Int(value) ?? 5, suggestCountRange.lowerBound), suggestCountRange.upperBound)
            case "completion_extended": settings.completionExtended = value != "off"
            case "skkserv": settings.skkservEnabled = value == "on"
            case "skkserv_host": settings.skkservHost = value
            case "skkserv_port": settings.skkservPort = Int(value) ?? 1178
            case "skkserv_encoding": settings.skkservEncoding = SkkservEncoding(rawValue: value.lowercased()) ?? .eucJP
            default: break
            }
        }
        return settings
    }

    func serialize() -> String {
        Self.header + "\n"
            + "comma=\(fullWidthComma ? "fullwidth" : "japanese")\n"
            + "period=\(fullWidthPeriod ? "fullwidth" : "japanese")\n"
            + "suggest=\(suggestEnabled ? "on" : "off")\n"
            + "suggest_count=\(suggestCount)\n"
            + "completion_extended=\(completionExtended ? "on" : "off")\n"
            + "skkserv=\(skkservEnabled ? "on" : "off")\n"
            + "skkserv_host=\(skkservHost)\n"
            + "skkserv_port=\(skkservPort)\n"
            + "skkserv_encoding=\(skkservEncoding.rawValue)\n"
    }

    static func load() -> InputSettings {
        guard let text = try? String(contentsOf: fileURL, encoding: .utf8) else { return InputSettings() }
        return parse(text)
    }

    func save() throws {
        try serialize().write(to: Self.fileURL, atomically: true, encoding: .utf8)
    }

    static var modificationDate: Date? {
        (try? FileManager.default.attributesOfItem(atPath: fileURL.path))?[.modificationDate] as? Date
    }
}
