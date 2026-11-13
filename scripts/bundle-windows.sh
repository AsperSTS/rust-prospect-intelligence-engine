#!/bin/sh
set -eu
cd "$(dirname "$0")/.."

case "$(uname -s)" in
    MINGW*|MSYS*|CYGWIN*) ;;
    *)
        printf '%s\n' 'Este empaquetador debe ejecutarse en Windows (Git Bash / MSYS2).' >&2
        exit 1
        ;;
esac

. ./scripts/_embedded_tokens.sh

EMBEDDED_TOKENS=0
TOKENS_FILE=""
OFFLINE_FLAG=""

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
        --offline)
            OFFLINE_FLAG="--offline"
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

# shellcheck disable=SC2086
cargo build --release --locked $OFFLINE_FLAG $FEATURE_FLAG -p prospect-desktop -p prospect-cli

app_version=$(sed -n 's/^version = "\([^"]*\)"$/\1/p' Cargo.toml)
case "$app_version" in ''|*[!0-9.]*) printf '%s\n' 'Versión de workspace inválida.' >&2; exit 1;; esac

mkdir -p dist/windows/app dist/windows/cli
rm -rf dist/windows/app/* dist/windows/cli/*

cp target/release/prospect-desktop.exe "dist/windows/app/ProspectIntelligence.exe"
cp LICENSE "dist/windows/app/LICENSE"
if [ -f THIRD_PARTY_NOTICES.md ]; then cp THIRD_PARTY_NOTICES.md "dist/windows/app/"; fi
if [ -d docs/licenses ]; then cp -R docs/licenses "dist/windows/app/licenses"; fi

cp target/release/prospect-cli.exe "dist/windows/cli/prospectos.exe"
cp LICENSE "dist/windows/cli/LICENSE"
if [ -f THIRD_PARTY_NOTICES.md ]; then cp THIRD_PARTY_NOTICES.md "dist/windows/cli/"; fi

printf '%s\n' "Artefactos Windows preparados en dist/windows/ ($app_version)."
