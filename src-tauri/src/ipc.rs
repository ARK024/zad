use crate::config::ConfigStore;
use crate::data_loader::{DataLoader, PageAyahs, SearchHit};
use crate::tray;
use crate::windows;
use crate::AppContext;
use serde_json::{json, Value};
use base64::prelude::*;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_dialog::{DialogExt, FilePath};

// Whitelist for allowed Quran config keys to prevent prototype pollution
const ALLOWED_QURAN_KEYS: &[&str] = &[
    "audioAutoPlay",
    "audioBasePath",
    "audioReciter",
    "audioRepeatCount",
    "completedPages",
    "currentQuranPage",
    "dailyGoal",
    "dailyStreak",
    "dayStartHour",
    "fontSizePx",
    "hideHeader",
    "hide_header",
    "lastCompletedDate",
    "lastCompletedTime",
    "lastRecentReviewDate",
    "lastReviewDate",
    "memorizationInterval",
    "memorizedPages",
    "pausedUntil",
    "preloadedPages",
    "progressiveChunk",
    "progressiveLevel",
    "progressiveModeEnabled",
    "recentPagesPerSession",
    "recentReadings",
    "recentRetryPages",
    "recentReviewEnabled",
    "recentReviewIndex",
    "reviewCycleStartDate",
    "reviewDays",
    "reviewEnabled",
    "reviewIndex",
    "reviewPagesPerSession",
    "reviewRetryPages",
    "reviewSessionStart",
    "testModeEnabled",
    "totalReadCount",
    "weakPages",
    "widgetCustomHeight",
    "widgetCustomWidth",
    "widgetShownAt",
    "widgetSize",
    "widgetX",
    "widgetY",
];

fn is_allowed_quran_key(key: &str) -> bool {
    ALLOWED_QURAN_KEYS.binary_search(&key).is_ok()
}

fn apply_auto_launch(app: &AppHandle, enable: bool) {
    let manager = app.autolaunch();
    let _ = if enable {
        manager.enable()
    } else {
        manager.disable()
    };
}

// ── Widget Events ──────────────────────────────────────────────────────────

#[tauri::command]
pub fn w_hide(app: AppHandle, ctx: State<'_, AppContext>) {
    windows::destroy_widget(&app);
    ctx.restart_orchestrator(&app);
}

#[tauri::command]
pub fn w_memorized(
    app: AppHandle,
    store: State<'_, ConfigStore>,
    data: State<'_, DataLoader>,
    ctx: State<'_, AppContext>,
    index: usize,
) {
    let cfg = store.cfg_get();
    if cfg
        .get("hReviewEnabled")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
    {
        let days = cfg
            .get("hReviewDays")
            .and_then(|v| v.as_i64())
            .unwrap_or(7);
        let next_ms = chrono::Utc::now().timestamp_millis() + days * 86_400_000;
        let mut reviews = cfg
            .get("hReviews")
            .cloned()
            .unwrap_or_else(|| json!({}));
        if let Some(obj) = reviews.as_object_mut() {
            obj.insert(index.to_string(), json!(next_ms));
        }
        store.cfg_set("hReviews", reviews);
    }

    let cfg_index = store.cfg_value("index").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
    if cfg_index == index {
        let total = data.hadiths_len().max(1);
        let next = if index + 1 >= total { 0 } else { index + 1 };
        store.cfg_set("index", json!(next));
    }

    store.save_cfg(&app);
    tray::refresh(&app, &store, &data);
    windows::destroy_widget(&app);
    ctx.restart_orchestrator(&app);
}

#[tauri::command]
pub fn w_forgot(
    app: AppHandle,
    store: State<'_, ConfigStore>,
    data: State<'_, DataLoader>,
    ctx: State<'_, AppContext>,
    index: usize,
) {
    let cfg = store.cfg_get();
    if cfg.get("hReviews").is_some() {
        let mut reviews = cfg
            .get("hReviews")
            .cloned()
            .unwrap_or_else(|| json!({}));
        if let Some(obj) = reviews.as_object_mut() {
            obj.insert(
                index.to_string(),
                json!(chrono::Utc::now().timestamp_millis() + 86_400_000),
            );
        }
        store.cfg_set("hReviews", reviews);
    }
    store.save_cfg(&app);
    tray::refresh(&app, &store, &data);
    windows::destroy_widget(&app);
    ctx.restart_orchestrator(&app);
}

