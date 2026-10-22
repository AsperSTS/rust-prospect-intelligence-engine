# Prospect Intelligence

Aplicación personal en Rust y Dioxus Desktop. Trabajar dentro de este directorio; el proyecto Python original es referencia de paridad.

- Consultar graphify-out/graph.json con graphify query antes de explorar módulos. Ejecutar graphify update . después de modificar código.
- Usar Cargo.lock y comandos --locked. Ejecutar cargo-deny y documentar cualquier excepción.
- Prohibir unsafe en código propio, ejecución dinámica y desactivación de TLS. Validar comandos con Rust/Serde estrictos.
- Conservar CRM, notas, interacciones y fechas al actualizar datos públicos.
- No guardar tokens, bases reales, contactos ni exportaciones en Git. Las credenciales viven fuera del proyecto.
- Realizar pruebas de interfaz en PROSPECTOS_HOME aislado. Nunca registrar contactos ficticios en la base personal.
- No enviar mensajes ni registrar envíos al copiar o abrir canales. El usuario contacta manualmente.
- La auditoría legal profunda quedó diferida por el usuario; no afirmar que está completada.
- Mantener separados los perfiles de oferta, sus puntajes y sus métricas, con CRM compartido.
- Verificar builds, Clippy, pruebas relevantes y comportamiento nativo antes de entregar cambios.

- Actualizar fuentes sólo por acción explícita, con los presupuestos de updates::pull_batch. No restaurar importaciones ilimitadas ni reinicio automático de segmentos completos.
- No editar migraciones SQL ya aplicadas: RUST_SCHEMA (1) y updates::SCHEMA (2). Crear una nueva versión aditiva para cambios de esquema.
- La apariencia vive en Ajustes y la persistencia fuera del paquete. Nunca afirmar que una consulta reciente garantiza datos publicados recientemente.
