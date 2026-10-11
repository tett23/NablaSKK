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
                Image(systemName: "magnifyingglass").foregroundColor(.prefSecondary)
                    .accessibilityHidden(true)
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
                            .font(.subheadline)
                            .foregroundColor(.prefSecondary)
                            .lineLimit(1)
                    }
                    Spacer()
                    if entry.okuriAri {
                        Text("送り")
                            .font(.subheadline)
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
                    .accessibilityLabel(PreferencesAccessibility.addEntry)
                Button(action: {
                    if let selection { store.remove([selection]) }
                    selection = nil
                }) { Image(systemName: "minus") }
                    .disabled(selection == nil)
                    .help("選択した見出し語を削除")
                    .accessibilityLabel(PreferencesAccessibility.removeEntry)
                Spacer()
                Text("\(store.allEntries.count) 語")
                    .font(.subheadline)
                    .foregroundColor(.prefSecondary)
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
                Image(systemName: "exclamationmark.triangle").font(.largeTitle).foregroundColor(.prefWarning)
                    .accessibilityHidden(true)
                Text(error).multilineTextAlignment(.center)
                Text(UserDictionary.fileURL.path).font(.subheadline).foregroundColor(.prefSecondary)
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
                    .foregroundColor(.prefSecondary)
                    .accessibilityHidden(true)
                Text("左の一覧から見出し語を選ぶか、＋で追加してください")
                    .foregroundColor(.prefSecondary)
                Text(UserDictionary.fileURL.path)
                    .font(.subheadline)
                    .foregroundColor(.prefSecondary)
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
                .font(.subheadline)
                .foregroundColor(.prefSecondary)
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
    /// At most one id; a Set to share the list row modifier
    @State private var selectedCandidates = Set<UUID>()
    private var selectedCandidate: UUID? { selectedCandidates.first }

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            HStack(alignment: .firstTextBaseline) {
                Text("読み").font(.headline)
                TextField("読み", text: $entry.reading)
                    .textFieldStyle(.roundedBorder)
                    .font(.title3)
                    .frame(maxWidth: 260)
                if entry.okuriAri {
                    Text("送りあり").font(.subheadline).foregroundColor(.prefSecondary)
                }
            }

            HStack {
                Text("候補").font(.headline)
                Text("上から順に表示されます。行の先頭の ≡ をドラッグするか、↑↓ ボタンで並べ替えられます。")
                    .font(.subheadline).foregroundColor(.prefSecondary)
                Spacer()
            }

            // A ScrollView, not a selectable List: in a List the first click
            // on a field selected the row instead (ADR 0006)
            ScrollView {
                VStack(spacing: 2) {
                    ForEach($entry.candidates) { $candidate in
                        HStack(spacing: 12) {
                            ReorderHandle(id: candidate.id)
                            TextField("単語", text: $candidate.word)
                                .textFieldStyle(.roundedBorder)
                                .frame(maxWidth: 220)
                            TextField("注釈(任意)", text: $candidate.annotation)
                                .textFieldStyle(.roundedBorder)
                                .foregroundColor(.prefSecondary)
                        }
                        .selectableRow(id: candidate.id, selection: $selectedCandidates,
                                       label: PreferencesAccessibility.selectCandidate(
                                           word: candidate.word,
                                           number: (entry.candidates.firstIndex { $0.id == candidate.id } ?? 0) + 1))
                        .onDrop(of: [.text], delegate: ReorderDropDelegate(target: candidate.id, items: $entry.candidates))
                    }
                }
                .padding(6)
                .frame(maxWidth: .infinity, alignment: .leading)
            }
            .background(Color(nsColor: .textBackgroundColor))
            .overlay(RoundedRectangle(cornerRadius: 6).stroke(Color(nsColor: .separatorColor)))

            HStack(spacing: 4) {
                Button(action: {
                    let candidate = UserCandidate(word: "")
                    entry.candidates.append(candidate)
                    selectedCandidates = [candidate.id]
                }) { Image(systemName: "plus") }
                    .help("候補を追加")
                    .accessibilityLabel(PreferencesAccessibility.addCandidate)
                Button(action: {
                    guard let id = selectedCandidate else { return }
                    entry.candidates.removeAll { $0.id == id }
                    entry.hints = entry.hints.compactMap { hint in
                        // Keep hints consistent with the candidate list
                        var hint = hint
                        hint.words = hint.words.filter { word in entry.candidates.contains { $0.word == word } }
                        return hint.words.isEmpty ? nil : hint
                    }
                    selectedCandidates = []
                }) { Image(systemName: "minus") }
                    .disabled(selectedCandidate == nil)
                    .help("選択した候補を削除")
                    .accessibilityLabel(PreferencesAccessibility.removeCandidate)
                Button(action: { move(by: -1) }) { Image(systemName: "arrow.up") }
                    .disabled(!canMove(by: -1))
                    .accessibilityLabel(PreferencesAccessibility.moveCandidateUp)
                Button(action: { move(by: 1) }) { Image(systemName: "arrow.down") }
                    .disabled(!canMove(by: 1))
                    .accessibilityLabel(PreferencesAccessibility.moveCandidateDown)
                Spacer()
            }
            .buttonStyle(.borderless)

            if !entry.hints.isEmpty {
                VStack(alignment: .leading, spacing: 4) {
                    Text("送り仮名ごとの候補").font(.headline)
                    ForEach(entry.hints, id: \.okuri) { hint in
                        HStack {
                            Text(hint.okuri).font(.body.monospaced())
                            Text(hint.words.joined(separator: "、")).foregroundColor(.prefSecondary)
                        }
                    }
                    Text("変換時に学習された送り仮名との組み合わせです。候補を削除すると対応する項目も外れます。")
                        .font(.subheadline)
                        .foregroundColor(.prefSecondary)
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
