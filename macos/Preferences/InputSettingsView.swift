// NablaSKK Preferences: input settings tab.
//
// Written by tett23, 2026.
// License: GPL-2.0-or-later. See the LICENSE file for details.

import SwiftUI

final class InputSettingsStore: ObservableObject {
    @Published var settings: InputSettings {
        didSet { save() }
    }
    @Published var saveError: String?

    init() {
        settings = InputSettings.load()
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

struct InputSettingsView: View {
    @StateObject private var store = InputSettingsStore()

    var body: some View {
        Form {
            Section {
                Toggle(isOn: $store.settings.fullWidthComma) {
                    VStack(alignment: .leading, spacing: 2) {
                        Text("読点に「，」を使う")
                        Text("オフのときは「、」").font(.caption).foregroundColor(.secondary)
                    }
                }
                Toggle(isOn: $store.settings.fullWidthPeriod) {
                    VStack(alignment: .leading, spacing: 2) {
                        Text("句点に「．」を使う")
                        Text("オフのときは「。」").font(.caption).foregroundColor(.secondary)
                    }
                }
            } header: {
                Text("句読点").font(.headline)
            } footer: {
                Text("設定は次にどこかのアプリで入力を始めたときに反映されます。")
                    .font(.caption)
                    .foregroundColor(.secondary)
            }

            if let error = store.saveError {
                Text("保存できません: \(error)").foregroundColor(.red).font(.callout)
            }
        }
        .padding()
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
    }
}
