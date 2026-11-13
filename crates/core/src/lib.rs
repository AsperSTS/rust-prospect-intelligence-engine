#![forbid(unsafe_code)]
pub mod analytics;
pub mod archive;
pub mod config;
pub mod contacts;
pub mod db;
pub mod domain;
pub mod enrich;
pub mod groups;
pub mod jobs;
pub mod maps;
pub mod messages;
pub mod rules;
pub mod sources;
pub mod updates;
pub mod workspace;
pub use config::{AppPaths, Settings};
pub use db::Db;
pub use domain::*;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Validation(String),
    #[error("Error SQLite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("Error de archivo: {0}")]
    Io(#[from] std::io::Error),
    #[error("Datos JSON inválidos: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Configuración TOML inválida: {0}")]
    Toml(#[from] toml::de::Error),
    #[error("Exportación inválida: {0}")]
    Excel(#[from] rust_xlsxwriter::XlsxError),
    #[error("CSV inválido: {0}")]
    Csv(#[from] csv::Error),
    #[error("La base de datos cerró su trabajador")]
    WorkerClosed,
    #[error("Operación cancelada")]
    Cancelled,
}
pub type Result<T> = std::result::Result<T, Error>;
pub fn invalid(message: impl Into<String>) -> Error {
    Error::Validation(message.into())
}
