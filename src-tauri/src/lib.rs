use parking_lot::Mutex;
use serde_json::Value;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, Manager};
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};

mod backup;
mod config;
mod data_loader;
mod ipc;
mod orchestrator;
mod tray;
mod windows;

use crate::config::ConfigStore;
use crate::data_loader::DataLoader;
use crate::orchestrator::{Orchestrator, TickAction};
use crate::tray::TrayState;
use crate::windows::WidgetGuard;

/// Shared, cheap-to-clone bundle of app-wide state used by IPC handlers and the
/// orchestrator. Stored as a Tauri-managed state so commands can pull it via `State`.
#[derive(Clone)]
pub struct AppContext {
    pub widget_guard: WidgetGuard,
    pub orchestrator: Orchestrator,
    pub data_dir: Arc<Mutex<PathBuf>>,
    /// Latest payload prepared for the widget renderer; consumed by `widget_ready`.
    pub pending_widget_payload: Arc<Mutex<Option<Value>>>,
}

impl AppContext {
    pub fn new(data_dir: PathBuf) -> Self {
        Self {
            widget_guard: WidgetGuard::new(),
            orchestrator: Orchestrator::new(),
            data_dir: Arc::new(Mutex::new(data_dir)),
            pending_widget_payload: Arc::new(Mutex::new(None)),
        }
    }

    /// (Re)start the orchestrator with the current cfg/quran snapshots.
    pub fn restart_orchestrator(&self, app: &AppHandle) {
        let store: tauri::State<ConfigStore> = app.state();
        let app_for_tick = app.clone();
        let me = self.clone();
        self.orchestrator.start(store.inner().clone(), move |action| {
            on_tick(&app_for_tick, &me, action);
        });
    }
}

/// Translate a `TickAction` into actual window operations.
fn on_tick(app: &AppHandle, ctx: &AppContext, action: TickAction) {
    let store: tauri::State<ConfigStore> = app.state();
    let data: tauri::State<DataLoader> = app.state();
    match action {
        TickAction::ShowHadith => {
            log::debug!("Tick: ShowHadith");
            windows::show_widget(app, &store, &data, &ctx.widget_guard, None, false);
        }
        TickAction::ShowQuran => {
            log::debug!("Tick: ShowQuran");
            windows::show_quran_window(app);
        }
        TickAction::ShowBoth => {
            log::debug!("Tick: ShowBoth");
            windows::show_widget(app, &store, &data, &ctx.widget_guard, None, false);
            windows::show_quran_window(app);
        }
        TickAction::HideQuranShowHadith => {
            log::debug!("Tick: HideQuranShowHadith");
            windows::show_widget(app, &store, &data, &ctx.widget_guard, None, false);
            windows::hide_quran_window(app);
        }
    }
}

fn show_missing_data_dialog(app: &AppHandle) {
    let _ = app
        .dialog()
        .message(
            "تأكد من وجود ملف Riyadh_AlSaliheen_V2.json في مجلد data بجوار التطبيق."
        )
        .title("ملف الأحاديث غير موجود أو به خطأ")
        .kind(MessageDialogKind::Error)
        .buttons(MessageDialogButtons::Ok)
        .blocking_show();
}