#[tauri::command]
pub fn w_next(
    app: AppHandle,
    store: State<'_, ConfigStore>,
    data: State<'_, DataLoader>,
    ctx: State<'_, AppContext>,
    index: usize,
) {
    let total = data.hadiths_len().max(1);
    let new_idx = (index + 1).min(total - 1);
    store.cfg_set("index", json!(new_idx));
    store.save_cfg(&app);
    tray::refresh(&app, &store, &data);
    windows::refresh_widget_payload(&app, &store, &data, &ctx.widget_guard);
}

#[tauri::command]
pub fn w_prev(
    app: AppHandle,
    store: State<'_, ConfigStore>,
    data: State<'_, DataLoader>,
    ctx: State<'_, AppContext>,
    index: usize,
) {
    let new_idx = index.saturating_sub(1);
    store.cfg_set("index", json!(new_idx));
    store.save_cfg(&app);
    tray::refresh(&app, &store, &data);
    windows::refresh_widget_payload(&app, &store, &data, &ctx.widget_guard);
}

/// Used by widget.html to signal the renderer is ready. We push the cached
/// hadith payload to it and finally show the window.
#[tauri::command]
pub fn widget_ready(
    app: AppHandle,
    store: State<'_, ConfigStore>,
    data: State<'_, DataLoader>,
    ctx: State<'_, AppContext>,
) {
    let payload = ctx
        .pending_widget_payload
        .lock()
        .take()
        .or_else(|| {
            let cfg = store.cfg_get();
            let idx = cfg.get("index").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
            data.build_widget_payload(idx, false, &cfg)
        });
    if let (Some(w), Some(payload)) = (app.get_webview_window(windows::WIDGET_LABEL), payload) {
        let _ = w.emit("hadith", payload);
        let _ = w.show();
    }
}

// ── Quran Window ───────────────────────────────────────────────────────────

#[tauri::command]
pub fn q_window_show(app: AppHandle) {
    windows::reveal_quran_window(&app);
}

#[tauri::command]
pub fn q_window_hide(app: AppHandle) {
    windows::hide_quran_window(&app);
}

#[tauri::command]
pub fn q_show_now(app: AppHandle) {
    windows::show_quran_window(&app);
}

// ── Quran Storage ──────────────────────────────────────────────────────────

#[tauri::command]
pub fn q_store_get(store: State<'_, ConfigStore>, keys: Value) -> Value {
    let mut q = store.quran_get();
    if let Some(obj) = q.as_object_mut() {
        if let Some(v) = obj.get("hideHeader").cloned().or_else(|| obj.get("hide_header").cloned()) {
            obj.insert("hideHeader".to_string(), v.clone());
            obj.insert("hide_header".to_string(), v);
        }
    }
    if keys.is_null() {
        return q;
    }
    let resolve_q_val = |k: &str| -> Option<Value> {
        q.get(k).cloned().or_else(|| {
            if k == "hideHeader" {
                q.get("hide_header").cloned()
            } else if k == "hide_header" {
                q.get("hideHeader").cloned()
            } else {
                None
            }
        })
    };

    if let Some(arr) = keys.as_array() {
        let mut out = serde_json::Map::new();
        for k in arr {
            if let Some(s) = k.as_str() {
                out.insert(s.to_string(), resolve_q_val(s).unwrap_or(Value::Null));
            }
        }
        return Value::Object(out);
    }
    if let Some(obj) = keys.as_object() {
        let mut out = serde_json::Map::new();
        for (k, default_v) in obj {
            let v = resolve_q_val(k);
            // Treat stored `null` the same as missing — fall back to caller's default.
            let resolved = match v {
                Some(Value::Null) | None => default_v.clone(),
                Some(val) => val,
            };
            out.insert(k.clone(), resolved);
        }
        return Value::Object(out);
    }
    if let Some(s) = keys.as_str() {
        let mut out = serde_json::Map::new();
        out.insert(s.to_string(), resolve_q_val(s).unwrap_or(Value::Null));
        return Value::Object(out);
    }
    q
}

