// NablaSKK: accessibility labels and list reordering for the preferences
// app (ADR 0006), kept free of SwiftUI so they can be unit tested.
//
// Written by tett23, 2026.
// License: GPL-2.0-or-later. See the LICENSE file for details.

import Foundation

/// VoiceOver names for controls that show only an icon, or whose visible
/// label is hidden or is just an example (ADR 0006, audit A-1 to A-3).
enum PreferencesAccessibility {
    // 辞書
    static let addDictionary = "辞書を追加"
    static let removeDictionary = "選択した辞書を削除"
    static let moveDictionaryUp = "選択した辞書を上へ"
    static let moveDictionaryDown = "選択した辞書を下へ"
    static func dictionaryEnabled(_ name: String) -> String { "\(name)を検索に使う" }
    static func dictionaryKind(_ name: String) -> String { "\(name)の種類" }
    static func dictionaryLocation(_ name: String) -> String { "\(name)の場所" }

    // ユーザー辞書
    static let addEntry = "見出し語を追加"
    static let removeEntry = "選択した見出し語を削除"
    static let addCandidate = "候補を追加"
    static let removeCandidate = "選択した候補を削除"
    static let moveCandidateUp = "選択した候補を上へ"
    static let moveCandidateDown = "選択した候補を下へ"

    // キー
    static func resetKey(_ action: String) -> String { "「\(action)」をデフォルトに戻す" }
    static func keyField(_ action: String) -> String { "\(action)のキー" }

    // ローマ字
    static func deleteRule(input: String, output: String) -> String {
        input.isEmpty && output.isEmpty ? "空のルールを削除" : "「\(input) → \(output)」のルールを削除"
    }
    static func ruleInput(_ number: Int) -> String { "\(number) 番目のルールの入力キー" }
    static func ruleOutput(_ number: Int) -> String { "\(number) 番目のルールの出力" }
    static func builtinRule(input: String, output: String) -> String { "\(input) を打つと \(output)" }
}

/// Drag reordering for the hand-built lists (ADR 0006): dropping the item
/// at `from` on the row at `to` puts it at that position.
enum ListReorder {
    static func moved<T>(_ items: [T], from: Int, to: Int) -> [T] {
        guard items.indices.contains(from), items.indices.contains(to), from != to else { return items }
        var result = items
        let item = result.remove(at: from)
        result.insert(item, at: to)
        return result
    }
}
