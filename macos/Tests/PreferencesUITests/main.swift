// NablaSKK: preferences app UI tests (ADR 0005, ADR 0006). Builds RootView
// in an offscreen window against a scratch support directory, sends real
// mouse clicks and checks focus and rendered pixels.
// Run with: sh macos/run-ui-tests.sh
//
// Written by tett23, 2026.
// License: GPL-2.0-or-later. See the LICENSE file for details.

import SwiftUI

let app = NSApplication.shared
app.setActivationPolicy(.accessory)

var failures = 0
func check(_ ok: Bool, _ name: String) {
    print(ok ? "ok   \(name)" : "FAIL \(name)")
    if !ok { failures += 1 }
}

// MARK: - Colours (ADR 0006): 4.5:1 in both appearances

func luminance(_ color: NSColor) -> CGFloat {
    let c = color.usingColorSpace(.sRGB)!
    func channel(_ v: CGFloat) -> CGFloat { v <= 0.04045 ? v / 12.92 : pow((v + 0.055) / 1.055, 2.4) }
    return 0.2126 * channel(c.redComponent) + 0.7152 * channel(c.greenComponent) + 0.0722 * channel(c.blueComponent)
}
func composite(_ foreground: NSColor, over background: NSColor) -> NSColor {
    let f = foreground.usingColorSpace(.sRGB)!, b = background.usingColorSpace(.sRGB)!, a = f.alphaComponent
    return NSColor(srgbRed: f.redComponent * a + b.redComponent * (1 - a), green: f.greenComponent * a + b.greenComponent * (1 - a),
                   blue: f.blueComponent * a + b.blueComponent * (1 - a), alpha: 1)
}
func contrast(_ foreground: NSColor, _ background: NSColor) -> CGFloat {
    let f = luminance(composite(foreground, over: background)), b = luminance(background)
    return (max(f, b) + 0.05) / (min(f, b) + 0.05)
}

for (mode, appearance) in [("light", NSAppearance(named: .aqua)!), ("dark", NSAppearance(named: .darkAqua)!)] {
    appearance.performAsCurrentDrawingAppearance {
        let window = NSColor.windowBackgroundColor.usingColorSpace(.sRGB)!
        let list = NSColor.textBackgroundColor.usingColorSpace(.sRGB)!
        let redRow = composite(PreferencesPalette.errorRowBackground, over: list)
        for (name, color) in [("warning", PreferencesPalette.warning), ("error", PreferencesPalette.error),
                              ("secondary", PreferencesPalette.secondary)] {
            let ratios = [contrast(color, window), contrast(color, list), contrast(color, redRow)]
            check(ratios.allSatisfy { $0 >= 4.5 },
                  String(format: "%@ %@ text reaches 4.5:1 (window %.2f, list %.2f, error row %.2f)", mode, name, ratios[0], ratios[1], ratios[2]))
        }
        let selection = PreferencesPalette.sidebarSelection.usingColorSpace(.sRGB)!
        check(contrast(.white, selection) >= 4.5,
              String(format: "%@ selected sidebar item reaches 4.5:1 (%.2f)", mode, contrast(.white, selection)))
    }
}

// MARK: - Scratch settings (the support directory is a scratch one, see run-ui-tests.sh)

let support = DictionaryConfig.supportDirectory
let presentDictionary = support.appendingPathComponent("SKK-JISYO.present").path
let missingDictionary = support.appendingPathComponent("SKK-JISYO.missing").path
let disabledDictionary = support.appendingPathComponent("SKK-JISYO.off").path
try! "".write(toFile: presentDictionary, atomically: true, encoding: .utf8)
try! "".write(toFile: disabledDictionary, atomically: true, encoding: .utf8)
// The missing one first: the row under the present one (gadget) has no error
try! "0 \(missingDictionary)\n0 \(presentDictionary)\n4\n- 0 \(disabledDictionary)\n"
    .write(to: DictionaryConfig.fileURL, atomically: true, encoding: .utf8)
try! "SKK_JMODE bad spec\nSKK_CANCEL ctrl::g\n".write(to: KeymapSettings.fileURL, atomically: true, encoding: .utf8)
try! "jb,★,★,★\n".write(to: KanaRuleSettings.fileURL, atomically: true, encoding: .utf8)
try! ";; okuri-ari entries.\n;; okuri-nasi entries.\nてすと /試験/\n"
    .write(to: support.appendingPathComponent("skk-jisyo"), atomically: true, encoding: .utf8)

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

