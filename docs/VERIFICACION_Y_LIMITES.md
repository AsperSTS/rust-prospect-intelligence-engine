# Estado de la migración

Aplicación personal local en Rust y Dioxus Desktop, con SQLite. No necesita suscripción, Python, Streamlit, servidor propio ni servicios de pago para funcionar. Las consultas necesitan conexión y tokens gratuitos de las fuentes oficiales.

## Alcance funcional

- Hoy, seguimientos, conversaciones y base paginada, búsqueda literal y filtros guardados.
- Perfiles separados para automatización/sistemas y web/marketing; CRM común para evitar repetir contactos.
- Puntajes explicados y borradores editables; copiar o abrir el canal nunca registra un envío.
- Registro manual de contactos, respuestas, notas, revisión, posposición y cierres.
- Importación DENUE con cursor persistente, enriquecimiento de sitios declarados y trabajos cancelables.
- Métricas de embudo, municipios y señales; mapa de calles OpenStreetMap con coordenadas DENUE y ficha del negocio.
- Oportunidades, contactos, siguientes acciones, importes ganados y cobros separados por moneda.
- Series Banxico e Indicadores INEGI almacenadas para lectura sin conexión.
- CSV y XLSX con archivo de atribución; respaldo SQLite consistente y recuperación a una ruta nueva.

## Datos migrados

Se copiaron 4,709 establecimientos, 448 señales web y 227 resultados de caché. La base original se conservó. La copia Rust agrega tablas sin destruir el esquema v2. Las pruebas de interfaz usan otra base aislada.

Los tokens quedan fuera del proyecto, en `~/Library/Application Support/Prospect Intelligence/credentials.env`, con permisos 600. No se incluyen en ejemplos, exportaciones de prueba ni gráficos de arquitectura.

## Comparación con Python

704 escenarios sintéticos comprueban puntajes, acciones, mensajes y seguimientos; hay pruebas adicionales de grupos, limpieza y redondeo. La comparación completa revisó 4,709 registros reales.

Se detectaron dos diferencias deliberadas en el motivo de agrupación: el dato `WWW.SEMILLASELRANCHERO.COM|` no es un dominio válido. Rust conserva el grupo por razón social y elimina la evidencia de dominio inválido. Los demás registros coincidieron.

Las pruebas de flujo cubren persistencia CRM al importar, doble clic, fechas reales de seguimiento, transacciones, cancelación, migración de esquemas antiguos, respaldo, recuperación, búsqueda, exportaciones, dinero, aislamiento de red y validación tipada.

## Dependencias y límites de seguridad

Cargo.lock fija versiones e integridad SHA-256 del registro. Eso no equivale a verificar firmas del autor ni attestations de todas las dependencias; Cargo no implementa `trustPolicy: no-downgrade`.

Se verificó una antigüedad mínima de siete días en los paquetes bloqueados. La lista permitida de 53 scripts nativos tiene versiones y hashes inventariados. Cargo requiere compiladores y generación de bindings para el webview, TLS y SQLite.

La comprobación cargo-deny aplica licencias, fuentes, versiones y avisos conocidos. Existe una excepción explícita para RUSTSEC-2024-0436: `paste` es una dependencia transitiva archivada; el aviso no describe una vulnerabilidad. Su reemplazo queda pendiente.

Se admite MPL-2.0 en la revisión básica, que exige evaluar obligaciones por archivo antes de distribuir. La auditoría legal profunda está pendiente, según lo solicitado. La revisión básica no es una conclusión jurídica.

El código propio prohíbe unsafe y no evalúa expresiones. La interfaz valida comandos con tipos Rust y Serde estrictos; Zod no se aplica al runtime Rust.

El enriquecimiento solo accede a IP públicas resueltas y fijadas, limita tamaño, páginas, concurrencia y tiempos. El parser se ejecuta en tareas del proceso; todavía no tiene sandbox de proceso ni límite duro de memoria.

El paquete local usa firma ad hoc. No se ha notarizado ni preparado un instalador para distribución pública.

## Mejoras posteriores

1. Validar diariamente resultados, filtros, seguimiento y conversión por oferta antes de ajustar puntajes.
2. Revisar licencias, permisos, atribución y conservación de datos por fuente y canal, con evidencia fechada. DENUE se considera autorizado conforme a la indicación del usuario.
3. Incorporar Censos Económicos y Marco Geoestadístico descargables; separar cobertura almacenada de universo del mercado.
4. Añadir diagnósticos por cliente: ventas, márgenes, flujo de caja, inventarios y operaciones mediante archivos del propio cliente.
5. Conectar censos y series macroeconómicas al territorio y actividad con unidades y periodos claros.
6. Revisar costos y condiciones antes de incorporar mapas alojados, correo, IA o sincronización remota. El núcleo local permanece gratuito.

Los archivos del cliente, cálculo de rutas y colaboración multiusuario no forman parte de esta primera versión personal.

## Resultado final de verificación

Clippy pasó sin advertencias; formato y compilación release correctos. Pasaron 20 pruebas Rust, con 704 escenarios sintéticos y comparación de 4,709 registros reales.

La aplicación nativa se validó en una base aislada: búsqueda, escritura rápida, notas persistentes al navegar, respuestas, separación de ofertas, métricas, oportunidades, importes exactos, preferencias Unicode y tema oscuro.

