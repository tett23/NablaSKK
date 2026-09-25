// NablaSKK Preferences: user dictionary editor.
//
// Written by tett23, 2026.
// License: GPL-2.0-or-later. See the LICENSE file for details.

import SwiftUI

/// Loads and edits the user dictionary. Edits are written to disk after a
/// short debounce; the input method re-reads the file when it next
/// becomes active.
final class UserDictionaryStore: ObservableObject {
    @Published var dictionary = UserDictionary()
    @Published var loadError: String?
    @Published var saveError: String?

    private var saveTimer: Timer?
    private var loadedDate: Date?

    init() {
        reload()
    }

    func reload() {
        do {
            dictionary = try UserDictionary.load()
            loadedDate = modificationDate
            loadError = nil
        } catch {
            loadError = error.localizedDescription
        }
    }

    /// Re-read when another program (the input method) wrote the file.
    func reloadIfChangedOnDisk() {
        guard saveTimer == nil, modificationDate != loadedDate else { return }
        reload()
    }

    private var modificationDate: Date? {
        (try? FileManager.default.attributesOfItem(atPath: UserDictionary.fileURL.path))?[.modificationDate] as? Date
    }

    /// Switching panes in the sidebar discards this store; flush an edit
    /// still waiting for its debounced save so it is not lost.
    deinit {
        if saveTimer != nil { saveNow() }
    }

    func scheduleSave() {
        saveTimer?.invalidate()
        saveTimer = Timer.scheduledTimer(withTimeInterval: 0.5, repeats: false) { [weak self] _ in
            self?.saveNow()
        }
    }

    func saveNow() {
        saveTimer?.invalidate()
        saveTimer = nil
        do {
            try dictionary.save()
            loadedDate = modificationDate
            saveError = nil
        } catch {
            saveError = error.localizedDescription
        }
    }

    // MARK: entries

    var allEntries: [UserDictionaryEntry] { dictionary.okuriNasi + dictionary.okuriAri }

    func entry(_ id: UUID) -> UserDictionaryEntry? {
        allEntries.first { $0.id == id }
    }

    func update(_ entry: UserDictionaryEntry) {
        if let index = dictionary.okuriNasi.firstIndex(where: { $0.id == entry.id }) {
            dictionary.okuriNasi[index] = entry
        } else if let index = dictionary.okuriAri.firstIndex(where: { $0.id == entry.id }) {
            dictionary.okuriAri[index] = entry
        }
        scheduleSave()
    }

    /// New entries go to the front, like freshly learned words.
    @discardableResult
    func addEntry(reading: String) -> UserDictionaryEntry {
        let okuriAri = UserDictionaryEntry.isOkuriAri(reading: reading)
        let entry = UserDictionaryEntry(reading: reading, okuriAri: okuriAri, candidates: [])
        if okuriAri { dictionary.okuriAri.insert(entry, at: 0) } else { dictionary.okuriNasi.insert(entry, at: 0) }
        scheduleSave()
        return entry
    }

    func remove(_ ids: Set<UUID>) {
        dictionary.okuriNasi.removeAll { ids.contains($0.id) }
        dictionary.okuriAri.removeAll { ids.contains($0.id) }
        scheduleSave()
    }

}

struct UserDictionaryView: View {
    @StateObject private var store = UserDictionaryStore()
    @State private var selection: UUID?
    @State private var search = ""
    @State private var showingAdd = false
    @State private var newReading = ""

    private var filtered: [UserDictionaryEntry] {
        let query = search.trimmingCharacters(in: .whitespaces)
        guard !query.isEmpty else { return store.allEntries }
        return store.allEntries.filter { entry in
            entry.reading.contains(query) || entry.candidates.contains { $0.word.contains(query) }
        }
    }

