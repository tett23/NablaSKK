// AquaSKK-Rust: macOS input method entry point.
//
// Starts the IMKServer named in Info.plist; the system instantiates
// SKKRustInputController for each client application.
//
// License: GPL-2.0-or-later.

import Cocoa
import InputMethodKit

guard let connectionName = Bundle.main.infoDictionary?["InputMethodConnectionName"] as? String,
      let bundleIdentifier = Bundle.main.bundleIdentifier
else {
    fatalError("AquaSKK-Rust: Info.plist is missing input method keys")
}

// Kept alive for the process lifetime.
let server = IMKServer(name: connectionName, bundleIdentifier: bundleIdentifier)

NSApplication.shared.run()
