#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
if [ "$(uname -s)" != "Darwin" ]; then
    printf '%s\n' 'Este empaquetador necesita macOS.' >&2
    exit 1
fi

. ./scripts/_embedded_tokens.sh

EMBEDDED_TOKENS=0
TOKENS_FILE=""
OFFLINE_FLAG="--offline"

while [ "$#" -gt 0 ]; do
    case "$1" in
        --embedded-tokens)
            EMBEDDED_TOKENS=1
            ;;
        --tokens-file)
            shift
            [ "$#" -gt 0 ] || { printf '%s\n' 'Falta la ruta para --tokens-file.' >&2; exit 1; }
            TOKENS_FILE=$1
            ;;
        --tokens-file=*)
            TOKENS_FILE=${1#--tokens-file=}
            ;;
        --online)
            OFFLINE_FLAG=""
            ;;
        *)
            printf 'Argumento desconocido: %s\n' "$1" >&2
            exit 1
            ;;
    esac
    shift
done

export MACOSX_DEPLOYMENT_TARGET="${MACOSX_DEPLOYMENT_TARGET:-12.0}"

FEATURE_FLAG=""
if [ "$EMBEDDED_TOKENS" -eq 1 ]; then
    load_embedded_tokens "$TOKENS_FILE"
    FEATURE_FLAG="--features embedded-tokens"
else
    clear_token_env
fi

./scripts/generate-icons.sh
# shellcheck disable=SC2086
cargo build --release --locked $OFFLINE_FLAG $FEATURE_FLAG --workspace
# shellcheck disable=SC2086
build_root=$(cargo metadata $OFFLINE_FLAG --locked --format-version 1 --no-deps | /usr/bin/plutil -extract target_directory raw -o - -)
app_version=$(sed -n 's/^version = "\([^"]*\)"$/\1/p' Cargo.toml)
case "$app_version" in ''|*[!0-9.]*) printf '%s\n' 'Versión de workspace inválida.' >&2; exit 1;; esac
mkdir -p dist
bundle_work=$(mktemp -d "$PWD/dist/.bundle.XXXXXX")
app="$bundle_work/Prospect Intelligence.app"
final_app="$PWD/dist/Prospect Intelligence.app"
cleanup() {
    if [ ! -d "$final_app" ] && [ -d "$bundle_work/previous.app" ]; then
        mv "$bundle_work/previous.app" "$final_app"
    fi
    rm -rf "$bundle_work"
}
trap cleanup EXIT HUP INT TERM
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
cp "$build_root/release/prospect-desktop" "$app/Contents/MacOS/ProspectIntelligence"
cp "$build_root/release/prospect-cli" "$bundle_work/prospectos"
cat > "$app/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleIdentifier</key><string>local.prospect.intelligence</string>
<key>CFBundleName</key><string>Prospect Intelligence</string>
<key>CFBundleDisplayName</key><string>Prospect Intelligence</string>
<key>CFBundleExecutable</key><string>ProspectIntelligence</string>
<key>CFBundlePackageType</key><string>APPL</string>
<key>CFBundleShortVersionString</key><string>$app_version</string>
<key>CFBundleVersion</key><string>5</string>
<key>CFBundleIconFile</key><string>AppIcon.icns</string>
<key>LSMinimumSystemVersion</key><string>12.0</string>
<key>NSHighResolutionCapable</key><true/>
<key>NSHumanReadableCopyright</key><string>Prospect Intelligence — MIT</string>
</dict></plist>
PLIST
cp assets/AppIcon.icns "$app/Contents/Resources/AppIcon.icns"
cp assets/app-icon.svg "$app/Contents/Resources/app-icon.svg"
cp LICENSE "$app/Contents/Resources/LICENSE"
if [ -d docs/licenses ]; then cp -R docs/licenses "$app/Contents/Resources/"; fi
if [ -f THIRD_PARTY_NOTICES.md ]; then cp THIRD_PARTY_NOTICES.md "$app/Contents/Resources/"; fi
/usr/bin/plutil -lint "$app/Contents/Info.plist"
codesign --force --sign - "$app"
codesign --verify --strict "$app"
# Publish only a fully built, validated bundle; retain the previous app on failure.
if [ -d "$final_app" ]; then mv "$final_app" "$bundle_work/previous.app"; fi
mv "$app" "$final_app"
mv "$bundle_work/prospectos" dist/prospectos
printf '%s\n' "Aplicación: $final_app ($app_version)"
