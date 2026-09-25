// NablaSKK Preferences: settings window with a sidebar of panes.
//
// Written by tett23, 2026.
// License: GPL-2.0-or-later. See the LICENSE file for details.

import SwiftUI
import UniformTypeIdentifiers

/// One page of the settings window. Add a case (and its view in
/// `RootView.detail`) to add a page to the sidebar.
enum SettingsPane: String, CaseIterable, Identifiable {
    case dictionaries
    case userDictionary
    case skkserv
    case input
    case keys

    var id: String { rawValue }

    var title: String {
        switch self {
        case .dictionaries: return "辞書"
        case .userDictionary: return "ユーザー辞書"
        case .skkserv: return "skkserv"
        case .input: return "入力"
        case .keys: return "キー"
        }
    }

    var systemImage: String {
        switch self {
        case .dictionaries: return "books.vertical"
        case .userDictionary: return "person.text.rectangle"
        case .skkserv: return "network"
        case .input: return "keyboard"
        case .keys: return "command"
        }
    }
}

/// Sidebar on the left, the selected pane on the right. The sidebar is
/// a column of buttons rather than a List with selection: List selection
/// did not switch the pane here, and NavigationSplitView needs macOS 13.
struct RootView: View {
    static let sidebarWidth: CGFloat = 180

    /// Remembered across launches so the window reopens where it was left.
    @AppStorage("selectedSettingsPane") private var selectedPane = SettingsPane.dictionaries.rawValue

    private var pane: SettingsPane { SettingsPane(rawValue: selectedPane) ?? .dictionaries }

    var body: some View {
        HStack(spacing: 0) {
            VStack(alignment: .leading, spacing: 2) {
                ForEach(SettingsPane.allCases) { item in
                    SidebarItem(pane: item, isSelected: item == pane) {
                        selectedPane = item.rawValue
                    }
                }
                Spacer()
            }
            .padding(.horizontal, 10)
            .padding(.vertical, 12)
            .frame(width: Self.sidebarWidth)
            .frame(maxHeight: .infinity)
            .background(SidebarBackground())

            Divider()

            // minWidth 0: a pane with long unwrapped text must not push
            // the whole row wider than the window (which shifted the
            // sidebar off its left edge)
            detail
                .id(pane)
                .frame(minWidth: 0, maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
                .clipped()
        }
        .frame(minWidth: Self.sidebarWidth + 720, minHeight: 440)
    }

    @ViewBuilder
    private var detail: some View {
        switch pane {
        case .dictionaries: DictionaryListView()
        case .userDictionary: UserDictionaryView()
        case .skkserv: SkkservSettingsView()
        case .input: InputSettingsView()
        case .keys: KeymapSettingsView()
        }
    }
}

/// The translucent sidebar material Finder and System Settings use; it
/// follows light / dark mode on its own.
struct SidebarBackground: NSViewRepresentable {
    func makeNSView(context: Context) -> NSVisualEffectView {
        let view = NSVisualEffectView()
        view.material = .sidebar
        view.blendingMode = .behindWindow
        view.state = .followsWindowActiveState
        return view
    }

    func updateNSView(_ nsView: NSVisualEffectView, context: Context) {}
}

/// One sidebar row; the selected row gets an accent-coloured background.
struct SidebarItem: View {
    let pane: SettingsPane
    let isSelected: Bool
    let action: () -> Void

    @State private var hovering = false

    var body: some View {
        Button(action: action) {
            HStack(spacing: 8) {
                Image(systemName: pane.systemImage)
                    .frame(width: 20)
                Text(pane.title)
                Spacer(minLength: 0)
            }
            .padding(.horizontal, 8)
            .padding(.vertical, 6)
            .foregroundColor(isSelected ? .white : .primary)
            .background(
                RoundedRectangle(cornerRadius: 6)
                    .fill(isSelected ? Color.accentColor
                          : hovering ? Color.primary.opacity(0.08) : Color.clear)
            )
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .onHover { hovering = $0 }
        .accessibilityAddTraits(isSelected ? .isSelected : [])
    }
}

/// AppKit lifecycle around a single preferences window. The SwiftUI `App`
/// lifecycle dropped the window once it went behind other apps: clicking
/// the Dock icon (or the input method asking to open the app again)
/// activated the process without showing anything. Owning the window
/// here keeps it alive, and every activation path orders it front.
@main
final class PreferencesAppDelegate: NSObject, NSApplicationDelegate {
    private var window: NSWindow?

    static func main() {
        let app = NSApplication.shared
        let delegate = PreferencesAppDelegate()
        app.delegate = delegate
        app.run()
    }

    func applicationDidFinishLaunching(_ notification: Notification) {
        NSApp.setActivationPolicy(.regular)
        NSApp.mainMenu = Self.makeMainMenu()
        showWindow()
    }

    /// Dock icon click, or `open` of a running app
    func applicationShouldHandleReopen(_ sender: NSApplication, hasVisibleWindows: Bool) -> Bool {
        showWindow()
        return false
    }

    /// Activation from the input method's "NablaSKK の設定..." item
    func applicationDidBecomeActive(_ notification: Notification) {
        showWindow()
    }