    var body: some View {
        HSplitView {
            entryList
                .frame(minWidth: 240, idealWidth: 280)

            detail
                .frame(minWidth: 360)
        }
        .onReceive(NotificationCenter.default.publisher(for: NSApplication.didBecomeActiveNotification)) { _ in
            store.reloadIfChangedOnDisk()
        }
        .onReceive(NotificationCenter.default.publisher(for: NSApplication.willResignActiveNotification)) { _ in
            store.saveNow()
        }
        .sheet(isPresented: $showingAdd) { addSheet }
    }

    // MARK: left: searchable entry list

    private var entryList: some View {
        VStack(spacing: 0) {
            HStack {
                Image(systemName: "magnifyingglass").foregroundColor(.secondary)
                TextField("読み・単語で検索", text: $search)
                    .textFieldStyle(.plain)
            }
            .padding(8)

            Divider()

            List(filtered, selection: $selection) { entry in
                HStack {
                    VStack(alignment: .leading, spacing: 2) {
                        Text(entry.reading)
                            .font(.body.weight(.medium))
                        Text(entry.candidates.map(\.word).joined(separator: "、"))
                            .font(.caption)
                            .foregroundColor(.secondary)
                            .lineLimit(1)
                    }
                    Spacer()
                    if entry.okuriAri {
                        Text("送り")
                            .font(.caption2)
                            .padding(.horizontal, 5).padding(.vertical, 1)
                            .background(Color.accentColor.opacity(0.15))
                            .cornerRadius(4)
                    }
                }
                .padding(.vertical, 2)
                .tag(entry.id)
            }
            .listStyle(.inset)

            Divider()

            HStack(spacing: 4) {
                Button(action: { newReading = ""; showingAdd = true }) { Image(systemName: "plus") }
                    .help("見出し語を追加")
                Button(action: {
                    if let selection { store.remove([selection]) }
                    selection = nil
                }) { Image(systemName: "minus") }
                    .disabled(selection == nil)
                    .help("選択した見出し語を削除")
                Spacer()
                Text("\(store.allEntries.count) 語")
                    .font(.caption)
                    .foregroundColor(.secondary)
            }
            .buttonStyle(.borderless)
            .padding(8)
        }
    }

    // MARK: right: entry detail

    @ViewBuilder
    private var detail: some View {
        if let error = store.loadError ?? store.saveError {
            VStack(spacing: 8) {
                Image(systemName: "exclamationmark.triangle").font(.largeTitle).foregroundColor(.orange)
                Text(error).multilineTextAlignment(.center)
                Text(UserDictionary.fileURL.path).font(.caption).foregroundColor(.secondary)
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity)
            .padding()
        } else if let selection, let entry = store.entry(selection) {
            EntryDetailView(
                entry: Binding(get: { store.entry(selection) ?? entry },
                               set: { store.update($0) })
            )
        } else {
            VStack(spacing: 8) {
                Image(systemName: "character.book.closed")
                    .font(.system(size: 40))
                    .foregroundColor(.secondary)
                Text("左の一覧から見出し語を選ぶか、＋で追加してください")
                    .foregroundColor(.secondary)
                Text(UserDictionary.fileURL.path)
                    .font(.caption)
                    .foregroundColor(.secondary)
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity)
            .padding()
        }
    }

    // MARK: add sheet

    private var addSheet: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text("見出し語を追加").font(.headline)
            TextField("読み(例: かんじ / 送りありは おくr)", text: $newReading)
                .textFieldStyle(.roundedBorder)
                .frame(width: 320)
            Text(UserDictionaryEntry.isOkuriAri(reading: newReading) ? "送りあり見出し語として追加します" : "送りなし見出し語として追加します")
                .font(.caption)
                .foregroundColor(.secondary)
            HStack {
                Spacer()
                Button("キャンセル") { showingAdd = false }
                    .keyboardShortcut(.cancelAction)
                Button("追加") {
                    let reading = newReading.trimmingCharacters(in: .whitespaces)
                    guard !reading.isEmpty else { return }
                    let entry = store.addEntry(reading: reading)
                    search = ""
                    selection = entry.id
                    showingAdd = false
                }
                .keyboardShortcut(.defaultAction)
                .disabled(newReading.trimmingCharacters(in: .whitespaces).isEmpty)
            }
        }
        .padding()
    }
}

