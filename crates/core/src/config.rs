use crate::{Result, invalid};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug)]
pub struct AppPaths {
    pub home: PathBuf,
    pub db: PathBuf,
    pub backups: PathBuf,
    pub exports: PathBuf,
}
impl AppPaths {
    pub fn discover() -> Result<Self> {
        Self::at(Self::location()?)
    }
    /// Resolve the data directory without creating files, including on startup failure.
    pub fn location() -> Result<PathBuf> {
        let home = std::env::var_os("PROSPECTOS_HOME")
            .map(PathBuf::from)
            .or_else(|| dirs::data_dir().map(|p| p.join("Prospect Intelligence")))
            .ok_or_else(|| invalid("No se encontró un directorio de datos"))?;
        Ok(home)
    }
    pub fn at(home: impl Into<PathBuf>) -> Result<Self> {
        let home = home.into();
        let paths = Self {
            db: home.join("prospectos.db"),
            backups: home.join("backups"),
            exports: home.join("exports"),
            home,
        };
        for dir in [&paths.home, &paths.backups, &paths.exports] {
            std::fs::create_dir_all(dir)?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700))?;
            }
        }
        Ok(paths)
    }
    pub fn secrets(&self) -> Result<Secrets> {
        let path = self.home.join("credentials.env");
        let contents = if path.exists() {
            std::fs::read_to_string(path)?
        } else {
            String::new()
        };
        let mut values = BTreeMap::new();
        for line in contents.lines() {
            if let Some((k, v)) = line.split_once('=') {
                values.insert(
                    k.trim().to_owned(),
                    v.trim().trim_matches(['\'', '"']).to_owned(),
                );
            }
        }
        for key in ["DENUE_TOKEN", "INEGI_TOKEN", "BANXICO_TOKEN"] {
            if let Ok(v) = std::env::var(key) {
                values.insert(key.into(), v);
            }
        }
        let denue = values.get("DENUE_TOKEN").cloned().unwrap_or_default();
        Ok(Secrets {
            inegi: values
                .get("INEGI_TOKEN")
                .cloned()
                .unwrap_or_else(|| denue.clone()),
            denue,
            banxico: values.get("BANXICO_TOKEN").cloned().unwrap_or_default(),
        })
    }
    pub fn write_secrets(&self, values: &Secrets) -> Result<()> {
        if [&values.denue, &values.inegi, &values.banxico]
            .iter()
            .any(|s| s.contains(['\n', '\r']))
        {
            return Err(invalid("Token inválido"));
        }
        write_private(
            &self.home.join("credentials.env"),
            format!(
                "DENUE_TOKEN={}\nINEGI_TOKEN={}\nBANXICO_TOKEN={}\n",
                values.denue, values.inegi, values.banxico
            )
            .as_bytes(),
        )
    }
}
#[derive(Clone, Default)]
pub struct Secrets {
    pub denue: String,
    pub inegi: String,
    pub banxico: String,
}
// Deliberately no Debug/Serialize: never emit credentials into logs or snapshots.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Settings {
    pub schema_version: u32,
    pub denue_pages: usize,
    pub denue_max_age_days: u32,
    pub web_max_age_days: u32,
    pub inegi_max_age_days: u32,
    pub banxico_max_age_days: u32,
    pub owner_name: String,
    pub owner_whatsapp: String,
    pub owner_credential: String,
    pub profile: String,
    pub theme: String,
    pub entities: Vec<String>,
    pub estratos: Vec<String>,
    pub daily_research: usize,
    pub daily_contacts: usize,
    pub daily_followups: usize,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            schema_version: 2,
            denue_pages: 5,
            denue_max_age_days: 90,
            web_max_age_days: 30,
            inegi_max_age_days: 30,
            banxico_max_age_days: 1,
            owner_name: "Tu nombre".into(),
            owner_whatsapp: String::new(),
            owner_credential: String::new(),
            profile: "automatizacion".into(),
            theme: "light".into(),
            entities: vec!["21".into(), "29".into()],
            estratos: vec!["3".into(), "4".into(), "5".into()],
            daily_research: 5,
            daily_contacts: 3,
            daily_followups: 1,
        }
    }
}
impl Settings {
    pub fn load(paths: &AppPaths) -> Result<Self> {
        let file = paths.home.join("settings.json");
        if !file.exists() {
            return Ok(Self::default());
        }
        let mut s: Self = serde_json::from_slice(&std::fs::read(file)?)?;
        s.entities = s
            .entities
            .iter()
            .map(|e| {
                e.parse::<u8>()
                    .map(|n| format!("{n:02}"))
                    .unwrap_or_else(|_| e.clone())
            })
            .collect();
        s.entities.sort();
        s.entities.dedup();
        s.validate()?;
        Ok(s)
    }
    pub fn save(&self, paths: &AppPaths) -> Result<()> {
        self.validate()?;
        let file = paths.home.join("settings.json");
        if file.exists() {
            let old = std::fs::read(&file)?;
            let value: serde_json::Value = serde_json::from_slice(&old)?;
            let version = value["schema_version"].as_u64();
            if version.is_some_and(|v| v > 2) {
                return Err(invalid(
                    "No se pueden sobrescribir ajustes de una versión posterior",
                ));
            }
            if version != Some(2) {
                write_private(
                    &paths
                        .backups
                        .join(format!("settings-before-v2-{}.json", crate::operation_id())),
                    &old,
                )?;
            }
        }

        write_private(
            &paths.home.join("settings.json"),
            &serde_json::to_vec_pretty(self)?,
        )
    }
    pub fn validate(&self) -> Result<()> {
        if self.schema_version != 2
            || !(1..=20).contains(&self.denue_pages)
            || [
                self.denue_max_age_days,
                self.web_max_age_days,
                self.inegi_max_age_days,
                self.banxico_max_age_days,
            ]
            .iter()
            .any(|n| !(1..=3650).contains(n))
            || [
                self.daily_research,
                self.daily_contacts,
                self.daily_followups,
            ]
            .iter()
            .any(|n| *n > 1000)
        {
            return Err(invalid("Ajustes fuera de rango o versión no compatible"));
        }
        if self.owner_name.trim().is_empty()
            || self.owner_name.chars().count() > 200
            || self.owner_credential.chars().count() > 1000
        {
            return Err(invalid("Nombre o credencial inválidos"));
        }
        if !["automatizacion", "web_marketing"].contains(&self.profile.as_str())
            || !["light", "dark"].contains(&self.theme.as_str())
        {
            return Err(invalid("Perfil o tema inválidos"));
        }
        if self.entities.is_empty()
            || self.entities.iter().any(|e| {
                e.parse::<u8>().is_err() || !(1..=32).contains(&e.parse::<u8>().unwrap_or(0))
            })
        {
            return Err(invalid("Selecciona entidades entre 01 y 32"));
        }
        if self.estratos.is_empty()
            || self
                .estratos
                .iter()
                .any(|e| !["1", "2", "3", "4", "5", "6", "7"].contains(&e.as_str()))
        {
            return Err(invalid("Estratos inválidos"));
        }
        Ok(())
    }
}
pub fn write_private(path: &Path, bytes: &[u8]) -> Result<()> {
    use std::io::Write;
    let temp = path.with_file_name(format!(".private-{}.tmp", crate::operation_id()));
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut f = options.open(&temp)?;
    f.write_all(bytes)?;
    f.sync_all()?;
    std::fs::rename(&temp, path)?;
    Ok(())
}
pub const STATES: &[&str] = &[
    "nuevo",
    "revisado",
    "contactado",
    "respondio",
    "respuesta_positiva",
    "reunion",
    "propuesta",
    "cliente",
    "no_interesado",
    "sin_respuesta",
    "descartado",
];
pub const CLOSED: &[&str] = &["descartado", "no_interesado", "sin_respuesta", "cliente"];
pub fn today() -> chrono::NaiveDate {
    chrono::Utc::now()
        .with_timezone(&chrono::FixedOffset::west_opt(6 * 3600).unwrap())
        .date_naive()
}
pub fn now() -> String {
    chrono::Utc::now()
        .with_timezone(&chrono::FixedOffset::west_opt(6 * 3600).unwrap())
        .format("%Y-%m-%dT%H:%M:%S")
        .to_string()
}
pub const ENTITIES: &[(&str, &str)] = &[
    ("01", "Aguascalientes"),
    ("02", "Baja California"),
    ("03", "Baja California Sur"),
    ("04", "Campeche"),
    ("05", "Coahuila"),
    ("06", "Colima"),
    ("07", "Chiapas"),
    ("08", "Chihuahua"),
    ("09", "Ciudad de México"),
    ("10", "Durango"),
    ("11", "Guanajuato"),
    ("12", "Guerrero"),
    ("13", "Hidalgo"),
    ("14", "Jalisco"),
    ("15", "Estado de México"),
    ("16", "Michoacán"),
    ("17", "Morelos"),
    ("18", "Nayarit"),
    ("19", "Nuevo León"),
    ("20", "Oaxaca"),
    ("21", "Puebla"),
    ("22", "Querétaro"),
    ("23", "Quintana Roo"),
    ("24", "San Luis Potosí"),
    ("25", "Sinaloa"),
    ("26", "Sonora"),
    ("27", "Tabasco"),
    ("28", "Tamaulipas"),
    ("29", "Tlaxcala"),
    ("30", "Veracruz"),
    ("31", "Yucatán"),
    ("32", "Zacatecas"),
];
pub fn entity_name(code: &str) -> &str {
    let n = code.parse::<u8>().unwrap_or(0);
    ENTITIES
        .iter()
        .find(|(c, _)| c.parse::<u8>().ok() == Some(n))
        .map(|(_, name)| *name)
        .unwrap_or(code)
}