/// A click at a point in window coordinates (origin bottom-left). The
/// mouse-up comes from a timer so it also reaches tracking loops run
/// inside mouseDown (text fields, buttons).
func click(at point: NSPoint) {
    func event(_ type: NSEvent.EventType) -> NSEvent {
        NSEvent.mouseEvent(with: type, location: point, modifierFlags: [],
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
func click(xFromLeft x: CGFloat, yFromTop y: CGFloat) { click(at: NSPoint(x: x, y: size.height - y)) }

func textFields(in view: NSView) -> [NSTextField] {
    (view as? NSTextField).map { [$0] } ?? view.subviews.flatMap(textFields(in:))
}
func field(_ value: String) -> NSTextField? {
    textFields(in: host).first { $0.stringValue == value && $0.isEditable }
}
func frameInWindow(_ view: NSView) -> NSRect { view.convert(view.bounds, to: nil) }
func isEditing(_ field: NSTextField) -> Bool {
    (window.firstResponder as? NSTextView).map { $0.delegate === field } ?? false
}
func showPane(_ pane: SettingsPane) {
    click(xFromLeft: 70, yFromTop: rowCenterY(SettingsPane.allCases.firstIndex(of: pane)!))
}

func snapshot() -> NSBitmapImageRep {
    let rep = host.bitmapImageRepForCachingDisplay(in: host.bounds)!
    host.cacheDisplay(in: host.bounds, to: rep)
    return rep
}
/// Colour at a window point, from a fresh rendering.
func color(in rep: NSBitmapImageRep, at point: NSPoint) -> NSColor? {
    let scale = CGFloat(rep.pixelsWide) / size.width
    return rep.colorAt(x: Int(point.x * scale), y: Int((size.height - point.y) * scale))?.usingColorSpace(.sRGB)
}
func distance(_ a: NSColor, _ b: NSColor) -> CGFloat {
    let a = a.usingColorSpace(.sRGB)!, b = b.usingColorSpace(.sRGB)!
    return abs(a.redComponent - b.redComponent) + abs(a.greenComponent - b.greenComponent) + abs(a.blueComponent - b.blueComponent)
}
func resolved(_ color: NSColor) -> NSColor {
    var result = color
    host.effectiveAppearance.performAsCurrentDrawingAppearance { result = color.usingColorSpace(.sRGB)! }
    return result
}
/// Whether any pixel in the band just below a view is close to `target` (written error text).
func bandBelow(_ view: NSView, contains target: NSColor) -> Bool {
    let rep = snapshot(), frame = frameInWindow(view), target = resolved(target)
    for y in stride(from: frame.minY - 3, to: frame.minY - 18, by: -1) {
        for x in stride(from: frame.minX, to: min(frame.maxX + 200, size.width - 1), by: 1) {
            if let pixel = color(in: rep, at: NSPoint(x: x, y: y)), distance(pixel, target) < 0.25 { return true }
        }
    }
    return false
}

// MARK: - Sidebar (ADR 0005)

/// Sidebar row geometry (top-left points): 12pt padding, rows ~30pt apart.
func rowCenterY(_ index: Int) -> CGFloat { 12 + 14 + CGFloat(index) * 30 }

func highlightedRows() -> [Int] {
    let rep = snapshot(), selection = resolved(PreferencesPalette.sidebarSelection)
    return SettingsPane.allCases.indices.filter { index in
        guard let pixel = color(in: rep, at: NSPoint(x: 150, y: size.height - rowCenterY(index))) else { return false }
        return distance(pixel, selection) < 0.15
    }
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
        showPane(pane)
        check(UserDefaults.standard.string(forKey: paneKey) == pane.rawValue, "click \(pane.title): the page switches")
        let rows = highlightedRows()
        check(rows == [SettingsPane.allCases.firstIndex(of: pane)!], "click \(pane.title): only its row is highlighted (got \(rows))")
    }
}

// MARK: - Sidebar keyboard (ADR 0007)

func press(keyCode: UInt16, character: Int) {
    let chars = String(UnicodeScalar(character)!)
    for type in [NSEvent.EventType.keyDown, .keyUp] {
        window.sendEvent(NSEvent.keyEvent(with: type, location: .zero, modifierFlags: [.numericPad, .function],
                                          timestamp: ProcessInfo.processInfo.systemUptime, windowNumber: window.windowNumber,
                                          context: nil, characters: chars, charactersIgnoringModifiers: chars,
                                          isARepeat: false, keyCode: keyCode)!)
    }
    settle()
}
func pressDown() { press(keyCode: 125, character: NSDownArrowFunctionKey) }
func pressUp() { press(keyCode: 126, character: NSUpArrowFunctionKey) }
func currentPane() -> String? { UserDefaults.standard.string(forKey: paneKey) }

showPane(.input)
pressDown()
check(currentPane() == SettingsPane.kanaRules.rawValue && highlightedRows() == [4], "sidebar: ↓ moves to the next page")
pressUp()
check(currentPane() == SettingsPane.input.rawValue && highlightedRows() == [3], "sidebar: ↑ moves to the previous page")
showPane(.keys)
pressDown()
check(currentPane() == SettingsPane.keys.rawValue, "sidebar: ↓ on the last page stays")
showPane(.dictionaries)
pressUp()
check(currentPane() == SettingsPane.dictionaries.rawValue, "sidebar: ↑ on the first page stays")

// MARK: - Dictionary page (ADR 0006)

showPane(.dictionaries)
if let present = field(presentDictionary), let missing = field(missingDictionary) {
    click(at: NSPoint(x: frameInWindow(present).midX, y: frameInWindow(present).midY))
    check(isEditing(present), "dictionaries: one click puts the cursor in the location field")
    check(bandBelow(missing, contains: PreferencesPalette.error), "dictionaries: a missing file is explained in error text under its field")
    check(!bandBelow(present, contains: PreferencesPalette.error), "dictionaries: no error text under an existing file")

    // Clicking the row (its name column) selects it
    let frame = frameInWindow(missing)
    let nameColumn = NSPoint(x: frame.minX - 12 - DictionaryRow.kindWidth - 12 - 8, y: frame.midY)
    click(at: nameColumn)
    let pixel = color(in: snapshot(), at: nameColumn)
    let selected = resolved(composite(PreferencesPalette.selectedRow, over: .textBackgroundColor))
    check(pixel.map { distance($0, selected) < 0.12 } == true, "dictionaries: clicking a row selects it")
} else {
    check(false, "dictionaries: location fields are shown")
}

// A disabled dictionary is written "(無効)" in the secondary colour,
// not faded to half opacity (ADR 0007)
if let off = field(disabledDictionary) {
    let frame = frameInWindow(off), rep = snapshot()
    let nameStart = frame.minX - 12 - DictionaryRow.kindWidth - 12 - DictionaryRow.nameWidth
    var darkest: CGFloat = 1
    for x in stride(from: nameStart, to: nameStart + DictionaryRow.nameWidth, by: 1) {
        for y in stride(from: frame.midY - 6, to: frame.midY + 6, by: 1) {
            if let pixel = color(in: rep, at: NSPoint(x: x, y: y)) { darkest = min(darkest, pixel.greenComponent) }
        }
    }
    let secondary = resolved(PreferencesPalette.secondary).greenComponent
    check(abs(darkest - secondary) < 0.08,
          String(format: "dictionaries: a disabled row's name is in the secondary colour (darkest %.2f, secondary %.2f)", darkest, secondary))
} else {
    check(false, "dictionaries: the disabled dictionary is shown")
}

// MARK: - Key page (ADR 0006)

showPane(.keys)
if let bad = field("bad spec") {
    check(bandBelow(bad, contains: PreferencesPalette.error), "keys: an invalid key spec is explained in error text under its field")
} else {
    check(false, "keys: the invalid override is shown")
}
// An overridden key shows its default under the field (ADR 0007)
if let overridden = field("ctrl::g") {
    check(bandBelow(overridden, contains: PreferencesPalette.secondary), "keys: an overridden key shows the default under its field")
} else {
    check(false, "keys: the valid override is shown")
}

// MARK: - Romaji rule page (ADR 0006)

showPane(.kanaRules)
if let input = field("jb") {
    click(at: NSPoint(x: frameInWindow(input).midX, y: frameInWindow(input).midY))
    check(isEditing(input), "romaji rules: one click puts the cursor in the input field")
} else {
    check(false, "romaji rules: the user rule is shown")
}
// The built-in table sits next to the test binary (run-ui-tests.sh copies it)
check(textFields(in: host).filter { !$0.isEnabled && $0.stringValue == "z." }.isEmpty,
      "romaji rules: built-in rules are text, not disabled fields")

// MARK: - User dictionary page (ADR 0006)

showPane(.userDictionary)
func tables(in view: NSView) -> [NSTableView] {
    (view as? NSTableView).map { [$0] } ?? view.subviews.flatMap(tables(in:))
}
if let entries = tables(in: host).first, entries.numberOfRows > 0 {
    // The entry list is still a List (no fields in it); synthesized clicks
    // do not reach its table, so select through the table itself. What is
    // tested is the click on the candidate field below.
    entries.selectRowIndexes(IndexSet(integer: 0), byExtendingSelection: false)
    settle()
    if let candidate = field("試験") {
        click(at: NSPoint(x: frameInWindow(candidate).midX, y: frameInWindow(candidate).midY))
        check(isEditing(candidate), "user dictionary: one click puts the cursor in a candidate field")
    } else {
        check(false, "user dictionary: the candidate is shown after selecting the entry")
    }
} else {
    check(false, "user dictionary: the entry list is shown")
}

window.orderOut(nil)
if failures > 0 {
    print("\(failures) FAILED")
    exit(1)
}
print("PASS all preferences UI tests")
