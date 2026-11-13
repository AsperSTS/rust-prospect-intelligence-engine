#!/bin/sh
set -eu
cd "$(dirname "$0")/.."

if [ "$(uname -s)" != "Darwin" ]; then
    printf '%s\n' 'Este empaquetador necesita macOS.' >&2
    exit 1
fi

. ./scripts/_embedded_tokens.sh

TARGET="aarch64-apple-ios-sim"
RUN_SIM=0
PROFILE="debug"
CARGO_FLAG=""
EMBEDDED_TOKENS=0
TOKENS_FILE=""

while [ "$#" -gt 0 ]; do
    case "$1" in
        --device)
            TARGET="aarch64-apple-ios"
            ;;
        --simulator|--sim)
            TARGET="aarch64-apple-ios-sim"
            ;;
        --release)
            PROFILE="release"
            CARGO_FLAG="--release"
            ;;
        --run)
            RUN_SIM=1
            ;;
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
        *)
            printf 'Argumento desconocido: %s\n' "$1" >&2
            exit 1
            ;;
    esac
    shift
done

FEATURE_FLAG=""
if [ "$EMBEDDED_TOKENS" -eq 1 ]; then
    load_embedded_tokens "$TOKENS_FILE"
    FEATURE_FLAG="--features embedded-tokens"
else
    clear_token_env
fi

if [ "$RUN_SIM" -eq 1 ] && [ "$TARGET" = "aarch64-apple-ios" ]; then
    printf '%s\n' '--run sólo funciona con --simulator. Un iPhone físico requiere firma de desarrollo.' >&2
    exit 1
fi

if [ -d "/Applications/Xcode.app/Contents/Developer" ]; then
    export DEVELOPER_DIR="/Applications/Xcode.app/Contents/Developer"
fi

export IPHONEOS_DEPLOYMENT_TARGET=16.0

app_version=$(sed -n 's/^version = "\([^"]*\)"$/\1/p' Cargo.toml)
case "$app_version" in ''|*[!0-9.]*) printf '%s\n' 'Versión de workspace inválida.' >&2; exit 1;; esac

printf 'Compilando prospect-mobile para target %s (%s)...\n' "$TARGET" "$PROFILE"
# shellcheck disable=SC2086
cargo build --locked --target "$TARGET" -p prospect-mobile $CARGO_FLAG $FEATURE_FLAG

out_dir="dist/ios"
mkdir -p "$out_dir"
app="$out_dir/Prospect Intelligence.app"
if [ "$TARGET" = "aarch64-apple-ios" ]; then
    app="$out_dir/device/Prospect Intelligence.app"
elif [ "$RUN_SIM" -eq 1 ] && ! xcrun simctl list devices booted | rg -q "Booted"; then
    printf '%s\n' 'Inicia un simulador antes de usar --run.' >&2
    exit 1
fi
rm -rf "$app"
mkdir -p "$app"

bin_path="target/$TARGET/$PROFILE/prospect-mobile"
if [ ! -f "$bin_path" ]; then
    printf 'No se encontró el binario compilado en %s\n' "$bin_path" >&2
    exit 1
fi

# Raw PNG icons work in the local simulator bundle without embedding user data.
sips -z 120 120 assets/app-icon.png --out "$app/AppIcon60x60@2x.png" >/dev/null
sips -z 180 180 assets/app-icon.png --out "$app/AppIcon60x60@3x.png" >/dev/null
sips -z 152 152 assets/app-icon.png --out "$app/AppIcon76x76@2x.png" >/dev/null
sips -z 167 167 assets/app-icon.png --out "$app/AppIcon83.5x83.5@2x.png" >/dev/null
cp "$bin_path" "$app/prospect-mobile"
chmod +x "$app/prospect-mobile"

cat > "$app/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleDevelopmentRegion</key>
    <string>es</string>
    <key>CFBundleExecutable</key>
    <string>prospect-mobile</string>
    <key>CFBundleIdentifier</key>
    <string>com.asper.prospect-intelligence</string>
    <key>CFBundleInfoDictionaryVersion</key>
    <string>6.0</string>
    <key>CFBundleName</key>
    <string>Prospect Intelligence</string>
    <key>CFBundleDisplayName</key>
    <string>Prospect</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleShortVersionString</key>
    <string>$app_version</string>
    <key>CFBundleVersion</key>
    <string>1</string>
    <key>CFBundleIcons</key>
    <dict><key>CFBundlePrimaryIcon</key><dict><key>CFBundleIconFiles</key><array><string>AppIcon60x60</string></array></dict></dict>
    <key>CFBundleIcons~ipad</key>
    <dict><key>CFBundlePrimaryIcon</key><dict><key>CFBundleIconFiles</key><array><string>AppIcon76x76</string><string>AppIcon83.5x83.5</string></array></dict></dict>
    <key>LSApplicationQueriesSchemes</key>
    <array><string>mailto</string><string>tel</string><string>https</string><string>http</string></array>
    <key>UIFileSharingEnabled</key>
    <true/>
    <key>LSSupportsOpeningDocumentsInPlace</key>
    <true/>
    <key>UILaunchScreen</key>
    <dict/>
    <key>LSRequiresIPhoneOS</key>
    <true/>
    <key>MinimumOSVersion</key>
    <string>16.0</string>
    <key>UIDeviceFamily</key>
    <array>
        <integer>1</integer>
        <integer>2</integer>
    </array>
    <key>UIRequiredDeviceCapabilities</key>
    <array>
        <string>arm64</string>
    </array>
    <key>UISupportedInterfaceOrientations</key>
    <array>
        <string>UIInterfaceOrientationPortrait</string>
        <string>UIInterfaceOrientationLandscapeLeft</string>
        <string>UIInterfaceOrientationLandscapeRight</string>
    </array>
    <key>UISupportedInterfaceOrientations~ipad</key>
    <array>
        <string>UIInterfaceOrientationPortrait</string>
        <string>UIInterfaceOrientationPortraitUpsideDown</string>
        <string>UIInterfaceOrientationLandscapeLeft</string>
        <string>UIInterfaceOrientationLandscapeRight</string>
    </array>
</dict>
</plist>
PLIST

/usr/bin/plutil -lint "$app/Info.plist"

codesign --force --sign - "$app"
codesign --verify --deep --strict "$app"

printf 'Paquete iOS generado exitosamente en: %s\n' "$app"

if [ "$RUN_SIM" -eq 1 ]; then
    printf 'Instalando y ejecutando en simulador iniciado...\n'
    xcrun simctl install booted "$app"
    xcrun simctl launch booted com.asper.prospect-intelligence
    printf 'Aplicación ejecutada en el simulador.\n'
fi
