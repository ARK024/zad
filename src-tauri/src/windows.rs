use crate::config::ConfigStore;
use crate::data_loader::DataLoader;
use crate::AppContext;
use chrono::{DateTime, Utc};
use serde_json::{json, Value};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::{
    AppHandle, Emitter, LogicalPosition, LogicalSize, Manager, WebviewUrl, WebviewWindowBuilder,
    WindowEvent,
};

pub const WIDGET_LABEL: &str = "widget";
pub const QURAN_LABEL: &str = "qwindow";
pub const SETTINGS_LABEL: &str = "settings";
pub const WELCOME_LABEL: &str = "welcome";

const WIDGET_MIN_W: f64 = 360.0;
const WIDGET_MIN_H: f64 = 300.0;
const WIDGET_MAX_W: f64 = 700.0;
const WIDGET_MAX_H: f64 = 800.0;
const WIDGET_DEFAULT_W: f64 = 430.0;
const WIDGET_DEFAULT_H: f64 = 420.0;

/// Tracks whether `show_widget` is in flight to prevent re-entrant creation
/// (mirrors `widgetBusy` in src/main/windows.js).
#[derive(Clone, Default)]
pub struct WidgetGuard {
    busy: Arc<AtomicBool>,
}

impl WidgetGuard {
    pub fn new() -> Self {
        Self::default()
    }

    /// Try to claim the guard; returns true on success.
    pub fn try_acquire(&self) -> bool {
        !self.busy.swap(true, Ordering::SeqCst)
    }

    pub fn release(&self) {
        self.busy.store(false, Ordering::SeqCst);
    }
}

/// Compute the on-screen widget position, falling back to bottom-right of the
/// primary monitor when no saved position is on any current display.
fn compute_widget_pos(app: &AppHandle, cfg: &Value, w: f64, h: f64) -> (f64, f64) {
    let saved_x = cfg.get("widgetX").and_then(|v| v.as_f64());
    let saved_y = cfg.get("widgetY").and_then(|v| v.as_f64());

    if let (Some(x), Some(y)) = (saved_x, saved_y) {
        if let Ok(monitors) = app.available_monitors() {
            for m in &monitors {
                let pos = m.position();
                let size = m.size();
                let scale = m.scale_factor();
                let mx = pos.x as f64 / scale;
                let my = pos.y as f64 / scale;
                let mw = size.width as f64 / scale;
                let mh = size.height as f64 / scale;
                if x >= mx && x + w <= mx + mw && y >= my && y + h <= my + mh {
                    return (x, y);
                }
            }
        }
    }

    if let Ok(Some(primary)) = app.primary_monitor() {
        let scale = primary.scale_factor();
        let size = primary.size();
        let sw = size.width as f64 / scale;
        let sh = size.height as f64 / scale;
        return (sw - w - 16.0, sh - h - 16.0);
    }
    (40.0, 40.0)
}