#[tauri::command]
pub fn q_store_set(
    app: AppHandle,
    store: State<'_, ConfigStore>,
    ctx: State<'_, AppContext>,
    data: Value,
) {
    let mut changed = serde_json::Map::new();
    let mut had_progress = false;
    let mut pause_changed = false;
    let snapshot = store.quran_get();
    
    if let Some(map) = data.as_object() {
        for (k, v) in map {
            // Security: validate key against whitelist
            if !is_allowed_quran_key(k) {
                log::warn!("Attempted to set disallowed quran config key: {}", k);
                continue;
            }
            
            let old = snapshot.get(k);
            if old != Some(v) {
                store.quran_set(k, v.clone());
                changed.insert(k.clone(), json!({"newValue": v}));
                log::debug!("Quran config changed: {} = {:?}", k, v);
                
                if k == "currentQuranPage"
                    || k == "reviewIndex"
                    || k == "recentReviewIndex"
                {
                    had_progress = true;
                }
                if k == "pausedUntil" {
                    pause_changed = true;
                }
            }
        }
    }
    if !changed.is_empty() {
        store.save_quran_cfg(&app);
        if had_progress || pause_changed {
            ctx.restart_orchestrator(&app);
        }
        if pause_changed {
            // Hide widgets immediately when entering a pause; users explicitly
            // chose to silence reminders.
            let q = store.quran_get();
            if crate::orchestrator::is_paused(&q) {
                windows::hide_quran_window(&app);
                windows::destroy_widget(&app);
            }
            if let Some(data) = app.try_state::<crate::data_loader::DataLoader>() {
                crate::tray::refresh(&app, &store, &data);
            }
        }
        broadcast_quran_changed(&app, &Value::Object(changed));
    }
}

#[tauri::command]
pub fn q_store_remove(app: AppHandle, store: State<'_, ConfigStore>, keys: Value) {
    let mut to_remove: Vec<String> = Vec::new();
    if let Some(arr) = keys.as_array() {
        for k in arr {
            if let Some(s) = k.as_str() {
                if is_allowed_quran_key(s) {
                    to_remove.push(s.to_string());
                } else {
                    log::warn!("Attempted to remove disallowed quran config key: {}", s);
                }
            }
        }
    } else if let Some(s) = keys.as_str() {
        if is_allowed_quran_key(s) {
            to_remove.push(s.to_string());
        } else {
            log::warn!("Attempted to remove disallowed quran config key: {}", s);
        }
    }
    let mut changed = serde_json::Map::new();
    for k in &to_remove {
        if store.quran_remove(k) {
            changed.insert(k.clone(), json!({"newValue": Value::Null}));
            log::debug!("Quran config removed: {}", k);
        }
    }
    if !changed.is_empty() {
        store.save_quran_cfg(&app);
        broadcast_quran_changed(&app, &Value::Object(changed));
    }
}

#[tauri::command]
pub fn q_store_clear(app: AppHandle, store: State<'_, ConfigStore>) {
    let removed = store.quran_clear();
    let mut changed = serde_json::Map::new();
    for k in removed {
        changed.insert(k, json!({"newValue": Value::Null}));
    }
    store.save_quran_cfg(&app);
    broadcast_quran_changed(&app, &Value::Object(changed));
}

#[tauri::command]
pub fn q_set_pages_per_session(
    app: AppHandle,
    store: State<'_, ConfigStore>,
    ctx: State<'_, AppContext>,
    pages: i64,
) {
    let safe_pages = pages.clamp(1, 50);
    store.quran_set("reviewPagesPerSession", json!(safe_pages));
    store.save_quran_cfg(&app);
    ctx.restart_orchestrator(&app);
    broadcast_quran_changed(&app, &json!({"reviewPagesPerSession": json!(safe_pages)}));
}

