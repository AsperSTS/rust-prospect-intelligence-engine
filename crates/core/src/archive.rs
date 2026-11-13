//! Portable backup: one zip with the SQLite snapshot, settings, templates and tokens.
//!
//! The archive is user data and untrusted on the way back in: entries are read by fixed
//! name with size caps, never extracted by their own paths, and validated before use.
use crate::{AppPaths, Db, Result, Settings, config, invalid, messages::Templates};
use rusqlite::{Connection, OpenFlags};
use serde::{Deserialize, Serialize};
use std::{
    io::{Read, Write},
    path::{Path, PathBuf},
    time::Duration,
};

const FORMAT: u32 = 1;
const MANIFEST: &str = "manifest.json";
const DATABASE: &str = "prospectos.db";
const SETTINGS: &str = "settings.json";
const TEMPLATES: &str = "templates.toml";
const CREDENTIALS: &str = "credentials.env";
const MAX_TEXT: u64 = 1_000_000;
const MAX_DATABASE: u64 = 8_000_000_000;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    format: u32,
    application: String,
    created_at: String,
}

/// What a restore brought back, for the interface to reload and report.
#[derive(Clone, Debug, PartialEq)]
pub struct Restored {
    pub settings: Settings,
    pub records: usize,
    pub tokens: bool,
    pub created_at: String,
}

fn zip_error(e: zip::result::ZipError) -> crate::Error {
    invalid(format!("Respaldo ilegible: {e}"))
}

fn private_file(path: &Path) -> Result<std::fs::File> {
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    Ok(options.open(path)?)
}

fn write_archive(
    destination: &Path,
    snapshot: &Path,
    settings: &Settings,
    home: &Path,
) -> Result<()> {
    let name = destination
        .file_name()
        .ok_or_else(|| invalid("Elige un nombre de archivo para el respaldo"))?
        .to_string_lossy()
        .into_owned();
    let temp = destination.with_file_name(format!(".{name}.{}.tmp", crate::operation_id()));
    let result = (|| -> Result<()> {
        let mut zip = zip::ZipWriter::new(private_file(&temp)?);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated)
            .unix_permissions(0o600);
        let manifest = Manifest {
            format: FORMAT,
            application: format!("Prospect Intelligence {}", env!("CARGO_PKG_VERSION")),
            created_at: config::now(),
        };
        let mut text = vec![
            (MANIFEST, serde_json::to_vec_pretty(&manifest)?),
            (SETTINGS, serde_json::to_vec_pretty(settings)?),
            (TEMPLATES, Templates::source(home)?.into_bytes()),
        ];
        let credentials = home.join(CREDENTIALS);
        if credentials.exists() {
            text.push((CREDENTIALS, std::fs::read(credentials)?));
        }
        for (entry, bytes) in text {
            zip.start_file(entry, options).map_err(zip_error)?;
            zip.write_all(&bytes)?;
        }
        zip.start_file(DATABASE, options.large_file(true))
            .map_err(zip_error)?;
        std::io::copy(&mut std::fs::File::open(snapshot)?, &mut zip)?;
        zip.finish().map_err(zip_error)?.sync_all()?;
        std::fs::rename(&temp, destination)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temp);
    }
    result
}

struct Unpacked {
    manifest: Manifest,
    settings: Option<Settings>,
    templates: Option<String>,
    credentials: Option<config::Secrets>,
}

fn read_text<R: Read + std::io::Seek>(
    zip: &mut zip::ZipArchive<R>,
    name: &str,
) -> Result<Option<String>> {
    let entry = match zip.by_name(name) {
        Ok(entry) => entry,
        Err(zip::result::ZipError::FileNotFound) => return Ok(None),
        Err(e) => return Err(zip_error(e)),
    };
    let mut text = String::new();
    entry.take(MAX_TEXT + 1).read_to_string(&mut text)?;
    if text.len() as u64 > MAX_TEXT {
        return Err(invalid(format!("{name} excede el tamaño permitido")));
    }
    Ok(Some(text))
}

