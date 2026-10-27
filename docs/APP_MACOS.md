# Aplicación macOS 0.2.1

La aplicación de escritorio ya está generada en el directorio del proyecto. Se abre con doble clic y no requiere Python, Streamlit ni Rust instalado para ejecutarse.

## Entregables

- `dist/Prospect Intelligence.app`: aplicación compilada para Apple Silicon (arm64).
- `dist/Prospect-Intelligence-0.2.1-macOS.dmg`: paquete para arrastrar la app a Applications.
- `dist/Prospect-Intelligence-0.2.1-macOS.dmg.sha256`: huella de integridad del paquete.
- `assets/app-icon.svg`: icono vectorial editable, creado para este proyecto.
- `assets/app-icon.png`: render de 1024 px.
- `assets/AppIcon.icns`: variantes del icono para macOS.

El símbolo combina una lupa y barras ascendentes: prospección y análisis. Usa la paleta azul oscuro y menta de la app, sin fuentes ni recursos gráficos externos. El SVG es la fuente de los formatos generados.

Los datos, tokens y respaldos permanecen en `~/Library/Application Support/Prospect Intelligence`. El paquete incluye exclusivamente ejecutable, iconos y avisos de licencias; no incluye la base comercial ni credenciales.

## Revisión de esta versión

| Hallazgo | Corrección | Verificación |
|---|---|---|
| Paquete sin identidad visual propia | SVG, PNG e ICNS integrados en sidebar y bundle | Render e Info.plist comprobados |
| Métricas y otras pantallas confundían carga/error con datos vacíos | Carga explícita, errores visibles y botón de reintento | Prueba reactiva y fallo reversible en base aislada |
| Configuración inválida cerraba la app con mensaje sólo en consola | Pantalla de recuperación con carpeta y cierre nativo | Arranque inválido, cierre y nuevo arranque correcto |
| Valores oficiales con demasiados ceros decimales | Formato legible sin conversión a float | Casos de precisión, negativos y valores no numéricos |
| Reemplazo del bundle en uso antes de verificarlo | Construcción temporal, firma y verificación antes de publicar | Empaquetado reproducible local |

Pasaron 33 pruebas Rust en total: 26 del núcleo y siete de interfaz. La paridad conserva 704 escenarios sintéticos. Clippy y formato pasan; cargo-deny mantiene la excepción documentada de paste. Semgrep ejecutó tres reglas locales sin hallazgos.

La revisión no equivale a una auditoría completa de accesibilidad, seguridad ni compatibilidad con todos los Macs. La arquitectura compilada es arm64; Intel requiere otra compilación o un bundle universal.

## Interfaz compacta 0.1.2

La navegación ocupa 176 px y baja a 156 px en ventanas pequeñas. Se redujeron márgenes, cabeceras y separación entre controles; ambos temas usan tonos más suaves.

Contexto tiene Resumen, Evidencia e Historial. Acciones separa Mensaje, Avance y Notas. Base alterna listado y ficha, evitando apilar ambos. Oportunidades y Configuración tienen tres vistas cada una. Los borradores permanecen montados al alternarlas.

Métricas muestra resultados e importes juntos y evita grandes paneles vacíos. Territorios muestra calles reales, el marcador del negocio seleccionado, zoom, selección municipal y acceso directo a su ficha.

El mapa solicita únicamente imágenes del área visible mediante HTTPS, con User-Agent propio, cuatro peticiones simultáneas y caché local de siete días. No incorpora claves, notas, mensajes ni importes en solicitudes. Las imágenes no se incluyen en la app o el DMG.

