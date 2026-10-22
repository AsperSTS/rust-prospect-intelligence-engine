# Rendimiento de navegación — 0.2.1

## Cambio

Las pantallas se desmontaban al navegar; cada montaje esperaba sus recursos. Métricas cargaba registros completos y consultaba `profile_scores` individualmente. Dashboard, Base y Territorios repetían estas lecturas.

Ahora SQLite guarda hasta 32 lecturas mediante LRU y la UI hasta 24 instantáneas. Son cachés en memoria de la sesión, sin nuevos paquetes, migraciones SQL o copias persistentes de datos.

Cada consulta se identifica por oferta, filtros o dimensión. La instantánea UI también incluye revisión de datos y preferencias pertinentes. Sólo se renderiza un resultado si corresponde a su clave actual.

Las escrituras del actor invalidan SQLite antes de devolver el resultado, incluso si después fallan. La UI cambia de revisión después de guardar, finalizar un trabajo, guardar preferencias o cambiar de día.

SQLite detecta commits externos mediante `PRAGMA data_version`. El bucle local existente consulta ese contador cada 60 segundos y descarta instantáneas si otra instancia cambió la base. Nunca consulta DENUE, INEGI o Banxico.

Las métricas usan una proyección conjunta de las columnas necesarias. Base y ficha incorporan la evidencia del perfil desde el mismo JOIN. Se mantienen embudo, cohortes, reglas, priorización y monedas existentes.

La cola de prospectos no ejecuta una consulta al cambiar el perfil en pantallas que no la utilizan. No se montan mapas ocultos ni se precargan fuentes externas.

## Medición reproducible

Base aislada de QA: 4,709 establecimientos. Diez repeticiones por oferta y dimensión; incluye clonación y comparación del resultado. Sin jobs externos, tokens ni cambios en el CRM personal.

| Lectura local en desarrollo | Antes, promedio repetido | Después, promedio repetido |
|---|---:|---:|
| Métricas automatización / giro | 876.88 ms | 1.11 ms |
| Métricas automatización / municipio | 775.78 ms | 1.09 ms |
| Métricas web / giro | 927.96 ms | 1.08 ms |
| Métricas web / municipio | 745.81 ms | 1.09 ms |
| Base automatización, 60 filas | 53.44 ms | 0.18 ms |
| Base web, 60 filas | 47.32 ms | 0.18 ms |

En release, las métricas repetidas midieron 0.80–0.84 ms; Base, 0.12–0.14 ms. Una primera consulta de métricas midió 36–38 ms después de inicializar validadores; la primera absoluta, 148 ms.

Estas cifras son lecturas de este equipo, no latencia de clic a pantalla ni garantías para bases mayores. Reiniciar, modificar datos o expulsar una entrada LRU requiere una lectura inicial.

La caché del mapa conserva su política existente de siete días en disco. La primera vista de calles o un cambio de ubicación/escala puede esperar imágenes; esta optimización no precarga mapas.

```sh
PROSPECT_PERF_DB=/ruta/a/base-QA/prospectos.db cargo test --release --offline --locked -p prospect-core --test read_cache -- --ignored --nocapture
```

El benchmark copia el archivo a un directorio temporal. Usar una base QA cerrada o una copia SQLite consistente, nunca un archivo de producción activo con cambios pendientes en WAL.

## Validación

33 pruebas automáticas regulares y benchmark optativo ejecutado. La prueba Dioxus comprueba instantáneamente el primer render al remontar vistas, alternar ofertas y cambiar revisión; no depende de esperas visuales.

Las pruebas de núcleo comprueban coalescencia, filtros, perfiles, puntajes de mapa/ficha, actualización de notas e ingresos, invalidación de escrituras fallidas y externas, límite LRU y vencimiento al cambiar de día.

QA nativo verificado en 1440×900 y 1000×650: Dashboard, Base, Métricas, Territorios y alternancia de ofertas. Automatización mantiene ingresos de QA en cero; web conserva su oportunidad y $2,500 de prueba.

La entrega optimizada también detectó en QA un cambio público hecho desde otra conexión SQLite. Dashboard lo reflejó sin navegar ni reiniciar; después se restauró el dato y se cerró QA.

La app personal 0.2.1 abrió 4,709 registros. Las huellas de CRM, interacciones, oportunidades, ajustes y credenciales permanecieron idénticas. Integridad SQLite correcta y cero consultas nuevas a fuentes durante la actualización.