/// Validates every entry and leaves the database copy at `database`.
fn unpack(archive: &Path, database: &Path) -> Result<Unpacked> {
    let mut zip = zip::ZipArchive::new(std::fs::File::open(archive)?).map_err(zip_error)?;
    let manifest: Manifest = serde_json::from_str(
        &read_text(&mut zip, MANIFEST)?
            .ok_or_else(|| invalid("El archivo no es un respaldo de Prospect Intelligence"))?,
    )
    .map_err(|_| invalid("El archivo no es un respaldo de Prospect Intelligence"))?;
    if manifest.format != FORMAT {
        return Err(invalid(
            "Este respaldo viene de una versión más reciente de la aplicación",
        ));
    }
    let settings = read_text(&mut zip, SETTINGS)?
        .map(|text| Settings::from_json(text.as_bytes()))
        .transpose()?;
    let templates = read_text(&mut zip, TEMPLATES)?;
    if let Some(source) = &templates {
        Templates::parse_toml(source)?.validate()?;
    }
    let credentials = read_text(&mut zip, CREDENTIALS)?.map(|text| config::Secrets::parse(&text));
    if let Some(secrets) = &credentials
        && [&secrets.denue, &secrets.inegi, &secrets.banxico]
            .iter()
            .any(|token| !token.is_empty() && !config::token_shape_ok(token))
    {
        return Err(invalid(
            "El respaldo contiene un token con formato inválido",
        ));
    }
    {
        let entry = zip
            .by_name(DATABASE)
            .map_err(|_| invalid("El respaldo no contiene la base de datos"))?;
        let mut out = private_file(database)?;
        let copied = std::io::copy(&mut entry.take(MAX_DATABASE + 1), &mut out)?;
        if copied > MAX_DATABASE {
            return Err(invalid("La base del respaldo excede el tamaño permitido"));
        }
        out.sync_all()?;
    }
    let source = Connection::open_with_flags(database, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    crate::db::verify(&source)?;
    let version: i64 = source.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    let has_table: i64 = source.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='prospectos'",
        [],
        |r| r.get(0),
    )?;
    if version > 2 || has_table == 0 {
        return Err(invalid(
            "La base del respaldo no es compatible con esta versión",
        ));
    }
    Ok(Unpacked {
        manifest,
        settings,
        templates,
        credentials,
    })
}

impl Db {
    /// Writes a consistent, self-contained backup to a path chosen by the user.
    pub async fn backup_archive(
        &self,
        destination: PathBuf,
        settings: Settings,
    ) -> Result<PathBuf> {
        settings.validate()?;
        let snapshot = self
            .paths
            .backups
            .join(format!(".snapshot-{}.db", crate::operation_id()));
        let snapshot = self.restore_to(snapshot).await?;
        let home = self.paths.home.clone();
        let (target, source) = (destination.clone(), snapshot.clone());
        let result =
            tokio::task::spawn_blocking(move || write_archive(&target, &source, &settings, &home))
                .await
                .map_err(|_| invalid("No se pudo completar el respaldo"));
        let _ = std::fs::remove_file(&snapshot);
        result??;
        Ok(destination)
    }

    /// Loads a backup into a fresh installation. Refuses to replace a base that has records.
    pub async fn restore_archive(&self, archive: PathBuf) -> Result<Restored> {
        let paths = self.paths.clone();
        let staged = paths
            .home
            .join(format!(".restore-{}.db", crate::operation_id()));
        let result = self.restore_staged(archive, staged.clone(), paths).await;
        let _ = std::fs::remove_file(&staged);
        self.clear_cache();
        result
    }

