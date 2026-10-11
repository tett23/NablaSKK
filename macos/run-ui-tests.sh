#!/bin/sh
# Build and run the preferences app UI test (ADR 0005) against a scratch
# support directory, so the user's settings are never touched.
set -e
cd "$(dirname "$0")/.."

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

# The same sources as the preferences app (see build-app.sh)
eval "$(sed -n '/^PREFS_SOURCES="/,/"$/p' macos/build-app.sh)"
for source in $PREFS_SOURCES; do cp "$source" "$work/"; done

# The app's entry point is replaced by the test's main.swift, and its
# support directory by a scratch one.
# (sed without -i: GNU and BSD sed disagree on its argument)
sed 's/^@main$//' "$work/PreferencesApp.swift" > "$work/app.tmp"
mv "$work/app.tmp" "$work/PreferencesApp.swift"
sed "s|FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)\[0\]|URL(fileURLWithPath: \"$work/support-root\", isDirectory: true)|" \
    "$work/DictionaryConfig.swift" > "$work/config.tmp"
mv "$work/config.tmp" "$work/DictionaryConfig.swift"
grep -q "support-root" "$work/DictionaryConfig.swift"
grep -q "@main" "$work/PreferencesApp.swift" && { echo "could not remove @main"; exit 1; }
cp macos/Tests/PreferencesUITests/main.swift "$work/main.swift"

swiftc "$work"/*.swift -framework SwiftUI -framework Cocoa -o "$work/preferences-ui-tests"
# Resources the app bundles (Bundle.main of a plain executable is its directory)
cp data/kana-rule.utf8.conf data/azik.rule data/azik_us.rule data/act.rule data/azik.conf data/act.conf \
    data/yakumono-ippai.rule data/old-kana.rule "$work/"
"$work/preferences-ui-tests"
