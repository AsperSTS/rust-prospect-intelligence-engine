# Prospect Intelligence — Rust + Dioxus · 0.2.1

Primera versión personal para prospección y seguimiento comercial. Funciona localmente en macOS; no requiere Python, Streamlit, servidor ni suscripciones.

## Abrir la aplicación

Doble clic en `Abrir app.command`, o en `dist/Prospect Intelligence.app`.

```sh
open "dist/Prospect Intelligence.app"
```

El paquete está compilado para Apple Silicon. Usa una firma local ad hoc para uso personal. También tienes `dist/Prospect-Intelligence-0.2.1-macOS.dmg` para instalarla mediante arrastre a Applications.

## Actualizar sólo cuando lo decides

El **Dashboard** lee SQLite y propone tres prospectos para empezar. No consulta APIs ni revisa sitios al abrirlo. **Hoy** mantiene su estructura.

En **Base → Gestionar datos** encontrarás:

- **Importar nuevos:** sólo segmentos que no comenzaron; los completos se omiten.
- **Reanudar pendientes:** retoma la siguiente página guardada, sin volver al principio.
- **Iniciar revisión:** reinicia explícitamente los grupos seleccionados; requiere cerrar los pendientes actuales.
- **Límite global:** cinco páginas por clic de forma predeterminada, hasta 200 registros por página. Puedes elegir entre una y veinte páginas en Ajustes. Cada página tiene como máximo tres intentos HTTP.
- Cada lote se detiene. La UI muestra intentos y cantidades nuevas, modificadas y sin cambios. El CRM se conserva y los registros públicos idénticos no se reescriben.

Las fechas reflejan consultas completas, no una publicación nueva de INEGI. Los días configurados sólo sugieren revisar. Sitios revisa hasta 100 dominios por clic: pendientes, antiguos o fallidos. Banxico e INEGI también requieren botones.

El recálculo periódico de prioridades ahora ocurre una vez al abrir y cuando cambia el día; los cambios del CRM siguen recalculando sus propios registros. No realiza consultas de red.

## Ajustes persistentes

**General:** apariencia claro/oscuro, firma y metas. **Datos:** selector de las 32 entidades, tamaños, presupuesto DENUE y días de revisión. **Fuentes:** tokens propios y respaldo. Guardar preferencias aplica los cambios; no inicia descargas.

Los ajustes se guardan atómicamente fuera del paquete, con versión de esquema. Antes de sustituir ajustes antiguos se conserva una copia privada en respaldos. Los tokens siguen en su archivo privado independiente.

Una instalación vacía muestra cómo configurar DENUE y elegir estados. Tu edición conserva su base y credenciales; no se exige volver a introducirlas.

## Empezar el día

1. Elige **Automatización y sistemas** o **Web y marketing**. Cada oferta tiene sus puntajes y mensajes; comparten el historial del establecimiento.
2. En **Hoy**, revisa contexto, evidencia y borrador. Copiar o abrir el canal no registra el envío.
3. Después de enviar tú, registra el contacto; revisa **Seguimientos** y **Conversaciones**.
4. Captura propuestas y acuerdos en **Oportunidades**. Separa importe de la oportunidad y dinero realmente cobrado.
5. Usa **Métricas**, **Territorios** e **Indicadores** para decidir qué actividad y municipio priorizar.

La interfaz compacta limita cada grupo a dos o tres vistas. Resumen, evidencia e historial se consultan por separado; cambiar de vista conserva los borradores.

En **Base** puedes importar DENUE, recalcular reglas, revisar sitios y exportar CSV/XLSX. Cada exportación incluye un archivo de atribución y filtros.

En **Territorios**, selecciona municipio y negocio para ver calles, marcador y zoom. La comparación municipal y el botón **Ver ficha** conectan el mapa con la prospección. El mapa usa OpenStreetMap sin clave ni suscripción y necesita conexión para vistas nuevas.

## Datos y fuentes

La copia migrada conserva 4,709 establecimientos, 448 señales web y 227 resultados de caché. La base Python original sigue disponible.