    func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool {
        true
    }

    private func showWindow() {
        if window == nil {
            let window = NSWindow(
                contentRect: NSRect(x: 0, y: 0, width: 920, height: 520),
                styleMask: [.titled, .closable, .miniaturizable, .resizable],
                backing: .buffered,
                defer: false
            )
            window.title = "NablaSKK の設定"
            window.contentView = NSHostingView(rootView: RootView())
            window.isReleasedWhenClosed = false
            window.setFrameAutosaveName("NablaSKKPreferences")
            window.center()
            self.window = window
        }

        window?.makeKeyAndOrderFront(nil)
        NSApp.activate(ignoringOtherApps: true)

        if ProcessInfo.processInfo.environment["NABLASKK_PREFS_DEBUG"] != nil, let window {
            NSLog("showWindow: visible=%d key=%d frontmostApp=%d",
                  window.isVisible ? 1 : 0, window.isKeyWindow ? 1 : 0, NSApp.isActive ? 1 : 0)
        }
    }

    /// The standard menus a swiftc-built app does not get for free; the
    /// Edit menu is what makes Cmd-C/V/X and Cmd-Z work in text fields.
    private static func makeMainMenu() -> NSMenu {
        let mainMenu = NSMenu()

        let appMenu = NSMenu()
        appMenu.addItem(withTitle: "NablaSKK の設定を隠す", action: #selector(NSApplication.hide(_:)), keyEquivalent: "h")
        appMenu.addItem(.separator())
        appMenu.addItem(withTitle: "NablaSKK の設定を終了", action: #selector(NSApplication.terminate(_:)), keyEquivalent: "q")
        let appItem = NSMenuItem()
        appItem.submenu = appMenu
        mainMenu.addItem(appItem)

        let editMenu = NSMenu(title: "編集")
        editMenu.addItem(withTitle: "取り消す", action: Selector(("undo:")), keyEquivalent: "z")
        editMenu.addItem(withTitle: "やり直す", action: Selector(("redo:")), keyEquivalent: "Z")
        editMenu.addItem(.separator())
        editMenu.addItem(withTitle: "カット", action: #selector(NSText.cut(_:)), keyEquivalent: "x")
        editMenu.addItem(withTitle: "コピー", action: #selector(NSText.copy(_:)), keyEquivalent: "c")
        editMenu.addItem(withTitle: "ペースト", action: #selector(NSText.paste(_:)), keyEquivalent: "v")
        editMenu.addItem(withTitle: "すべてを選択", action: #selector(NSText.selectAll(_:)), keyEquivalent: "a")
        let editItem = NSMenuItem()
        editItem.submenu = editMenu
        mainMenu.addItem(editItem)

        let windowMenu = NSMenu(title: "ウインドウ")
        windowMenu.addItem(withTitle: "しまう", action: #selector(NSWindow.miniaturize(_:)), keyEquivalent: "m")
        windowMenu.addItem(withTitle: "閉じる", action: #selector(NSWindow.performClose(_:)), keyEquivalent: "w")
        let windowItem = NSMenuItem()
        windowItem.submenu = windowMenu
        mainMenu.addItem(windowItem)
        NSApp.windowsMenu = windowMenu

        return mainMenu
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

    /// Dropped or chosen files are copied into the support directory. A file
    /// with the same name overwrites the earlier copy, and the entry that
    /// already points at that copy is kept (and re-enabled) instead of
    /// being listed twice.
    func add(fileURLs: [URL]) {
        for url in fileURLs where url.isFileURL {
            do {
                let path = try DictionaryConfig.importDictionary(at: url)
                if let index = entries.firstIndex(where: { $0.kind.isFileBased && $0.location == path }) {
                    entries[index].enabled = true
                } else {
                    entries.append(DictionaryEntry(kind: .common, location: path))
                }
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

            HStack(spacing: 12) {
                Text("").frame(width: DictionaryRow.toggleWidth)
                Text("名前").frame(width: DictionaryRow.nameWidth, alignment: .leading)
                Text("種類").frame(width: DictionaryRow.kindWidth, alignment: .leading)
                Text("場所")
                Spacer()
            }
            .font(.caption)
            .foregroundColor(.secondary)
            .padding(.horizontal, 8)

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
                    ForEach(DictionaryEntry.Kind.addable) { kind in
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
    static let toggleWidth: CGFloat = 20
    static let nameWidth: CGFloat = 160
    static let kindWidth: CGFloat = 250

    @Binding var entry: DictionaryEntry

    var body: some View {
        HStack(spacing: 12) {
            Toggle("", isOn: $entry.enabled)
                .labelsHidden()
                .frame(width: Self.toggleWidth)
                .help("チェックを外すと検索対象から外れます")

            Text(entry.name)
                .frame(width: Self.nameWidth, alignment: .leading)
                .lineLimit(1)
                .truncationMode(.middle)
                .help(entry.name)

            Picker("", selection: $entry.kind) {
                ForEach(DictionaryEntry.Kind.allCases) { kind in
                    Text(kind.label).tag(kind)
                }
            }
            .labelsHidden()
            .frame(width: Self.kindWidth)

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
