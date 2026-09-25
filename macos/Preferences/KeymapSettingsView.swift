// NablaSKK Preferences: key binding pane.
//
// Written by tett23, 2026.
// License: GPL-2.0-or-later. See the LICENSE file for details.

import SwiftUI

final class KeymapStore: ObservableObject {
    @Published var settings: KeymapSettings {
        didSet { save() }
    }
    @Published var saveError: String?

    init() {
        settings = KeymapSettings.load()
    }

    private func save() {
        do {
            try settings.save()
            saveError = nil
        } catch {
            saveError = error.localizedDescription
        }
    }
}

struct KeymapSettingsView: View {
    @StateObject private var store = KeymapStore()

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text("キーは keymap.conf の書き方で指定します。修飾キーは ctrl:: / shift:: / alt:: / meta::、"
                 + "文字コードは hex::0x1b、キーコードは keycode::7b、複数のキーは || で区切ります。"
                 + "空にするとデフォルトに戻ります。")
                .font(.callout)
                .foregroundColor(.secondary)

            HStack(spacing: 12) {
                Text("操作").frame(width: KeymapRow.labelWidth, alignment: .leading)
                Text("キー")
                Spacer()
                Text("").frame(width: KeymapRow.resetWidth)
            }
            .font(.caption)
            .foregroundColor(.secondary)
            .padding(.horizontal, 8)

            List {
                ForEach(KeymapSettings.actions) { action in
                    KeymapRow(action: action, store: store)
                }
            }

            HStack {
                Button("すべてデフォルトに戻す") { store.settings.overrides.removeAll() }
                    .disabled(store.settings.overrides.isEmpty)
                Spacer()
                if let error = store.saveError {
                    Text("保存できません: \(error)").foregroundColor(.red).font(.callout)
                } else {
                    Text("変更は次にどこかのアプリで入力を始めたときに反映されます。")
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
            }
        }
        .padding()
    }
}

struct KeymapRow: View {
    static let labelWidth: CGFloat = 200
    static let resetWidth: CGFloat = 28

    let action: KeymapAction
    @ObservedObject var store: KeymapStore
    @State private var text = ""

    private var isOverridden: Bool { store.settings.overrides[action.symbol] != nil }
    private var isValid: Bool {
        text.trimmingCharacters(in: .whitespaces).isEmpty || KeymapSettings.isValid(text)
    }

    var body: some View {
        HStack(spacing: 12) {
            VStack(alignment: .leading, spacing: 1) {
                Text(action.label)
                Text(action.symbol).font(.caption2).foregroundColor(.secondary)
            }
            .frame(width: Self.labelWidth, alignment: .leading)

            TextField(action.defaultKeys, text: $text, onCommit: commit)
                .textFieldStyle(.roundedBorder)
                .font(.system(.body, design: .monospaced))
                .foregroundColor(isValid ? .primary : .red)
                .help(isValid ? "デフォルト: \(action.defaultKeys)" : "書き方が正しくありません")
                .onChange(of: text) { _ in
                    if isValid { commit() }
                }

            Button(action: { text = ""; commit() }) {
                Image(systemName: "arrow.uturn.backward")
            }
            .buttonStyle(.borderless)
            .frame(width: Self.resetWidth)
            .disabled(!isOverridden)
            .help("デフォルト (\(action.defaultKeys)) に戻す")
        }
        .padding(.vertical, 2)
        .onAppear { text = store.settings.overrides[action.symbol] ?? "" }
        .onChange(of: store.settings.overrides[action.symbol]) { value in
            if (value ?? "") != text.trimmingCharacters(in: .whitespaces) { text = value ?? "" }
        }
    }

    private func commit() {
        guard isValid else { return }
        var settings = store.settings
        settings.set(text, for: action)
        if settings != store.settings { store.settings = settings }
    }
}
