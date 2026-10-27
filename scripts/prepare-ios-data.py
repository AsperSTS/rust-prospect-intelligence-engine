#!/usr/bin/env python3
"""Create an isolated, consistent Desktop snapshot without packaging credentials."""
import argparse
import os
from pathlib import Path
import shutil
import sqlite3


def prepare(source: Path, destination: Path) -> int:
    source = source.expanduser().resolve(strict=True)
    destination = destination.expanduser().resolve()
    if source == destination or source in destination.parents:
        raise ValueError("El destino debe estar fuera de la base personal de Desktop")
    if destination.exists():
        raise ValueError("El destino ya existe; elige una carpeta nueva para conservar sus datos")
    database = source / "prospectos.db"
    if not database.is_file():
        raise ValueError("No se encontró prospectos.db en el origen")
    destination.mkdir(parents=True, mode=0o700)
    try:
        # SQLite backup also includes committed WAL pages; the source is read only.
        with sqlite3.connect(database.as_uri() + "?mode=ro", uri=True) as original:
            with sqlite3.connect(destination / "prospectos.db") as snapshot:
                original.backup(snapshot)
                if snapshot.execute("PRAGMA integrity_check").fetchone()[0] != "ok":
                    raise ValueError("La copia no pasó la verificación de integridad")
                count = snapshot.execute("SELECT COUNT(*) FROM prospectos").fetchone()[0]
        for name in ("settings.json", "templates.toml"):
            if (source / name).is_file():
                shutil.copyfile(source / name, destination / name)
        for file in destination.iterdir():
            file.chmod(0o600)
    except Exception:
        # Only this newly created output can be removed; never the source.
        shutil.rmtree(destination)
        raise
    return count


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("destination", type=Path)
    parser.add_argument("--source", type=Path, default=Path.home() / "Library/Application Support/Prospect Intelligence")
    args = parser.parse_args()
    os.umask(0o077)
    try:
        count = prepare(args.source, args.destination)
    except (ValueError, OSError, sqlite3.Error) as error:
        parser.exit(1, f"No se preparó la copia: {error}\n")
    print(f"Copia aislada preparada: {count:,} establecimientos. Sin tokens ni exportaciones.")


if __name__ == "__main__":
    main()
