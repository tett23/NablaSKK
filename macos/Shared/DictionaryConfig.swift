// NablaSKK: dictionaries.conf model shared by the input method and the
// preferences app.
//
// Written by tett23, 2026.
// License: GPL-2.0-or-later. See the LICENSE file for details.

import Foundation

/// One line of dictionaries.conf.
///
/// File format, one entry per line: `type location`. A leading `-` marks
/// a disabled entry (`- type location`); `#` lines are comments. Types use
/// AquaSKK's DictionarySet numbering.
struct DictionaryEntry: Identifiable, Equatable, Codable {
    enum Kind: Int32, CaseIterable, Identifiable, Codable {
        case common = 0
        case autoUpdate = 1
        case proxy = 2
        case gadget = 4
        case commonUTF8 = 5

        var id: Int32 { rawValue }

        var label: String {
            switch self {
            case .common: return "SKK 辞書 (EUC-JP / UTF-8 自動判別)"
            case .autoUpdate: return "SKK 辞書 (自動ダウンロード)"
            case .proxy: return "skkserv"
            case .gadget: return "プログラム実行変換 (today, now, =式)"
            case .commonUTF8: return "SKK 辞書 (UTF-8)"
            }
        }

        var locationHint: String {
            switch self {
            case .common, .commonUTF8: return "辞書ファイルのパス"
            case .autoUpdate: return "host url 保存先パス (例: openlab.jp /skk/dict/SKK-JISYO.L /path/to/SKK-JISYO.L)"
            case .proxy: return "host:port (例: localhost:1178)"
            case .gadget: return "(場所は不要)"
            }
        }

        var needsLocation: Bool { self != .gadget }
    }

    var id = UUID()
    var enabled = true
    var kind: Kind
    var location: String

    var line: String {
        let body = kind.needsLocation ? "\(kind.rawValue) \(location)" : "\(kind.rawValue)"
        return enabled ? body : "- \(body)"
    }
}

enum DictionaryConfig {
    static let fileName = "dictionaries.conf"

    static var supportDirectory: URL {
        let url = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
            .appendingPathComponent("NablaSKK", isDirectory: true)
        try? FileManager.default.createDirectory(at: url, withIntermediateDirectories: true)
        return url
    }

    static var fileURL: URL { supportDirectory.appendingPathComponent(fileName) }

    static let header = """
    # NablaSKK dictionaries: one "type location" per line, searched in order.
    #   0 = SKK-JISYO (encoding auto-detected)
    #   1 = auto-update "host url path"
    #   2 = skkserv host:port    4 = gadget (today/now/=expr)
    #   5 = SKK-JISYO (UTF-8 forced)
    # A leading "-" disables an entry. Edit with the NablaSKK preferences
    # app (input menu > 辞書を管理...) or by hand.
    """

    /// Entries written on first launch.
    static func defaultEntries() -> [DictionaryEntry] {
        var entries = [DictionaryEntry(kind: .gadget, location: "")]

        // Reuse an existing AquaSKK dictionary when present
        let aquaskkDictionary = FileManager.default
            .urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
            .appendingPathComponent("AquaSKK/SKK-JISYO.L")
        if FileManager.default.fileExists(atPath: aquaskkDictionary.path) {
            entries.insert(DictionaryEntry(kind: .common, location: aquaskkDictionary.path), at: 0)
        }

        return entries
    }

    static func parse(_ text: String) -> [DictionaryEntry] {
        var entries: [DictionaryEntry] = []

        for rawLine in text.split(separator: "\n", omittingEmptySubsequences: false) {
            var line = rawLine.trimmingCharacters(in: .whitespaces)
            if line.isEmpty || line.hasPrefix("#") { continue }

            var enabled = true
            if line.hasPrefix("-") {
                enabled = false
                line = line.dropFirst().trimmingCharacters(in: .whitespaces)
            }

            let fields = line.split(separator: " ", maxSplits: 1)
            guard let first = fields.first, let value = Int32(first),
                  let kind = DictionaryEntry.Kind(rawValue: value)
            else { continue }

            let location = fields.count > 1 ? String(fields[1]) : ""
            entries.append(DictionaryEntry(enabled: enabled, kind: kind, location: location))
        }

        return entries
    }

    static func serialize(_ entries: [DictionaryEntry]) -> String {
        ([header] + entries.map(\.line)).joined(separator: "\n") + "\n"
    }

    /// Load the configuration, creating it with defaults on first use.
    static func load() -> [DictionaryEntry] {
        if let text = try? String(contentsOf: fileURL, encoding: .utf8) {
            return parse(text)
        }

        let entries = defaultEntries()
        try? save(entries)
        return entries
    }

    static func save(_ entries: [DictionaryEntry]) throws {
        try serialize(entries).write(to: fileURL, atomically: true, encoding: .utf8)
    }

    /// Directory that holds dictionaries imported through the preferences
    /// app, so the configuration keeps working if the original file moves.
    static var dictionariesDirectory: URL {
        let url = supportDirectory.appendingPathComponent("dictionaries", isDirectory: true)
        try? FileManager.default.createDirectory(at: url, withIntermediateDirectories: true)
        return url
    }

    /// Copy a dictionary file into `dictionariesDirectory` and return the
    /// copy's path. A file already inside the support directory is used in
    /// place; an existing copy with the same name and contents is reused;
    /// otherwise the copy gets a numbered suffix.
    static func importDictionary(at source: URL) throws -> String {
        let manager = FileManager.default
        let directory = dictionariesDirectory

        if source.standardizedFileURL.path.hasPrefix(supportDirectory.standardizedFileURL.path + "/") {
            return source.path
        }

        let base = source.deletingPathExtension().lastPathComponent
        let ext = source.pathExtension
        var attempt = 0

        while true {
            var name = base
            if attempt > 0 { name += "-\(attempt)" }
            if !ext.isEmpty { name += ".\(ext)" }
            let destination = directory.appendingPathComponent(name)

            if !manager.fileExists(atPath: destination.path) {
                try manager.copyItem(at: source, to: destination)
                return destination.path
            }

            if manager.contentsEqual(atPath: source.path, andPath: destination.path) {
                return destination.path
            }

            attempt += 1
        }
    }

    static var modificationDate: Date? {
        (try? FileManager.default.attributesOfItem(atPath: fileURL.path))?[.modificationDate] as? Date
    }
}
