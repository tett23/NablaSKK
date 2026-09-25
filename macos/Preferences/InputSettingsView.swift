// NablaSKK Preferences: input settings and skkserv panes.
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
            }

            Section {
                Toggle(isOn: $store.settings.suggestEnabled) {
                    VStack(alignment: .leading, spacing: 2) {
                        Text("入力中にサジェストを表示する")
                        Text("▽で読みを入力している間、その読みで始まる辞書の見出し語をカーソルの下に一覧します。TAB で補完できます。")
                            .font(.caption).foregroundColor(.secondary)
                    }
                }
                Stepper(value: $store.settings.suggestCount, in: InputSettings.suggestCountRange) {
                    HStack {
                        Text("表示する件数")
                        Text("\(store.settings.suggestCount)")
                            .monospacedDigit()
                            .frame(minWidth: 24, alignment: .trailing)
                    }
                }
                .disabled(!store.settings.suggestEnabled)
                Toggle(isOn: $store.settings.completionExtended) {
                    VStack(alignment: .leading, spacing: 2) {
                        Text("補完にシステム辞書も使う")
                        Text("オフのときはユーザー辞書(自分が変換した読み)だけから補完・サジェストします。TAB 補完にも効きます。")
                            .font(.caption).foregroundColor(.secondary)
                    }
                }
            } header: {
                Text("サジェスト").font(.headline)
            } footer: {
                Text("設定は次にどこかのアプリで入力を始めたときに反映されます。")
                    .font(.caption)
                    .foregroundColor(.secondary)
                    .fixedSize(horizontal: false, vertical: true)
            }

            if let error = store.saveError {
                Text("保存できません: \(error)").foregroundColor(.red).font(.callout)
            }
        }
        .padding()
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
    }
}

/// skkserv pane, modelled on AquaSKK's dictionary server settings. The
/// server is queried after every dictionary in dictionaries.conf, so
/// local dictionaries always win.
struct SkkservSettingsView: View {
    @StateObject private var store = InputSettingsStore()
    @State private var portText = ""

    var body: some View {
        Form {
            Section {
                Toggle("skkserv を使う", isOn: $store.settings.skkservEnabled)

                TextField("ホスト", text: $store.settings.skkservHost, prompt: Text("localhost"))
                    .textFieldStyle(.roundedBorder)
                    .frame(maxWidth: 320)
                    .disabled(!store.settings.skkservEnabled)

                HStack {
                    TextField("ポート", text: $portText, prompt: Text("1178"))
                        .textFieldStyle(.roundedBorder)
                        .frame(width: 100)
                        .disabled(!store.settings.skkservEnabled)
                        .onChange(of: portText) { text in
                            if let port = Int(text), (1...65535).contains(port) {
                                store.settings.skkservPort = port
                            }
                        }
                    if Int(portText).map({ (1...65535).contains($0) }) != true {
                        Text("1〜65535 の数字を入力してください").font(.caption).foregroundColor(.red)
                    }
                }

                Picker("文字コード", selection: $store.settings.skkservEncoding) {
                    ForEach(InputSettings.SkkservEncoding.allCases) { encoding in
                        Text(encoding.label).tag(encoding)
                    }
                }
                .pickerStyle(.radioGroup)
                .disabled(!store.settings.skkservEnabled)
            } header: {
                Text("skkserv").font(.headline)
            } footer: {
                VStack(alignment: .leading, spacing: 4) {
                    Text("「辞書」ページの辞書をすべて検索したあとで skkserv に問い合わせます"
                         + "(ローカルの辞書が優先されます)。")
                    Text("従来の skkserv は EUC-JP です。yaskkserv2 などを UTF-8 で動かしている場合は UTF-8 を選んでください。"
                         + " 設定は次にどこかのアプリで入力を始めたときに反映されます。")
                }
                .font(.caption)
                .foregroundColor(.secondary)
                .fixedSize(horizontal: false, vertical: true)
                .frame(maxWidth: 480, alignment: .leading)
            }

            if let error = store.saveError {
                Text("保存できません: \(error)").foregroundColor(.red).font(.callout)
            }
        }
        .padding()
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
        .onAppear { portText = String(store.settings.skkservPort) }
    }
}
