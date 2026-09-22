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

    static let fileName = "settings.conf"
    static var fileURL: URL { DictionaryConfig.supportDirectory.appendingPathComponent(fileName) }

    static let header = """
    # NablaSKK input settings: key=value per line.
    #   comma=fullwidth   type , as ， (default: 、)
    #   period=fullwidth  type . as ． (default: 。)
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
            default: break
            }
        }
        return settings
    }

    func serialize() -> String {
        Self.header + "\n"
            + "comma=\(fullWidthComma ? "fullwidth" : "japanese")\n"
            + "period=\(fullWidthPeriod ? "fullwidth" : "japanese")\n"
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
