// NablaSKK: preferences app UI test (ADR 0005). Builds RootView in an
// offscreen window, clicks the sidebar and checks from the rendered
// pixels that exactly the selected row has the accent background, and
// that the page switched. Run with: sh macos/run-ui-tests.sh
//
// Written by tett23, 2026.
// License: GPL-2.0-or-later. See the LICENSE file for details.

import SwiftUI

let app = NSApplication.shared
app.setActivationPolicy(.accessory)

// This binary has its own defaults domain, so the user's last page is untouched
let paneKey = "selectedSettingsPane"
UserDefaults.standard.set(SettingsPane.dictionaries.rawValue, forKey: paneKey)

let size = NSSize(width: 920, height: 520)
let host = NSHostingView(rootView: RootView().frame(width: size.width, height: size.height))
host.frame = NSRect(origin: .zero, size: size)
final class KeyWindow: NSWindow { override var canBecomeKey: Bool { true } }
let window = KeyWindow(contentRect: NSRect(origin: NSPoint(x: -20000, y: -20000), size: size),
                       styleMask: [.borderless], backing: .buffered, defer: false)
window.contentView = host
window.orderFrontRegardless()
window.makeKey()

func settle(_ seconds: Double = 0.4) {
    host.layoutSubtreeIfNeeded()
    RunLoop.main.run(until: Date().addingTimeInterval(seconds))
}

/// Sidebar row geometry (top-left points): 12pt padding, rows ~30pt apart.
func rowCenterY(_ index: Int) -> CGFloat { 12 + 14 + CGFloat(index) * 30 }
let sampleX: CGFloat = 150

/// A click at a top-left point. The mouse-up comes from a timer so it
/// also reaches tracking loops run inside mouseDown.
func click(x: CGFloat, yFromTop: CGFloat) {
    func event(_ type: NSEvent.EventType) -> NSEvent {
        NSEvent.mouseEvent(with: type, location: NSPoint(x: x, y: size.height - yFromTop), modifierFlags: [],
                           timestamp: ProcessInfo.processInfo.systemUptime, windowNumber: window.windowNumber,
                           context: nil, eventNumber: 0, clickCount: 1, pressure: 1)!
    }
    let up = event(.leftMouseUp)
    RunLoop.main.add(Timer(timeInterval: 0.1, repeats: false) { _ in NSApp.postEvent(up, atStart: false) }, forMode: .common)
    window.sendEvent(event(.leftMouseDown))
    if let pending = NSApp.nextEvent(matching: .leftMouseUp, until: Date().addingTimeInterval(0.3),
                                     inMode: .default, dequeue: true) {
        window.sendEvent(pending)
    }
    settle()
}

/// Rows whose background at `sampleX` is the accent colour.
func highlightedRows() -> [Int] {
    let rep = host.bitmapImageRepForCachingDisplay(in: host.bounds)!
    host.cacheDisplay(in: host.bounds, to: rep)
    let scale = CGFloat(rep.pixelsWide) / size.width
    let accent = NSColor.controlAccentColor.usingColorSpace(.sRGB)!
    return SettingsPane.allCases.indices.filter { index in
        guard let color = rep.colorAt(x: Int(sampleX * scale), y: Int(rowCenterY(index) * scale))?
            .usingColorSpace(.sRGB) else { return false }
        let distance = abs(color.redComponent - accent.redComponent)
            + abs(color.greenComponent - accent.greenComponent)
            + abs(color.blueComponent - accent.blueComponent)
        return distance < 0.15
    }
}

var failures = 0
func check(_ ok: Bool, _ name: String) {
    print(ok ? "ok   \(name)" : "FAIL \(name)")
    if !ok { failures += 1 }
}

settle()
check(highlightedRows() == [0], "initially 辞書 is highlighted")

let orders: [[SettingsPane]] = [
    [.input, .skkserv, .dictionaries],
    [.keys, .input],
    [.kanaRules, .input, .dictionaries],
    [.userDictionary, .keys, .skkserv, .kanaRules],
]
for order in orders {
    for pane in order {
        let index = SettingsPane.allCases.firstIndex(of: pane)!
        click(x: 70, yFromTop: rowCenterY(index))
        let selected = UserDefaults.standard.string(forKey: paneKey)
        check(selected == pane.rawValue, "click \(pane.title): the page switches")
        let rows = highlightedRows()
        check(rows == [index], "click \(pane.title): only its row is highlighted (got \(rows))")
    }
}

window.orderOut(nil)
if failures > 0 {
    print("\(failures) FAILED")
    exit(1)
}
print("PASS all preferences UI tests")
