// NablaSKK Preferences: user romaji-kana rules pane.
//
// Written by tett23, 2026.
// License: GPL-2.0-or-later. See the LICENSE file for details.

import SwiftUI

final class KanaRuleStore: ObservableObject {
    /// Every row on screen, including ones with problems. Only the usable
    /// ones are written to kana-rule.conf.
    @Published var settings: KanaRuleSettings {
        didSet { if settings != oldValue { save() } }
    }
    @Published var saveError: String?

    /// The built-in table (data/kana-rule.utf8.conf).
    private let baseRules: [KanaRule]
    /// The built-in table with the selected sub-rules (AZIK, ACT,
    /// yakumono-ippai, old-kana) layered on, as the engine has it: what
    /// user rules are checked against (ADR 0003).
    @Published private(set) var builtinRules: [KanaRule]
    /// Names the layered table in conflict messages.
    @Published private(set) var builtinOrigin = "組み込み"

    /// Symbol rules (z. → …, [ → 「 ...) listed read-only below the
    /// user's rules; a → あ and the like are obvious and left out.
    var visibleBuiltinRules: [KanaRule] { builtinRules.filter(\.isSymbolRule) }

    init(inputSettings: InputSettings) {
        settings = KanaRuleSettings.load()
        let url = Bundle.main.url(forResource: "kana-rule.utf8", withExtension: "conf")
        baseRules = url.flatMap { try? String(contentsOf: $0, encoding: .utf8) }
            .map(KanaRuleSettings.builtinRules(from:)) ?? []
        builtinRules = baseRules
        useSubRules(of: inputSettings)
    }

    /// Re-layer the table when the input style or extras change. The
    /// saved file is left alone until the next edit.
    func useSubRules(of inputSettings: InputSettings) {
        let directory = Bundle.main.resourceURL ?? Bundle.main.bundleURL
        builtinRules = SubRules.layeredRules(base: baseRules, settings: inputSettings, in: directory)
        builtinOrigin = SubRules.origin(for: inputSettings)
    }

    private func save() {
        do {
            try KanaRuleSettings(rules: settings.acceptedRules(builtin: builtinRules)).save()
            saveError = nil
        } catch {
            saveError = error.localizedDescription
        }
    }

    @discardableResult
    func add() -> UUID {
        let rule = KanaRule(input: "", output: "")
        settings.rules.append(rule)
        return rule.id
    }

    func remove(_ id: UUID) {
        settings.rules.removeAll { $0.id == id }
    }

    /// Why a row is not saved, and which of its fields is at fault.
    func issues() -> [UUID: KanaRuleIssue] {
        var issues = settings.conflicts(builtin: builtinRules, builtinOrigin: builtinOrigin)
            .mapValues { KanaRuleIssue(message: $0, field: .input) }
        for rule in settings.rules {
            if let message = rule.problemMessage, !(rule.input.isEmpty && rule.output.isEmpty) {
                issues[rule.id] = KanaRuleIssue(message: message, field: rule.problem == .emptyOutput ? .output : .input)
            }
        }
        return issues
    }
}

struct KanaRuleIssue {
    enum Field { case input, output }
    let message: String
    let field: Field
}

/// Rows live in a plain ScrollView rather than a List: in a List with
/// selection the table took the click and the text fields never got focus.
struct KanaRuleSettingsView: View {
    /// settings.conf: the input style and the extra sub-rules live there
    @StateObject private var inputStore = InputSettingsStore()
    @StateObject private var store = KanaRuleStore(inputSettings: InputSettings.load())
    @FocusState private var focusedRule: UUID?

    static let inputWidth: CGFloat = 120
    static let outputWidth: CGFloat = 160

