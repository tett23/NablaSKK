// NablaSKK: annotation window, after AquaSKK's AnnotationWindow /
// MacAnnotator (platform/mac/src/gui, src/server): the dictionary's
// annotation of the candidate being selected ("悪;わるし（形）"), in a small
// box under the caret.
//
// Written by tett23, 2026.
// License: GPL-2.0-or-later. See the LICENSE file for details.

import Cocoa
import InputMethodKit

final class AnnotationWindow {
    static let shared = AnnotationWindow()

    private let panel: NSPanel
    private let label = NSTextField(wrappingLabelWithString: "")
    private var shown: String?

    static let maxWidth: CGFloat = 360
    static let padding: CGFloat = 6

    private init() {
        panel = NSPanel(contentRect: .zero, styleMask: [.borderless, .nonactivatingPanel],
                        backing: .buffered, defer: true)
        panel.isOpaque = false
        panel.backgroundColor = .clear
        panel.hasShadow = true
        panel.ignoresMouseEvents = true
        panel.hidesOnDeactivate = false
        panel.isReleasedWhenClosed = false
        panel.collectionBehavior = [.canJoinAllSpaces, .fullScreenAuxiliary]

        let background = NSVisualEffectView()
        background.material = .toolTip
        background.state = .active
        background.wantsLayer = true
        background.layer?.cornerRadius = 4
        background.layer?.borderWidth = 0.5
        background.layer?.borderColor = NSColor.separatorColor.cgColor

        label.font = NSFont.systemFont(ofSize: NSFont.smallSystemFontSize + 1)
        label.textColor = .labelColor
        label.maximumNumberOfLines = 0
        label.preferredMaxLayoutWidth = Self.maxWidth
        background.addSubview(label)
        panel.contentView = background
    }

    /// Show `annotation` under the caret of `client`.
    func show(_ annotation: String, client: IMKTextInput) {
        if shown != annotation {
            label.stringValue = annotation
            let fitting = label.sizeThatFits(NSSize(width: Self.maxWidth, height: .greatestFiniteMagnitude))
            let textSize = NSSize(width: min(ceil(fitting.width), Self.maxWidth), height: ceil(fitting.height))
            label.frame = NSRect(origin: NSPoint(x: Self.padding, y: Self.padding), size: textSize)
            panel.setContentSize(NSSize(width: textSize.width + Self.padding * 2, height: textSize.height + Self.padding * 2))
            shown = annotation
        }

        let size = panel.frame.size
        let caret = CompletionWindow.caretRect(of: client, fallbackHeight: size.height)
        panel.setFrameOrigin(CompletionWindow.origin(for: size, caret: caret))
        panel.level = NSWindow.Level(rawValue: Int(client.windowLevel()) + 1)
        panel.orderFront(nil)
    }

    func hide() {
        shown = nil
        panel.orderOut(nil)
    }
}