/// Editor for one entry: its reading and candidate list.
struct EntryDetailView: View {
    @Binding var entry: UserDictionaryEntry
    @State private var selectedCandidate: UUID?

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            HStack(alignment: .firstTextBaseline) {
                Text("読み").font(.headline)
                TextField("読み", text: $entry.reading)
                    .textFieldStyle(.roundedBorder)
                    .font(.title3)
                    .frame(maxWidth: 260)
                if entry.okuriAri {
                    Text("送りあり").font(.caption).foregroundColor(.secondary)
                }
            }

            HStack {
                Text("候補").font(.headline)
                Text("上から順に表示されます").font(.caption).foregroundColor(.secondary)
                Spacer()
            }

            List(selection: $selectedCandidate) {
                ForEach($entry.candidates) { $candidate in
                    HStack(spacing: 12) {
                        TextField("単語", text: $candidate.word)
                            .textFieldStyle(.roundedBorder)
                            .frame(maxWidth: 220)
                        TextField("注釈(任意)", text: $candidate.annotation)
                            .textFieldStyle(.roundedBorder)
                            .foregroundColor(.secondary)
                    }
                    .padding(.vertical, 2)
                    .tag(candidate.id)
                }
                .onMove { from, to in entry.candidates.move(fromOffsets: from, toOffset: to) }
            }
            .listStyle(.bordered(alternatesRowBackgrounds: true))

            HStack(spacing: 4) {
                Button(action: {
                    let candidate = UserCandidate(word: "")
                    entry.candidates.append(candidate)
                    selectedCandidate = candidate.id
                }) { Image(systemName: "plus") }
                    .help("候補を追加")
                Button(action: {
                    guard let id = selectedCandidate else { return }
                    entry.candidates.removeAll { $0.id == id }
                    entry.hints = entry.hints.compactMap { hint in
                        // Keep hints consistent with the candidate list
                        var hint = hint
                        hint.words = hint.words.filter { word in entry.candidates.contains { $0.word == word } }
                        return hint.words.isEmpty ? nil : hint
                    }
                    selectedCandidate = nil
                }) { Image(systemName: "minus") }
                    .disabled(selectedCandidate == nil)
                    .help("選択した候補を削除")
                Button(action: { move(by: -1) }) { Image(systemName: "arrow.up") }
                    .disabled(!canMove(by: -1))
                Button(action: { move(by: 1) }) { Image(systemName: "arrow.down") }
                    .disabled(!canMove(by: 1))
                Spacer()
            }
            .buttonStyle(.borderless)

            if !entry.hints.isEmpty {
                VStack(alignment: .leading, spacing: 4) {
                    Text("送り仮名ごとの候補").font(.headline)
                    ForEach(entry.hints, id: \.okuri) { hint in
                        HStack {
                            Text(hint.okuri).font(.body.monospaced())
                            Text(hint.words.joined(separator: "、")).foregroundColor(.secondary)
                        }
                    }
                    Text("変換時に学習された送り仮名との組み合わせです。候補を削除すると対応する項目も外れます。")
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
            }
        }
        .padding()
    }

    private func canMove(by offset: Int) -> Bool {
        guard let id = selectedCandidate,
              let index = entry.candidates.firstIndex(where: { $0.id == id }) else { return false }
        return entry.candidates.indices.contains(index + offset)
    }

    private func move(by offset: Int) {
        guard let id = selectedCandidate,
              let index = entry.candidates.firstIndex(where: { $0.id == id }) else { return }
        entry.candidates.swapAt(index, index + offset)
    }
}