    var body: some View {
        let issues = store.issues()

        VStack(alignment: .leading, spacing: 8) {
            subRulesSection

            Text("かなモードで入力キーを続けて打つと、出力の文字になります(組み込みの「z.」→「…」と同じ仕組みです)。"
                 + "例えば入力キーを「fj」、出力を「、」にすると、fj で読点を打てます。"
                 + "カタカナ・半角カナモードでは出力をカタカナ・半角に変えて使います。")
                .font(.callout)
                .foregroundColor(.prefSecondary)
                .fixedSize(horizontal: false, vertical: true)

            ScrollView {
                VStack(alignment: .leading, spacing: 6) {
                    sectionHeader("追加したルール")
                    if store.settings.rules.isEmpty {
                        Text("まだありません。下の + で追加できます。")
                            .foregroundColor(.prefSecondary)
                            .padding(.vertical, 4)
                    }
                    ForEach($store.settings.rules) { $rule in
                        KanaRuleRow(rule: $rule, number: (store.settings.rules.firstIndex { $0.id == rule.id } ?? 0) + 1,
                                    issue: issues[rule.id], focus: $focusedRule) {
                            store.remove(rule.id)
                        }
                    }

                    Divider().padding(.vertical, 8)

                    sectionHeader(builtinHeader)
                    ForEach(store.visibleBuiltinRules) { rule in
                        BuiltinKanaRuleRow(rule: rule)
                    }
                }
                .padding(.horizontal, 4)
                .padding(.vertical, 8)
                .frame(maxWidth: .infinity, alignment: .leading)
            }
            .background(Color(nsColor: .textBackgroundColor))
            .overlay(RoundedRectangle(cornerRadius: 4).stroke(Color(nsColor: .separatorColor)))

            HStack {
                Button(action: { focusedRule = store.add() }) {
                    Label("ルールを追加", systemImage: "plus")
                }
                Spacer()
                if let error = store.saveError {
                    Text("保存できません: \(error)").foregroundColor(.prefError).font(.callout)
                } else {
                    Text("赤い注意が出ている行は保存されません。変更は次にどこかのアプリで入力を始めたときに反映されます。")
                        .font(.subheadline)
                        .foregroundColor(.prefSecondary)
                }
            }
        }
        .padding()
    }

    /// AquaSKK's sub-rules (ADR 0003): one input style, plus symbols and
    /// old kana that combine with any style.
    private var subRulesSection: some View {
        let settings = inputStore.settings
        return VStack(alignment: .leading, spacing: 6) {
            Picker("入力方式", selection: $inputStore.settings.inputStyle) {
                ForEach(InputSettings.InputStyle.allCases) { style in
                    Text(style.label).tag(style)
                }
            }
            .pickerStyle(.radioGroup)
            .horizontalRadioGroupLayout()
            Text(settings.inputStyle.summary)
                .font(.subheadline)
                .foregroundColor(.prefSecondary)
            HStack(spacing: 16) {
                Toggle("記号入力(SKK日本語入力FEP/Egg 互換。z1 → ○ など)", isOn: $inputStore.settings.yakumonoIppai)
                Toggle("旧かな(ゐ・ゑ・くゎ・ぐゎ)", isOn: $inputStore.settings.oldKana)
            }
            if settings.inputStyle == .act && (settings.yakumonoIppai || settings.oldKana) {
                Text("ACT と併用すると、一部のキーは記号入力・旧かなのほうになります(例: z; は ACT の「ざん」ではなく「゛」、wi は「ゐ」)。")
                    .font(.subheadline)
                    .foregroundColor(.prefWarning)
                    .fixedSize(horizontal: false, vertical: true)
            }
            if settings.inputStyle != .standard {
                Text("ひらがな⇔カタカナなどのキーも入力方式に合わせて変わりますが、「キー」ページの既定の表示は標準のままです。")
                    .font(.subheadline)
                    .foregroundColor(.prefSecondary)
                    .fixedSize(horizontal: false, vertical: true)
            }
            if let error = inputStore.saveError {
                Text("保存できません: \(error)").foregroundColor(.prefError).font(.callout)
            }
        }
        .onChange(of: inputStore.settings) { store.useSubRules(of: $0) }
    }

