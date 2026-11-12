# Prospect Intelligence

Aplicación de escritorio en Rust y Dioxus Desktop para prospección B2B, calificación de cuentas y seguimiento comercial local. Utiliza datos públicos de INEGI (DENUE), persistencia en SQLite y reglas deterministas para priorización de contactos.

## Características

- **Persistencia local:** Base de datos SQLite (`prospectos.db`) en el directorio de usuario. No requiere servidores externos ni servicios de suscripción.
- **Calificación por reglas:** Cálculo explicable de *fit* y prioridad comercial mediante configuraciones JSON auditables, sin modelos opacos.
- **Enriquecimiento web:** Detección de señales técnicas, comerciales y de contacto en sitios web declarados por los establecimientos.
- **Contacto manual:** Generación de borradores contextuales a partir de evidencia y plantillas. La aplicación no realiza envíos automáticos ni registra interacciones sin acción explícita del usuario.
- **Métricas y oportunidades:** Control de conversiones por giro económico y seguimiento de importes presupuestados frente a cobros reales en MXN y USD.
- **Doble perfil de oferta:** Separación de reglas, puntuaciones y borradores para *Automatización y sistemas* y *Web y marketing*, manteniendo el historial compartido por establecimiento.
- **Expansión territorial en Desktop:** Puebla y Tlaxcala más Estado de México, Querétaro, Guanajuato, Jalisco y Nuevo León en instalaciones nuevas. En configuraciones existentes, **Ajustes → Datos → Añadir recomendados → Guardar preferencias** amplía la selección sin reemplazar otros estados ni descargar datos. [Análisis territorial y de proyectos abiertos](reports/Expansión%20territorial%20y%20mejoras.md).
- **Vistas de trabajo en Desktop:** En **Base → Personalizar tabla**, elige columnas, su orden, agrupación, densidad y filas por página; **Vistas guardadas** conserva la presentación junto a los filtros, por oferta. Las búsquedas anteriores siguen disponibles.
- **Cambios por lote DENUE:** **Actualizaciones** muestra los últimos veinte lotes de la oferta, con registros nuevos, modificados, sin cambios e inválidos, y el detalle de campos modificados o vaciados. Se conservan los resúmenes de páginas completas ante cancelaciones y reinicios; no se reconstruye detalle de lotes antiguos.
- **Primer uso sin asistencia:** La distribución comienza sin registros ni tokens. Ajustes abre Fuentes cuando falta DENUE, enlaza a la guía oficial para obtener el token y permite pasar a estados/límites y descargas. **Actualizaciones → Cobertura** muestra los registros reales de cada estado y sus botones **Importar/Reanudar**, con el mismo presupuesto por clic.

## Capturas de pantalla

### Lista priorizada del día
Flujo principal de trabajo: selección de prospectos según prioridad, desglose de evidencia web e hipótesis comercial, y edición del borrador de mensaje.

![Lista priorizada del día](assets/portfolio/01-today-prioritized-list.png)

### Seguimientos
Control de contactos con fecha de seguimiento cumplida y generación de respuestas de seguimiento.

![Seguimientos](assets/portfolio/02-follow-ups.png)

### Conversaciones abiertas
Seguimiento de prospectos en interacción, notas de avance y actualización de etapas del CRM.

![Conversaciones abiertas](assets/portfolio/03-open-conversations.png)

### Métricas comerciales
Visualización del embudo de ventas, tasas de avance y comparativa por actividad económica.

![Métricas comerciales](assets/portfolio/04-commercial-metrics.png)

### Cartera de oportunidades
Administración de propuestas comerciales con desglose independiente de importes cotizados y cobros acumulados.

![Cartera de oportunidades](assets/portfolio/05-opportunities.png)

## Ejecución en macOS

### Binario empaquetado
El paquete compilado para Apple Silicon se encuentra en `dist/`:

```sh
open "dist/Prospect Intelligence.app"
```

## Versión iOS

La app móvil comparte funciones y lógica con Desktop, con listas agrupadas, ficha a pantalla completa y cinco pestañas para iPhone. Incluye ambos perfiles comerciales, CRM, oportunidades, métricas, territorios, indicadores y actualizaciones manuales. Consulta [compilación, datos reales y límites de iOS](docs/APP_IOS.md).

```sh
./scripts/bundle-ios.sh --simulator --run
```

## Estructura del proyecto

El repositorio está organizado como un workspace de Rust:

```text
.
├── Cargo.toml             # Configuración del workspace y perfil de release
├── crates/
│   ├── core/              # Dominio, almacenamiento SQLite (rusqlite), scoring y métricas
│   ├── desktop/           # Interfaz gráfica en Dioxus Desktop
│   ├── cli/               # Interfaz de línea de comandos para gestión de datos y backups
│   ├── mobile/            # Lanzador iOS
│   └── ui/                # Paneles compartidos y navegación para iPhone
├── config/                # Diccionarios SCIAN, señales, reglas de scoring y plantillas
├── assets/                # Iconos del sistema y capturas
├── scripts/               # Scripts de compilación, empaquetado para macOS y utilidades
└── docs/                  # Documentación técnica, inventario de licencias y seguridad
```

## Desarrollo y verificación

Comandos para compilación, pruebas y análisis estático:

```sh
# Ejecutar en modo desarrollo
cargo run --locked -p prospect-desktop

# Ejecutar pruebas unitarias y de integración
cargo test --locked --workspace

# Análisis estático estricto con Clippy
cargo clippy --locked --workspace --all-targets -- -D warnings

# Verificación de formato
cargo fmt --all -- --check

# Auditoría de dependencias y licencias
cargo deny --offline --locked check

# Generar bundle .app para macOS
./scripts/bundle-macos.sh
```

## Datos y credenciales

- Los datos operativos, respaldos y plantillas se almacenan en `~/Library/Application Support/Prospect Intelligence/`.
- Los tokens de consulta para APIs públicas (INEGI DENUE, Banxico SIE) se configuran en el archivo privado `credentials.env` con permisos `0600`.
- El repositorio no almacena tokens, bases de datos comerciales reales ni exportaciones de contactos.

## Licencia

Distribuido bajo licencia [MIT](LICENSE). Las licencias de dependencias de terceros se detallan en [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).
