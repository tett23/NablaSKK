// NablaSKK Preferences: colours with enough contrast in both appearances
// (ADR 0006, audit C-1 to C-5).
//
// Written by tett23, 2026.
// License: GPL-2.0-or-later. See the LICENSE file for details.

import SwiftUI

/// The system orange, red and secondary label colours fall below 4.5:1 on
/// the light-mode backgrounds; these keep the system colours where they
/// already pass and use darker ones in light mode. Checked by
/// macos/run-ui-tests.sh against the window, list and error-row backgrounds.
enum PreferencesPalette {
    private static func adaptive(light: NSColor, dark: NSColor) -> NSColor {
        NSColor(name: nil) { appearance in
            appearance.bestMatch(from: [.darkAqua, .aqua]) == .darkAqua ? dark : light
        }
    }

    private static func hex(_ value: Int) -> NSColor {
        NSColor(srgbRed: CGFloat((value >> 16) & 0xff) / 255, green: CGFloat((value >> 8) & 0xff) / 255,
                blue: CGFloat(value & 0xff) / 255, alpha: 1)
    }

    /// Notes worth reading (ACT combined with extras, replaced rules ...)
    static let warning = adaptive(light: hex(0xB25000), dark: .systemOrange)
    /// Errors: reasons a row is not saved, missing files, invalid keys
    static let error = adaptive(light: hex(0xC42B1C), dark: hex(0xFF6961))
    /// Explanations and other secondary text
    static let secondary = adaptive(light: hex(0x666666), dark: .secondaryLabelColor)

    /// Background of a row with an error
    static let errorRowBackground = NSColor.systemRed.withAlphaComponent(0.08)
    /// Background of the selected row in a list
    static let selectedRow = NSColor.controlAccentColor.withAlphaComponent(0.15)
    /// Background of the selected sidebar item: the accent darkened so
    /// white text reaches 4.5:1
    static let sidebarSelection = NSColor(name: nil) { _ in
        NSColor.controlAccentColor.blended(withFraction: 0.15, of: .black) ?? .controlAccentColor
    }
}

extension Color {
    static let prefWarning = Color(nsColor: PreferencesPalette.warning)
    static let prefError = Color(nsColor: PreferencesPalette.error)
    static let prefSecondary = Color(nsColor: PreferencesPalette.secondary)
    static let prefErrorRow = Color(nsColor: PreferencesPalette.errorRowBackground)
    static let prefSelectedRow = Color(nsColor: PreferencesPalette.selectedRow)
    static let prefSidebarSelection = Color(nsColor: PreferencesPalette.sidebarSelection)
}