/// Create or recreate the hadith widget window and load `widget.html`.
/// Mirrors `showWidget()` in src/main/windows.js.
pub fn show_widget(
    app: &AppHandle,
    store: &ConfigStore,
    data: &DataLoader,
    guard: &WidgetGuard,
    index_override: Option<usize>,
    is_review_override: bool,
) {
    let hadith_count = data.hadiths_len();
    if hadith_count == 0 {
        log::warn!("Cannot show widget: no hadith data loaded");
        return;
    }
    
    if !guard.try_acquire() {
        log::debug!("WidgetGuard already acquired, skipping show_widget");
        return;
    }
    log::debug!("show_widget called, index_override={:?}, is_review_override={}", index_override, is_review_override);

    if let Some(existing) = app.get_webview_window(WIDGET_LABEL) {
        let _ = existing.destroy();
    }

    let cfg = store.cfg_get();
    let cfg_index = cfg.get("index").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
    let (idx, is_review) = match index_override {
        Some(i) => (i, is_review_override),
        None => {
            let mut found: Option<(usize, bool)> = None;
            if cfg
                .get("hReviewEnabled")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
            {
                if let Some(reviews) = cfg.get("hReviews").and_then(|v| v.as_object()) {
                    let now = chrono_now_ms();
                    for (k, v) in reviews {
                        if let (Ok(idx), Some(t)) = (k.parse::<usize>(), v.as_i64()) {
                            if now >= t {
                                log::debug!("Found review hadith at index {}", idx);
                                found = Some((idx, true));
                                break;
                            }
                        }
                    }
                }
            }
            found.unwrap_or((cfg_index, false))
        }
    };
    
    log::info!("Showing widget for hadith index {} (is_review={})", idx, is_review);

    let payload = match data.build_widget_payload(idx, is_review, &cfg) {
        Some(p) => {
            log::debug!("Built widget payload successfully");
            p
        }
        None => {
            log::error!("Failed to build widget payload for index {}", idx);
            guard.release();
            return;
        }
    };

    let w = cfg
        .get("widgetW")
        .and_then(|v| v.as_f64())
        .unwrap_or(WIDGET_DEFAULT_W)
        .clamp(WIDGET_MIN_W, WIDGET_MAX_W);
    let h = cfg
        .get("widgetH")
        .and_then(|v| v.as_f64())
        .unwrap_or(WIDGET_DEFAULT_H)
        .clamp(WIDGET_MIN_H, WIDGET_MAX_H);
    let (x, y) = compute_widget_pos(app, &cfg, w, h);

    let builder = WebviewWindowBuilder::new(app, WIDGET_LABEL, WebviewUrl::App("widget.html".into()))
        .title("رياض الصالحين")
        .inner_size(w, h)
        .position(x, y)
        .min_inner_size(WIDGET_MIN_W, WIDGET_MIN_H)
        .max_inner_size(WIDGET_MAX_W, WIDGET_MAX_H)
        .decorations(false)
        .transparent(false)
        .always_on_top(true)
        .resizable(true)
        .skip_taskbar(true)
        .visible(false);

    // Stash the prepared payload so the renderer can pull it back via `widget_ready`.
    if let Some(ctx) = app.try_state::<AppContext>() {
        *ctx.pending_widget_payload.lock() = Some(payload);
        log::debug!("Widget payload stashed in AppContext");
    }

    let window = match builder.build() {
        Ok(w) => {
            log::info!("Widget window created successfully");
            w
        }
        Err(e) => {
            log::error!("Failed to create widget window: {}", e);
            guard.release();
            return;
        }
    };
    guard.release();

    let app_for_listener = app.clone();
    let store_for_listener = store.clone();
    let win_for_listener = window.clone();
    window.on_window_event(move |event| match event {
        WindowEvent::Moved(_) | WindowEvent::Resized(_) => {
            if let Ok(pos) = win_for_listener.outer_position() {
                if let Ok(size) = win_for_listener.outer_size() {
                    let scale = win_for_listener.scale_factor().unwrap_or(1.0);
                    store_for_listener.cfg_set("widgetX", json!(pos.x as f64 / scale));
                    store_for_listener.cfg_set("widgetY", json!(pos.y as f64 / scale));
                    store_for_listener.cfg_set("widgetW", json!(size.width as f64 / scale));
                    store_for_listener.cfg_set("widgetH", json!(size.height as f64 / scale));
                    store_for_listener.save_cfg(&app_for_listener);
                    log::debug!("Widget geometry saved: pos=({},{}) size=({},{}) scale={}", 
                        pos.x, pos.y, size.width, size.height, scale);
                }
            }
        }
        _ => {}
    });
}

pub fn destroy_widget(app: &AppHandle) {
    log::debug!("Destroying widget window");
    if let Some(w) = app.get_webview_window(WIDGET_LABEL) {
        let _ = w.destroy();
        log::debug!("Widget window destroyed");
    }
}

/// Send a fresh hadith payload to the existing widget window if any, otherwise create it.
pub fn refresh_widget_payload(
    app: &AppHandle,
    store: &ConfigStore,
    data: &DataLoader,
    guard: &WidgetGuard,
) {
    let cfg = store.cfg_get();
    let idx = cfg.get("index").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
    if let Some(payload) = data.build_widget_payload(idx, false, &cfg) {
        if let Some(w) = app.get_webview_window(WIDGET_LABEL) {
            let _ = w.emit("hadith", payload);
            return;
        }
    }
    show_widget(app, store, data, guard, None, false);
}

/// Create the (initially hidden) Quran widget window.
pub fn create_quran_window(app: &AppHandle, store: &ConfigStore) {
    if app.get_webview_window(QURAN_LABEL).is_some() {
        log::debug!("Quran window already exists, skipping creation");
        return;
    }
    
    let (sw, sh) = primary_size(app).unwrap_or((1280.0, 800.0));
    let qcfg = store.quran_get();
    let w = qcfg
        .get("widgetCustomWidth")
        .and_then(|v| v.as_f64())
        .unwrap_or(500.0);
    let h = qcfg
        .get("widgetCustomHeight")
        .and_then(|v| v.as_f64())
        .unwrap_or((sh - 60.0).max(360.0));
    let x = qcfg
        .get("widgetX")
        .and_then(|v| v.as_f64())
        .unwrap_or(sw - w - 20.0);
    let y = qcfg.get("widgetY").and_then(|v| v.as_f64()).unwrap_or(30.0);

    log::info!("Creating Quran window at ({}, {}) size ({}x{})", x, y, w, h);

    let builder = WebviewWindowBuilder::new(
        app,
        QURAN_LABEL,
        WebviewUrl::App("quran_widget.html".into()),
    )
    .title("التذكير بالقرآن")
    .inner_size(w, h)
    .position(x, y)
    .decorations(false)
    .transparent(false)
    .always_on_top(true)
    .resizable(true)
    .skip_taskbar(true)
    .visible(false);

    if let Ok(window) = builder.build() {
        log::info!("Quran window created successfully");
        let app_for_listener = app.clone();
        let store_for_listener = store.clone();
        let win_for_listener = window.clone();
        window.on_window_event(move |event| match event {
            WindowEvent::Moved(_) | WindowEvent::Resized(_) => {
                if let Ok(pos) = win_for_listener.outer_position() {
                    if let Ok(size) = win_for_listener.outer_size() {
                        let scale = win_for_listener.scale_factor().unwrap_or(1.0);
                        store_for_listener.quran_set("widgetX", json!(pos.x as f64 / scale));
                        store_for_listener.quran_set("widgetY", json!(pos.y as f64 / scale));
                        store_for_listener
                            .quran_set("widgetCustomWidth", json!(size.width as f64 / scale));
                        store_for_listener
                            .quran_set("widgetCustomHeight", json!(size.height as f64 / scale));
                        store_for_listener.save_quran_cfg(&app_for_listener);
                        log::debug!("Quran geometry saved: pos=({},{}) size=({},{}) scale={}", 
                            pos.x, pos.y, size.width, size.height, scale);
                    }
                }
            }
            _ => {}
        });
    } else {
        log::error!("Failed to create Quran window");
    }
}

