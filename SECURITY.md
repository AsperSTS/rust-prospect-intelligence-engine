# Seguridad

## Reportar una vulnerabilidad

Si detectas un problema de seguridad, **no abras un Issue público**.

Repórtalo de forma privada mediante **GitHub Security Advisories** en este repositorio (`Security → Advisories → Report a vulnerability`), incluyendo pasos para reproducirlo y el impacto esperado.

## Alcance

- Almacenamiento y permisos de credenciales locales (`credentials.env`).
- Validación TLS y manejo de solicitudes externas (INEGI DENUE, indicadores y enriquecimiento web).
- Scripts de empaquetado y flujos de CI/Release.

## Notas actuales

- La app guarda tokens ingresados por el usuario en `credentials.env` con permisos `0600` fuera del repositorio.
- Los releases públicos se compilan sin tokens embebidos y pasan verificación de higiene (`scripts/verify-public-hygiene.sh`).
- Está pendiente migrar el almacenamiento de credenciales locales a Apple Keychain en macOS e iOS.