    async fn restore_staged(
        &self,
        archive: PathBuf,
        staged: PathBuf,
        paths: AppPaths,
    ) -> Result<Restored> {
        let target = staged.clone();
        let unpacked = tokio::task::spawn_blocking(move || unpack(&archive, &target))
            .await
            .map_err(|_| invalid("No se pudo leer el respaldo"))??;
        let worker_paths = paths.clone();
        let records = self
            .call(move |live| {
                let current: i64 =
                    live.query_row("SELECT COUNT(*) FROM prospectos", [], |r| r.get(0))?;
                if current > 0 {
                    return Err(invalid(
                        "Tu base ya tiene empresas; un respaldo sólo se carga en una instalación nueva",
                    ));
                }
                let source =
                    Connection::open_with_flags(&staged, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
                rusqlite::backup::Backup::new(&source, live)?.run_to_completion(
                    100,
                    Duration::from_millis(5),
                    None,
                )?;
                drop(source);
                crate::db::initialize(live, &worker_paths)?;
                crate::db::verify(live)?;
                let records: i64 =
                    live.query_row("SELECT COUNT(*) FROM prospectos", [], |r| r.get(0))?;
                Ok(records as usize)
            })
            .await?;
        let settings = match unpacked.settings {
            Some(settings) => {
                settings.save(&paths)?;
                settings
            }
            None => Settings::load_desktop(&paths)?,
        };
        if let Some(source) = &unpacked.templates {
            Templates::save(&paths.home, source)?;
        }
        let tokens = match &unpacked.credentials {
            Some(secrets) if !secrets.denue.is_empty() || !secrets.banxico.is_empty() => {
                paths.write_secrets(secrets)?;
                true
            }
            _ => false,
        };
        Ok(Restored {
            settings,
            records,
            tokens,
            created_at: unpacked.manifest.created_at,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn archive_roundtrip_restores_base_settings_templates_and_tokens_into_a_fresh_home() {
        let origin_dir = tempfile::tempdir().unwrap();
        let origin_paths = AppPaths::at(origin_dir.path()).unwrap();
        let origin = Db::open(origin_paths.clone()).unwrap();
        origin
            .call(|c| {
                c.execute(
                    "INSERT INTO prospectos(id,nombre,entidad) VALUES('a1','Taller Uno','Puebla')",
                    [],
                )?;
                Ok(())
            })
            .await
            .unwrap();
        origin_paths
            .write_secrets(&config::Secrets {
                denue: "00000000-0000-0000-0000-000000000001".into(),
                inegi: "00000000-0000-0000-0000-000000000001".into(),
                banxico: String::new(),
            })
            .unwrap();
        let settings = Settings {
            owner_name: "Persona de prueba".into(),
            entities: vec!["09".into(), "21".into()],
            ..Default::default()
        };
        let file = origin_dir.path().join("fuera").join("respaldo.zip");
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        origin
            .backup_archive(file.clone(), settings.clone())
            .await
            .unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&file).unwrap().permissions().mode() & 0o777;
            assert_eq!(mode, 0o600);
        }
        assert_eq!(std::fs::read_dir(&origin_paths.backups).unwrap().count(), 0);

        let fresh_dir = tempfile::tempdir().unwrap();
        let fresh_paths = AppPaths::at(fresh_dir.path()).unwrap();
        let fresh = Db::open(fresh_paths.clone()).unwrap();
        let restored = fresh.restore_archive(file.clone()).await.unwrap();
        assert_eq!(restored.records, 1);
        assert!(restored.tokens);
        assert_eq!(restored.settings, settings);
        assert_eq!(Settings::load(&fresh_paths).unwrap(), settings);
        assert_eq!(
            fresh_paths.secrets().unwrap().denue,
            "00000000-0000-0000-0000-000000000001"
        );
        let name: String = fresh
            .call(|c| Ok(c.query_row("SELECT nombre FROM prospectos", [], |r| r.get(0))?))
            .await
            .unwrap();
        assert_eq!(name, "Taller Uno");
        assert_eq!(fresh.entity_counts().await.unwrap().get("Puebla"), Some(&1));

        // A base with records is never replaced.
        assert!(fresh.restore_archive(file).await.is_err());
        assert!(std::fs::read_dir(&fresh_paths.home).unwrap().all(|e| {
            !e.unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with(".restore-")
        }));
    }

    #[tokio::test]
    async fn foreign_or_damaged_archives_are_rejected_without_touching_the_base() {
        let dir = tempfile::tempdir().unwrap();
        let paths = AppPaths::at(dir.path()).unwrap();
        let db = Db::open(paths.clone()).unwrap();
        let not_zip = dir.path().join("nota.zip");
        std::fs::write(&not_zip, b"no es un zip").unwrap();
        assert!(db.restore_archive(not_zip).await.is_err());

        let foreign = dir.path().join("ajeno.zip");
        let mut zip = zip::ZipWriter::new(std::fs::File::create(&foreign).unwrap());
        let options = zip::write::SimpleFileOptions::default();
        zip.start_file("../../evil.txt", options).unwrap();
        zip.write_all(b"x").unwrap();
        zip.start_file(MANIFEST, options).unwrap();
        zip.write_all(br#"{"format":1,"application":"x","created_at":"x","extra":1}"#)
            .unwrap();
        zip.finish().unwrap();
        assert!(db.restore_archive(foreign).await.is_err());
        assert!(!dir.path().join("evil.txt").exists());
        assert!(!paths.home.join("settings.json").exists());
        assert!(db.entity_counts().await.unwrap().is_empty());
    }
}