/// Initialize a rotating file logger that writes to the first writable
/// location among: exe-dir, %LOCALAPPDATA%\zad-al-muslim, %TEMP%, cwd.
/// Log files are rotated when they exceed 1MB, keeping up to 5 old files.
fn init_file_logger() {
    use std::io::Write;
    fn pick_log_path() -> std::path::PathBuf {
        let mut candidates: Vec<std::path::PathBuf> = Vec::new();
        if let Ok(exe) = std::env::current_exe() {
            if let Some(d) = exe.parent() {
                candidates.push(d.join("zad-al-muslim.log"));
            }
        }
        if let Ok(local) = std::env::var("LOCALAPPDATA") {
            candidates.push(
                std::path::PathBuf::from(local)
                    .join("zad-al-muslim")
                    .join("zad-al-muslim.log"),
            );
        }
        candidates.push(std::env::temp_dir().join("zad-al-muslim.log"));
        candidates.push(std::path::PathBuf::from("zad-al-muslim.log"));

        for c in &candidates {
            if let Some(parent) = c.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            if let Ok(mut f) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(c)
            {
                let _ = writeln!(
                    f,
                    "[{}] log file: {}",
                    chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                    c.display()
                );
                log::info!("Logger initialized to: {:?}", c);
                return c.clone();
            }
        }
        candidates.pop().unwrap()
    }
    let log_path = pick_log_path();

    struct FileLogger {
        path: std::path::PathBuf,
        max_size: u64,
        max_files: u32,
    }
    
    impl FileLogger {
        fn should_rotate(&self) -> bool {
            if let Ok(metadata) = std::fs::metadata(&self.path) {
                metadata.len() >= self.max_size
            } else {
                false
            }
        }
        
        fn rotate(&self) {
            // Remove oldest log file
            let oldest = format!("{}.{}", self.path.display(), self.max_files - 1);
            let _ = std::fs::remove_file(&oldest);
            
            // Shift existing rotated files
            for i in (1..self.max_files).rev() {
                let old_path = if i == 1 {
                    self.path.clone()
                } else {
                    std::path::PathBuf::from(format!("{}.{}", self.path.display(), i - 1))
                };
                let new_path = std::path::PathBuf::from(format!("{}.{}", self.path.display(), i));
                if old_path.exists() {
                    let _ = std::fs::rename(&old_path, &new_path);
                }
            }
            
            // Create new log file
            if let Ok(mut f) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&self.path)
            {
                let _ = writeln!(
                    f,
                    "[{}] log file rotated",
                    chrono::Local::now().format("%Y-%m-%d %H:%M:%S")
                );
            }
        }
    }
    
    impl log::Log for FileLogger {
        fn enabled(&self, _: &log::Metadata) -> bool {
            true
        }
        fn log(&self, record: &log::Record) {
            // Check if rotation is needed
            if self.should_rotate() {
                self.rotate();
            }
            
            if let Ok(mut f) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&self.path)
            {
                let _ = writeln!(
                    f,
                    "[{}] {} {} - {}",
                    chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                    record.level(),
                    record.target(),
                    record.args()
                );
            }
        }
        fn flush(&self) {}
    }
    
    let logger = FileLogger {
        path: log_path,
        max_size: 1_048_576, // 1MB
        max_files: 5,
    };
    
    let _ = log::set_boxed_logger(Box::new(logger))
        .map(|()| log::set_max_level(log::LevelFilter::Info));
    
    log::info!("Rotating file logger initialized (max 1MB, 5 files)");
}

