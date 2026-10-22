#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
icon_work=$(mktemp -d "${TMPDIR:-/tmp}/prospect-icons.XXXXXX")
trap 'rm -rf "$icon_work"' EXIT HUP INT TERM
mkdir -p "$icon_work/AppIcon.iconset"
/usr/bin/sips -s format png assets/app-icon.svg --out "$icon_work/master.png" >/dev/null
for size in 16 32 128 256 512; do
    /usr/bin/sips -z "$size" "$size" "$icon_work/master.png" --out "$icon_work/AppIcon.iconset/icon_${size}x${size}.png" >/dev/null
    retina=$((size * 2))
    /usr/bin/sips -z "$retina" "$retina" "$icon_work/master.png" --out "$icon_work/AppIcon.iconset/icon_${size}x${size}@2x.png" >/dev/null
done
/usr/bin/iconutil -c icns "$icon_work/AppIcon.iconset" -o "$icon_work/AppIcon.icns"
cp "$icon_work/master.png" assets/app-icon.png
cp "$icon_work/AppIcon.icns" assets/AppIcon.icns
printf '%s\n' 'Iconos SVG, PNG e ICNS listos en assets/.'
