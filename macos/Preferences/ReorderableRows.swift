// NablaSKK Preferences: drag reordering for the hand-built lists
// (ADR 0006). The lists are ScrollViews rather than selectable Lists, so
// a text field takes the first click; reordering is dragged from a handle.
//
// Written by tett23, 2026.
// License: GPL-2.0-or-later. See the LICENSE file for details.

import SwiftUI
import UniformTypeIdentifiers

/// The grip at the start of a row; dragging it carries the row's id.
/// Hidden from VoiceOver: the ↑↓ buttons reorder without dragging.
struct ReorderHandle: View {
    let id: UUID

    var body: some View {
        Image(systemName: "line.3.horizontal")
            .foregroundColor(.prefSecondary)
            .frame(width: 16, height: 22)
            .contentShape(Rectangle())
            .onDrag { NSItemProvider(object: id.uuidString as NSString) }
            .help("ドラッグして並べ替え")
            .accessibilityHidden(true)
    }
}

/// Drop target on a row: a dragged row id moves to this row's position.
/// Only text (the ids) is accepted, so dropped files fall through to the
/// list's own drop target.
struct ReorderDropDelegate<Item: Identifiable>: DropDelegate where Item.ID == UUID {
    let target: UUID
    @Binding var items: [Item]

    func validateDrop(info: DropInfo) -> Bool {
        info.hasItemsConforming(to: [.text])
    }

    func dropUpdated(info: DropInfo) -> DropProposal? {
        DropProposal(operation: .move)
    }

    func performDrop(info: DropInfo) -> Bool {
        guard let provider = info.itemProviders(for: [.text]).first else { return false }
        provider.loadObject(ofClass: NSString.self) { object, _ in
            guard let text = object as? String, let dragged = UUID(uuidString: text) else { return }
            DispatchQueue.main.async {
                guard let from = items.firstIndex(where: { $0.id == dragged }),
                      let to = items.firstIndex(where: { $0.id == target }) else { return }
                items = ListReorder.moved(items, from: from, to: to)
            }
        }
        return true
    }
}

/// Row chrome for the hand-built lists: click the row's empty space or
/// its plain text to select it; the selected row is tinted with the
/// accent colour and marked selected for VoiceOver, which can also select
/// it with the "この行を選択" action.
///
/// The selection is read through the binding rather than taken as an
/// `isSelected` Bool next to an action closure: with the closure SwiftUI
/// skipped re-rendering the rows and the tint stayed on the old row (the
/// same cause as ADR 0005's sidebar). The click is caught by a button
/// behind the row (plain text in the row passes clicks through with
/// `allowsHitTesting(false)`), not a tap gesture: buttons also respond to
/// the clicks the UI tests send, tap gestures do not.
struct SelectableRow: ViewModifier {
    let id: UUID
    @Binding var selection: Set<UUID>

    func body(content: Content) -> some View {
        let isSelected = selection.contains(id)
        return content
            .padding(.vertical, 2)
            .padding(.horizontal, 6)
            .background(
                Button(action: { selection = [id] }) {
                    RoundedRectangle(cornerRadius: 6)
                        .fill(isSelected ? Color.prefSelectedRow : Color.clear)
                        .contentShape(Rectangle())
                }
                .buttonStyle(.plain)
                .accessibilityHidden(true)
            )
            .accessibilityElement(children: .contain)
            .accessibilityAddTraits(isSelected ? .isSelected : [])
            .accessibilityAction(named: Text("この行を選択")) { selection = [id] }
    }
}

extension View {
    func selectableRow(id: UUID, selection: Binding<Set<UUID>>) -> some View {
        modifier(SelectableRow(id: id, selection: selection))
    }
}
