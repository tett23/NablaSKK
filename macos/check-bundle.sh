#!/bin/sh
# Written by tett23, 2026.
#
# Checks that a built NablaSKK.app has everything it needs at run time
# (ADR 0008): the executables, the sub-rule files the IME and the
# preferences app load, the bundled preferences app, and an Info.plist
# version matching Cargo.toml.
#
#   sh macos/check-bundle.sh [path/to/NablaSKK.app]
set -eu

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
APP="${1:-$ROOT/macos/dist/NablaSKK.app}"
PREFS="$APP/Contents/Resources/NablaSKK Preferences.app"
SUB_RULES="azik.rule azik_us.rule azik.conf act.rule act.conf yakumono-ippai.rule old-kana.rule"

failures=0
fail() { echo "FAIL $1"; failures=$((failures + 1)); }
need() { [ -e "$1" ] || fail "missing: ${1#"$APP"/}"; }
version_of() { /usr/libexec/PlistBuddy -c "Print :CFBundleShortVersionString" "$1" 2>/dev/null || echo "?"; }

need "$APP/Contents/Info.plist"
[ -x "$APP/Contents/MacOS/NablaSKK" ] || fail "missing executable: Contents/MacOS/NablaSKK"
for f in $SUB_RULES; do need "$APP/Contents/Resources/$f"; done

need "$PREFS/Contents/Info.plist"
[ -x "$PREFS/Contents/MacOS/NablaSKK Preferences" ] || fail "missing executable: NablaSKK Preferences"
for f in $SUB_RULES kana-rule.utf8.conf; do need "$PREFS/Contents/Resources/$f"; done

expected="$(sed -n 's/^version = "\(.*\)"/\1/p' "$ROOT/Cargo.toml" | head -n 1)"
for plist in "$APP/Contents/Info.plist" "$PREFS/Contents/Info.plist"; do
    actual="$(version_of "$plist")"
    [ "$actual" = "$expected" ] || fail "version ${plist#"$APP"/}: $actual (Cargo.toml: $expected)"
done

if [ "$failures" -ne 0 ]; then
    echo "FAIL $failures bundle check(s): $APP"
    exit 1
fi
echo "PASS bundle check: $APP ($expected)"
