# Prospect Intelligence

Aplicación de escritorio para macOS que ayuda a un consultor independiente a encontrar, priorizar y dar seguimiento a establecimientos B2B a partir de datos públicos de INEGI/DENUE. Reúne la prospección, un CRM ligero y las métricas comerciales en una base SQLite local.

## Problem

La prospección B2B suele repartirse entre listados públicos, hojas de cálculo y notas sueltas. No hay un criterio uniforme para decidir a quién contactar primero. Tampoco se sabe qué seguimientos están vencidos ni qué segmentos responden mejor.

## Solution

El sistema importa establecimientos desde la API de DENUE y los guarda en SQLite. Calcula dos puntajes con reglas explícitas: *fit* (qué tan bien encaja el giro y el tamaño) y *prioridad* (qué conviene revisar hoy). Revisa el sitio web declarado de cada negocio para detectar señales públicas verificables, como certificación ISO o fabricación sobre pedido. Con esa evidencia redacta un mensaje editable. La persona lo revisa y lo envía por su cuenta, y luego registra el contacto, la respuesta y las oportunidades.

## Main capabilities

- Importación de establecimientos desde DENUE por entidad y estrato de personal.
- Enriquecimiento web: detección de señales textuales, correo y WhatsApp del sitio declarado.
- Puntaje de fit y prioridad con desglose consultable ("Por qué tiene este Fit").
- Detección de posibles cadenas o grupos (mismo teléfono, mismo dominio) para no contar empresas duplicadas.
- Borradores de mensaje a partir de plantillas y evidencia; el envío es manual.
- Registro de contactos, respuestas, seguimientos programados y notas.
- Cartera de oportunidades con etapa, importe en MXN o USD y cobrado acumulado.
- Métricas de embudo y comparación por giro, con aviso explícito de muestra pequeña.
- Exportación a CSV y XLSX, y respaldo de la base local.
- Consulta de indicadores de Banxico e INEGI como contexto económico.

## Stack

- Rust (workspace con tres crates: `core`, `desktop`, `cli`).
- Dioxus Desktop para la interfaz.
- SQLite para el almacenamiento local.
- Fuentes de datos: API DENUE de INEGI, API de indicadores INEGI y Banxico SIE.

## How it works

`DENUE -> importación -> enriquecimiento web -> reglas de fit y prioridad -> lista del día -> mensaje editable -> registro de contacto y respuesta -> métricas y oportunidades`

La interfaz y la línea de comandos usan el mismo crate `core`. La CLI permite importar, analizar, exportar y respaldar sin abrir la aplicación.

## Technical decisions

- **Reglas, no modelos.** Los puntajes salen de reglas configurables en JSON. Cada puntaje se puede explicar. El proyecto no usa AI ni ML.
- **Datos locales.** La base, los respaldos y las credenciales viven en el directorio de la aplicación del usuario. Los tokens no se guardan en el código.
- **Envío manual.** La aplicación abre o copia el mensaje. No envía correos ni WhatsApp.
- **Lenguaje cuidadoso.** Las hipótesis se redactan como posibles fricciones con su evidencia. Una lista de frases prohibidas y de afirmaciones sin sustento valida las plantillas.
- **Migración verificable.** El código se migra desde una versión en Python. Hay pruebas de paridad con fixtures para comparar los resultados de ambas.

## Project status

Prototipo funcional en migración. La compilación nativa de desarrollo funciona. El empaquetado para macOS y las verificaciones finales siguen en curso. Las capturas son de una compilación de desarrollo.

## Limitations

- Uso personal y de un solo usuario. No hay cuentas, sincronización ni acceso web.
- Un establecimiento de DENUE no equivale a una empresa única. Los conteos describen la base guardada, no el mercado.
- La vista de mapa de Territorios necesita coordenadas. Sin ellas solo muestra la tabla por municipio.
- Los puntajes son reglas de prioridad. No son probabilidades de compra.
- La importación y el enriquecimiento requieren token de DENUE y red. La revisión legal de las fuentes se hará después de la migración.

## Screenshots

Todas las capturas usan datos sintéticos: 142 establecimientos ficticios con dominios `.test` y teléfonos de prueba, y un remitente ficticio. No provienen de la base real. Miden 1920x957 px, el máximo que permitió la pantalla.

### 01 - Lista priorizada del día
`01-today-prioritized-list.png`

Muestra la lista ordenada por prioridad, el contexto del establecimiento con su hipótesis y evidencia, y el borrador editable. Es el flujo principal: decidir a quién contactar y con qué mensaje.

### 02 - Seguimientos vencidos
`02-follow-ups.png`

Lista los contactos cuya fecha de seguimiento ya llegó, con un borrador de seguimiento. Evita que un contacto sin respuesta se pierda.

### 03 - Conversaciones abiertas
`03-open-conversations.png`

Agrupa los establecimientos que respondieron, tienen reunión o propuesta. Desde aquí se registra el siguiente avance y se guardan notas.

### 04 - Métricas comerciales
`04-commercial-metrics.png`

Muestra el embudo de contactados a clientes y una comparación por giro. Incluye el aviso de que una muestra pequeña no permite declarar ganadores.

### 05 - Cartera de oportunidades
`05-opportunities.png`

Lista las oportunidades por etapa con importe y cobrado, junto con el formulario de captura. Separa el importe de una propuesta del ingreso realmente cobrado.

## Short portfolio description

Aplicación de escritorio en Rust y Dioxus para prospección B2B con datos públicos de INEGI/DENUE. Calcula fit y prioridad con reglas explicables, detecta señales en sitios web, redacta mensajes editables y da seguimiento a contactos, oportunidades y métricas en una base SQLite local. Prototipo funcional en migración desde Python.

## One-line description

App de escritorio en Rust que prioriza prospectos B2B de DENUE con reglas explicables y da seguimiento comercial local.
