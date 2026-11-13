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
import os, pathlib, platform, re, subprocess, sys, tarfile, tempfile, time, zipfile

def run_hdiutil(cmd, **kwargs):
    last_err = None
    for attempt in range(4):
        try:
            return subprocess.run(cmd, check=True, **kwargs)
        except subprocess.CalledProcessError as err:
            last_err = err
            if attempt == 3:
                raise
            time.sleep(2 * (attempt + 1))
    raise last_err

forbidden = re.compile(
    r"(^|/)(credentials\.env|\.env(\..*)?|prospectos\.db|settings\.json|templates\.toml|AGENTS\.md)$|\.(db|db-wal|db-shm|sqlite|sqlite3|xlsx|csv)$",
    re.IGNORECASE,
)
checked = 0
for archive in sorted(pathlib.Path("release-assets").iterdir()):
    if archive.is_dir():
        continue
    if archive.suffix == ".zip":
        checked += 1
        with zipfile.ZipFile(archive) as zf:
            names = zf.namelist()
    elif archive.name.endswith(".tar.gz"):
        checked += 1
        with tarfile.open(archive, "r:gz") as tf:
            names = [m.name for m in tf.getmembers()]
    elif archive.suffix == ".exe":
        checked += 1
        names = [archive.name]
        with archive.open("rb") as fh:
            if fh.read(2) != b"MZ":
                print(f"Error de formato PE en {archive}", file=sys.stderr)
                sys.exit(1)
    elif archive.suffix == ".dmg":
        checked += 1
        names = [archive.name]
        size = archive.stat().st_size
        if size < 512:
            print(f"Error: DMG vacío o corrupto en {archive}", file=sys.stderr)
            sys.exit(1)
        with archive.open("rb") as fh:
            fh.seek(size - 512)
            if fh.read(4) != b"koly":
                print(f"Error de firma UDIF en {archive}", file=sys.stderr)
                sys.exit(1)
        if platform.system() == "Darwin":
            run_hdiutil(["/usr/bin/hdiutil", "verify", str(archive)])
            with tempfile.TemporaryDirectory() as mnt:
                attached = False
                try:
                    run_hdiutil(
                        [
                            "/usr/bin/hdiutil",
                            "attach",
                            "-readonly",
                            "-nobrowse",
                            "-noverify",
                            "-noautoopen",
                            "-mountpoint",
                            mnt,
                            str(archive),
                        ],
                        stdout=subprocess.DEVNULL,
                    )
                    attached = True
                    for root, dirs, files in os.walk(mnt, followlinks=False):
                        dirs[:] = [d for d in dirs if not (pathlib.Path(root) / d).is_symlink()]
                        for fname in files:
                            fpath = pathlib.Path(root) / fname
                            if fpath.is_symlink():
                                continue
                            names.append(fpath.relative_to(mnt).as_posix())
                    app_inside = pathlib.Path(mnt) / "Prospect Intelligence.app"
                    if not app_inside.is_dir():
                        print(f"Falta Prospect Intelligence.app dentro de {archive}", file=sys.stderr)
                        sys.exit(1)
                    subprocess.run(
                        ["/usr/bin/plutil", "-lint", str(app_inside / "Contents/Info.plist")],
                        check=True,
                    )
                    subprocess.run(
                        ["/usr/bin/codesign", "--verify", "--deep", "--strict", str(app_inside)],
                        check=True,
                    )
                finally:
                    if attached:
                        subprocess.run(
                            ["/usr/bin/hdiutil", "detach", mnt, "-force"],
                            check=False,
                            stdout=subprocess.DEVNULL,
                            stderr=subprocess.DEVNULL,
                        )
    else:
        continue
    bad = [n for n in names if forbidden.search(n.replace("\\", "/"))]
    if bad:
        print(f"Error de higiene en {archive}: {bad}", file=sys.stderr)
        sys.exit(1)
if checked == 0:
    print("No se encontraron archivos de release verificables en release-assets/.", file=sys.stderr)
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