Los datos, respaldos, exportaciones, plantillas y credenciales permanecen en `~/Library/Application Support/Prospect Intelligence`. Los tokens quedan fuera del código con permisos 600.

DENUE e Indicadores INEGI usan el token compartido. Banxico usa el token SIE configurado. Las tres fuentes se validaron con consultas reales; los resultados económicos quedan almacenados para consulta sin conexión.

## Desarrollar y comprobar

```sh
cargo run --locked -p prospect-desktop
cargo test --locked --workspace
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
cargo deny --offline --locked check
./scripts/bundle-macos.sh
./scripts/package-dmg.sh
```

El empaquetador usa dependencias previamente descargadas. Para preparar un equipo nuevo, ejecuta primero `cargo fetch --locked`.

- `crates/core`: SQLite, reglas, mensajes, fuentes, trabajos y métricas.
- `crates/desktop`: interfaz Dioxus.
- `crates/cli`: importación, análisis, respaldo, recuperación y exportación.
- `config`: taxonomías, señales y plantillas originales.
- `docs`: investigación, verificaciones, dependencias y limitaciones.

La CLI compilada está en `dist/prospectos`. Usa `./dist/prospectos --help` para ver comandos.

## Estado de verificación

Pasaron las 33 pruebas Rust, con 704 escenarios sintéticos de paridad y verificaciones adicionales de grupos, textos y redondeo. Se compararon también los 4,709 registros reales.

Dos motivos de agrupación difieren al rechazar un dominio malformado; está documentado. Clippy pasa sin advertencias. cargo-deny pasa con una excepción explícita por una dependencia archivada.

Se probaron nativamente búsqueda, edición rápida, notas, navegación durante guardado, respuestas, perfiles separados, oportunidades, importes, preferencias Unicode, métricas, territorios y tema oscuro. Las pruebas usaron una base aislada.

El análisis completo medido en release tarda 2.29 s; una consulta filtrada medida tarda 0.03 s, incluyendo arranque de la CLI. Son mediciones de este equipo, no garantías de escala.

## Plan y mejoras

Consulta [PLAN_MIGRACION.md](PLAN_MIGRACION.md) y [verificaciones y límites](docs/VERIFICACION_Y_LIMITES.md). La auditoría legal profunda queda para después de la migración, conforme a lo solicitado.

El núcleo gratuito prioriza conseguir clientes. Censos Económicos, Marco Geoestadístico, archivos del cliente y análisis operativo forman parte de la ampliación posterior de BI.

El icono propio está en `assets/app-icon.svg`. Consulta [la revisión y distribución macOS](docs/APP_MACOS.md) para el estado 0.2.1 y los pasos de publicación.

## CLI con lotes acotados

```sh
./dist/prospectos pull --modo nuevos
./dist/prospectos pull --modo reanudar
./dist/prospectos pull --modo revisar --limite 1000
```

`--limite` es un presupuesto global, en múltiplos de 200 entre 200 y 4000. Si se omite, usa el presupuesto de Ajustes. Los modos de `run` respetan el mismo límite; su enriquecimiento adicional sigue siendo una acción manual explícita.

La ampliación de BI, MCP y preparación para GitHub está en [la hoja de ruta](docs/HOJA_DE_RUTA_BI_MCP.md).

## Navegación y caché 0.2.1

Dashboard, Base, Métricas y Territorios conservan instantáneas locales durante la sesión. Las claves incluyen oferta, filtros, dimensión, preferencias pertinentes y revisión de datos; volver a una vista no muestra otra pantalla de carga.

SQLite comparte lecturas repetidas y reúne los perfiles en una consulta. Guardar CRM, importar o enriquecer invalida sus resultados. La UI también invalida después de trabajos, preferencias y cambio de fecha.

Una comprobación de metadatos SQLite cada 60 segundos detecta escrituras de otra instancia local. No descarga fuentes ni recalcula registros. Ambas cachés tienen límites de entradas y se vacían al cerrar la app.

Ver [la medición y sus límites](docs/RENDIMIENTO_NAVEGACION.md). Hoy conserva su distribución; cambiar de oferta conserva puntajes, mensajes y métricas separados.
