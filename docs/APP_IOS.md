# Prospect Intelligence para iOS

La app utiliza Rust, Dioxus Mobile y WKWebView. Comparte los paneles, consultas, reglas, comandos CRM y caché con Desktop mediante `prospect-ui`. Su presentación se adapta al iPhone: títulos grandes, listas agrupadas, navegación inferior estable, botón de regreso y ficha a pantalla completa. No es una aplicación escrita en SwiftUI.

La navegación sigue las referencias de Apple para [pestañas](https://developer.apple.com/design/human-interface-guidelines/tab-bars) y [áreas seguras](https://developer.apple.com/design/human-interface-guidelines/layout). Conserva los colores de Prospect, la tipografía del sistema y la apariencia elegida en Ajustes.

## Funciones

- Hoy, seguimientos, conversaciones y base con búsqueda, filtros y paginación.
- Ficha con puntajes, evidencia, historial, borrador editable, respuestas, estados y notas.
- Más: dashboard, oportunidades, métricas, territorios, indicadores, actualizaciones y ajustes.
- Ofertas independientes para Automatización y sistemas y Web y marketing; CRM compartido.
- Portapapeles mediante APIs seguras de UIKit, sin programas evaluados ni interpolación de datos en código.
- Preparar correo entrega destinatario, asunto y borrador a la app de correo predeterminada de iOS, incluida Gmail. Los espacios, acentos, saltos y signos se codifican como `mailto` (RFC 6068), sin convertir espacios en signos `+`.
- Alto y áreas seguras tomados de UIKit, actualizados al rotar. Header y pestañas permanecen fuera del scroll; se reserva sólo el área del gesto de inicio, sin la franja inferior extra de WebKit. Un único viewport con `viewport-fit=cover`.
- El teclado reserva su área mediante UIKit sólo mientras un campo está en edición. Al cerrar o volver se libera esa reserva. La apariencia también se aplica a los controles nativos y al contraste del reloj.
- Búsqueda y encabezados compactos; sin subtítulos repetidos ni paginación vacía. Las pestañas se ocultan al editar para dar espacio al teclado.
- Bordes y separadores reforzados en las dos apariencias; transición de entrada entre pestañas y desplazamiento breve al abrir fichas. Las animaciones se desactivan con Reducir movimiento.
- Editor de plantillas TOML dentro de Ajustes; valida antes de reemplazar el archivo.
- CSV/XLSX y respaldos disponibles en Archivos → En mi iPhone → Prospect → Prospect. Las exportaciones conservan su archivo de metadatos.
- Fuentes manuales con los mismos presupuestos `updates::pull_batch` de Desktop. Navegar y guardar ajustes no descarga fuentes.

Abrir o copiar un canal no registra envíos. Cada contacto o seguimiento requiere el botón explícito después de que el usuario lo haya realizado. Para que Gmail reciba el borrador, debe ser la [app de correo predeterminada de iOS](https://support.google.com/accounts/answer/17304849?hl=en). La entrega a Mail, teléfono o WhatsApp depende de las aplicaciones instaladas; el simulador no valida estas integraciones en un iPhone físico. Copiar el correo, teléfono o borrador permite continuar manualmente.

## Navegación de iPhone

Dentro de la pestaña seleccionada, las tareas usan una pila de pantallas con regreso. Abrir una tarea conserva el estado del panel y guarda la posición de su lista para recuperarla al volver.

- Territorios muestra municipios y negocios. Tocar un negocio abre el mapa a pantalla completa; su ficha se abre dentro de esa misma navegación. El mapa se monta y solicita teselas sólo cuando se abre.
- La ficha ofrece Mensaje, Avance y Notas como pantallas dedicadas, con Evidencia e Historial separados del resumen.
- Oportunidades comienza en la cartera. Nueva y cada oportunidad abren el editor; guardar vuelve a la lista.
- Métricas separa embudo, ingresos y segmentos. Municipios, segmentos y cobertura usan filas legibles en lugar de tablas que requieren desplazamiento horizontal.
- Resumen separa jornada, resultados y estado de los datos. Actualizaciones separa DENUE, cobertura y otras fuentes; Indicadores abre su consulta en otra pantalla.
- Ajustes ofrece perfil, apariencia, metas, datos, credenciales, respaldos y plantillas. Filtros y acciones de Base también se abren en su propia pantalla.

## Compilar

Requiere Xcode completo, el runtime de simulador y los targets Rust instalados. Desde la raíz:

```sh
./scripts/bundle-ios.sh --simulator
./scripts/bundle-ios.sh --simulator --run
./scripts/bundle-ios.sh --device
```

El script fija `IPHONEOS_DEPLOYMENT_TARGET=16.0` tanto para Rust como para las dependencias nativas. Usa `Cargo.lock` y `--locked`, incluye iconos y verifica la firma local.

El bundle de simulador está en `dist/ios/Prospect Intelligence.app`. El de dispositivo está separado en `dist/ios/device/Prospect Intelligence.app`. La firma ad hoc sólo sirve para las comprobaciones locales; instalar en un iPhone requiere firma de desarrollo, perfil y dispositivo autorizados. No se generó una entrega de App Store ni TestFlight.

## Datos reales y pruebas aisladas

La app mantiene SQLite, preferencias y tokens en su contenedor privado, fuera del paquete. Un iPhone recién instalado comienza con su propia base. No existe sincronización automática con Desktop ni restauración de cambios en sentido inverso.

Para preparar una copia consistente de Desktop:

```sh
python3 scripts/prepare-ios-data.py qa-data/ios-real
```

Se usa el backup de SQLite con origen de sólo lectura para incluir transacciones confirmadas en WAL. Copia también preferencias y plantillas, verifica integridad y rechaza destinos existentes o dentro de la base personal. Conserva CRM, interacciones, fechas, oportunidades, métricas por oferta e indicadores porque copia la base completa. Excluye credenciales, exportaciones y cachés web.

Para un simulador, copia ese snapshot a una carpeta **nueva** dentro de su contenedor y lanza con `SIMCTL_CHILD_PROSPECTOS_HOME` apuntando a ella. No reemplaces una base móvil existente. La sesión de QA de esta revisión utiliza una copia de 4,709 establecimientos; se conservó la base móvil anterior de 142 registros. Los datos reales no forman parte de Git ni del `.app`.

La copia es una foto del momento de preparación. No acredita publicación reciente de los datos ni actualización posterior de fuentes.

## Verificación y límites

Se verificaron builds de macOS, simulador arm64 y dispositivo arm64; 37 pruebas del workspace, Clippy en host e iOS y formato (un benchmark de rendimiento queda ignorado). En iPhone 16e / iOS 26.2 se revisaron datos reales, navegación, ficha y áreas seguras en vertical y horizontal. Las pruebas de navegación móvil ejecutan los paneles compartidos con SQLite temporal, comprueban cambio de oferta, ausencia de selección automática de fichas y que navegar no inicia trabajos de fuentes ni interacciones. Las plantillas inválidas no reemplazan las guardadas.

La comprobación `cargo deny --offline --locked check` actual **no pasa advisories**: el lock previo contiene rustls 0.23.45 (RUSTSEC-2026-0346, 0347, 0348 y 0349) y zip 8.6.0 (RUSTSEC-2026-0367, 0368 y 0369). Bans, licencias y fuentes pasan. No se añadieron excepciones para silenciar estos avisos; sigue vigente la excepción documentada de `paste`. Corregir las versiones y revisar su inventario queda pendiente antes de distribución. El código de la aplicación escribe XLSX; no extrae ZIP externos. Esto no equivale a resolver los avisos de dependencias.

La auditoría legal profunda sigue diferida. La revisión visual de iOS tampoco equivale a una auditoría completa de accesibilidad o una validación en hardware físico.
