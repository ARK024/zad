use crate::config::ConfigStore;
use crate::data_loader::{DataLoader, PageAyahs, SearchHit};
use crate::tray;
use crate::windows;
use crate::AppContext;
use serde_json::{json, Value};
use std::collections::HashMap;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_autostart::ManagerExt;

// Whitelist for allowed Quran config keys to prevent prototype pollution
const ALLOWED_QURAN_KEYS: &[&str] = &[
    "currentQuranPage",
    "dailyGoal",
    "memorizationInterval",
    "reviewIndex",
    "recentReviewIndex",
    "pausedUntil",
    "widgetX",
    "widgetY",
    "widgetCustomWidth",
    "widgetCustomHeight",
    "recentReadings",
    "widgetSize",
    "fontSizePx",
    "reviewEnabled",
    "recentReviewEnabled",
    "reviewDays",
    "reviewPagesPerSession",
    "hideHeader",
    "memorizedPages",
    "preloadedPages",
];

fn is_allowed_quran_key(key: &str) -> bool {
    ALLOWED_QURAN_KEYS.contains(&key)
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

// ── Quran Storage ──────────────────────────────────────────────────────────

#[tauri::command]
pub fn q_store_get(store: State<'_, ConfigStore>, keys: Value) -> Value {
    let q = store.quran_get();
    if keys.is_null() {
        return q;
    }
    if let Some(arr) = keys.as_array() {
        let mut out = serde_json::Map::new();
        for k in arr {
            if let Some(s) = k.as_str() {
                out.insert(s.to_string(), q.get(s).cloned().unwrap_or(Value::Null));
            }
        }
        return Value::Object(out);
    }
    if let Some(obj) = keys.as_object() {
        let mut out = serde_json::Map::new();
        for (k, default_v) in obj {
            let v = q.get(k).cloned();
            out.insert(k.clone(), v.unwrap_or_else(|| default_v.clone()));
        }
        return Value::Object(out);
    }
    if let Some(s) = keys.as_str() {
        let mut out = serde_json::Map::new();
        out.insert(s.to_string(), q.get(s).cloned().unwrap_or(Value::Null));
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
    if let Some(w) = app.get_webview_window(windows::QURAN_LABEL) {
        let _ = w.emit("q_store_changed", changed);
    }
    if let Some(w) = app.get_webview_window(windows::SETTINGS_LABEL) {
        let _ = w.emit("q_store_changed", changed);
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
    let font_size = parse_int("fontSize", 17).clamp(12, 72);
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
) -> Result<Value, String> {
    Ok(crate::backup::do_restore(&app, &store, &data).await)
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
    ctx.restart_orchestrator(&app);
    log::info!("welcome_done: orchestrator restarted");
    windows::open_settings(&app);
    log::info!("welcome_done: settings opened");
}
