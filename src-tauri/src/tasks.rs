//! Transfer state belongs to the launcher process, independently of any screen.
use crate::{error::AppError, state::AppState};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Mutex,
    },
    time::{Instant, SystemTime, UNIX_EPOCH},
};
use tauri::{Emitter, Listener, Manager};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transfer {
    pub id: String,
    pub slot: String,
    pub kind: String,
    pub status: String,
    pub game_dir: String,
    pub download_dir: String,
    pub progress: Value,
    pub error: Option<String>,
    pub result: Value,
    #[serde(default)]
    pub discard: bool,
}
impl Transfer {
    pub fn active(&self) -> bool {
        matches!(self.status.as_str(), "running" | "pausing" | "cancelling")
    }
    pub fn can_stop(&self) -> bool {
        self.active() && self.progress["stage"] != "extracting"
    }
}

pub struct Transfers {
    data: Mutex<BTreeMap<String, Transfer>>,
    path: PathBuf,
    persisted: Mutex<Instant>,
}
impl Transfers {
    pub fn load() -> Self {
        Self::from_path(crate::config::paths::config_dir().join("transfers.json"))
    }
    fn from_path(path: PathBuf) -> Self {
        let mut data: BTreeMap<String, Transfer> = std::fs::read(&path)
            .ok()
            .and_then(|s| serde_json::from_slice(&s).ok())
            .unwrap_or_default();
        for task in data.values_mut() {
            if task.active() {
                task.status = "paused".into();
                task.discard = false;
            }
        }
        Self {
            data: Mutex::new(data),
            path,
            persisted: Mutex::new(Instant::now()),
        }
    }
    fn persist(&self, data: &BTreeMap<String, Transfer>) {
        if let Some(parent) = self.path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(bytes) = serde_json::to_vec(data) {
            let temp = self.path.with_extension("tmp");
            if std::fs::write(&temp, bytes).is_ok() {
                let _ = std::fs::rename(temp, &self.path);
            }
        }
    }
    pub fn snapshot(&self) -> BTreeMap<String, Transfer> {
        self.data.lock().unwrap().clone()
    }
    pub fn busy(&self, slot: &str) -> bool {
        self.data
            .lock()
            .unwrap()
            .get(slot)
            .is_some_and(Transfer::active)
    }
    pub fn cancelled(&self, slot: &str) -> bool {
        self.data
            .lock()
            .unwrap()
            .get(slot)
            .is_some_and(|t| matches!(t.status.as_str(), "pausing" | "cancelling"))
    }
    pub fn begin(
        &self,
        app: &tauri::AppHandle,
        slot: &str,
        kind: &str,
        game_dir: &str,
        download_dir: &str,
        active: &AtomicBool,
    ) -> Result<String, AppError> {
        let mut data = self.data.lock().unwrap();
        if data.get(slot).is_some_and(Transfer::active) {
            return Err(AppError::Api("A transfer is already running".into()));
        }
        let id = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
            .to_string();
        let task = Transfer {
            id: id.clone(),
            slot: slot.into(),
            kind: kind.into(),
            status: "running".into(),
            game_dir: game_dir.into(),
            download_dir: download_dir.into(),
            progress: json!({"stage":"fetching"}),
            error: None,
            result: Value::Null,
            discard: false,
        };
        active.store(true, Ordering::SeqCst);
        data.insert(slot.into(), task.clone());
        self.persist(&data);
        drop(data);
        let _ = app.emit("task://changed", task);
        Ok(id)
    }
    pub fn progress(
        &self,
        app: &tauri::AppHandle,
        slot: &str,
        mut progress: Value,
        stage: Option<&str>,
    ) {
        let mut data = self.data.lock().unwrap();
        let Some(task) = data.get_mut(slot).filter(|t| t.active()) else {
            return;
        };
        if let Some(stage) = stage {
            progress["stage"] = stage.into();
        }
        // Normalise VFS and pack progress into the same byte/file contract.
        if progress.get("bytes_done").is_some() {
            progress["bytes_downloaded"] = progress["bytes_done"].clone();
        }
        let stage_changed = task.progress["stage"] != progress["stage"];
        task.progress = progress;
        let next = task.clone();
        let mut persisted = self.persisted.lock().unwrap();
        if stage_changed || persisted.elapsed().as_secs() >= 2 {
            self.persist(&data);
            *persisted = Instant::now();
        }
        drop(persisted);
        drop(data);
        let _ = app.emit("task://changed", next);
    }
    pub fn stop(
        &self,
        app: &tauri::AppHandle,
        slot: &str,
        discard: bool,
        active: &AtomicBool,
    ) -> Result<(), AppError> {
        let mut data = self.data.lock().unwrap();
        let Some(task) = data.get_mut(slot) else {
            return Ok(());
        };
        if task.active() && !task.can_stop() {
            return Err(AppError::Api("Wait for file extraction to finish".into()));
        }
        active.store(false, Ordering::SeqCst);
        task.discard = discard;
        if task.active() {
            task.status = if discard { "cancelling" } else { "pausing" }.into();
        } else if discard {
            Self::discard_files(task)?;
            task.status = "idle".into();
            task.progress = Value::Null;
            task.error = None;
        }
        let next = task.clone();
        self.persist(&data);
        drop(data);
        let _ = app.emit("task://changed", next);
        Ok(())
    }
    fn discard_files(task: &Transfer) -> Result<(), AppError> {
        // Only the dedicated launcher cache may be deleted, never an arbitrary folder.
        let path = PathBuf::from(&task.download_dir);
        if task.slot == "game"
            && path.file_name().is_some_and(|n| n == "_download")
            && path.exists()
        {
            std::fs::remove_dir_all(path)?;
        }
        Ok(())
    }
    pub fn finish(
        &self,
        app: &tauri::AppHandle,
        slot: &str,
        id: &str,
        result: Result<Value, &AppError>,
    ) {
        let mut data = self.data.lock().unwrap();
        let Some(task) = data.get_mut(slot).filter(|t| t.id == id) else {
            return;
        };
        match result {
            Ok(value) => {
                task.status = "completed".into();
                task.result = value;
                task.error = None;
            }
            Err(AppError::Cancelled) => {
                task.status = "paused".into();
                task.error = None;
            }
            Err(err) => {
                task.status = "error".into();
                task.error = Some(err.to_string());
            }
        }
        if task.discard && task.status != "completed" {
            match Self::discard_files(task) {
                Ok(()) => {
                    task.status = "idle".into();
                    task.progress = Value::Null;
                    task.error = None;
                }
                Err(e) => {
                    task.status = "error".into();
                    task.error = Some(e.to_string());
                }
            }
        }
        let next = task.clone();
        self.persist(&data);
        drop(data);
        let _ = app.emit("task://changed", next);
    }
}