fn broadcast_quran_changed(app: &AppHandle, changed: &Value) {
    let mut payload = changed.clone();
    if let Some(obj) = payload.as_object_mut() {
        if let Some(v) = obj.get("hideHeader").cloned().or_else(|| obj.get("hide_header").cloned()) {
            obj.insert("hideHeader".to_string(), v.clone());
            obj.insert("hide_header".to_string(), v);
        }
    }
    let _ = app.emit("q_store_changed", &payload);
    let _ = app.emit("q:store:changed", &payload);
    if let Some(w) = app.get_webview_window(windows::QURAN_LABEL) {
        let _ = w.emit("q_store_changed", &payload);
        let _ = w.emit("q:store:changed", &payload);
    }
    if let Some(w) = app.get_webview_window(windows::SETTINGS_LABEL) {
        let _ = w.emit("q_store_changed", &payload);
        let _ = w.emit("q:store:changed", &payload);
    }
}

// ── Quran Background Messages ──────────────────────────────────────────────

#[derive(Debug, serde::Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum QBgMessage {
    GetPageAyahs { page: i64 },
    GetMultiplePages { pages: Vec<i64> },
}

#[derive(Debug, serde::Serialize)]
#[serde(untagged)]
pub enum QBgResponse {
    Page(Option<PageAyahs>),
    MultiplePages(Vec<HashMap<String, Value>>),
}

#[tauri::command]
pub fn q_bg_message(data: State<'_, DataLoader>, req: QBgMessage) -> QBgResponse {
    match req {
        QBgMessage::GetPageAyahs { page } => QBgResponse::Page(data.get_page_ayahs(page)),
        QBgMessage::GetMultiplePages { pages } => {
            let mut out = Vec::new();
            for p in pages {
                if let Some(pd) = data.get_page_ayahs(p) {
                    let mut m = HashMap::new();
                    m.insert("pageNum".to_string(), json!(p));
                    m.insert(
                        "pageData".to_string(),
                        serde_json::to_value(&pd).unwrap_or(Value::Null),
                    );
                    out.push(m);
                }
            }
            QBgResponse::MultiplePages(out)
        }
    }
}

// ── Settings ───────────────────────────────────────────────────────────────

#[tauri::command]
pub fn s_get(store: State<'_, ConfigStore>, data: State<'_, DataLoader>) -> Value {
    let mut cfg = store.cfg_get();
    if let Some(obj) = cfg.as_object_mut() {
        obj.insert("total".to_string(), json!(data.hadiths_len()));
    }
    cfg
}

