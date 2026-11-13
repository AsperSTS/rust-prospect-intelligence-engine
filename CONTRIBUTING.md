# Contribuir

Las mejoras y correcciones son bienvenidas. Mantén los cambios acotados y probados.

## Requisitos

- Toolchain definido en [rust-toolchain.toml](rust-toolchain.toml) (`stable`, `rustfmt`, `clippy`).
- macOS para compilar Desktop e iOS (`prospect-desktop`, `prospect-mobile`).

## Antes de abrir un Pull Request

Ejecuta estas verificaciones en la raíz del workspace:

```sh
./scripts/verify-public-hygiene.sh --repo
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace
cargo deny --offline --locked check
```

## Reglas del repositorio

- No subas tokens, `credentials.env`, archivos `.db`/`.sqlite`, exportaciones ni datos reales de contacto.
- No uses `unsafe` en código propio ni desactives la validación TLS.
- No modifiques migraciones SQL ya publicadas; crea una nueva versión aditiva si cambia el esquema.
- No actives `--features embedded-tokens` en workflows ni en artefactos pensados para distribución.
