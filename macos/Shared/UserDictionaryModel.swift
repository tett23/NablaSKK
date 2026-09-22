// NablaSKK: user dictionary (skk-jisyo) model shared by the input method
// and the preferences app.
//
// Written by tett23, 2026.
// License: GPL-2.0-or-later. See the LICENSE file for details.

import Foundation

/// One candidate of a user dictionary entry.
struct UserCandidate: Identifiable, Equatable {
    var id = UUID()
    var word: String
    var annotation: String

    /// Serialized form: "word;annotation".
    var text: String {
        annotation.isEmpty ? word : "\(word);\(annotation)"
    }

    init(word: String, annotation: String = "") {
        self.word = word
        self.annotation = annotation
    }

    init(text: String) {
        if let separator = text.firstIndex(of: ";") {
            word = String(text[..<separator])
            annotation = String(text[text.index(after: separator)...])
        } else {
            word = text
            annotation = ""
        }
    }
}

/// Okuri hint block "[okuri/word/word/]" attached to an okuri-ari entry.
struct UserOkuriHint: Equatable {
    var okuri: String
    var words: [String]
}

/// One line of the user dictionary.
struct UserDictionaryEntry: Identifiable, Equatable {
    var id = UUID()
    var reading: String
    var okuriAri: Bool
    var candidates: [UserCandidate]
    var hints: [UserOkuriHint]

    init(reading: String, okuriAri: Bool, candidates: [UserCandidate], hints: [UserOkuriHint] = []) {
        self.reading = reading
        self.okuriAri = okuriAri
        self.candidates = candidates
        self.hints = hints
    }

    /// An okuri-ari reading ends with the romaji consonant of the okurigana
    /// ("おくr"); everything else, including ASCII abbrev readings, is
    /// okuri-nasi.
    static func isOkuriAri(reading: String) -> Bool {
        let scalars = Array(reading.unicodeScalars)
        guard scalars.count > 1, let last = scalars.last, let first = scalars.first else { return false }
        return ("a"..."z").contains(Character(last)) && first.value > 0x7f
    }

    // Okuri-nasi words are stored with "[", "/" and ";" escaped
    // (SKKCandidate::Encode); okuri-ari lines are stored verbatim.
    static func encode(_ word: String) -> String {
        word.replacingOccurrences(of: "[", with: "[5b]")
            .replacingOccurrences(of: "/", with: "[2f]")
            .replacingOccurrences(of: ";", with: "[3b]")
    }

    static func decode(_ word: String) -> String {
        word.replacingOccurrences(of: "[5b]", with: "[")
            .replacingOccurrences(of: "[2f]", with: "/")
            .replacingOccurrences(of: "[3b]", with: ";")
    }

    /// Parse "reading /cand/cand;note/[okuri/cand/]/" (port of
    /// SKKCandidateParser's two phases).
    static func parse(line: String, okuriAri: Bool) -> UserDictionaryEntry? {
        guard let space = line.firstIndex(of: " ") else { return nil }
        let reading = String(line[..<space])
        let body = line[line.index(after: space)...]
        guard !reading.isEmpty, body.hasPrefix("/") else { return nil }

        var candidates: [UserCandidate] = []
        var hints: [UserOkuriHint] = []
        var buffer = ""
        var inHint = false
        var hintOkuri = ""
        var hintWords: [String] = []

        for character in body {
            if inHint {
                if buffer.isEmpty {
                    if character == "/" || character == "[" { continue }
                    if character == "]" {
                        if !hintWords.isEmpty {
                            hints.append(UserOkuriHint(okuri: hintOkuri, words: hintWords))
                        }
                        hintOkuri = ""
                        hintWords = []
                        inHint = false
                        continue
                    }
                }
                if character == "/" {
                    if hintOkuri.isEmpty { hintOkuri = buffer } else { hintWords.append(buffer) }
                    buffer = ""
                    continue
                }
                buffer.append(character)
            } else {
                if character == "/" {
                    if !buffer.isEmpty {
                        let text = okuriAri ? buffer : decode(buffer)
                        candidates.append(UserCandidate(text: text))
                        buffer = ""
                    }
                    continue
                }
                if character == "[" && buffer.isEmpty {
                    inHint = true
                    continue
                }
                buffer.append(character)
            }
        }

        return UserDictionaryEntry(reading: reading, okuriAri: okuriAri, candidates: candidates, hints: hints)
    }

    var line: String {
        var text = reading + " "
        for candidate in candidates {
            let encoded = okuriAri
                ? candidate.text
                : UserCandidate(word: Self.encode(candidate.word), annotation: candidate.annotation).text
            text += "/" + encoded
        }
        for hint in hints {
            text += "/[" + hint.okuri + hint.words.map { "/" + $0 }.joined() + "/]"
        }
        return text + "/"
    }
}

/// The whole user dictionary file, kept in file order (most recently used
/// first, as the engine writes it).
struct UserDictionary: Equatable {
    var okuriAri: [UserDictionaryEntry] = []
    var okuriNasi: [UserDictionaryEntry] = []
    var isEUC = false

    static var fileURL: URL { DictionaryConfig.supportDirectory.appendingPathComponent("skk-jisyo") }

    static func parse(_ text: String) -> UserDictionary {
        var dictionary = UserDictionary()
        var section: Int? = nil  // 0 = okuri-ari, 1 = okuri-nasi

        for rawLine in text.split(separator: "\n", omittingEmptySubsequences: true) {
            let line = String(rawLine)
            if line.hasPrefix(";") {
                if line.contains("okuri-ari entries") { section = 0 }
                else if line.contains("okuri-nasi entries") { section = 1 }
                continue
            }
            guard let section else { continue }
            guard let entry = UserDictionaryEntry.parse(line: line, okuriAri: section == 0) else { continue }
            if section == 0 { dictionary.okuriAri.append(entry) } else { dictionary.okuriNasi.append(entry) }
        }

        return dictionary
    }

    func serialize() -> String {
        var text = ";; okuri-ari entries.\n"
        text += okuriAri.map { $0.line + "\n" }.joined()
        text += ";; okuri-nasi entries.\n"
        text += okuriNasi.map { $0.line + "\n" }.joined()
        return text
    }

    static func load(from url: URL = fileURL) throws -> UserDictionary {
        guard let data = try? Data(contentsOf: url) else { return UserDictionary() }

        if let text = String(data: data, encoding: .utf8) {
            return parse(text)
        }
        guard let text = String(data: data, encoding: .japaneseEUC) else {
            throw CocoaError(.fileReadInapplicableStringEncoding)
        }
        var dictionary = parse(text)
        dictionary.isEUC = true
        return dictionary
    }

    /// Write atomically, preserving the file's encoding.
    func save(to url: URL = fileURL) throws {
        let encoding: String.Encoding = isEUC ? .japaneseEUC : .utf8
        guard let data = serialize().data(using: encoding) else {
            throw CocoaError(.fileWriteInapplicableStringEncoding)
        }
        try data.write(to: url, options: .atomic)
    }
}
