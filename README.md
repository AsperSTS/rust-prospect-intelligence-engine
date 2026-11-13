# Prospect Intelligence

**Español** | [English](README.en.md)

[![CI](https://github.com/AsperSTS/rust-prospect-intelligence-engine/actions/workflows/ci.yml/badge.svg)](https://github.com/AsperSTS/rust-prospect-intelligence-engine/actions/workflows/ci.yml)
[![Release macOS & Windows](https://github.com/AsperSTS/rust-prospect-intelligence-engine/actions/workflows/release.yml/badge.svg)](https://github.com/AsperSTS/rust-prospect-intelligence-engine/actions/workflows/release.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

Herramienta local de prospección B2B para México. Convierte el padrón público del INEGI (DENUE) en una lista priorizada de clientes potenciales, con borradores de contacto y CRM integrado en SQLite.

![Lista priorizada](assets/portfolio/01-today-prioritized-list.png)

## Qué problema resuelve

La mayoría de las plataformas de prospección cobran suscripciones en dólares y tienen cobertura limitada fuera de EE. UU. **Prospect Intelligence** corre en tu equipo sin servidores externos y te permite:

- **Encontrar negocios reales:** importa establecimientos del DENUE por estado y actividad económica.
- **Priorizar a quién contactar hoy:** calcula *fit* y prioridad con reglas JSON auditables (giro, tamaño y señales detectadas en su sitio web).
- **Preparar mensajes sin automatizar envíos:** genera borradores contextuales y abre tu cliente de correo o canal con un clic; tú revisas y envías manualmente.
- **Llevar el seguimiento comercial:** notas, recordatorios, oportunidades en MXN/USD y métricas de conversión separadas por perfil de oferta.

## Plataformas

- **macOS (Apple Silicon) y iOS:** aplicación nativa en escritorio (`Prospect Intelligence.app`) y punto de entrada móvil en desarrollo; los releases de macOS se publican en [Releases](https://github.com/AsperSTS/rust-prospect-intelligence-engine/releases).
- **Windows (x64):** compilado, probado en CI y empaquetado como zip portable en [Releases](https://github.com/AsperSTS/rust-prospect-intelligence-engine/releases) (`ProspectIntelligence.exe` + `prospectos.exe`, aún pendiente de validación manual en hardware Windows real; requiere Microsoft Edge WebView2 Runtime, incluido en Windows 11).

## Primer uso

La app inicia vacía, sin datos ni credenciales. Al abrirla te guía para ingresar los tokens gratuitos (se guardan localmente en `credentials.env` con permisos `0600`; consulta [`credentials.env.example`](credentials.env.example)):

| Token | Obligatorio | Uso | Registro oficial |
|---|---|---|---|
| `DENUE_TOKEN` | Sí | Padrón de negocios INEGI | [API DENUE](https://www.inegi.org.mx/servicios/api_denue.html) |
| `INEGI_TOKEN` | Opcional | Indicadores económicos | [API Indicadores](https://www.inegi.org.mx/servicios/api_indicadores.html) |
| `BANXICO_TOKEN` | Opcional | Tipo de cambio USD/MXN | [API Banxico SIE](https://www.banxico.org.mx/SieAPIRest/service/v1/token) |

Los datos y respaldos se almacenan localmente en `~/Library/Application Support/Prospect Intelligence/`.

**Respaldo portátil:** en Ajustes → Tokens y fuentes, «Crear respaldo» guarda donde elijas un `.zip` con base, plantillas, ajustes y tokens (guárdalo en privado). En una instalación nueva, la primera pantalla permite «Cargar respaldo» en lugar de empezar de cero.

## Para desarrolladores

Workspace en **Rust** + **Dioxus** + **SQLite** (`rusqlite`):

- `crates/core` — dominio, persistencia SQLite, cliente DENUE, scoring determinista y métricas.
- `crates/desktop` — lanzador de escritorio (Dioxus Desktop).
- `crates/ui` — vistas y componentes compartidos.
- `crates/cli` — utilidades por línea de comandos y respaldos.
- `crates/mobile` — punto de entrada para iOS (en desarrollo).
- `config/` — reglas configurables de giros, señales web y plantillas (`giros.json`, `senales.json`, `templates.toml`).
- `scripts/` — empaquetadores (`bundle-macos.sh`, `bundle-windows.sh`, `bundle-ios.sh`) y guardias de higiene pública (`verify-public-hygiene.sh`).
- `legacy/` — variantes experimentales archivadas fuera del árbol activo (incluye `simple_ui`).

```sh
# Ejecutar en desarrollo
cargo run --locked -p prospect-desktop

# Pruebas, lint y auditoría de dependencias
cargo test --locked --workspace
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
cargo deny --offline --locked check
```

## Contribución, seguridad y licencia

- Cómo contribuir y checks de PR: [CONTRIBUTING.md](CONTRIBUTING.md)
- Reporte privado de vulnerabilidades: [SECURITY.md](SECURITY.md)
- Licencia [MIT](LICENSE) · Dependencias de terceros: [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md)
