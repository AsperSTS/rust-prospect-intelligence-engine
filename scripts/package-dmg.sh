#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
app="dist/Prospect Intelligence.app"
if [ ! -d "$app" ]; then ./scripts/bundle-macos.sh; fi
codesign --verify --strict "$app"
app_version=$(/usr/libexec/PlistBuddy -c 'Print :CFBundleShortVersionString' "$app/Contents/Info.plist")
mkdir -p dist
dmg_work=$(mktemp -d "$PWD/dist/.dmg.XXXXXX")
trap 'rm -rf "$dmg_work"' EXIT HUP INT TERM
mkdir "$dmg_work/content"
/usr/bin/ditto "$app" "$dmg_work/content/Prospect Intelligence.app"
ln -s /Applications "$dmg_work/content/Applications"
cat > "$dmg_work/content/LEEME.txt" <<'TXT'
PROSPECT INTELLIGENCE

Arrastra Prospect Intelligence.app a Applications para usarla como aplicación macOS.
También puedes abrirla directamente desde la carpeta del proyecto.

Tu base y tokens permanecen en tu carpeta personal Application Support/Prospect Intelligence.
El paquete no incluye datos comerciales ni credenciales.

Esta compilación local usa firma ad hoc y no está notarizada por Apple.
Para distribución pública se necesita preparar Developer ID y notarización.

No requiere Python, Rust instalado, Streamlit ni una suscripción para ejecutarse.
TXT
/usr/bin/hdiutil create -volname 'Prospect Intelligence' -srcfolder "$dmg_work/content" -fs HFS+ -format UDZO "$dmg_work/package.dmg"
/usr/bin/hdiutil verify "$dmg_work/package.dmg"
mv "$dmg_work/package.dmg" "dist/Prospect-Intelligence-$app_version-macOS.dmg"
/usr/bin/shasum -a 256 "dist/Prospect-Intelligence-$app_version-macOS.dmg" > "dist/Prospect-Intelligence-$app_version-macOS.dmg.sha256"
printf '%s\n' "Paquete DMG: dist/Prospect-Intelligence-$app_version-macOS.dmg"