pub fn show_quran_window(app: &AppHandle) {
    log::debug!("Showing Quran window");
    if let Some(w) = app.get_webview_window(QURAN_LABEL) {
        let _ = w.eval("window.location.href = 'quran_widget.html?show=true';");
        log::debug!("Quran window reload triggered");
    } else {
        log::warn!("Quran window not found, cannot show");
    }
}

pub fn hide_quran_window(app: &AppHandle) {
    log::debug!("Hiding Quran window");
    if let Some(w) = app.get_webview_window(QURAN_LABEL) {
        let _ = w.hide();
        log::debug!("Quran window hidden");
    }
}

pub fn open_settings(app: &AppHandle) {
    log::debug!("Opening settings window");
    if let Some(existing) = app.get_webview_window(SETTINGS_LABEL) {
        let _ = existing.set_focus();
        return;
    }
    let _ = WebviewWindowBuilder::new(
        app,
        SETTINGS_LABEL,
        WebviewUrl::App("settings.html".into()),
    )
    .title("رياض الصالحين — الإعدادات")
    .inner_size(420.0, 700.0)
    .resizable(true)
    .build();
    log::debug!("Settings window opened");
}

pub fn open_welcome(app: &AppHandle) {
    log::info!("Opening welcome window");
    if app.get_webview_window(WELCOME_LABEL).is_some() {
        log::debug!("Welcome window already exists");
        return;
    }
    let _ = WebviewWindowBuilder::new(app, WELCOME_LABEL, WebviewUrl::App("welcome.html".into()))
        .title("مرحبا بك في رياض الصالحين")
        .inner_size(420.0, 520.0)
        .resizable(false)
        .center()
        .build();
    log::debug!("Welcome window created");
}

pub fn close_welcome(app: &AppHandle) {
    log::debug!("Closing welcome window");
    if let Some(w) = app.get_webview_window(WELCOME_LABEL) {
        let _ = w.close();
        log::debug!("Welcome window closed");
    }
}

pub fn reset_widget_geometry(app: &AppHandle, store: &ConfigStore) {
    log::info!("Resetting widget geometry to defaults");
    store.cfg_set("widgetX", Value::Null);
    store.cfg_set("widgetY", Value::Null);
    store.cfg_set("widgetW", json!(WIDGET_DEFAULT_W as i64));
    store.cfg_set("widgetH", json!(WIDGET_DEFAULT_H as i64));
    store.save_cfg(app);
    if let Some(w) = app.get_webview_window(WIDGET_LABEL) {
        let (sw, sh) = primary_size(app).unwrap_or((1280.0, 800.0));
        let _ = w.set_size(LogicalSize::new(WIDGET_DEFAULT_W, WIDGET_DEFAULT_H));
        let _ = w.set_position(LogicalPosition::new(
            sw - WIDGET_DEFAULT_W - 16.0,
            sh - WIDGET_DEFAULT_H - 16.0,
        ));
        log::debug!("Widget geometry reset");
    }
}

pub fn reset_quran_geometry(app: &AppHandle, store: &ConfigStore) {
    log::info!("Resetting Quran geometry to defaults");
    let (sw, _sh) = primary_size(app).unwrap_or((1280.0, 800.0));
    store.quran_set("widgetX", Value::Null);
    store.quran_set("widgetY", Value::Null);
    store.quran_set("widgetCustomWidth", Value::Null);
    store.quran_set("widgetCustomHeight", Value::Null);
    store.save_quran_cfg(app);
    if let Some(w) = app.get_webview_window(QURAN_LABEL) {
        let height = if let Ok(Some(p)) = app.primary_monitor() {
            (p.size().height as f64 / p.scale_factor() - 60.0).max(360.0)
        } else {
            740.0
        };
        let _ = w.set_size(LogicalSize::new(500.0, height));
        let _ = w.set_position(LogicalPosition::new(sw - 500.0 - 20.0, 30.0));
        log::debug!("Quran geometry reset");
    }
}

fn primary_size(app: &AppHandle) -> Option<(f64, f64)> {
    let p = app.primary_monitor().ok().flatten()?;
    let scale = p.scale_factor();
    let size = p.size();
    Some((size.width as f64 / scale, size.height as f64 / scale))
}

fn chrono_now_ms() -> i64 {
    let now: DateTime<Utc> = Utc::now();
    now.timestamp_millis()
}