#[tauri::command]
pub fn s_save(
    app: AppHandle,
    store: State<'_, ConfigStore>,
    data: State<'_, DataLoader>,
    ctx: State<'_, AppContext>,
    payload: Value,
) -> Value {
    let cur_cfg = store.cfg_get();
    let parse_int = |k: &str, fallback: i64| -> i64 {
        payload
            .get(k)
            .and_then(|v| v.as_i64().or_else(|| v.as_str().and_then(|s| s.parse().ok())))
            .unwrap_or(fallback)
    };
    let parse_str = |k: &str, fallback: &str| -> String {
        payload
            .get(k)
            .and_then(|v| v.as_str())
            .unwrap_or(fallback)
            .to_string()
    };
    let parse_bool = |k: &str, fallback: bool| -> bool {
        payload
            .get(k)
            .and_then(|v| v.as_bool())
            .unwrap_or(fallback)
    };

    let interval = parse_int("interval", 30).max(1);
    let font_size = parse_int("fontSize", 22).clamp(12, 72);
    let font_family = parse_str(
        "fontFamily",
        cur_cfg
            .get("fontFamily")
            .and_then(|v| v.as_str())
            .unwrap_or("'QuranFont', 'Traditional Arabic'"),
    );
    let theme = if parse_str("theme", "light") == "dark" {
        "dark"
    } else {
        "light"
    };
    let mode = parse_str("appMode", "sequential");
    let mode = match mode.as_str() {
        "sequential" | "both" | "quranOnly" | "hadithOnly" | "alternating" => mode,
        _ => "sequential".to_string(),
    };

    store.cfg_set("interval", json!(interval));
    store.cfg_set("fontSize", json!(font_size));
    store.cfg_set("fontFamily", json!(font_family));
    store.cfg_set(
        "cSanad",
        json!(parse_str(
            "cSanad",
            cur_cfg.get("cSanad").and_then(|v| v.as_str()).unwrap_or("#5d7a69"),
        )),
    );
    store.cfg_set(
        "cMatn",
        json!(parse_str(
            "cMatn",
            cur_cfg.get("cMatn").and_then(|v| v.as_str()).unwrap_or("#182820"),
        )),
    );
    store.cfg_set(
        "cTakhrij",
        json!(parse_str(
            "cTakhrij",
            cur_cfg.get("cTakhrij").and_then(|v| v.as_str()).unwrap_or("#1a9850"),
        )),
    );
    store.cfg_set(
        "cSharh",
        json!(parse_str(
            "cSharh",
            cur_cfg.get("cSharh").and_then(|v| v.as_str()).unwrap_or("#b35900"),
        )),
    );
    store.cfg_set("theme", json!(theme));
    let auto_launch = parse_bool("autoLaunch", false);
    store.cfg_set("autoLaunch", json!(auto_launch));
    store.cfg_set("appMode", json!(mode));
    store.cfg_set(
        "hReviewEnabled",
        json!(parse_bool("hReviewEnabled", false)),
    );
    store.cfg_set("hReviewDays", json!(parse_int("hReviewDays", 7).max(1)));

    store.save_cfg(&app);
    apply_auto_launch(&app, auto_launch);
    ctx.restart_orchestrator(&app);
    tray::refresh(&app, &store, &data);
    json!({"ok": true})
}

#[tauri::command]
pub fn s_reset(
    app: AppHandle,
    store: State<'_, ConfigStore>,
    data: State<'_, DataLoader>,
) -> Value {
    store.cfg_set("index", json!(0));
    store.save_cfg(&app);
    tray::refresh(&app, &store, &data);
    json!({"ok": true})
}

#[tauri::command]
pub fn s_jump(
    app: AppHandle,
    store: State<'_, ConfigStore>,
    data: State<'_, DataLoader>,
    index: i64,
) -> Value {
    let total = data.hadiths_len() as i64;
    if index < 0 || index >= total {
        return json!({"ok": false});
    }
    store.cfg_set("index", json!(index));
    store.save_cfg(&app);
    tray::refresh(&app, &store, &data);
    json!({"ok": true})
}

#[tauri::command]
pub fn s_search(data: State<'_, DataLoader>, query: String) -> Vec<SearchHit> {
    data.search_hadiths(&query)
}

#[tauri::command]
pub fn s_show_now(
    app: AppHandle,
    store: State<'_, ConfigStore>,
    data: State<'_, DataLoader>,
    ctx: State<'_, AppContext>,
) {
    windows::show_widget(&app, &store, &data, &ctx.widget_guard, None, false);
    ctx.restart_orchestrator(&app);
}

#[tauri::command]
pub async fn s_backup(app: AppHandle) -> Result<Value, String> {
    Ok(crate::backup::do_backup(&app).await)
}

#[tauri::command]
pub async fn s_restore(
    app: AppHandle,
    store: State<'_, ConfigStore>,
    data: State<'_, DataLoader>,
    ctx: State<'_, crate::AppContext>,
) -> Result<Value, String> {
    let result = crate::backup::do_restore(&app, &store, &data).await;
    // Restart orchestrator so restored timing/mode settings take effect immediately
    if result.get("ok").and_then(|v| v.as_bool()).unwrap_or(false) {
        ctx.restart_orchestrator(&app);
    }
    Ok(result)
}

