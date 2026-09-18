// Swift wrapper over the nablaskk-ffi C ABI.
//
// Build against the Rust library:
//   cargo build --release
//   swiftc -import-objc-header crates/nablaskk-ffi/include/nablaskk.h \
//          swift/SKKSession.swift your-code.swift \
//          target/release/libnablaskk_ffi.a
//
// License: GPL-2.0-or-later.

import Foundation

/// SKK input session backed by the Rust engine.
///
/// Feed key events with `handle(charcode:keycode:mods:)`; read committed
/// text from `takeFixed()` and the marked text from `composing`. From an
/// InputMethodKit controller, call these from `handle(_:client:)` and
/// forward the results to `insertText` / `setMarkedText`.
public final class SKKSession {
    public struct Modifiers: OptionSet {
        public let rawValue: UInt32
        public init(rawValue: UInt32) { self.rawValue = rawValue }

        public static let shift = Modifiers(rawValue: 1 << 1)
        public static let ctrl = Modifiers(rawValue: 1 << 2)
        public static let alt = Modifiers(rawValue: 1 << 3)
        public static let meta = Modifiers(rawValue: 1 << 4)
    }

    public enum DictionaryType: Int32 {
        case common = 0
        case autoUpdate = 1
        case proxy = 2
        case gadget = 4
        case commonUTF8 = 5
    }

    public enum InputMode: Int32 {
        case hirakana = 0
        case katakana = 1
        case jisx0201kana = 2
        case ascii = 3
        case jisx0208latin = 4
    }

    private let session: OpaquePointer

    /// - Parameter userDictionaryPath: created on first save;
    ///   nil defaults to `~/.rust-skk-jisyo`.
    public init(userDictionaryPath: String? = nil) {
        session = userDictionaryPath.flatMap { path in
            path.withCString { skk_session_new($0) }
        } ?? skk_session_new(nil)!
    }

    deinit {
        skk_session_save(session)
        skk_session_free(session)
    }

    /// Add a system dictionary. Returns false when the configuration
    /// was rejected.
    @discardableResult
    public func addDictionary(_ type: DictionaryType, location: String) -> Bool {
        location.withCString {
            skk_session_add_dictionary(session, type.rawValue, $0) == 0
        }
    }

    /// Feed one key event. Returns true when the IME consumed it.
    @discardableResult
    public func handle(charcode: UInt8, keycode: UInt8 = 0, mods: Modifiers = []) -> Bool {
        skk_session_handle(session, charcode, keycode, mods.rawValue) == 1
    }

    /// Text committed since the last call.
    public func takeFixed() -> String {
        takeString(skk_session_take_fixed(session))
    }

    /// The current marked (composing) text.
    public var composing: String {
        takeString(skk_session_composing(session))
    }

    public var inputMode: InputMode {
        InputMode(rawValue: skk_session_input_mode(session)) ?? .hirakana
    }

    /// True while the candidate window should be shown.
    public var candidatesVisible: Bool {
        skk_session_candidates_visible(session) == 1
    }

    /// Candidates on the current window page.
    public var candidates: [String] {
        (0..<skk_session_candidate_count(session)).map { index in
            takeString(skk_session_candidate(session, index))
        }
    }

    /// Cursor position within the current page.
    public var candidateCursor: Int {
        Int(skk_session_candidate_cursor(session))
    }

    /// Current page (1-based) and total page count.
    public var candidatePage: (page: Int, count: Int) {
        let packed = skk_session_candidate_page(session)
        return (Int(packed >> 16), Int(packed & 0xffff))
    }

    public enum Option: Int32 {
        case suppressNewlineOnCommit = 0
        case inlineBackspaceImpliesCommit = 1
        case deleteOkuriWhenQuit = 2
        case handleRecursiveEntryAsOkuri = 3
        case fixIntermediateConversion = 4
        case displayShortestMatch = 5
        case useNumericConversion = 6
        case maxInlineCandidates = 7
    }

    /// Set an engine option (booleans take 0/1).
    @discardableResult
    public func setOption(_ option: Option, _ value: Int32) -> Bool {
        skk_session_set_option(session, option.rawValue, value) == 0
    }

    public func commit() {
        skk_session_commit(session)
    }

    public func clear() {
        skk_session_clear(session)
    }

    /// Flush the user dictionary to disk.
    public func save() {
        skk_session_save(session)
    }

    private func takeString(_ ptr: UnsafeMutablePointer<CChar>?) -> String {
        guard let ptr else { return "" }
        defer { skk_string_free(ptr) }
        return String(cString: ptr)
    }
}
