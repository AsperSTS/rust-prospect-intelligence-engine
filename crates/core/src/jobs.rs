use crate::{Error, Result, operation_id};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use tokio_util::sync::CancellationToken;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct JobState {
    pub pages: usize,
    pub requests: usize,
    pub unchanged: usize,
    pub id: String,
    pub kind: String,
    pub status: String,
    pub processed: usize,
    pub inserted: usize,
    pub updated: usize,
    pub invalid: usize,
    pub message: String,
}
#[derive(Clone)]
pub struct JobControl {
    pub token: CancellationToken,
    state: Arc<Mutex<JobState>>,
}
impl JobControl {
    pub fn new(kind: &str) -> Self {
        Self {
            token: CancellationToken::new(),
            state: Arc::new(Mutex::new(JobState {
                id: operation_id(),
                kind: kind.into(),
                status: "running".into(),
                message: "Preparando…".into(),
                ..Default::default()
            })),
        }
    }
    pub fn check(&self) -> Result<()> {
        if self.token.is_cancelled() {
            Err(Error::Cancelled)
        } else {
            Ok(())
        }
    }
    pub fn snapshot(&self) -> JobState {
        self.state.lock().unwrap_or_else(|p| p.into_inner()).clone()
    }
    pub fn update(&self, update: impl FnOnce(&mut JobState)) {
        update(&mut self.state.lock().unwrap_or_else(|p| p.into_inner()));
    }
    pub fn finish(&self, result: &Result<usize>) {
        self.update(|s| match result {
            Ok(n) => {
                s.status = "completed".into();
                s.message = if s.pages > 0 {
                    format!(
                        "Lote terminado · {} páginas · {} nuevos · {} cambios · {} sin cambios",
                        s.pages, s.inserted, s.updated, s.unchanged
                    )
                } else {
                    format!("Listo · {n} registros procesados")
                };
            }
            Err(Error::Cancelled) => {
                s.status = "cancelled".into();
                s.message = "Cancelado. Se conservaron los lotes completos.".into();
            }
            Err(e) => {
                s.status = "failed".into();
                s.message = e.to_string();
            }
        });
    }
    pub async fn wait(&self, duration: std::time::Duration) -> Result<()> {
        tokio::select! {_=self.token.cancelled()=>Err(Error::Cancelled),_=tokio::time::sleep(duration)=>Ok(())}
    }
}
