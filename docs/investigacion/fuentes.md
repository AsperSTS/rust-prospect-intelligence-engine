# Fuentes gratuitas y mejoras comerciales para BI mexicano

Investigación: 11 de octubre de 2026. Sólo fuentes primarias; no se ejecutaron consultas autenticadas, instalaciones ni código del proyecto. Fechas visibles verificadas; no se supone que una publicación programada ocurrió.

## ¿Qué fuentes pueden sostener la aplicación sin cuotas comerciales?

### Takeaway

INEGI y los datos propios del CRM deben sostener la primera versión. Banxico amplía el contexto económico; OSM añade cartografía opcional. Data México exige aclarar condiciones por dataset antes de integrarlo comercialmente.

«Gratis» significa sin pago por licencia o consulta en las fuentes propuestas. Equipo, almacenamiento, conexión y mantenimiento siguen consumiendo recursos; operar localmente evita contratar infraestructura.

### Cited Findings

**Licencia transversal INEGI:** permite extracción, adaptación y explotación comercial, exige conservar metadatos, atribuir producto y distinguir transformaciones propias. No implica aval institucional. — [Términos de libre uso](https://www.inegi.org.mx/contenidos/inegi/doc/terminos_info.pdf)

| Fuente y acceso | Costo / token / límites | Frescura y unidad | Beneficio comercial / complejidad | Limitación verificable |
|---|---|---|---|---|
| DENUE API | Libre acceso; token por registro. Radio de `Buscar`: máximo 5 km. Métodos territoriales usan rangos inicial/final; cuota diaria no encontrada. | Establecimiento geolocalizado; respuesta incluye actividad, estrato, razón social y campos de teléfono, correo y web. | Prospectos segmentados y ficha; complejidad baja/media. | La documentación enumera campos, pero no garantiza presencia, vigencia ni decisor. [API oficial](https://www.inegi.org.mx/servicios/api_denue.html) |
| DENUE descarga CSV/SHP | Descarga gratuita; acceso local sin token ni cuota de consulta API. Usar archivos, no ejecutar descargador Windows. | Catálogo muestra edición 05/2026; congelar edición, descarga y hash. | Índice local, filtros rápidos, cobertura territorial y operación sin conexión; complejidad media. | Edición reciente no prueba actualización individual reciente. [Descarga masiva](https://www.inegi.org.mx/app/descarga/?ti=6), [ficha metodología 05/2026](https://www.gslb.inegi.org.mx/app/biblioteca/ficha.html?upc=889463930044) |
| Censos Económicos 2024 / SAIC | Consulta, tabulados y datos abiertos INEGI; descarga sin token. Restricciones de confidencialidad por tabla. | Variables económicas refieren principalmente a 2023; resultados definitivos de julio 2025, corrección septiembre 2025. Nacional/estado/municipio/localidad y detalle permitido. | Comparar nichos, estructura sectorial y territorios; complejidad media/alta. | No confundir año de levantamiento con periodo económico ni universo censal con total DENUE. [Programa](https://www.inegi.org.mx/programas/ce/2024/default.html), [SAIC](https://www.inegi.org.mx/app/saic/) |
| Banco de Indicadores / BIE INEGI | Libre acceso; token. Cuota de solicitudes no documentada en página revisada. | Nacional/estado/municipio según indicador. Metadatos incluyen periodicidad, unidad, actualización y periodo de observación. | Tendencias y contexto por sector/territorio; complejidad media. | No todos los indicadores existen a todos los niveles. BIE/BISE no son series intercambiables. [API oficial](https://www.inegi.org.mx/servicios/api_indicadores.html) |
| Marco Geoestadístico INEGI | Archivos vectoriales públicos sin token. Shapefile; formatos varían por producto. | AGEE, AGEM, localidad, AGEB y manzana; escoger versión compatible. Portal ofrece versión «2026 Encuesta Intercensal 2025». | Territorios, mapas de densidad, agregación espacial; complejidad media/alta. | Límites geoestadísticos no definen pertenencia política oficial. [Programa](https://www.inegi.org.mx/programas/mg/default.html), [selector de versiones](https://www.inegi.org.mx/temas/mg/) |
| Censo Población/Vivienda 2020: ITER y AGEB/manzana | Descargas oficiales; sin token. Licencia INEGI. | Base 2020; ITER hasta localidad y resultados urbanos por AGEB/manzana. | Entorno de clientes finales para retail/servicios; complejidad media. | Población 2020 no es población 2026; hogares no son compradores empresariales. [Metadatos y descargas](https://www.inegi.org.mx/rnm/index.php/catalog/632/related_materials) |
| Banxico SIE API | Consulta pública con token. TLS 1.3. Límites por token detallados abajo. | Series temporales; geografía y frecuencia según serie, muchas nacionales. | Tipo de cambio, tasas, inflación y contexto de cotizaciones; complejidad baja/media. | Contexto macroeconómico, no directorio de compradores. [Documentación API](https://www.banxico.org.mx/SieAPIRest/service/v1/) |
| Data México | Plataforma pública y manual API; no pago de datos dentro del servicio. Token/cuotas actuales no verificados. | Cubos agregados; mes/trimestre/año, municipio/metroárea/estado/nación según cubo. | Comercio y complejidad para expansión futura; complejidad alta por esquema/licencias. | API Explorer histórico no validado en vivo; licencias del contenido/sitio y de datasets requieren separar análisis. [Manual técnico](https://www.economia.gob.mx/datamexico/files/tesseract.pdf), [términos indexados](https://www.economia.gob.mx/datamexico/es/about/legal) |
| datos.gob.mx | Catálogo público con CSV/SHP/GPKG; etiqueta licencias CC BY 4.0. API unificada y token no verificados. | Variable por institución/dataset; catálogo indexa actualizaciones 2026. | Buscar futuras capas oficiales; complejidad alta si se integra genéricamente. | Acceso directo del investigador dio error/403; verificar cada recurso, licencia y fecha antes de incorporarlo. [Catálogo](https://www.datos.gob.mx/dataset/), [filtro licencias](https://www.datos.gob.mx/dataset/?_groups_limit=0&q=6ae2a2d0-5047-4498-8393-9abf91c01645) |
| OSM / Overpass | Datos libres; consultas públicas sin token comercial. Orientación general: ~10,000/día y <1 GB/día; disponibilidad y carga compartidas. | Objetos OSM, no unidades INEGI; revisión comunitaria y cobertura variable. | Vialidades, POI y contexto territorial; complejidad media. | Manual desaconseja depender del servidor público como backend de una app general. Preferir extracto local si la dependencia crece. [Manual operador](https://dev.overpass-api.de/overpass-doc/en/preface/commons.html) |
| Sitios propios de negocios | Sin tarifa uniforme para navegar páginas públicas; condiciones y licencia de cada sitio. | Negocio/página; fecha de inspección propia. | Verificación de web, servicios, formularios y contacto; complejidad media. | `robots.txt` no constituye autorización de acceso ni licencia de reutilización. [RFC 9309](https://www.rfc-editor.org/info/rfc9309/) |

**Banxico licencia:** autoriza reproducir/utilizar su información con fuente y dirección web; advierte diferencias posibles entre información oportuna y definitiva. No atribuirle un resultado comercial propio. — [Salvedad de uso](https://www.banxico.org.mx/footer-es/salvedad-politicas-uso-banco-.html)

**Banxico cuotas:** oportunas/metadatos: 80/minuto, 40,000/día; históricas: 200/5 minutos, 10,000/día. Devuelve 400 y tiempos de desbloqueo; recomienda caché. — [Límites oficiales](https://www.banxico.org.mx/SieAPIRest/service/v1/doc/limiteConsultas)

**OSM licencia:** uso comercial permitido bajo ODbL; exige atribución y condiciones sobre bases derivadas publicadas. Separar origen OSM del CRM privado para preservar trazabilidad. — [Uso comercial y casos](https://wiki.openstreetmap.org/wiki/License/Use_Cases), [copyright](https://www.openstreetmap.org/copyright)

**Nominatim público:** máximo absoluto 1 solicitud/segundo por aplicación, identificación y caché; prohíbe autocompletar y extraer sistemáticamente todos los POI. No usarlo para enriquecer masivamente DENUE. — [Política obligatoria](https://operations.osmfoundation.org/policies/nominatim/)

**Tiles OSM públicos:** servicio sin SLA; identificación, atribución y caché HTTP. Prohíbe descarga masiva, prefetch y modo offline. Para offline, usar geometrías INEGI o mapas generados desde extractos permitidos. — [Política de tiles](https://operations.osmfoundation.org/policies/tiles/)

**Data México: distinción legal.** La página indexada restringe copias/descargas del contenido a uso personal no comercial y menciona AGPLv3 de manera confusa. El acceso directo entregó una página sin cuerpo textual completo. — [Términos](https://www.economia.gob.mx/datamexico/es/about/legal)

Esto no prueba que cada dataset público subyacente sea no comercial. Debe comprobarse su fuente/licencia y las condiciones API; no confundir licencia de software, contenido del sitio y datos de terceros.

**datos.gob.mx: alcance.** Los lineamientos federales de septiembre 2025 promueven acceso libre, reutilización y redistribución. La licencia aplicable debe comprobarse por recurso; CC BY 4.0 permite uso comercial con atribución. — [Lineamientos oficiales](https://sidof.segob.gob.mx/notas/docFuente/5767742), [CC BY 4.0](https://creativecommons.org/licenses/by/4.0/deed.es)

### Inferences

- Base recomendada: DENUE CSV local por estados objetivo + API para consultas puntuales; el diseño es una inferencia operativa, no recomendación oficial de INEGI.
- Mapas básicos pueden renderizar polígonos INEGI y puntos DENUE localmente. OSM es opcional; geocodificación externa no hace falta cuando DENUE ya aporta coordenadas.
- Data México sirve ahora como referencia de exploración. El conector comercial queda condicionado a validar dataset, licencia, esquema, disponibilidad y términos API; preferir la institución productora original.
- datos.gob.mx requiere conectores específicos a recursos seleccionados, no depender de un supuesto API universal. Su existencia indexada no certifica disponibilidad estable ni calidad uniforme.

### Gaps

- No se identificó cuota vigente publicada para DENUE ni Banco de Indicadores. No equivale a consultas ilimitadas; aplicar colas, caché, reintentos con espera y cancelación.
- No se verificó respuesta actual de Data México API ni una política vigente de cuotas/token. La documentación técnica disponible es de 2021/2022; no asumir equivalencia con plataforma 2026.
- No se verificó cobertura/contactabilidad real de la edición DENUE 05/2026. Debe medirse con archivos descargados, no extrapolar porcentajes de artículos externos.
- El acceso directo a datos.gob.mx falló durante esta investigación. El catálogo oficial indexado sí existe; no se confirma descarga completa ni API productiva.

## ¿Qué mejoras gratuitas aumentan primero la captación de clientes?

### Takeaway

Priorizar convertir registros en cuentas contactables, oportunidades y seguimiento. Dos perfiles comerciales comparten CRM, pero usan criterios distintos: automatización/sistemas y web/marketing.

### Cited Findings

- DENUE identifica establecimientos mediante ID; la CLEE puede vincular establecimientos de una misma empresa según su estructura. RFC no es público en el directorio. — [Metadatos DENUE](https://www.inegi.org.mx/rnm/index.php/catalog/1103)
- Los Censos Económicos incluyen enfoques de TIC, compras/ventas por internet, financiamiento y problemáticas. Son estadísticas agregadas para contexto; no diagnóstico individual de una empresa. — [Presentación definitiva, páginas 3–4](https://www.inegi.org.mx/contenidos/programas/ce/2024/doc/ce2024_presenta.pdf)

### Inferences

Prioridades sugeridas considerando el contexto facilitado por el coordinador: 4,709 establecimientos almacenados, sin interacciones/resultados; foco actual Puebla/Tlaxcala y estratos 11–100. Esta base no fue inspeccionada por este investigador.

| Prioridad | Mejora | Perfil automatización/sistemas | Perfil web/marketing | Evidencia de avance |
|---|---|---|---|---|
| P0 | Cuenta, establecimiento y contacto separados | Agrupar plantas/sucursales con evidencia; capturar cargo y capacidad de decisión. | Mismo grupo empresarial con diagnóstico por sucursal/web. | Cuentas únicas; asociaciones revisadas; contactos confirmados. |
| P0 | Perfiles de oferta y filtros guardados | Manufactura, logística y sectores con procesos repetibles; Puebla/Tlaxcala, 11–100 como hipótesis inicial. | Retail/servicios pueden entrar; priorizar actividades con necesidad comercial verificable. | Lista accionable distinta por oferta; motivos visibles. |
| P0 | Estado de contacto y enriquecimiento verificable | Sitio, teléfono, correo, cargo, fecha y fuente; registrar validación humana. | Comprobar web real y presencia comercial antes de calificar. | Proporción con canal válido, fecha y fuente. |
| P0 | Scoring explicable de ajuste + evidencia | Tamaño/actividad son ajuste; necesidad requiere entrevista, proceso o evidencia pública concreta. | Defectos observables: formulario roto, falta de llamada a la acción o catálogo inadecuado. | Reglas visibles y revisión; sin porcentajes predictivos inventados. |
| P0 | CRM de venta y siguiente acción | Llamada → diagnóstico → propuesta → seguimiento → ganado/perdido. | Misma historia comercial; interés en una o ambas ofertas. | Tareas vencidas, respuestas, reuniones, propuestas, ventas. |
| P0 | Territorios y lista de visita | Zonas industriales y proximidad; ordenar por distancia aproximada. | Centros comerciales/servicios y concentración territorial. | Lista manejable por jornada; no prometer ruta óptima sin motor. |
| P0 | Calidad, actualización e historial | No borrar notas al refrescar fuente; conservar cambios y contactos confirmados. | Campo web: desconocido / encontrado / verificado / inactivo. | Importación idempotente, duplicados y cambios auditables. |
| P1 | Mercado local por sector | Densidad, estratos, especialización y cadenas proveedor-cliente como hipótesis. | Competencia local y entorno demográfico 2020 etiquetado. | Comparaciones reproducibles por edición, área y actividad. |
| P1 | Segmentos con adopción digital | TIC censal por sector/territorio orienta conversación. | Comercio electrónico agregado orienta nichos. | No atribuir resultado regional a empresa específica. |
| P1 | Tendencias y escenarios | Series BIE/Banxico para contexto, costos y exposición cambiaria. | Tendencias comerciales/demográficas según fuente disponible. | Unidad, moneda, periodo y fecha visibles. |
| P1 | Inteligencia comercial propia | Conversión por sector/territorio/oferta y motivos de pérdida. | Comparar captación y rendimiento de oferta. | Denominadores y etapa medidos; ingresos reales diferenciados de estimados. |

- Una sola cuenta debe compartir historial y responsable: evita ofrecer «automatización industrial» y «marketing» mediante mensajes contradictorios o contactos duplicados.
- «No hay web en DENUE» sólo significa campo ausente. No acredita falta de web, problema digital o intención de compra. Guardar incertidumbre explícita.
- No estimar presupuesto o necesidad individual desde estrato, actividad o media censal. Separar ajuste demográfico/industrial de evidencia de necesidad y de respuesta comercial.
- Plantillas, enlaces `mailto:`/teléfono y borradores locales ayudan sin servicios pagos. Una acción comercial siempre debe quedar registrada; no introducir envíos masivos como dependencia de la migración.
- IA opcional futura: modelo local y reglas deterministas con evidencia. No usar APIs LLM con créditos como requisito para prospectar, calificar o mantener datos.

### Gaps

- La oferta concreta, precio, capacidad de entrega y cliente ideal requieren validación comercial; el coordinador confirma ambos perfiles, pero no hay ventas históricas para aprender prioridades.
- No existe fuente pública gratuita universal de decisores, correos verificados o intención de compra. El MVP debe aceptar captura manual y estados desconocidos.
- La calidad del scoring no puede evaluarse mediante ingresos inexistentes; iniciar reglas transparentes y comparar resultados de contacto con etiquetas reales.

## ¿Qué reglas evitan BI engañoso y costos ocultos?

### Takeaway

Guardar origen, periodo, geografía, unidad y confianza de cada dato. El BI debe distinguir establecimientos, cuentas, contactos, población y estadísticas sectoriales; sus cantidades no se suman entre sí.

### Cited Findings

- CE2024 informa sobre 2023 para actividad económica; también presenta universos de establecimientos encontrados en 2024. Sus totales difieren por cobertura y universo. — [Presentación, páginas 5–8](https://www.inegi.org.mx/contenidos/programas/ce/2024/doc/ce2024_presenta.pdf)
- La API de Indicadores distingue fecha de actualización, periodo de observación, unidad y multiplicador; conservar esos metadatos evita interpretar mal una serie. — [Estructura oficial](https://www.inegi.org.mx/servicios/api_indicadores.html)
- Marco Geoestadístico determina posiciones estadísticas y puede cambiar; no constituye definición oficial de límites políticos. — [Alcance del producto](https://www.inegi.org.mx/programas/mg/default.html)

### Inferences

- Modelar `establishment`, `account`, `contact`, `opportunity`, `interaction`, `offer_profile`, `source_record` y `dataset_snapshot` por separado; entidades propuestas, pendientes del diseño de arquitectura principal.
- Mantener ID DENUE como identificador externo de establecimiento. Conservar CLEE tal como se recibe; no asumir que la variante pública contiene todos los componentes para agrupar empresas.
- Agrupaciones por razón social, dominio o teléfono son sugerencias con evidencia, nunca fusiones automáticas irreversibles. Franquicia, marca y sociedad legal pueden divergir.
- Un establecimiento que desaparece de una edición no prueba cierre. Marcar «no observado», preservar CRM y verificar; comparar sólo snapshots realmente equivalentes.
- Nunca sumar DENUE y OSM como dos mercados: pueden describir el mismo negocio, múltiples objetos o establecimientos distintos. Resolver coincidencias, procedencia y nivel antes de contar.
- No sumar filas censales de total y subtotales. Indicadores de porcentajes, índices, dinero y población necesitan agregación específica; una suma genérica es incorrecta.
- TIC baja en un sector sugiere tema de investigación, no necesidad de digitalización de cada prospecto. Ventas censales por unidad son contexto, no facturación del establecimiento individual.
- Llamar «mercado observable» al recuento de establecimientos filtrados. TAM/SAM/SOM requieren hipótesis de oferta, gasto, territorio, capacidad y conversión; etiquetar estimación, método y sensibilidad.
- «Ingreso potencial» no es ingreso ganado: mostrar importes reales, propuestas y escenarios en columnas separadas. Una base sin historial comercial no permite tasas de conversión aprendidas.
- Separar caché técnica, registros propios y datasets con obligaciones ODbL. Publicar una base derivada puede activar obligaciones distintas de usar un mapa como fondo.
- Los datos abiertos de INEGI permiten distribuir una app comercial con atribución; no permiten afirmar aval del Instituto. Incluir producto/edición y notas de transformación en exportaciones.
- Mantener mapas y proveedores intercambiables. El núcleo de prospección debe funcionar sin Nominatim, tiles públicos, Data México ni datos.gob.mx.

### Gaps

- Comprobar estructura exacta CLEE de los archivos reales antes de diseñar agrupación empresarial. La norma describe componentes, pero la forma efectivamente publicada no se validó con una descarga.
- Comprobar claves territoriales/versiones antes de unir CPV2020, MG2026 y DENUE05/2026; una coincidencia textual de código no garantiza equivalencia geométrica histórica.
- Licencia de cada sitio empresarial y datos.gob.mx depende del recurso. No asumir que una página accesible permite republicar todo su contenido o automatizar extracción ilimitada.
- No se investigaron ni se recomiendan como base freemium de créditos, Google Places ilimitado, WhatsApp API gratuito, scraping de LinkedIn ni proveedores LLM pagos.
