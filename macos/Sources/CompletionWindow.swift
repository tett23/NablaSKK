// NablaSKK: dynamic completion ("suggest") window, drawn after AquaSKK's
// CompletionWindow / CompletionView / MacDynamicCompletor
// (platform/mac/src/gui, src/server): a pale box below the caret listing
// dictionary readings that extend what is being typed, the untyped part
// in bold, with a "TAB で補完" guide plate.
//
// Written by tett23, 2026.
// License: GPL-2.0-or-later. See the LICENSE file for details.

import Cocoa
import InputMethodKit

final class CompletionWindow {
    static let shared = CompletionWindow()

    private let panel: NSPanel
    private let view = CompletionView()

    private init() {
        panel = NSPanel(contentRect: .zero, styleMask: [.borderless, .nonactivatingPanel],
                        backing: .buffered, defer: true)
        panel.isOpaque = false
        panel.backgroundColor = .clear
        panel.hasShadow = false
        panel.ignoresMouseEvents = true
        panel.hidesOnDeactivate = false
        panel.isReleasedWhenClosed = false
        panel.collectionBehavior = [.canJoinAllSpaces, .fullScreenAuxiliary]
        panel.contentView = view
    }

    /// Show `completions` under the caret of `client`. `prefixLength` is
    /// the number of leading characters already typed (drawn normal;
    /// the rest bold, like AquaSKK).
    func show(completions: [String], prefixLength: Int, client: IMKTextInput) {
        guard !completions.isEmpty else { return hide() }

        view.setCompletion(Self.attributedList(completions, prefixLength: prefixLength))
        let size = view.frame.size
        panel.setContentSize(size)

        let caret = caretRect(of: client, fallbackHeight: size.height)
        panel.setFrameOrigin(origin(for: size, caret: caret))
        panel.level = NSWindow.Level(rawValue: Int(client.windowLevel()) + 1)
        panel.orderFront(nil)
    }

    func hide() {
        panel.orderOut(nil)
    }

    /// MacDynamicCompletor::makeAttributedString: one completion per
    /// line, the part after the common prefix in bold.
    static func attributedList(_ completions: [String], prefixLength: Int) -> NSAttributedString {
        let normal: [NSAttributedString.Key: Any] = [.font: NSFont.systemFont(ofSize: 0), .foregroundColor: NSColor.labelColor]
        let bold: [NSAttributedString.Key: Any] = [.font: NSFont.boldSystemFont(ofSize: 0), .foregroundColor: NSColor.labelColor]
        let result = NSMutableAttributedString()
        for (index, completion) in completions.enumerated() {
            if index > 0 { result.append(NSAttributedString(string: "\n", attributes: normal)) }
            let characters = Array(completion)
            let split = min(prefixLength, characters.count)
            result.append(NSAttributedString(string: String(characters[..<split]), attributes: normal))
            result.append(NSAttributedString(string: String(characters[split...]), attributes: bold))
        }
        return result
    }

    /// Caret line rectangle in screen coordinates; clients that do not
    /// report one get the mouse location (AquaSKK uses the screen origin).
    private func caretRect(of client: IMKTextInput, fallbackHeight: CGFloat) -> NSRect {
        var rect = NSRect.zero
        let attributes = client.attributes(forCharacterIndex: 0, lineHeightRectangle: &rect)
        if rect.origin == .zero && rect.size == .zero {
            let mouse = NSEvent.mouseLocation
            return NSRect(x: mouse.x, y: mouse.y, width: 0, height: fallbackHeight)
        }
        if let font = attributes?[NSAttributedString.Key.font.rawValue] as? NSFont
            ?? attributes?[NSAttributedString.Key.font] as? NSFont {
            rect.size.height = font.boundingRectForFont.height
        }
        if rect.height == 0 { rect.size.height = fallbackHeight }
        return rect
    }

    /// Top-left at the caret's bottom-left (CompletionWindow showCompletion),
    /// kept on screen: pulled left at the right edge, flipped above the
    /// caret at the bottom edge.
    private func origin(for size: NSSize, caret: NSRect) -> NSPoint {
        let screen = (NSScreen.screens.first { $0.frame.contains(caret.origin) } ?? NSScreen.main)?.visibleFrame
            ?? NSRect(x: 0, y: 0, width: 1440, height: 900)

        var point = NSPoint(x: caret.minX, y: caret.minY - size.height)
        if point.x + size.width > screen.maxX { point.x = screen.maxX - size.width }
        point.x = max(point.x, screen.minX)
        if point.y < screen.minY { point.y = caret.maxY }
        return point
    }
}

/// CompletionView: pale background, thin frame, guide plate at the bottom.
final class CompletionView: NSView {
    private var completion = NSAttributedString()
    private let guide: NSAttributedString
    private let guideSize: NSSize
    private static let strokeColor = NSColor.separatorColor
    private static let backgroundColor = NSColor(name: nil) { appearance in
        appearance.bestMatch(from: [.darkAqua, .aqua]) == .darkAqua
            ? NSColor(calibratedRed: 0.24, green: 0.24, blue: 0.20, alpha: 1)
            : NSColor(calibratedRed: 1.0, green: 1.0, blue: 0.94, alpha: 1)
    }

    init() {
        guide = NSAttributedString(string: "  TAB で補完  ", attributes: [
            .font: NSFont.boldSystemFont(ofSize: NSFont.labelFontSize),
            .foregroundColor: NSColor.white,
        ])
        var size = guide.size()
        size.height += 2
        guideSize = size
        super.init(frame: .zero)
    }

    required init?(coder: NSCoder) { fatalError("not used") }

    func setCompletion(_ text: NSAttributedString) {
        completion = text
        var size = text.size()
        size.width = max(size.width, guideSize.width) + 8
        size.height += guideSize.height + 8
        frame = NSRect(origin: .zero, size: size)
        needsDisplay = true
    }

    override func draw(_ dirtyRect: NSRect) {
        var box = bounds
        box.origin.y += guideSize.height
        box.size.height -= guideSize.height

        Self.backgroundColor.setFill()
        box.fill()
        Self.strokeColor.setStroke()
        NSBezierPath(rect: box.insetBy(dx: 0.5, dy: 0.5)).stroke()

        // Guide plate: rounded at the bottom, square where it meets the box
        var plateRect = bounds
        plateRect.size.height = guideSize.height
        let plate = NSBezierPath(roundedRect: plateRect, xRadius: plateRect.height / 2, yRadius: plateRect.height / 2)
        plate.appendRect(NSRect(x: plateRect.minX, y: plateRect.midY, width: plateRect.width, height: plateRect.height / 2))
        NSColor.systemGray.setFill()
        plate.fill()
        guide.draw(at: NSPoint(x: (bounds.width - guideSize.width) / 2, y: 1))

        completion.draw(at: NSPoint(x: 3, y: guideSize.height + 4))
    }
}