pub fn register(app: &tauri::AppHandle) {
    for (event, slot, stage) in [
        ("download://progress", "game", Some("downloading")),
        ("download://verify-progress", "game", Some("verifying")),
        ("download://extract-progress", "game", Some("extracting")),
        ("update://progress", "game", None),
        ("integrity://progress", "game", None),
        ("proton://progress", "proton", None),
    ] {
        let handle = app.clone();
        app.listen(event, move |e| {
            if let Ok(progress) = serde_json::from_str(e.payload()) {
                handle
                    .state::<AppState>()
                    .transfers
                    .progress(&handle, slot, progress, stage);
            }
        });
    }
}

#[tauri::command]
pub fn get_transfers(state: tauri::State<'_, AppState>) -> BTreeMap<String, Transfer> {
    state.transfers.snapshot()
}

#[tauri::command]
pub fn stop_transfer(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    slot: String,
    discard: bool,
) -> Result<(), AppError> {
    let flag = match slot.as_str() {
        "game" => &state.download_active,
        "proton" => &state.proton_download_active,
        _ => return Err(AppError::Api("Unknown transfer".into())),
    };
    state.transfers.stop(&app, &slot, discard, flag)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn restart_recovers_active_task_as_paused() {
        let path =
            std::env::temp_dir().join(format!("llauncher-transfer-{}.json", std::process::id()));
        let task = Transfer {
            id: "1".into(),
            slot: "game".into(),
            kind: "update".into(),
            status: "running".into(),
            game_dir: "/game".into(),
            download_dir: "/game/_download".into(),
            progress: json!({"stage":"downloading","bytes_downloaded":10}),
            error: None,
            result: Value::Null,
            discard: false,
        };
        std::fs::write(
            &path,
            serde_json::to_vec(&BTreeMap::from([("game", task)])).unwrap(),
        )
        .unwrap();
        let registry = Transfers::from_path(path.clone());
        let snapshot = registry.snapshot();
        assert_eq!(snapshot["game"].status, "paused");
        assert_eq!(snapshot["game"].kind, "update");
        assert_eq!(snapshot["game"].progress["bytes_downloaded"], 10);
        assert!(!registry.busy("game"));
        let _ = std::fs::remove_file(path);
    }
}