/// Resolve where to look for `Riyadh_AlSaliheen_V2.json` and `quran.json`.
/// Search order:
///   1. `<exe-dir>/data/`
///   2. `<exe-dir>/../data/` (when running from the build output / dev mode)
///   3. `<resource-dir>/data/`
///   4. `<current-dir>/data/`
fn locate_data_dir(app: &AppHandle) -> PathBuf {
    let candidate_dirs = {
        let mut v: Vec<PathBuf> = Vec::new();
        if let Ok(exe) = std::env::current_exe() {
            if let Some(parent) = exe.parent() {
                v.push(parent.join("data"));
                if let Some(grand) = parent.parent() {
                    v.push(grand.join("data"));
                }
            }
        }
        if let Ok(resource_dir) = app.path().resource_dir() {
            v.push(resource_dir.join("data"));
        }
        if let Ok(cwd) = std::env::current_dir() {
            v.push(cwd.join("data"));
        }
        v
    };
    for dir in &candidate_dirs {
        if dir.join("Riyadh_AlSaliheen_V2.json").exists() {
            return dir.clone();
        }
    }
    candidate_dirs
        .into_iter()
        .next()
        .unwrap_or_else(|| PathBuf::from("data"))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let _ = env_logger::try_init();
    init_file_logger();
    log::info!("=== زاد المسلم v1.1.0 starting ===");
    log::info!("exe: {:?}", std::env::current_exe());
    log::info!("cwd: {:?}", std::env::current_dir());

    std::panic::set_hook(Box::new(|info| {
        log::error!("PANIC: {}", info);
        log::error!("Application will crash. Check log file for details.");
    }));

    // Setup cleanup on application exit
    let cleanup_handle = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let cleanup = cleanup_handle.clone();
    
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            // Show settings on second launch (mirrors `app.on('second-instance')` in JS).
            log::info!("Second instance detected, opening settings");
            windows::open_settings(app);
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .invoke_handler(tauri::generate_handler![
            ipc::w_hide,
            ipc::w_memorized,
            ipc::w_forgot,
            ipc::w_next,
            ipc::w_prev,
            ipc::widget_ready,
            ipc::q_window_show,
            ipc::q_window_hide,
            ipc::q_store_get,
            ipc::q_store_set,
            ipc::q_store_remove,
            ipc::q_store_clear,
            ipc::q_set_pages_per_session,
            ipc::q_bg_message,
            ipc::s_get,
            ipc::s_save,
            ipc::s_reset,
            ipc::s_jump,
            ipc::s_search,
            ipc::s_show_now,
            ipc::s_backup,
            ipc::s_restore,
            ipc::s_reset_quran_geometry,
            ipc::s_reset_geometry,
            ipc::m_recalculate_sequence,
            ipc::m_get_fonts,
            ipc::welcome_done,
        ])
        .setup(|app| {
            let app_handle = app.handle();
            let store = ConfigStore::new();
            let data = DataLoader::new();
            let tray_state = TrayState::new();

            let data_dir = locate_data_dir(app_handle);
            log::info!("data_dir resolved: {:?}", data_dir);
            let ctx = AppContext::new(data_dir.clone());

            store.load_cfg(app_handle);
            log::info!("AppConfig loaded");
            store.load_quran_cfg(app_handle);
            log::info!("QuranConfig loaded");

            let has_data = data.load_hadith_data(&data_dir);
            log::info!("hadith data loaded: has_data={}", has_data);
            data.load_quran_data(&data_dir);
            
            if !has_data {
                log::error!("CRITICAL: hadith data missing — exiting");
                show_missing_data_dialog(app_handle);
                app_handle.exit(0);
                return Ok(());
            }
            data.build_search_index();
            log::info!("search index built successfully");

            app.manage(store.clone());
            app.manage(data.clone());
            app.manage(ctx.clone());
            app.manage(tray_state.clone());
            log::info!("AppContext and state managed successfully");

            // Wire up the tray. Each callback re-fetches state from the AppHandle.
            match tray::create(
                app_handle,
                store.clone(),
                data.clone(),
                tray_state.clone(),
                |app| windows::show_quran_window(&app),
                |app| windows::hide_quran_window(&app),
                |app| {
                    let store: tauri::State<ConfigStore> = app.state();
                    let data: tauri::State<DataLoader> = app.state();
                    let ctx: tauri::State<AppContext> = app.state();
                    windows::show_widget(&app, &store, &data, &ctx.widget_guard, None, false);
                    ctx.restart_orchestrator(&app);
                },
                |app| {
                    advance(&app, 1);
                },
                |app| {
                    advance(&app, -1);
                },
                |app| windows::open_settings(&app),
                |app| {
                    let store: tauri::State<ConfigStore> = app.state();
                    let data: tauri::State<DataLoader> = app.state();
                    let q_cfg = store.quran_get();
                    let cfg = store.cfg_get();
                    let ctx: tauri::State<AppContext> = app.state();
                    let action = ctx.orchestrator.tick_once(&cfg, &q_cfg);
                    on_tick(&app, &ctx, action);
                    ctx.restart_orchestrator(&app);
                    let _ = data;
                },
            ) {
                Ok(_) => log::info!("Tray icon created and wired successfully"),
                Err(e) => {
                    log::error!("Failed to create tray icon: {}", e);
                    log::warn!("Application will continue without tray");
                }
            };

            // Apply autoLaunch preference at startup.
            let auto_launch = store
                .cfg_value("autoLaunch")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let manager = app_handle.autolaunch();
            match if auto_launch {
                log::info!("Enabling autostart");
                manager.enable()
            } else {
                log::info!("Disabling autostart");
                manager.disable()
            } {
                Ok(_) => log::debug!("Autostart configured successfully"),
                Err(e) => log::warn!("Failed to configure autostart: {}", e),
            };

            // First-run flow.
            let first_run = store
                .cfg_value("firstRun")
                .and_then(|v| v.as_bool())
                .unwrap_or(true);
            log::info!("first_run = {}", first_run);
            if first_run {
                windows::open_welcome(app_handle);
                log::info!("Welcome window opened for first-time setup");
            } else {
                windows::create_quran_window(app_handle, &store);
                ctx.restart_orchestrator(app_handle);
                log::info!("Orchestrator started (not first run)");
            }

            log::info!("=== Zad Al-Muslim setup completed successfully ===");

            Ok(())
        })
        .on_page_load(|webview, payload| {
            log::debug!("Page load event: {}", payload.url());
            if webview.label() == windows::WELCOME_LABEL {
                log::info!("Welcome page loaded");
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");

    log::info!("Application shutdown");
}

/// Move the current hadith index by `delta`, save, refresh tray, show widget,
/// restart the orchestrator. Mirrors the `advance(delta)` helper in main.js.
fn advance(app: &AppHandle, delta: i64) {
    let store: tauri::State<ConfigStore> = app.state();
    let data: tauri::State<DataLoader> = app.state();
    let ctx: tauri::State<AppContext> = app.state();
    
    let cur = store
        .cfg_value("index")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);
    let total = data.hadiths_len() as i64;
    let new_idx = (cur + delta).clamp(0, total.saturating_sub(1).max(0));
    
    log::info!("Advancing hadith: {} -> {} (delta: {})", cur, new_idx, delta);
    
    store.cfg_set("index", serde_json::json!(new_idx));
    store.save_cfg(app);
    tray::refresh(app, &store, &data);
    windows::show_widget(app, &store, &data, &ctx.widget_guard, None, false);
    ctx.restart_orchestrator(app);
}

pub fn run_app() {
    run()
}
