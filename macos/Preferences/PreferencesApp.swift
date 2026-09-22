// NablaSKK Preferences: dictionary management window.
//
// Written by tett23, 2026.
// License: GPL-2.0-or-later. See the LICENSE file for details.

import SwiftUI
import UniformTypeIdentifiers

@main
struct PreferencesApp: App {
    var body: some Scene {
        WindowGroup("NablaSKK 辞書の設定") {
            DictionaryListView()
                .frame(minWidth: 640, minHeight: 360)
        }
        .commands {
            CommandGroup(replacing: .newItem) {}
        }
    }
}

/// Loads, edits and saves dictionaries.conf. Every change is saved
/// immediately; the input method reloads when it next becomes active.
final class DictionaryStore: ObservableObject {
    @Published var entries: [DictionaryEntry] {
        didSet { save() }
    }
    @Published var saveError: String?

    init() {
        entries = DictionaryConfig.load()
    }

    private func save() {
        do {
            try DictionaryConfig.save(entries)
            saveError = nil
        } catch {
            saveError = error.localizedDescription
        }
    }

    /// Dropped or chosen files are copied into the support directory.
    func add(fileURLs: [URL]) {
        for url in fileURLs where url.isFileURL {
            do {
                let path = try DictionaryConfig.importDictionary(at: url)
                entries.append(DictionaryEntry(kind: .common, location: path))
            } catch {
                saveError = "\(url.lastPathComponent) をコピーできません: \(error.localizedDescription)"
            }
        }
    }

    func add(kind: DictionaryEntry.Kind) {
        entries.append(DictionaryEntry(kind: kind, location: ""))
    }

    func remove(_ ids: Set<UUID>) {
        entries.removeAll { ids.contains($0.id) }
    }

    func move(_ ids: Set<UUID>, by offset: Int) {
        guard let index = entries.firstIndex(where: { ids.contains($0.id) }) else { return }
        let target = index + offset
        guard entries.indices.contains(target) else { return }
        entries.swapAt(index, target)
    }
}

struct DictionaryListView: View {
    @StateObject private var store = DictionaryStore()
    @State private var selection = Set<UUID>()
    @State private var dropTargeted = false

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text("辞書は上から順に検索されます。ファイルをここにドラッグ&ドロップすると "
                 + "~/Library/Application Support/NablaSKK/dictionaries にコピーして追加します。")
                .font(.callout)
                .foregroundColor(.secondary)

            List(selection: $selection) {
                ForEach($store.entries) { $entry in
                    DictionaryRow(entry: $entry)
                }
                .onMove { from, to in
                    store.entries.move(fromOffsets: from, toOffset: to)
                }
            }
            .overlay(
                RoundedRectangle(cornerRadius: 6)
                    .stroke(dropTargeted ? Color.accentColor : Color.clear, lineWidth: 2)
            )
            .onDrop(of: [.fileURL], isTargeted: $dropTargeted) { providers in
                loadDroppedFiles(providers)
                return true
            }

            HStack {
                Menu {
                    Button("辞書ファイルを選択...") { chooseFiles() }
                    Divider()
                    ForEach(DictionaryEntry.Kind.allCases) { kind in
                        Button(kind.label) { store.add(kind: kind) }
                    }
                } label: {
                    Image(systemName: "plus")
                }
                .menuStyle(.borderlessButton)
                .frame(width: 40)

                Button(action: { store.remove(selection); selection.removeAll() }) {
                    Image(systemName: "minus")
                }
                .disabled(selection.isEmpty)

                Button(action: { store.move(selection, by: -1) }) {
                    Image(systemName: "arrow.up")
                }
                .disabled(selection.count != 1)

                Button(action: { store.move(selection, by: 1) }) {
                    Image(systemName: "arrow.down")
                }
                .disabled(selection.count != 1)

                Spacer()

                if let error = store.saveError {
                    Text("保存できません: \(error)")
                        .foregroundColor(.red)
                        .font(.callout)
                } else {
                    Text(DictionaryConfig.fileURL.path)
                        .font(.caption)
                        .foregroundColor(.secondary)
                        .truncationMode(.middle)
                        .lineLimit(1)
                }
            }
        }
        .padding()
    }

    private func chooseFiles() {
        let panel = NSOpenPanel()
        panel.allowsMultipleSelection = true
        panel.canChooseDirectories = false
        panel.message = "SKK 辞書ファイルを選択してください"
        if panel.runModal() == .OK {
            store.add(fileURLs: panel.urls)
        }
    }

    private func loadDroppedFiles(_ providers: [NSItemProvider]) {
        for provider in providers {
            provider.loadItem(forTypeIdentifier: UTType.fileURL.identifier) { item, _ in
                guard let data = item as? Data,
                      let url = URL(dataRepresentation: data, relativeTo: nil)
                else { return }
                DispatchQueue.main.async { store.add(fileURLs: [url]) }
            }
        }
    }
}

struct DictionaryRow: View {
    @Binding var entry: DictionaryEntry

    var body: some View {
        HStack(spacing: 12) {
            Toggle("", isOn: $entry.enabled)
                .labelsHidden()
                .help("チェックを外すと検索対象から外れます")

            Picker("", selection: $entry.kind) {
                ForEach(DictionaryEntry.Kind.allCases) { kind in
                    Text(kind.label).tag(kind)
                }
            }
            .labelsHidden()
            .frame(width: 250)

            if entry.kind.needsLocation {
                TextField(entry.kind.locationHint, text: $entry.location)
                    .textFieldStyle(.roundedBorder)
                    .foregroundColor(locationExists ? .primary : .red)
                    .help(locationExists ? entry.location : "ファイルが見つかりません")
            } else {
                Text(entry.kind.locationHint)
                    .foregroundColor(.secondary)
                Spacer()
            }
        }
        .padding(.vertical, 2)
        .opacity(entry.enabled ? 1 : 0.5)
    }

    private var locationExists: Bool {
        switch entry.kind {
        case .common, .commonUTF8:
            return FileManager.default.fileExists(atPath: entry.location)
        default:
            return true
        }
    }
}
