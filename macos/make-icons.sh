#!/bin/sh
# Written by tett23, 2026.
#
# Builds the icons from their SVG sources with tools that ship with macOS
# (ADR 0014, ADR 0015). The results are committed, so run this only after
# changing an SVG:
#
#   macos/icons/AppIcon.svg  -> macos/icons/AppIcon.icns  (16-1024px, 1x and 2x)
#   macos/icons/MenuIcon.svg -> macos/icons/MenuIcon.tiff (16px and 32px)
#
#   sh macos/make-icons.sh
set -eu

cd "$(dirname "$0")/icons"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

# sips draws an SVG at each size on a transparent background (qlmanage
# would fill it with white); drawing every size from the SVG keeps small
# sizes sharp
render() { # $1 svg, $2 size, $3 output png
    sips -s format png -z "$2" "$2" "$1" --out "$3" >/dev/null
}

# Application icon
iconset="$work/AppIcon.iconset"
mkdir "$iconset"
for size in 16 32 128 256 512; do
    render AppIcon.svg "$size" "$iconset/icon_${size}x${size}.png"
    render AppIcon.svg "$((size * 2))" "$iconset/icon_${size}x${size}@2x.png"
done
iconutil -c icns "$iconset" -o AppIcon.icns

# Input menu icon (template image): 16px and its 2x in one TIFF
for size in 16 32; do
    render MenuIcon.svg "$size" "$work/menu-$size.png"
    sips -s format tiff "$work/menu-$size.png" --out "$work/menu-$size.tiff" >/dev/null
done
tiffutil -cathidpicheck "$work/menu-16.tiff" "$work/menu-32.tiff" -out MenuIcon.tiff 2>/dev/null

echo "==> wrote macos/icons/AppIcon.icns and macos/icons/MenuIcon.tiff"
