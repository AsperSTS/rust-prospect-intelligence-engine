# Próximas etapas: BI, agentes y publicación

## Implementado en 0.2.0

Dashboard local, jornada con tres candidatos, actualizaciones manuales limitadas, reanudación, comparación de registros y fechas de consulta por grupo. Ajustes versionados y persistentes, tema en Ajustes, selección de 32 entidades y guía de instalación vacía. Hoy conserva su estructura.

## 1. Validación comercial diaria

Medir sesiones iniciadas, contactos reales, respuestas, objeciones y siguiente acción por oferta. Ajustar metas pequeñas con uso real. Probar mensajes y segmentos con muestras suficientes; ninguna apertura o copia registra envíos.

Agregar biblioteca de objeciones y experimentos comerciales, seguimiento semanal y búsqueda de contactos decisores confirmados. Mantener el CRM compartido y resultados separados para automatización/sistemas y web/marketing.

## 2. Trazabilidad de datos

Guardar fecha de consulta por establecimiento y procedencia por campo. Detectar ausencias sólo después de una revisión completa; conservar CRM y exigir revisión antes de marcar bajas. No confundir ausencia en un lote con cierre de un negocio.

Añadir catálogo de unidades INEGI y fechas/versiones publicadas cuando la fuente las proporcione. Facilitar elegir sólo grupos cuyo plazo de revisión venció, con presupuesto y alcance visibles antes de consultar.

## 3. Business intelligence completo

Importación local de CSV/XLSX del propio negocio o cliente, con validación y correspondencia de columnas. Ventas, márgenes, flujo de caja, inventario y operaciones. Separar cantidades facturadas, pactadas y cobradas; mantener monedas y periodos claros.

Cruzar Censos Económicos, Marco Geoestadístico y series oficiales con territorio y actividad. Distinguir base almacenada de universo de mercado. Priorizar herramientas locales sin suscripciones y evaluar licencias antes de añadir fuentes.

## 4. MCP local y Antigravity

Primero validar compatibilidad real del cliente con un prototipo aislado. Empezar con herramientas de lectura: consultas, métricas, pendientes y evidencia. No exponer tokens ni archivos privados mediante herramientas.

Después permitir propuestas de cambios mediante comandos Rust/Serde tipados, límites de consultas, permisos por acción e historial. Revisar manualmente las mutaciones comerciales; no permitir envío autónomo ni registrar contactos que no ocurrieron.

## 5. Antes de publicar en GitHub

Mantener la edición personal con datos y credenciales externos al repositorio. Distribuir instalaciones vacías y plantillas con marcadores: cada usuario configura sus propios tokens en Ajustes.

Revisar secretos en archivos y en todo el historial; excluir bases, contactos, exportaciones, respaldos, capturas personales y caché cartográfica. Preparar instrucciones de arranque desde cero y ejemplos completamente ficticios.

Completar auditoría legal de dependencias, atribuciones y fuentes, además de condiciones de uso y almacenamiento. La autorización del usuario para DENUE se mantiene; no se afirma que la revisión legal profunda haya terminado.

## Límites actuales

No existe actualización automática de fuentes ni un servidor MCP. La edad de consulta no garantiza una publicación reciente. El mapa requiere conexión para imágenes no almacenadas. No se ha probado escala de 50,000 registros ni distribución pública notarizada.