#[tauri::command]
pub fn s_reset_quran_geometry(
    app: AppHandle,
    store: State<'_, ConfigStore>,
) -> Value {
    windows::reset_quran_geometry(&app, &store);
    json!({"ok": true})
}

#[tauri::command]
pub fn s_reset_geometry(app: AppHandle, store: State<'_, ConfigStore>) -> Value {
    windows::reset_widget_geometry(&app, &store);
    json!({"ok": true})
}

// ── Main ───────────────────────────────────────────────────────────────────

#[tauri::command]
pub fn m_recalculate_sequence(app: AppHandle, ctx: State<'_, AppContext>) -> Value {
    ctx.restart_orchestrator(&app);
    json!({"ok": true})
}

/// Returns a list of system-typical Arabic-friendly fonts. Mirrors the fallback
/// list in src/main/ipc-handlers.js when `font-list` enumeration fails.
#[tauri::command]
pub fn m_get_fonts() -> Vec<String> {
    vec![
        "Segoe UI".to_string(),
        "Tahoma".to_string(),
        "Arial".to_string(),
        "Traditional Arabic".to_string(),
        "Simplified Arabic".to_string(),
        "Sakkal Majalla".to_string(),
        "Microsoft Sans Serif".to_string(),
        "Times New Roman".to_string(),
        "Courier New".to_string(),
        "Droid Arabic Naskh".to_string(),
        "Cairo".to_string(),
        "Amiri".to_string(),
        "QuranFont".to_string(),
    ]
}

// ── Welcome ────────────────────────────────────────────────────────────────

#[tauri::command]
pub fn welcome_done(
    app: AppHandle,
    store: State<'_, ConfigStore>,
    ctx: State<'_, AppContext>,
    auto_launch: bool,
) {
    log::info!("welcome_done called: auto_launch={}", auto_launch);
    store.cfg_set("firstRun", json!(false));
    store.cfg_set("autoLaunch", json!(auto_launch));
    store.save_cfg(&app);
    log::info!("welcome_done: config saved");
    apply_auto_launch(&app, auto_launch);
    log::info!("welcome_done: auto_launch applied");
    windows::close_welcome(&app);
    log::info!("welcome_done: welcome closed");
    windows::create_quran_window(&app, &store);
    log::info!("welcome_done: quran window created");
    // إظهار نافذة القرآن للمستخدم فوراً في أول تشغيل كما توضح شاشة الترحيب
    windows::show_quran_window(&app);
    log::info!("welcome_done: quran window shown");
    ctx.restart_orchestrator(&app);
    log::info!("welcome_done: orchestrator restarted");
    windows::open_settings(&app);
    log::info!("welcome_done: settings opened");
}


// ── Quran Audio ────────────────────────────────────────────────────────────

fn path_buf_from_fp(fp: &FilePath) -> Option<PathBuf> {
    match fp {
        FilePath::Path(pb) => Some(pb.clone()),
        FilePath::Url(u) => u.to_file_path().ok(),
    }
}

