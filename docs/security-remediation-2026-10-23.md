# Revisión de dependencias de seguridad — 2026-10-23

Se conservan las ramas y los datos personales. Cambios de lock/manifiesto:

- webpki-root-certs1.0.10: retirada de raíces eMudhra, checksum de crates.io verificado; licencia CDLA-Permissive-2.0. Excepción de antigüedad por seguridad.
- rustls-webpki0.103.16: RUSTSEC-2026-0382, rechazo de intermediarios comprometidos. ISC; sin build.rs. Excepción de antigüedad por seguridad.
- rustls0.23.46: RUSTSEC-2026-0346/0347/0348/0349. Apache-2.0 OR ISC OR MIT. build.rs revisado completamente: sólo configura read_buf para Rust nightly; sin red, procesos ni lectura de credenciales. La allowlist de cargo-deny pasa exclusivamente de0.23.45 a0.23.46.
- tokio1.53.3: RUSTSEC-2026-0386, dentro de la rama1.53; MIT y sin build.rs.
- rust_xlsxwriter0.100.0 permite ZIP9; ZIP9.0.0 corrige RUSTSEC-2026-0367/0368/0369. MIT/Apache y MIT respectivamente; ambos sin build.rs. Excepción de siete días limitada a rust_xlsxwriter porque el cambio de dependencia es necesario para retirar ZIP8 vulnerable. ZIP9.0.0 se publicó2026-10-09;9.0.5 tiene menos de siete días y sus notas sólo describen correcciones BSD/AES ajenas a esta exportación.

Fuentes primarias: https://rustsec.org/advisories/ y https://github.com/rustls/webpki-roots/releases/tag/v/1.0.10 . Metadatos/licencias y hashes comprobados en crates.io. La auditoría legal profunda continúa fuera de alcance; cargo-deny aplica la política macOS existente.

Las validaciones usan PROSPECTOS_HOME desechable. No se leen ni actualizan el CRM ni las bases personales.
