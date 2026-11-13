# Código archivado (`legacy/`)

Este directorio conserva piezas de interfaz experimentales retiradas del workspace activo para no interferir con la aplicación principal (`crates/ui/src/ui.rs`):

- `simple_ui/simple_ui.rs` y `simple_ui/simple_ui.css`: flujo simplificado introducido en `1a8418f` (*Simplify prospect workflow and expand outreach templates*). La aplicación Desktop e iOS utiliza directamente `ui::App` (`Dashboard`, `Hoy`, `Seguimientos`, `Conversaciones`, `Base`, `Métricas`, `Territorios`, `Oportunidades`, `Indicadores` y `Ajustes`).