Semgrep ejecutó tres reglas locales sobre 16 archivos Rust, sin hallazgos ni errores de análisis. Es una revisión limitada de patrones; CodeQL no se ejecutó.

La comprobación cargo-deny terminó con advisories, bans, licenses y sources correctos bajo la excepción documentada de paste. Se inventariaron 416 paquetes alcanzables y los textos de licencia disponibles; los textos ausentes están registrados para la auditoría posterior.

En este equipo, release analizó 4,709 registros en 2.29 segundos. Una consulta filtrada con página de 60 registros tardó 0.03 segundos, incluyendo arranque de CLI. No se verificó rendimiento con 50,000 registros.

Se verificó el arranque del paquete final desde la carpeta Rust y la carga de los 4,709 establecimientos, además de los indicadores previamente almacenados. La app quedó abierta en Hoy.

El catálogo de nombres y unidades legibles de Indicadores INEGI sigue pendiente: actualmente se conserva el código oficial de unidad y el ID del indicador.

## Revisión 0.1.1

Se integró el icono SVG propio y sus formatos PNG/ICNS; se generaron app y DMG en dist/. La revisión actual suma 17 pruebas, con regresiones de carga/error, inicio sin base y formato numérico.

Se verificaron firma local, integridad del DMG, contenido mediante montaje de sólo lectura, ausencia de bases y tokens en el paquete y arranque de la versión final con 4,709 registros. La app quedó abierta en Hoy.

Los pasos de distribución están en APP_MACOS.md. Esta revisión mantiene pendiente la auditoría legal profunda y la notarización.


## Revisión 0.1.2

Interfaz compacta, navegación más estrecha y máximo tres vistas por grupo. Base no apila el listado y la ficha. Los borradores de mensajes y notas se conservan al alternar vistas.

Territorios sustituye los puntos aislados por calles OpenStreetMap, marcador del negocio seleccionado, zoom y acceso a su ficha. Sólo carga las imágenes visibles; usa HTTPS, identificación propia y caché de siete días. El mapa muestra carga, error y reintento. La comparación municipal permanece local.

La revisión suma 20 pruebas Rust e incorpora casos de coordenadas inválidas, escala y cobertura del mapa. No se agregaron dependencias. El mapa usa la proyección Web Mercator; se rechazan coordenadas fuera de su rango.

Se verificaron nativamente calles y marcador, zoom, selección territorial y acceso a la ficha. Las notas conservaron su borrador al cambiar de vista. A 1000 × 650 se comprobó la acción principal visible; los textos extensos y listas conservan desplazamiento cuando hace falta.

La app y el DMG 0.1.2 se generaron en el proyecto; firma local, checksum y montaje de sólo lectura correctos. El ejecutable del DMG coincide con el bundle. Se abrió la app final y cargó los 4,709 establecimientos sin cambios en el CRM personal.

## Revisión 0.2.0

27 pruebas Rust: presupuestos y reanudación, omisión de grupos completos, comparación de registros sin cambios, conservación del CRM, consultas fallidas, sitios fallidos elegibles por botón, ajustes antiguos y reinicios, municipios homónimos y paridad previa.

La interfaz se probó en dos directorios aislados: vacío sin tokens y base de 4,709 establecimientos con oportunidades ficticias de QA. Se comprobó persistencia del tema, metas, estados y presupuesto. Abrir la app, guardar ajustes y consultar el dashboard no inicia trabajos de fuentes.

La CLI también tiene lotes globales limitados y modos nuevos/reanudar/revisar. Configuración v2 y migración SQL Rust 2 son aditivas; la migración Rust 1 conserva su checksum. Los ajustes antiguos se respaldan antes de su primera escritura en v2.

No se realizaron nuevas consultas con tokens personales durante esta revisión. Las pruebas de red de importación usaron un proveedor inyectado. La vigencia se informa por grupo; trazabilidad por campo y detección de bajas quedan pendientes.

La ampliación BI, MCP y GitHub están descritos en HOJA_DE_RUTA_BI_MCP.md. No se instaló ni expuso un servidor MCP.

Se generaron app y DMG 0.2.0, con firma local y checksum verificados. El montaje de sólo lectura confirmó igualdad de ejecutables y ausencia de bases y credenciales. La edición personal abrió y conservó 4,709 registros, hash de campos CRM y credenciales; no registró nuevas consultas a fuentes.

Cargo-deny pasó con la excepción previa documentada. Semgrep aplicó tres reglas locales a 17 archivos Rust: cero hallazgos y errores. La revisión sigue sin equivaler a una auditoría legal, de seguridad o accesibilidad completa.

## Revisión 0.2.1

33 pruebas regulares Rust y benchmark aislado ejecutado. Se verificaron navegación inmediata desde caché, ofertas y filtros independientes, invalidación al guardar, escrituras externas y fallidas, límites LRU y errores con reintento.

Se conservan ambas ofertas y Hoy. Las fuentes permanecen manuales; el comprobador de cambios externos sólo lee metadatos SQLite locales. Las mediciones y límites están en [rendimiento de navegación](RENDIMIENTO_NAVEGACION.md).

La app y el DMG 0.2.1 tienen firma local y checksum verificados; sus ejecutables coinciden y el paquete no incluye tokens ni base. La edición personal mantiene sus datos y preferencias.
