#!/bin/sh
set -eu
cd "$(dirname "$0")/.."

mode=${1:---repo}
FORBIDDEN_PATTERN='(^|/)(credentials\.env|\.env(\..*)?|prospectos\.db|settings\.json|templates\.toml|AGENTS\.md)$|\.(db|db-wal|db-shm|sqlite|sqlite3|xlsx|csv)$'

check_repo() {
    tracked=$(git ls-files)
    bad=$(printf '%s\n' "$tracked" | grep -E '(^|/)(AGENTS\.md|credentials\.env|\.env(\..*)?)$|\.(db|db-wal|db-shm|sqlite|sqlite3|xlsx|csv)$|^(data|exports|qa|qa-data)/' | grep -vE '^crates/core/tests/fixtures/' || true)
    if [ -n "$bad" ]; then
        printf 'Error de higiene en repositorio; archivos prohibidos rastreados por Git:\n%s\n' "$bad" >&2
        exit 1
    fi
    if [ -d .github/workflows ]; then
        bad_wf=$(grep -rnE -- '--features[^[:cntrl:]]*embedded-tokens|--all-features' .github/workflows/ || true)
        if [ -n "$bad_wf" ]; then
            printf 'Error: los workflows no deben activar embedded-tokens ni --all-features:\n%s\n' "$bad_wf" >&2
            exit 1
        fi
    fi
    printf '%s\n' 'Higiene del repositorio verificada: sin bases, contactos, credenciales ni AGENTS.md.'
}

check_ci_guard() {
    if CI=true DENUE_TOKEN=demo-embedded-test cargo check --locked -p prospect-core --features embedded-tokens >/dev/null 2>&1; then
        printf '%s\n' 'Error: embedded-tokens compiló aun con CI=true.' >&2
        exit 1
    fi
    printf '%s\n' 'Guardia CI de embedded-tokens verificada: falla al compilar bajo CI=true.'
}

check_archives() {
    if [ ! -d release-assets ]; then
        printf '%s\n' 'No existe release-assets/ para verificar archivos.' >&2
        exit 1
    fi
    python3 - <<'PY'
import pathlib, re, sys, tarfile, zipfile

forbidden = re.compile(
    r"(^|/)(credentials\.env|\.env(\..*)?|prospectos\.db|settings\.json|templates\.toml|AGENTS\.md)$|\.(db|db-wal|db-shm|sqlite|sqlite3|xlsx|csv)$",
    re.IGNORECASE,
)
checked = 0
for archive in sorted(pathlib.Path("release-assets").iterdir()):
    if archive.suffix == ".zip":
        checked += 1
        with zipfile.ZipFile(archive) as zf:
            names = zf.namelist()
    elif archive.name.endswith(".tar.gz"):
        checked += 1
        with tarfile.open(archive, "r:gz") as tf:
            names = [m.name for m in tf.getmembers()]
    else:
        continue
    bad = [n for n in names if forbidden.search(n.replace("\\", "/"))]
    if bad:
        print(f"Error de higiene en {archive}: {bad}", file=sys.stderr)
        sys.exit(1)
if checked == 0:
    print("No se encontraron archivos .zip o .tar.gz en release-assets/.", file=sys.stderr)
    sys.exit(1)
print(f"Higiene de archivos de release verificada ({checked} archivo(s)).")
PY
}

check_dist() {
    app_dir="dist/Prospect Intelligence.app"
    if [ ! -d "$app_dir" ]; then
        printf 'No existe %s\n' "$app_dir" >&2
        exit 1
    fi
    bad_bundle=$(find "$app_dir" -type f | grep -iE "$FORBIDDEN_PATTERN" || true)
    if [ -n "$bad_bundle" ]; then
        printf 'Error de higiene en bundle .app:\n%s\n' "$bad_bundle" >&2
        exit 1
    fi
    /usr/bin/plutil -lint "$app_dir/Contents/Info.plist"
    codesign --verify --deep --strict "$app_dir"

    if [ -d release-assets ]; then
        check_archives
    fi
    printf '%s\n' 'Higiene de artefactos de distribución verificada.'
}

case "$mode" in
    --repo) check_repo ;;
    --ci-guard) check_ci_guard ;;
    --archives) check_archives ;;
    --dist) check_dist ;;
    --all) check_repo; check_dist ;;
    *) printf 'Uso: %s [--repo|--ci-guard|--archives|--dist|--all]\n' "$0" >&2; exit 1 ;;
esac
