# Bitácora de Configuración y Entorno

Registro de componentes, herramientas y dependencias instaladas en el sistema para el desarrollo y soporte de Prospect Intelligence (incluyendo macOS y el soporte para iOS), junto con las instrucciones para desinstalarlas o revertirlas.

## Línea de tiempo

| Fecha y hora | Acción | Detalle |
|---|---|---|
| **2026-10-21 21:55** | Inicialización del repositorio Git | Inicialización de repositorio con usuario `Elvis Yael De los Santos <bakufix552@gmail.com>` y commit inicial: `"Commit inicial proyecto"`. |
| **2026-10-21 21:55** | Creación y activación de rama `dev` | Creación de la rama `dev` como entorno de trabajo activo. |
| **2026-10-21 22:05** | Investigación de arquitectura iOS | Análisis de viabilidad con Dioxus Mobile (WebKit), sustitución de dependencias incompatibles (`arboard`, `opener`) y sandbox de iOS. |
| **2026-10-21 22:12** | Limpieza de archivos obsoletos | Eliminación de archivos `.DS_Store`, retiro de `PLAN_MIGRACION.md` y consolidación de `PORTFOLIO.md` en el README. |
| **2026-10-21 22:12** | Reubicación de scripts | Reubicación de `seed_demo.py` de `assets/portfolio/` a `scripts/`. |
| **2026-10-21 22:13** | Actualización de documentación | Reescritura del `README.md` con capturas del sistema y actualización de `docs/VERIFICACION_Y_LIMITES.md` y `docs/HOJA_DE_RUTA_BI_MCP.md`. |
| **2026-10-21 22:16** | Instalación de targets de Rust para iOS | Adición de targets `aarch64-apple-ios` y `aarch64-apple-ios-sim` mediante rustup. |
| **2026-10-21 22:20** | Instalación de Dioxus CLI | Instalación de `dx` (versión 0.7.10) en `~/.cargo/bin/dx`. |

## Herramientas instaladas

### 1. Targets de Rust para iOS (`aarch64-apple-ios` y `aarch64-apple-ios-sim`)
- **Propósito:** Compilación cruzada hacia dispositivos iOS físicos (ARM64) y simulador de iOS en Apple Silicon.
- **Comando ejecutado:**
  ```bash
  rustup target add aarch64-apple-ios aarch64-apple-ios-sim
  ```
- **Ruta de instalación:** `~/.rustup/toolchains/<toolchain>/lib/rustlib/aarch64-apple-ios` y `aarch64-apple-ios-sim`.
- **Comando de desinstalación:**
  ```bash
  rustup target remove aarch64-apple-ios aarch64-apple-ios-sim
  ```

### 2. Dioxus CLI (`dx` 0.7.10)
- **Propósito:** Generación de configuración y empaquetado para plataformas móviles (`dx bundle --platform ios`, `dx serve --platform ios`).
- **Instalación:** Binario precompilado oficial para `aarch64-apple-darwin` colocado en `~/.cargo/bin/dx`.
- **Ruta de instalación:** `~/.cargo/bin/dx`
- **Comando de desinstalación:**
  ```bash
  rm -f ~/.cargo/bin/dx
  ```