pub fn parse_date(s: &str) -> Result<chrono::NaiveDate> {
    chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d")
        .map_err(|_| invalid("Fecha inválida, usa AAAA-MM-DD"))
}

#[cfg(test)]
mod persistence_tests {
    use super::*;
    #[test]
    fn old_settings_load_with_defaults_and_saved_preferences_survive_restart() {
        let temp = tempfile::tempdir().unwrap();
        let paths = AppPaths::at(temp.path()).unwrap();
        write_private(
            &paths.home.join("settings.json"),
            br#"{"owner_name":"Usuario", "theme":"dark", "entities":["1","01","21"]}"#,
        )
        .unwrap();
        let mut settings = Settings::load(&paths).unwrap();
        assert_eq!(settings.entities, vec!["01", "21"]);
        assert_eq!(settings.denue_pages, 5);
        assert_eq!(settings.schema_version, 2);
        settings.theme = "light".into();
        settings.daily_contacts = 1;
        settings.denue_pages = 2;
        settings.denue_max_age_days = 120;
        settings.save(&paths).unwrap();
        assert_eq!(Settings::load(&paths).unwrap(), settings);
        assert!(!paths.home.join("credentials.env").exists());
        assert_eq!(std::fs::read_dir(&paths.backups).unwrap().count(), 1);
        settings.save(&paths).unwrap();
        assert_eq!(std::fs::read_dir(&paths.backups).unwrap().count(), 1);
    }
    #[test]
    fn invalid_limits_and_future_settings_versions_are_rejected() {
        let mut settings = Settings {
            denue_pages: 0,
            ..Default::default()
        };
        assert!(settings.validate().is_err());
        settings.denue_pages = 21;
        assert!(settings.validate().is_err());
        settings.denue_pages = 1;
        settings.web_max_age_days = 0;
        assert!(settings.validate().is_err());
        settings.web_max_age_days = 30;
        settings.schema_version = 3;
        assert!(settings.validate().is_err());
        assert_eq!(entity_name("09"), "Ciudad de México");
        assert_eq!(entity_name("1"), "Aguascalientes");
        assert_eq!(ENTITIES.len(), 32);
    }
}
