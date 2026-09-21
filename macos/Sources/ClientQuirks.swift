// NablaSKK: per-client key handling quirks.
//
// Written by tett23, 2026.
// License: GPL-2.0-or-later. See the LICENSE file for details.

import Cocoa

/// Some clients act on a key even though the input method consumed it,
/// unless marked text exists before or after the key handler.
enum ClientQuirks {
    enum KeyLeak: Equatable {
        /// Honors the input method's result (ordinary Cocoa text views).
        case none
        /// Forwards consumed Control combinations to the page: Chromium only
        /// reports keyCode 229 when marked text exists, so xterm.js turns a
        /// consumed Ctrl-J into a newline. Plain keys are harmless there.
        case controlKeys
        /// Encodes every key that produced neither text nor marked text and
        /// sends it to the terminal, ignoring the input method's result
        /// (Ghostty: `l` switching to ASCII mode is typed as "l").
        case allKeys
    }

    private static var cache: [String: KeyLeak] = [:]

    private static let selfEncodingTerminals: Set<String> = [
        "com.mitchellh.ghostty",
    ]

    private static let chromiumFrameworks = [
        "Electron Framework.framework",
        "Chromium Embedded Framework.framework",
        "Google Chrome Framework.framework",
        "Chromium Framework.framework",
        "Microsoft Edge Framework.framework",
        "Brave Browser Framework.framework",
        "Vivaldi Framework.framework",
        "Arc Framework.framework",
    ]

    static func keyLeak(for bundleIdentifier: String?) -> KeyLeak {
        guard let bundleIdentifier else { return .none }
        if let known = cache[bundleIdentifier] { return known }

        var result = KeyLeak.none
        if selfEncodingTerminals.contains(bundleIdentifier) {
            result = .allKeys
        } else if let url = NSWorkspace.shared.urlForApplication(withBundleIdentifier: bundleIdentifier) {
            // Every Electron app has its own bundle id; detect the framework
            let directory = url.appendingPathComponent("Contents/Frameworks")
            let isChromium = chromiumFrameworks.contains {
                FileManager.default.fileExists(atPath: directory.appendingPathComponent($0).path)
            }
            if isChromium {
                result = .controlKeys
            }
        }

        cache[bundleIdentifier] = result
        return result
    }

    /// Whether a consumed key must be masked with transient marked text.
    static func shouldMask(_ leak: KeyLeak, handled: Bool, produced: Bool,
                           wasComposing: Bool, hasControl: Bool) -> Bool {
        // Marked text before or after the handler already hides the key
        guard handled, !produced, !wasComposing else { return false }

        switch leak {
        case .none: return false
        case .controlKeys: return hasControl
        case .allKeys: return true
        }
    }
}