La atribución se mantiene visible. Se revisó la [política de imágenes de OpenStreetMap](https://operations.osmfoundation.org/policies/tiles/); el servicio comunitario tiene capacidad limitada y disponibilidad sin garantía. No añade una suscripción al uso personal. La auditoría legal completa continúa pendiente.

## Generarla otra vez

```sh
./scripts/generate-icons.sh
./scripts/bundle-macos.sh
./scripts/package-dmg.sh
```

El empaquetador usa Cargo.lock y modo offline. En un equipo de desarrollo nuevo, prepara las dependencias con `cargo fetch --locked`. Ejecutar la app compilada no requiere estas herramientas.

## Pasar de uso personal a distribución

1. **Uso personal actual:** seguir validándola diariamente, conservar respaldos y publicar versiones locales. No exige servicios de pago.
2. **Producto estable:** completar el catálogo de unidades INEGI, pruebas con más datos, errores de red y distintas versiones de macOS; mejorar recuperación desde la interfaz y almacenamiento de tokens en el llavero del sistema.
3. **Distribución a otros usuarios:** terminar la auditoría legal de dependencias y fuentes, usar un identificador de aplicación propio y preparar firma Developer ID, Hardened Runtime y notarización. Apple documenta este proceso en [Notarizing macOS software](https://developer.apple.com/documentation/security/notarizing-macos-software-before-distribution).
4. **Actualizaciones:** preparar releases verificables y un canal de descarga; añadir actualizador firmado cuando la aplicación se distribuya fuera del proyecto.

Esta compilación usa firma ad hoc local y no está notarizada. El DMG facilita instalarla, pero no sustituye la confianza de Developer ID para distribución externa. No se desactivan las protecciones de macOS.

Apple indica una membresía de 99 USD por año, o equivalente local, para su programa: [Apple Developer Program](https://developer.apple.com/programs/enroll/). El funcionamiento local de esta app sigue siendo gratuito. No se ha comprado una membresía ni enviado el paquete a Apple.

Los costos y condiciones de publicación son independientes de las fuentes gratuitas INEGI y Banxico. La auditoría legal profunda sigue pendiente conforme a lo acordado.

## Dashboard y actualizaciones 0.2.0

Dashboard local con pendientes, resultados de la oferta, cobros y tres prospectos. El tema se configura únicamente en Ajustes. Selección de 32 entidades, metas y límites persistentes; copia privada de configuraciones antiguas antes de guardar la nueva versión.

Actualizaciones separa grupos nuevos, pendientes y revisión explícita. Presupuesto global por lote, registro de éxito y error por grupo, contador de intentos y comparación de datos públicos. No reescribe datos idénticos ni cambia el CRM. La CLI usa los mismos modos y límites.

Se probó una instalación vacía sin tokens: importación desactivada, guía inicial y cero consultas registradas después del arranque y de guardar preferencias. Tema, metas, entidades y límite sobrevivieron al reinicio.

La trazabilidad DENUE es por grupo de consulta. No certifica vigencia por campo ni identifica automáticamente las bajas del directorio; esa mejora queda en la hoja de ruta. Los datos migrados sin historial permanecen disponibles sin atribuirles una consulta reciente ficticia.

## Navegación 0.2.1

Las vistas guardan resultados de sesión por oferta y revisión. Se eliminaron consultas individuales de perfil por establecimiento. Las fuentes siguen siendo manuales; la caché no programa importaciones ni cambia los tokens.

Las pruebas verifican retorno sin pantalla intermedia desde el primer render, cambios de oferta y revisión, filtros independientes, escrituras fallidas y externas, límite de caché y errores con reintento.

Las mediciones corresponden a lecturas locales, no al tiempo completo de renderizado. Consulta [rendimiento de navegación](RENDIMIENTO_NAVEGACION.md).

## Expansión territorial Desktop · 22 octubre 2026

Puebla y Tlaxcala se complementan con Estado de México (15), Querétaro (22), Guanajuato (11), Jalisco (14) y Nuevo León (19). `Settings::load_desktop` aplica los siete sólo cuando aún no existe un archivo de preferencias; cargar configuraciones guardadas conserva la selección, incluidas exclusiones deliberadas. El botón **Ajustes → Datos → Añadir recomendados** agrega el piloto sin duplicar ni reemplazar entidades; **Guardar preferencias** lo aplica para ambas ofertas. iOS mantiene su configuración inicial independiente. El razonamiento territorial y las referencias de seis proyectos abiertos están en [el informe](../reports/Expansión%20territorial%20y%20mejoras.md).

No se modifica el esquema ni se inician consultas, trabajos o envíos al cambiar la selección. Se conserva el presupuesto por clic y la reanudación de segmentos. Para obtener establecimientos de los nuevos estados se necesita la acción explícita de Actualizaciones. Los conteos censales del informe no son prospectos importados ni demanda confirmada.

Verificación: 38 pruebas aprobadas, una prueba de rendimiento ignorada, Clippy estricto y compilación Desktop correctos con `--locked`. En UI nativa, una copia aislada de 4,709 establecimientos pasó de dos a siete estados, guardó cinco páginas por clic y conservó los hashes de las 17 tablas SQLite respecto de su origen. El preset quedó deshabilitado tras añadirse. La configuración personal se amplió por separado con respaldo privado, sin tocar SQLite ni otros ajustes. Cargo-deny se ejecutó: `bans`, `licenses` y `sources` pasan; `advisories` permanece bloqueado por los siete avisos previos RUSTSEC-2026-0346/0347/0348/0349 (rustls 0.23.45) y RUSTSEC-2026-0367/0368/0369 (zip 8.6.0), sin añadir excepciones ni cambiar dependencias. La auditoría legal profunda continúa diferida.

## Vistas de trabajo y resumen de lotes · 22 octubre 2026

En **Base → Personalizar tabla** se pueden elegir columnas visibles, moverlas con botones accesibles, ordenar resultados, agrupar por entidad/municipio/giro, cambiar la densidad y las filas por página. Establecimiento permanece como primera columna. El orden y la agrupación se aplican en SQLite antes de paginar, con desempate por ID; un municipio siempre incluye su entidad. Los encabezados de grupo indican que representan esta página, no el total del grupo.

**Vistas guardadas** conserva consulta y presentación por oferta. El mismo nombre actualiza una vista sólo dentro de esa oferta; abrirla vuelve a la primera página. No se aplica una vista automáticamente al iniciar. Las búsquedas guardadas anteriores siguen disponibles y admiten sus JSON anteriores; las exportaciones conservan su formato completo, independiente de las columnas de pantalla. Los selectores declaran también la opción seleccionada para reflejar correctamente una presentación restaurada en el primer render de WebKit.

**Actualizaciones → Cambios por lote · DENUE** presenta los últimos veinte lotes de la oferta y sus páginas confirmadas: nuevos, modificados, sin cambios e inválidos. Un desglose por campo muestra modificaciones y pérdidas de valores no vacíos. Se cuentan registros procesados, no empresas únicas; los reintentos/lotes distintos pueden consultar de nuevo el mismo establecimiento. El resumen conserva el estado terminado, cancelado, fallido o interrumpido, cuando hay páginas guardadas. Un fallo antes de guardar páginas sigue apareciendo en el estado del trabajo/fuente, sin inventar un resumen. No se reconstruyen diferencias de consultas históricas anteriores a esta entrega ni se almacenan los valores anteriores para revertirlos. Es auditoría posterior del lote, no revisión previa de conflictos. Los datos públicos y el CRM mantienen sus reglas previas de actualización.

La migración Rust **3** es aditiva: incorpora `saved_views` y `denue_page_audits`, con checksum; las migraciones 1 y 2 permanecen intactas. El guardado de datos, resumen de página y cursor ocurre en una sola transacción. La lista de campos auditados procede exclusivamente de `DENUE_COLS`; no incluye notas, interacciones o fechas comerciales. La UI es primero para Desktop; el núcleo compartido registra páginas DENUE en ambos targets. No hay nuevas dependencias, peticiones automáticas ni envíos.

Validación: **42 pruebas aprobadas y una de rendimiento ignorada**, Clippy estricto del workspace y del target `aarch64-apple-ios-sim`, con `--locked`. Las pruebas nuevas cubren persistencia/reinicio, nombres iguales en ofertas distintas, compatibilidad de búsquedas previas, validación estricta, orden/grupos globales antes de paginar, pérdida de campos, conservación de CRM, rollback atómico y conexión del importador acotado con su auditoría. La prueba de controles cubre el valor seleccionado del primer render.

UI nativa en `PROSPECTOS_HOME` aislado con 4,709 establecimientos: se añadieron/reordenaron columnas, guardó y restauró una vista con orden por nombre, grupo municipal y densidad compacta. Se revisó el detalle de cambios con una ficha de lote explícitamente rotulada **QA (sin consulta)**, creada sólo como fixture de interfaz; no representa una actualización real. Al cambiar a la otra oferta aparece el estado sin lotes. Los hashes de las tablas originales de establecimientos, CRM y fuentes se conservaron respecto a la copia de origen; sólo cambiaron metadatos de migración/trabajo y las tablas nuevas de QA. No se añadieron contactos ficticios a la base personal ni se consultaron fuentes externas. Cargo-deny mantiene los siete avisos previos de rustls/zip descritos arriba; `bans`, `licenses` y `sources` pasan y no se añadieron excepciones. La auditoría legal profunda permanece diferida.

## Datos reales y primer uso autónomo · 22 octubre 2026

La ampliación inicial sólo cambió estados de interés; no significaba que ya existieran establecimientos de esos estados. En esta entrega se ejecutaron diez lotes reales de DENUE —cinco estados, dos ofertas— con cinco páginas por lote: **50 páginas, 8,818 registros nuevos, cero inválidos**. La base personal quedó con **13,527 establecimientos**: Estado de México 1,892; Querétaro 1,410; Guanajuato 2,000; Jalisco 1,877; Nuevo León 1,639; Puebla 4,050 y Tlaxcala 659. Es una primera carga acotada de los segmentos seleccionados, no cobertura exhaustiva de esos estados ni garantía de publicación reciente. Hay puntajes propios de ambas ofertas para los 13,527 registros.

Antes de importar se creó un respaldo consistente privado. Después se verificaron integridad y relaciones SQLite, permanencia de los 4,709 IDs originales y conservación exacta de sus estados, notas, fechas, toques y etapas; también de las tablas comerciales de interacciones, cuentas, contactos, oportunidades, recibos y búsquedas guardadas. Los diez trabajos de importación quedaron terminados. Se corrigió el cierre de trabajos CLI para que una importación terminada no aparezca interrumpida al siguiente arranque.

La distribución conserva **cero bases, contactos y tokens**. El usuario obtiene su token desde el enlace oficial de Ajustes → Fuentes; los campos de entrada se limpian tras guardarlo y sus archivos privados quedan fuera del paquete. Sin token, la importación está deshabilitada. Guardar ajustes y abrir la app no consulta fuentes. Al configurar por primera vez, Ajustes abre Fuentes directamente y ofrece accesos a estados/límites y descarga. Los campos de token vacíos no producen un falso guardado y se recortan espacios exteriores.

En **Actualizaciones → Cobertura** se distinguen los establecimientos realmente almacenados en la base compartida de los segmentos consultados para la oferta actual. Cada estado tiene **Importar** o **Reanudar**, según su cursor; elegirlo afecta sólo ese lote y conserva la selección global y su presupuesto. Permite trabajar otro estado sin reiniciar ni agotar los segmentos de los anteriores. Los botones se bloquean durante un trabajo. La cobertura ocupa todo el ancho y precede al historial, evitando que éste comprima las acciones hasta volverlas inaccesibles.

La aceptación de entrega usa datos reales, sin fichas de lote ni contactos simulados. En un home inicialmente vacío se comprobó ausencia de registros, consultas, trabajos y tokens. Se guardaron siete estados y una página por clic desde la UI, sin consultas. Después, con el token propio provisionado privadamente fuera del paquete, la interfaz descargó 124 establecimientos de Querétaro y 200 de Guanajuato. Tras cerrar y reabrir esa instalación, **Reanudar Guanajuato** empezó en 201 y añadió otros 200: **524 establecimientos reales**, sin duplicar la página 1, sin tocar los 124 de Querétaro y sin registrar interacciones. El home de esta prueba y sus credenciales permanecen fuera del proyecto, bajo `/private/tmp`. No se afirma que se automatizó el registro del token ante INEGI.

La aceptación reproducible `crates/core/tests/real_data.rs` copia una base real a un home temporal y comprueba los siete estados, ambas ofertas, filtros/vistas, puntos de mapa y lotes persistidos sin consultar fuentes ni leer credenciales. Se ejecutó sobre la copia consistente de los 13,527 registros y pasó. No modifica la base original. Los fixtures unitarios siguen cubriendo casos límite, pero no sustituyen esta aceptación ni se presentan como datos de la entrega.

```sh
PROSPECT_REAL_DB=/ruta/privada/a/una/copia/real/prospectos.db cargo test --locked -p prospect-core --test real_data -- --ignored --nocapture
```

Clippy del workspace y del target iOS, compilaciones y comprobación nativa se ejecutaron. Cargo-deny mantiene los siete avisos previos descritos arriba; no se introdujeron dependencias ni excepciones. El paquete macOS contiene sólo binarios, recursos y licencias, sin archivos de datos o credenciales. La auditoría legal profunda continúa diferida.
