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