    private var builtinHeader: String {
        "\(SubRules.origin(for: inputStore.settings))の記号のルール(変更・削除はできません)"
    }

    private func sectionHeader(_ title: String) -> some View {
        HStack(spacing: 12) {
            Text(title).font(.headline)
            Spacer()
        }
        .padding(.bottom, 2)
    }
}

/// A problem row gets a red outline on the field at fault (the usual
/// macOS cue for an invalid field) and a faint red background, so it
/// stands out in a long list.
struct KanaRuleRow: View {
    @Binding var rule: KanaRule
    /// 1-based position, for VoiceOver names
    let number: Int
    let issue: KanaRuleIssue?
    var focus: FocusState<UUID?>.Binding
    let onDelete: () -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 2) {
            HStack(spacing: 12) {
                TextField("fj", text: $rule.input)
                    .textFieldStyle(.roundedBorder)
                    .font(.system(.body, design: .monospaced))
                    .frame(width: KanaRuleSettingsView.inputWidth)
                    .overlay(errorOutline(issue?.field == .input))
                    .focused(focus, equals: rule.id)
                    .accessibilityLabel(PreferencesAccessibility.ruleInput(number))
                Image(systemName: "arrow.right")
                    .foregroundColor(.prefSecondary)
                    .frame(width: 14)
                    .accessibilityHidden(true)
                TextField("、", text: $rule.output)
                    .textFieldStyle(.roundedBorder)
                    .frame(width: KanaRuleSettingsView.outputWidth)
                    .overlay(errorOutline(issue?.field == .output))
                    .accessibilityLabel(PreferencesAccessibility.ruleOutput(number))
                Button(action: onDelete) {
                    Image(systemName: "trash")
                }
                .buttonStyle(.borderless)
                .help("このルールを削除")
                .accessibilityLabel(PreferencesAccessibility.deleteRule(input: rule.input, output: rule.output))
                Spacer(minLength: 0)
            }
            if let issue {
                Text("\(issue.message)(保存されません)").font(.subheadline).foregroundColor(.prefError)
            }
        }
        .padding(.vertical, 4)
        .padding(.horizontal, 6)
        .background(
            RoundedRectangle(cornerRadius: 6)
                .fill(issue == nil ? Color.clear : Color.prefErrorRow)
        )
    }

    @ViewBuilder
    private func errorOutline(_ show: Bool) -> some View {
        if show {
            RoundedRectangle(cornerRadius: 5).stroke(Color.prefError, lineWidth: 1.5)
        }
    }
}

/// A built-in rule: same layout as an editable row, greyed out.
struct BuiltinKanaRuleRow: View {
    let rule: KanaRule

    /// Spaces would read as an empty field ("z " → full-width space).
    static func visible(_ text: String) -> String {
        text.replacingOccurrences(of: " ", with: "␣")
            .replacingOccurrences(of: "\u{3000}", with: "(全角スペース)")
    }

    /// Plain text, not disabled fields: the content is meant to be read
    /// (disabled fields fell to 1.8:1 contrast), read as one sentence
    /// ("z. を打つと …") by VoiceOver (ADR 0006).
    var body: some View {
        HStack(spacing: 12) {
            Text(Self.visible(rule.input))
                .font(.system(.body, design: .monospaced))
                .frame(width: KanaRuleSettingsView.inputWidth, alignment: .leading)
                .padding(.leading, 7)
            Image(systemName: "arrow.right")
                .foregroundColor(.prefSecondary)
                .frame(width: 14)
            Text(Self.visible(rule.output))
                .frame(width: KanaRuleSettingsView.outputWidth, alignment: .leading)
                .padding(.leading, 7)
            Spacer(minLength: 0)
        }
        .padding(.vertical, 3)
        .padding(.horizontal, 6)
        .accessibilityElement(children: .ignore)
        .accessibilityLabel(PreferencesAccessibility.builtinRule(input: Self.visible(rule.input), output: Self.visible(rule.output)))
    }
}