fn map_reciter_to_online(reciter: &str) -> &str {
    match reciter {
        "Hussary.teacher_64kbps" | "Hussary.teacher_32kbps" => "Husary_Muallim_128kbps",
        "AbdulSamad_64kbps" => "AbdulSamad_64kbps_QuranExplorer.Com",
        "Ahmed_ibn_Ali_al-Ajamy_64kbps" => "Ahmed_ibn_Ali_al-Ajamy_64kbps_QuranExplorer.Com",
        "Minshawy_Mujawwad_64kbps" => "Minshawy_Mujawwad_192kbps",
        "Minshawy_Teacher_128kbps" => "Minshawy_Teacher_128kbps",
        "Minshawy_Murattal_128kbps" => "Minshawy_Murattal_128kbps",
        "Minshawy_Murattal_48kbps" => "Minshawy_Murattal_128kbps",
        "Husary_64kbps" => "Husary_64kbps",
        "Husary_40kbps" => "Husary_40kbps",
        "Husary_Mujawwad_64kbps" => "Husary_Mujawwad_64kbps",
        "husary_qasr_64kbps" => "Husary_64kbps",
        "Alafasy_64kbps" => "Alafasy_64kbps",
        "Abdul_Basit_Murattal_40kbps" => "Abdul_Basit_Murattal_40kbps",
        "Maher_AlMuaiqly_64kbps" => "Maher_AlMuaiqly_64kbps",
        "Hudhaify_32kbps" => "Hudhaify_32kbps",
        "Ibrahim_Akhdar_32kbps" => "Ibrahim_Akhdar_32kbps",
        "Ayman_Sowaid_64kbps" => "Ayman_Sowaid_64kbps",
        "Fares_Abbad_64kbps" => "Fares_Abbad_64kbps",
        "Mohammad_al_Tablaway_64kbps" => "Mohammad_al_Tablaway_64kbps",
        "Muhammad_Ayyoub_32kbps" => "Muhammad_Ayyoub_32kbps",
        "Nasser_Alqatami_128kbps" => "Nasser_Alqatami_128kbps",
        "Abdullaah_3awwaad_Al-Juhaynee_128kbps" => "Abdullaah_3awwaad_Al-Juhaynee_128kbps",
        "tunaiji_64kbps" => "tunaiji_64kbps",
        "Banna_32kbps" => "Banna_32kbps",
        "English_Walk" => "English_Walk",
        other => other,
    }
}

#[tauri::command]
pub async fn q_pick_audio_dir(app: AppHandle) -> Value {
    let (tx, rx) = tokio::sync::oneshot::channel::<Option<FilePath>>();
    app.dialog()
        .file()
        .set_title("اختر مجلد التلاوات القرآنية")
        .pick_folder(move |p| {
            let _ = tx.send(p);
        });

    match rx.await {
        Ok(Some(fp)) => match path_buf_from_fp(&fp) {
            Some(pb) => {
                let s = pb.to_string_lossy().to_string();
                json!({ "ok": true, "path": s })
            }
            None => json!({ "ok": false, "err": "invalid_path" }),
        },
        Ok(None) => json!({ "ok": false, "err": "cancelled" }),
        Err(e) => json!({ "ok": false, "err": e.to_string() }),
    }
}

#[tauri::command]
pub fn q_get_audio_url(
    store: State<'_, ConfigStore>,
    surah: i64,
    ayah: i64,
    reciter: Option<String>,
) -> Value {
    let q = store.get_quran_config();
    let chosen_reciter = reciter
        .or(q.audio_reciter)
        .unwrap_or_else(|| "Husary_64kbps".to_string());
    let filename = format!("{:03}{:03}.mp3", surah, ayah);

    if let Some(base_str) = &q.audio_base_path {
        let base = Path::new(base_str);
        // Candidate 1: base / reciter / filename
        let candidate1 = base.join(&chosen_reciter).join(&filename);
        // Candidate 2: base / filename (if the user selected the reciter's folder directly)
        let candidate2 = base.join(&filename);

        let target = if candidate1.is_file() {
            Some(candidate1)
        } else if candidate2.is_file() {
            Some(candidate2)
        } else {
            None
        };

        if let Some(p) = target {
            if let Ok(bytes) = std::fs::read(&p) {
                let encoded = BASE64_STANDARD.encode(&bytes);
                return json!({
                    "url": format!("data:audio/mp3;base64,{}", encoded),
                    "local": true,
                    "surah": surah,
                    "ayah": ayah,
                    "reciter": chosen_reciter,
                    "filePath": p.to_string_lossy(),
                });
            }
        }
    }

    let online_folder = map_reciter_to_online(&chosen_reciter);
    let online_url = format!("https://everyayah.com/data/{}/{}", online_folder, filename);
    json!({
        "url": online_url,
        "local": false,
        "surah": surah,
        "ayah": ayah,
        "reciter": chosen_reciter,
    })
}
