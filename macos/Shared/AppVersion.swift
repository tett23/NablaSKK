// NablaSKK: the version shown in the preferences app (ADR 0013), kept free
// of SwiftUI so it can be unit tested.
//
// Written by tett23, 2026.
// License: GPL-2.0-or-later. See the LICENSE file for details.

import Foundation

/// Version and build commit from an Info.plist. `NablaSKKCommit` is written
/// by macos/build-app.sh (a short hash, `+` when the tree had changes).
enum AppVersion {
    static let commitKey = "NablaSKKCommit"

    private static func version(_ info: [String: Any]) -> String? {
        (info["CFBundleShortVersionString"] as? String).flatMap { $0.isEmpty ? nil : $0 }
    }

    private static func commit(_ info: [String: Any]) -> String? {
        (info[commitKey] as? String).flatMap { $0.isEmpty ? nil : $0 }
    }

    /// "NablaSKK 1.2.0 (8128940)"; parts that are missing are left out.
    static func display(_ info: [String: Any]) -> String {
        ["NablaSKK", version(info), commit(info).map { "(\($0))" }]
            .compactMap { $0 }
            .joined(separator: " ")
    }

    /// "バージョン 1.2.0、コミット 8128940"
    static func accessibilityLabel(_ info: [String: Any]) -> String {
        [version(info).map { "バージョン \($0)" }, commit(info).map { "コミット \($0)" }]
            .compactMap { $0 }
            .joined(separator: "、")
    }
}
