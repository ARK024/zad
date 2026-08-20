use crate::config::{self, ConfigStore};
use crate::data_loader::DataLoader;
use crate::tray;
use chrono::Local;
use serde_json::{json, Value};
use std::path::PathBuf;
use tauri::{AppHandle, Manager};
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_dialog::{DialogExt, FilePath};

pub async fn do_backup(app: &AppHandle) -> Value {
    let ts = Local::now().format("%Y-%m-%dT%H-%M-%S").to_string();
    let default_name = format!("riyad-backup-{}.json", ts);

    log::info!("Starting backup process");

    let (tx, rx) = tokio::sync::oneshot::channel::<Option<FilePath>>();
    app.dialog()
        .file()
        .add_filter("JSON", &["json"])
        .set_file_name(&default_name)
        .set_title("حفظ نسخة احتياطية")
        .save_file(move |p| {
            let _ = tx.send(p);
        });
    
    let path: Option<FilePath> = match rx.await {
        Ok(p) => p,
        Err(e) => {
            log::error!("Backup dialog channel error: {}", e);
            return json!({"ok": false, "err": "dialog_error"});
        }
    };
    
    let path = match path {
        Some(p) => match path_buf_from(&p) {
            Some(pb) => {
                log::info!("Backup path selected: {:?}", pb);
                pb
            }
            None => {
                log::error!("Failed to convert FilePath to PathBuf");
                return json!({"ok": false, "err": "invalid_path"});
            }
        },
        None => {
            log::warn!("User cancelled backup");
            return json!({"ok": false, "err": "user_cancelled"});
        }
    };

    let store: tauri::State<ConfigStore> = app.state();
    
    let backup = json!({
        "version": 2,
        "date": chrono::Utc::now().to_rfc3339(),
        "cfg": store.cfg_get(),
        "quran": store.quran_get(),
    });
    
    let backup_str = match serde_json::to_string_pretty(&backup) {
        Ok(s) => s,
        Err(e) => {
            log::error!("Failed to serialize backup: {}", e);
            return json!({"ok": false, "err": format!("serialization_error: {}", e)});
        }
    };

    match std::fs::write(&path, backup_str) {
        Ok(_) => {
            log::info!("Backup completed successfully: {:?}", path);
            json!({"ok": true, "path": path.to_string_lossy()})
        }
        Err(e) => {
            log::error!("Failed to write backup file {:?}: {}", path, e);
            json!({"ok": false, "err": e.to_string()})
        }
    }
}

pub async fn do_restore(app: &AppHandle, store: &ConfigStore, data: &DataLoader) -> Value {
    log::info!("Starting restore process");

    let (tx, rx) = tokio::sync::oneshot::channel::<Option<FilePath>>();
    app.dialog()
        .file()
        .add_filter("JSON", &["json"])
        .set_title("استيراد نسخة احتياطية")
        .pick_file(move |p| {
            let _ = tx.send(p);
        });
    
    let path: Option<FilePath> = match rx.await {
        Ok(p) => p,
        Err(e) => {
            log::error!("Restore dialog channel error: {}", e);
            return json!({"ok": false, "err": "dialog_error"});
        }
    };
    
    let path = match path {
        Some(p) => match path_buf_from(&p) {
            Some(pb) => {
                log::info!("Restore path selected: {:?}", pb);
                pb
            }
            None => {
                log::error!("Failed to convert FilePath to PathBuf");
                return json!({"ok": false, "err": "invalid_path"});
            }
        },
        None => {
            log::warn!("User cancelled restore");
            return json!({"ok": false, "err": "user_cancelled"});
        }
    };

    let raw = match std::fs::read_to_string(&path) {
        Ok(s) => {
            log::debug!("Read restore file: {:?} ({} bytes)", path, s.len());
            s
        }
        Err(e) => {
            log::error!("Failed to read restore file {:?}: {}", path, e);
            return json!({"ok": false, "err": e.to_string()});
        }
    };
    
    let parsed: Value = match serde_json::from_str(&raw) {
        Ok(v) => {
            log::debug!("Parsed restore JSON successfully");
            v
        }
        Err(e) => {
            log::error!("Failed to parse restore JSON: {}", e);
            return json!({"ok": false, "err": format!("invalid_json: {}", e)});
        }
    };

    let payload = parsed.get("cfg").cloned().unwrap_or_else(|| parsed.clone());
    
    // Validate required fields
    if !payload.get("index").is_some_and(|v| v.is_number()) {
        log::error!("Restore file missing required field: index");
        return json!({"ok": false, "err": "invalid_file_format"});
    }
    
    log::info!("Restoring config from backup");

    let mut merged = config::defaults();
    if let (Some(merged_obj), Some(input_obj)) = (merged.as_object_mut(), payload.as_object()) {
        for (k, v) in input_obj {
            merged_obj.insert(k.clone(), v.clone());
        }
    }
    store.cfg_update(&merged);
    store.save_cfg(app);

    let auto_launch = store
        .cfg_value("autoLaunch")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let manager = app.autolaunch();
    let _ = if auto_launch {
        log::info!("Enabling autostart");
        manager.enable()
    } else {
        log::info!("Disabling autostart");
        manager.disable()
    };

    tray::refresh(app, store, data);

    // Restore Quran data if present in backup
    if let Some(quran_data) = parsed.get("quran") {
        if let Some(obj) = quran_data.as_object() {
            for (k, v) in obj {
                store.quran_set(k, v.clone());
            }
            store.save_quran_cfg(app);
            log::info!("Quran data restored successfully");
        }
    }

    log::info!("Restore completed successfully");
    json!({"ok": true})
}

fn path_buf_from(fp: &FilePath) -> Option<PathBuf> {
    fp.clone().into_path().ok()
}
