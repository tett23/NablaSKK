// NablaSKK: writing and watching files in the support directory that may
// be symbolic links (ADR 0021), shared by the input method and the
// preferences app.
//
// Written by tett23, 2026.
// License: GPL-2.0-or-later. See the LICENSE file for details.

import Foundation

enum ConfigFile {
    /// Follow `url` through any chain of symbolic links to the file it
    /// names. Relative links resolve against the link's directory; a
    /// dangling link resolves to its missing target; a path that is not a
    /// link is returned as is. Bounded so a link loop cannot hang.
    static func resolvingLinks(_ url: URL) -> URL {
        var current = url
        for _ in 0..<40 {
            guard let destination = try? FileManager.default.destinationOfSymbolicLink(atPath: current.path) else {
                return current
            }
            current = destination.hasPrefix("/")
                ? URL(fileURLWithPath: destination)
                : current.deletingLastPathComponent().appendingPathComponent(destination)
        }
        return current
    }

    /// Write atomically to the file `url` names, keeping a symbolic link
    /// in place. When the target cannot be written the error is thrown and
    /// the link is left alone.
    static func write(_ data: Data, to url: URL) throws {
        try data.write(to: resolvingLinks(url), options: .atomic)
    }

    static func write(_ text: String, to url: URL) throws {
        try write(Data(text.utf8), to: url)
    }

    /// Modification date of the file `url` names (not of a link to it).
    static func modificationDate(of url: URL) -> Date? {
        (try? FileManager.default.attributesOfItem(atPath: resolvingLinks(url).path))?[.modificationDate] as? Date
    }
}
