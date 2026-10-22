# Rust, Dioxus y arquitectura local para prospección y BI

## ¿Qué modalidad y versión de Dioxus convienen?

### Takeaway

Recomiendo Dioxus Desktop para macOS, con núcleo Rust compartido por interfaz y CLI. La candidata observable es 0.7.10; aprobarla después de una prueba técnica, sin prometer estabilidad de APIs futuras.

### Cited Findings

- GitHub marca v0.7.10 como última versión estable; sus activos muestran publicación del 30 julio 2026. 0.8.0 aparece como alpha. — [Releases oficiales](https://github.com/DioxusLabs/dioxus/releases).
- La documentación publicada de `dioxus` y `dioxus-desktop` confirma 0.7.10. El manifest declara `MIT OR Apache-2.0`. — [Dioxus](https://docs.rs/dioxus/0.7.10/dioxus/), [Desktop](https://docs.rs/dioxus-desktop/0.7.10/dioxus_desktop/), [Manifest](https://docs.rs/crate/dioxus/0.7.10/source/Cargo.toml).
- Desktop ejecuta Rust nativo y representa HTML/CSS mediante WebView del sistema, basado en Wry. El código Rust tiene acceso a archivos y APIs nativas. — [Guía Desktop 0.7](https://dioxuslabs.com/learn/0.7/guides/platforms/desktop/).
- Web compila a WASM y manipula DOM. Fullstack divide compilación cliente/servidor mediante features y usa Axum. La misma UI no elimina diferencias de plataforma. — [Plataformas](https://dioxuslabs.com/learn/0.7/guides/platforms/), [Fullstack](https://dioxuslabs.com/learn/0.7/essentials/fullstack/project_setup/), [Axum](https://dioxuslabs.com/learn/0.7/essentials/fullstack/axum/).
- El renderer Native basado en WGPU/Blitz sigue descrito como experimental. LiveView está depriorizado; no recomendarlo para esta migración. — [Estructura del proyecto](https://dioxuslabs.com/learn/0.7/beyond/project_structure/).
- Los signals reaccionan a lecturas/escrituras; context permite compartir servicios y estado. `use_resource` reinicia futuros ante cambios y requiere seguridad al cancelar. — [Signals](https://dioxuslabs.com/learn/0.7/essentials/basics/signals/), [Context](https://dioxuslabs.com/learn/0.7/essentials/basics/context/), [Resources](https://dioxuslabs.com/learn/0.7/essentials/basics/resources/).
- Router ofrece rutas tipadas con `Routable`, layouts y `Link`; Desktop usa historial en memoria. — [Router API](https://docs.rs/dioxus-router/latest/dioxus_router/), [Navegación](https://dioxuslabs.com/learn/0.7/essentials/router/navigation/).
- `use_coroutine` facilita acciones asincrónicas mediante canales, pero su canal incorporado no es acotado; el desarrollador debe diseñar sincronización. — [Coroutine API](https://docs.rs/dioxus-hooks/latest/dioxus_hooks/fn.use_coroutine.html).
- Desktop permite JavaScript con `document::eval`, `send` y `recv`. `web-sys` no ofrece portabilidad directa al renderer Desktop; mapas Canvas/WebGL requieren una integración específica. — [Desktop](https://dioxuslabs.com/learn/0.7/guides/platforms/desktop/), [Escape hatches](https://dioxuslabs.com/learn/0.7/essentials/ui/escape/).
- Dioxus 0.7.10 lanza/adopta runtime Tokio multihilo con `tokio_runtime`; rechaza runtime `CurrentThread`. Su código incluye llamadas internas a eval para ciertos eventos Windows. — [Código versionado](https://github.com/DioxusLabs/dioxus/blob/v0.7.10/packages/desktop/src/launch.rs).
- El anuncio del 10 septiembre 2026 confirma incorporación del equipo a Cognition y continuidad declarada de proyectos abiertos, con más énfasis en Native/Blitz. No garantiza plazos ni soporte contractual. — [Anuncio oficial](https://dioxuslabs.com/blog/joining-cognition/).

### Inferences

- Candidata: `dioxus = "=0.7.10"`, Desktop y Router; `dx` del release 0.7.10. Registrar versiones reales y conservar `Cargo.lock`; comprobar compatibilidad de subcrates, toolchain y bundler antes de fijarlas.
- La app local puede operar sin servidor contratado: SQLite, búsqueda, filtros, CRM, puntuación, exportación y paneles usan procesos locales. HTTP sólo cuando el usuario solicita actualizar o enriquecer.
- No migrar línea por línea: traducir estado Streamlit a componentes, signals y servicios. Mantener reglas de negocio fuera de componentes y evitar consultas/persistencia dentro del render.
- Usar `use_resource` para lecturas cancelables; acciones CRM y lotes deben ir a servicios con identificadores, transacciones, progreso, cancelación cooperativa y colas acotadas, independientes de la vida del componente.
- Preferir CSS local, SVG y componentes propios pequeños. Una tabla paginada evita renderizar miles de filas y reduce dependencia de bibliotecas UI aún cambiantes.
- MVP geográfico: SVG local con puntos, capas GeoJSON y eventos Rust, más apertura opcional de coordenadas en navegador. Sin llamadas directas de aplicación a `document::eval`.
- Mapas Leaflet/MapLibre quedan opcionales. Requieren una decisión específica sobre puente JavaScript, CSP, assets locales, validación de mensajes y proveedores de mapas; no asumir que cero licencias implica servicio ilimitado.
- La prohibición de eval debe distinguir código de aplicación y funcionamiento interno del renderer: Dioxus Desktop ya usa evaluación JavaScript internamente. SVG evita agregar puentes explícitos, pero no elimina ese mecanismo del framework.
- Web/fullstack queda posterior a paridad Desktop. Aislar persistencia/HTTP nativos por features o adaptadores; el navegador no adquiere automáticamente acceso al SQLite local o a secretos sólo por compartir código.
- Prueba técnica obligatoria: ventana macOS, 0.7.10 coherente, tabla y SVG con datos representativos, consulta SQLite fuera del hilo UI, HTTP cancelable, guardado/exportación y bundle sin Python instalado.

### Gaps

- La CLI no expuso una página docs.rs accesible durante esta revisión. Los activos GitHub están disponibles; `dx --version`, instalación inspeccionada y compatibilidad deben verificarse cuando se autorice implementación.
- No medir tamaño final, RAM, rendimiento o accesibilidad sin compilar la aplicación concreta. La afirmación comercial de tamaños pequeños del framework no es un presupuesto de esta app.
- No se ejecutaron pruebas Desktop, integraciones de mapas, empaquetado ni compilaciones. La recomendación arquitectónica necesita el spike descrito.

## ¿Qué stack conserva paridad, seguridad y datos?

### Takeaway

Mantener SQLite y consultas SQL agregadas en el MVP. Recomiendo evaluar primero `rusqlite` con trabajador dedicado; `sqlx` es alternativa, no una segunda capa simultánea. Runtime final enteramente Rust.

### Cited Findings

- `rusqlite` ofrece conexiones, consultas parametrizadas, transacciones y API de backup SQLite; es una envoltura de SQLite, no un motor diferente. — [API rusqlite](https://docs.rs/rusqlite/latest/rusqlite/).
- `sqlx` aporta conexiones asincrónicas, pools y migraciones. SQLite ofrece opciones WAL, claves foráneas y timeout de bloqueos. — [SQLx](https://docs.rs/sqlx/latest/sqlx/), [SQLite options](https://docs.rs/sqlx/latest/sqlx/sqlite/struct.SqliteConnectOptions.html), [Migrator](https://docs.rs/sqlx/latest/sqlx/migrate/struct.Migrator.html).
- `sqlx::migrate!` incorpora migraciones; detectar nuevos archivos en stable necesita un build script con `rerun-if-changed=migrations`. — [Macro migrate](https://docs.rs/sqlx/latest/sqlx/macro.migrate.html).
- WAL permite lectores junto con escritor, pero sólo un escritor simultáneo y no funciona entre máquinas sobre sistemas de archivos de red. `SQLITE_BUSY` sigue siendo posible. — [SQLite WAL](https://sqlite.org/wal.html).
- SQLite corrigió un bug de corrupción WAL en 3.51.3, con backports 3.44.6 y 3.50.7. Importa el motor enlazado real, no sólo versión del crate. — [WAL-reset bug](https://sqlite.org/wal.html#walreset).
- Copiar un archivo SQLite activo puede producir problemas; la API de backup crea una copia consistente mediante el motor. El WAL forma parte del estado persistido. — [Backup API](https://sqlite.org/backup.html), [WAL](https://sqlite.org/wal.html).
- Reqwest ofrece HTTP asincrónico sobre Tokio, pooling y JSON. Por defecto sigue hasta diez redirecciones y respeta proxies del sistema. — [Reqwest](https://docs.rs/reqwest/latest/reqwest/), [ClientBuilder](https://docs.rs/reqwest/latest/reqwest/struct.ClientBuilder.html).
- `spawn_blocking` evita bloquear el executor, pero no permite abortar trabajo iniciado; CPU intenso requiere límites de concurrencia. — [Tokio spawn_blocking](https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html).
- Serde ignora campos desconocidos por defecto. `deny_unknown_fields` los rechaza y es incompatible con `flatten`; deserializar tipos no valida automáticamente invariantes del negocio. — [Serde attributes](https://serde.rs/container-attrs.html).
- OWASP recomienda validar destinos y direcciones resueltas; redirecciones y una segunda resolución DNS no verificada pueden eludir controles SSRF. — [OWASP SSRF](https://cheatsheetseries.owasp.org/cheatsheets/Server_Side_Request_Forgery_Prevention_Cheat_Sheet.html).
- `scraper` analiza HTML con selectores CSS. `rust_xlsxwriter` crea XLSX, formatos, tablas y gráficos; no lee ni modifica archivos existentes. `csv` facilita importación/exportación tabular. — [Scraper](https://docs.rs/scraper/latest/scraper/), [XLSX](https://docs.rs/rust_xlsxwriter/latest/rust_xlsxwriter/), [CSV](https://docs.rs/csv/latest/csv/).
- DuckDB dispone de cliente Rust y acceso a SQLite mediante extensión. Su modelo analítico y concurrencia difieren de SQLite; no implica una sustitución transparente del CRM. — [Cliente Rust](https://duckdb.org/docs/current/clients/rust), [SQLite extension](https://duckdb.org/docs/current/core_extensions/sqlite), [Concurrency](https://duckdb.org/docs/current/connect/concurrency).
- Polars ofrece API lazy y optimización de consultas; sus capacidades añaden features y dependencias. — [Lazy](https://docs.pola.rs/user-guide/lazy/), [Instalación](https://docs.pola.rs/user-guide/installation/).
- `cargo build --locked` exige el lockfile sin resolver cambios; `--frozen` combina locked y offline. No garantiza ausencia de código ejecutado durante compilación. — [Cargo build](https://doc.rust-lang.org/cargo/commands/cargo-build.html).
- Cargo compila y ejecuta `build.rs`; se utiliza también para bibliotecas C. Las macros procedimentales son otro punto de ejecución durante compilación. — [Build scripts](https://doc.rust-lang.org/cargo/reference/build-scripts.html), [Controles experimentales](https://doc.rust-lang.org/cargo/reference/unstable.html#build-scripts-and-proc-macros).
- Cargo-deny revisa advisories, licencias, sources y bans. Su configuración incluye controles de scripts mediante `build`; no reemplaza una sandbox del sistema. — [Cargo-deny](https://embarkstudios.github.io/cargo-deny/), [Bans config](https://embarkstudios.github.io/cargo-deny/checks/bans/cfg.html).

### Inferences

- Stack provisional: Rust stable fijado, Dioxus Desktop/Router, rusqlite, Tokio, Reqwest, Serde/serde_json, scraper, rust_xlsxwriter y csv. Elegir features mínimos y versiones después de revisar compatibilidad/licencias/build scripts.
- Un trabajador SQLite propietario de la conexión simplifica la exclusión de escrituras. UI y CLI usan los mismos comandos/repositorios; consultas largas y exportaciones nunca corren sobre el hilo UI.
- `sqlx` conviene si el spike demuestra valor de migraciones integradas y servicios async; no añadir ambos drivers sin necesidad comprobada. Sus macros/SQLite enlazado también requieren revisión de compilación.
- Dominio con tipos propios: identificador DENUE, coordenadas válidas, etapa CRM y contacto. Comandos internos/IPC estrictos con Serde, campos rechazados y validación explícita de rangos, longitud y transiciones.
- DTO externo DENUE separado: aceptar ampliaciones documentadas del proveedor, registrar cambios y rechazar campos indispensables ausentes o mal tipados. Convertir al dominio validado; nunca usar JSON externo como estado interno directamente.
- Zod corresponde al mundo TypeScript; no es requisito técnico del Rust puro. Aplicar la intención de schemas estrictos con Serde y validación propia; Zod sólo tendría lugar en un puente JS futuro.
- Paridad primero: copiar mediante backup a destino distinto, verificar integridad y mantener esquema v2 inicialmente. Nunca dejar Streamlit y Rust modificando simultáneamente el archivo migrado durante el corte.
- Añadir migraciones numeradas, checksums, backup previo y pruebas de ida/errores. Comparar IDs, conteos, valores nulos, JSON, puntuaciones, relaciones y exportaciones; no limitar verificación a que el archivo abra.
- Preservar JSON de fuente, normalización, procedencia, fecha de consulta y errores. La compatibilidad funcional necesita fixtures comparables; 1,097 tests Python no se traducen automáticamente en 1,097 tests Rust.
- Riesgo de paridad observado por coordinador: `round()` Python utiliza empate al par; `f64::round` no debe sustituirlo automáticamente. Fijar resultados esperados de scoring y redondeo antes del port.
- Conservar reglas exactas de Unicode, acentos, espacios y case-folding. Fixtures deben cubrir nombres mexicanos y claves de deduplicación; no reemplazar normalización por ASCII sin evidencia funcional.
- Mantener SHA-1 determinista del identificador para seleccionar variantes actuales; no usar `DefaultHasher`. Este hash organiza variantes, no representa un control criptográfico de seguridad.
- Conservar inicialmente fechas locales naïve y seguimiento real: hoy+4/+7 según envío, pese a rótulos D+7/D+14. Documentar inconsistencia; corregir semántica y migrar fechas sólo en fase posterior.
- Futuro comercial: conservar cuentas/contactos compartidos, scores por `profile_id` y `scoring_version`, oportunidades por oferta y supresión de contacto duplicado. Introducir ese modelo después de paridad de v2.
- Web enrichment: cliente distinto al cliente INEGI, destinos sólo HTTP(S), sin credenciales, límites de tiempo/tamaño/descompresión, bloqueo de loopback/privadas/link-local/metadata y control por salto/DNS; sin ejecutar HTML remoto.
- El parser HTML debe extraer señales; UI muestra texto escapado y evidencia, no HTML empresarial incrustado. TLS válido siempre; no convertir errores SSL en permiso para continuar.
- Para trabajo hostil que deba detenerse forzosamente, usar proceso aislado con límites. Un timeout sobre `spawn_blocking` o worker Rust no prueba aislamiento, límite de memoria ni detención efectiva.
- Excel: escribir teléfonos/URLs/texto como strings, tipos numéricos explícitos; no interpretar datos externos como fórmulas. CSV requiere tratamiento explícito de celdas que empiezan con caracteres de fórmula.
- No añadir DuckDB/Polars con 4,709 registros/19 MiB reportados por el inventario. SQL paginado y agregados bastan como punto inicial; medir antes de aumentar dependencias y tiempo de compilación.
- DuckDB podría procesar snapshots/Parquet para comparaciones longitudinales y agregaciones grandes; SQLite sigue como fuente operacional. Polars sólo si pipelines de transformación justifican su coste técnico.
- Aplicar `cargo fmt --check`, `cargo clippy`, tests y build con `--locked`, más `cargo deny check`; revisar scripts/macros, procedencia y antigüedad mínima antes de actualizar. Exceptions de seguridad deben quedar documentadas.
- La política pnpm `allowBuilds`/`trustPolicy` no se traslada literalmente a Cargo. `Cargo.lock` y hashes no equivalen a firmas de autor ni attestations; registrar alcance real del control elegido.
- Usar archivos de datos en directorio de aplicación del usuario; el código del repositorio destino y datos persistentes son conceptos separados. No depender del cwd ni escribir dentro del bundle instalado.

### Gaps

- El investigador no inspeccionó código fuente ni esquema del proyecto. Tamaño, registros, v2 y tests fueron aportados por el coordinador; todos los detalles de migración deben ligarse a su inventario verificado.
- No se fijaron versiones de cada dependencia: primero compatibilidad de MSRV, SQLite corregido, licencias, scripts y cooldown. Un manifest prescriptivo sin resolverlo sería una promesa falsa.
- No se ensayó `cargo-deny build` sobre el árbol Dioxus ni se comprobó soporte de firmas de paquetes en el entorno. No afirmar que esos controles ya están activos.

## ¿Qué partes son realmente gratuitas y cómo distribuir?

### Takeaway

El uso local puede evitar suscripciones de servidor y de software. Distribuir ampliamente aplicaciones firmadas sí puede generar costes; por eso el objetivo inicial debe ser uso propio en macOS.

### Cited Findings

- Dioxus se publica con licencia MIT o Apache-2.0. SQLite está en dominio público y no requiere licencia pagada de uso. — [Manifest Dioxus](https://docs.rs/crate/dioxus/0.7.10/source/Cargo.toml), [SQLite copyright](https://sqlite.org/copyright.html).
- `dx bundle` soporta macOS `.app/.dmg`, Windows `.exe/.msi` y otros formatos; la guía indica compilación sobre plataforma/arquitectura nativa y recomienda matriz CI para plataformas múltiples. — [Bundling](https://dioxuslabs.com/learn/0.7/tutorial/bundle/).
- La guía de bundling menciona firma/notarización según plataforma; los releases 0.7 incluyen correcciones específicas de bundling y firma macOS. — [Guía](https://dioxuslabs.com/learn/0.7/tutorial/bundle/), [Releases](https://github.com/DioxusLabs/dioxus/releases).
- Apple ofrece cuenta gratuita para pruebas propias, pero Apple Developer Program cuesta USD 99 anuales. Developer ID se usa para distribución fuera del Mac App Store y notarización. — [Apple Program](https://developer.apple.com/programs/), [Developer ID](https://developer.apple.com/help/account/certificates/create-developer-id-certificates/).
- Windows 11 incluye WebView2 Evergreen; algunos dispositivos necesitan instalación adicional. Microsoft documenta bootstrapper y un instalador completo offline. — [WebView2 distribución](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/distribution).
- Microsoft advierte que firma válida no garantiza reputación inmediata SmartScreen. Smart App Control puede bloquear binarios sin firma/reputación según política del equipo. — [SmartScreen](https://learn.microsoft.com/en-us/windows/apps/package-and-deploy/smartscreen-reputation).

### Inferences

- Clasificación de gratuidad: biblioteca abierta sin tarifa de licencia; API gratuita con token/cuota/condiciones; servicio alojado con plan gratuito cambiante; gasto de infraestructura/distribución. No usar estas categorías como sinónimos.
- MVP propio: compilación y ejecución en la Mac existente, datos locales, backups locales y actualizaciones manuales. Coste incremental de servicios puede ser cero; hardware, electricidad, conexión y trabajo siguen existiendo.
- No contratar hosting para Desktop. Versiones web compartidas necesitan servidor o alojamiento adecuado; un free tier no garantiza continuidad, capacidad suficiente o ausencia de pagos para uso comercial.
- Distribución macOS pública y cómoda: presupuestar Developer Program, certificados, notarización y prueba de Gatekeeper. No prometer que un `.dmg` sin firma se instala sin fricción.
- Windows: tener acceso a equipo/runner Windows, validar installer y WebView2, probar en máquina limpia. Revisar firma y elegibilidad del proveedor; no recomendar pagar EV para evitar todos los avisos.
- Primera entrega macOS; Windows después de la prueba nativa. Arquitectura Rust reutilizable no implica empaquetado universal desde una sola Mac ni coste CI garantizado cero.
- Mensajería manual: preparar/copiar textos y abrir enlace validado. No añadir proveedor de correo/WhatsApp, envío automático ni API pagada para lograr paridad funcional.
- Release sin Python: bundle contiene sólo ejecutable Rust, assets locales y dependencias nativas requeridas. Probarlo en entorno sin Python; migrador Rust y CLI deben compartir núcleo, no invocar scripts Python.
- Cualquier CDN, mapa remoto, enriquecimiento externo o IA transforma requisitos de disponibilidad y condiciones de uso. Debe ser optativo, con alternativa local y explicaciones específicas cuando requiera Internet.

### Gaps

- No se verificaron tarifas de firma Windows, cuotas de runners ni coste de distribuir por tiendas; varían por cuenta/proveedor. Evitar cifras inventadas y decidirlas sólo al ampliar distribución.
- No se verificó acceso del usuario a máquina Windows o cuenta Apple pagada. Estos requisitos no bloquean planificación ni uso local en macOS.
- Las cuotas y licencias de datos INEGI/mapas corresponden al investigador de fuentes de datos; este archivo no certifica gratuidad ilimitada de ninguna API.
