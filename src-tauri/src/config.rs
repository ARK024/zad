use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, Manager};

/// Strongly-typed app mode enum
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub enum AppMode {
    #[default]
    Sequential,
    Alternating,
    Both,
    QuranOnly,
    HadithOnly,
}

impl AppMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Sequential => "sequential",
            Self::Alternating => "alternating",
            Self::Both => "both",
            Self::QuranOnly => "quranOnly",
            Self::HadithOnly => "hadithOnly",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "alternating" => Self::Alternating,
            "both" => Self::Both,
            "quranOnly" => Self::QuranOnly,
            "hadithOnly" => Self::HadithOnly,
            _ => Self::Sequential,
        }
    }
}

/// Strongly-typed theme enum
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    #[default]
    Light,
    Dark,
}

impl Theme {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Light => "light",
            Self::Dark => "dark",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "dark" => Self::Dark,
            _ => Self::Light,
        }
    }
}

/// Strongly-typed hadith config struct (mirrors `DEFAULTS` in src/main/config.js)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    pub interval: i64,
    pub font_size: i64,
    pub font_family: String,
    pub c_sanad: String,
    pub c_matn: String,
    pub c_takhrij: String,
    pub c_sharh: String,
    pub theme: Theme,
    pub index: i64,
    pub first_run: bool,
    pub auto_launch: bool,
    pub app_mode: AppMode,
    pub widget_w: f64,
    pub widget_h: f64,
    pub widget_x: Option<f64>,
    pub widget_y: Option<f64>,
    pub h_review_enabled: bool,
    pub h_review_days: i64,
    pub h_reviews: HashMap<String, i64>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            interval: 30,
            font_size: 22,
            font_family: "'QuranFont', 'Traditional Arabic'".to_string(),
            c_sanad: "#5d7a69".to_string(),
            c_matn: "#182820".to_string(),
            c_takhrij: "#1a9850".to_string(),
            c_sharh: "#b35900".to_string(),
            theme: Theme::default(),
            index: 0,
            first_run: true,
            auto_launch: true,
            app_mode: AppMode::default(),
            widget_w: 430.0,
            widget_h: 420.0,
            widget_x: None,
            widget_y: None,
            h_review_enabled: false,
            h_review_days: 7,
            h_reviews: HashMap::new(),
        }
    }
}

/// Strongly-typed Quran config struct
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct QuranConfig {
    pub current_quran_page: Option<i64>,
    pub daily_goal: Option<i64>,
    pub memorization_interval: Option<i64>,
    pub review_index: Option<i64>,
    pub recent_review_index: Option<i64>,
    pub paused_until: Option<i64>,
    pub widget_x: Option<f64>,
    pub widget_y: Option<f64>,
    pub widget_custom_width: Option<f64>,
    pub widget_custom_height: Option<f64>,
    pub recent_readings: Option<Vec<ReadingEntry>>,
    pub widget_size: Option<String>,
    pub font_size_px: Option<i64>,
    pub review_enabled: Option<bool>,
    pub recent_review_enabled: Option<bool>,
    pub review_days: Option<i64>,
    pub review_pages_per_session: Option<i64>,
    #[serde(alias = "hide_header")]
    pub hide_header: Option<bool>,
    pub memorized_pages: Option<Vec<i64>>,
    pub preloaded_pages: Option<Vec<i64>>,
    // Additional runtime keys used by quran_storage.js
    pub daily_streak: Option<i64>,
    pub last_completed_date: Option<String>,
    pub last_completed_time: Option<i64>,
    pub total_read_count: Option<i64>,
    #[serde(default)]
    pub completed_pages: Option<Value>,
    pub recent_pages_per_session: Option<i64>,
    pub day_start_hour: Option<i64>,
    pub last_review_date: Option<String>,
    pub review_cycle_start_date: Option<String>,
    pub review_session_start: Option<i64>,
    pub review_retry_pages: Option<Vec<i64>>,
    pub last_recent_review_date: Option<String>,
    pub recent_retry_pages: Option<Vec<i64>>,
    pub test_mode_enabled: Option<bool>,
    pub widget_shown_at: Option<i64>,
    pub weak_pages: Option<Vec<i64>>,
    pub progressive_mode_enabled: Option<bool>,
    pub progressive_level: Option<i64>,
    pub progressive_chunk: Option<i64>,
    pub prep_mode_enabled: Option<bool>,
    pub prep_pages_count: Option<i64>,
    pub prep_pages_per_session: Option<i64>,
    pub prep_cycles_count: Option<i64>,
    pub prep_session_index: Option<i64>,
    pub prep_current_cycle: Option<i64>,
    pub prep_last_date: Option<String>,
    pub audio_base_path: Option<String>,
    pub audio_reciter: Option<String>,
    pub audio_repeat_count: Option<i64>,
    pub audio_auto_play: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ReadingEntry {
    pub date: String,
    pub page: Option<i64>,
}

/// Helper to create default Value for backward compatibility
pub fn defaults() -> Value {
    json!({
        "interval": 30,
        "fontSize": 22,
        "fontFamily": "'QuranFont', 'Traditional Arabic'",
        "cSanad": "#5d7a69",
        "cMatn": "#182820",
        "cTakhrij": "#1a9850",
        "cSharh": "#b35900",
        "theme": "light",
        "index": 0,
        "firstRun": true,
        "autoLaunch": true,
        "appMode": "sequential",
        "widgetW": 430,
        "widgetH": 420,
        "widgetX": Value::Null,
        "widgetY": Value::Null,
        "hReviewEnabled": false,
        "hReviewDays": 7,
        "hReviews": {},
    })
}

/// Path of the main config store.
fn store_path(app: &AppHandle) -> anyhow::Result<PathBuf> {
    let dir = app.path().app_data_dir()?;
    std::fs::create_dir_all(&dir)?;
    Ok(dir.join("store.json"))
}

/// Path of the Quran config store.
fn q_store_path(app: &AppHandle) -> anyhow::Result<PathBuf> {
    let dir = app.path().app_data_dir()?;
    std::fs::create_dir_all(&dir)?;
    Ok(dir.join("quran_store.json"))
}

fn to_bool(v: &Value) -> Option<bool> {
    v.as_bool()
        .or_else(|| match v.as_str() {
            Some("true") | Some("1") => Some(true),
            Some("false") | Some("0") => Some(false),
            _ => None,
        })
        .or_else(|| v.as_i64().map(|n| n != 0))
}

/// Thread-safe wrapper around the two JSON config blobs.
#[derive(Clone)]
pub struct ConfigStore {
    pub cfg: Arc<Mutex<AppConfig>>,
    pub quran: Arc<Mutex<QuranConfig>>,
}

impl ConfigStore {
    pub fn new() -> Self {
        Self {
            cfg: Arc::new(Mutex::new(AppConfig::default())),
            quran: Arc::new(Mutex::new(QuranConfig::default())),
        }
    }

    /// Load the hadith config from disk, merging on top of the defaults.
    pub fn load_cfg(&self, app: &AppHandle) {
        let path = match store_path(app) {
            Ok(p) => p,
            Err(e) => {
                log::error!("Failed to get store path: {}", e);
                return;
            }
        };
        
        if let Ok(raw) = std::fs::read_to_string(&path) {
            // Try to parse as strongly-typed config first
            if let Ok(parsed) = serde_json::from_str::<AppConfig>(&raw) {
                log::info!("Successfully loaded strongly-typed AppConfig");
                *self.cfg.lock() = parsed;
                return;
            }
            
            // Fallback to Value-based parsing for backward compatibility
            log::warn!("AppConfig not strongly-typed, falling back to Value parsing");
            if let Ok(parsed_value) = serde_json::from_str::<Value>(&raw) {
                if let Some(obj) = parsed_value.as_object() {
                    if let Ok(parsed_as_app_config) = serde_json::from_value::<AppConfig>(parsed_value.clone()) {
                        log::info!("Converted Value to AppConfig successfully");
                        *self.cfg.lock() = parsed_as_app_config;
                        return;
                    }
                    
                    // Manual merge for unknown fields
                    let mut cfg = self.cfg.lock();
                    if let Some(v) = obj.get("interval").and_then(|v| v.as_i64()) {
                        cfg.interval = v;
                    }
                    if let Some(v) = obj.get("fontSize").and_then(|v| v.as_i64()) {
                        cfg.font_size = v;
                    }
                    if let Some(v) = obj.get("fontFamily").and_then(|v| v.as_str()) {
                        cfg.font_family = v.to_string();
                    }
                    if let Some(v) = obj.get("cSanad").and_then(|v| v.as_str()) {
                        cfg.c_sanad = v.to_string();
                    }
                    if let Some(v) = obj.get("cMatn").and_then(|v| v.as_str()) {
                        cfg.c_matn = v.to_string();
                    }
                    if let Some(v) = obj.get("cTakhrij").and_then(|v| v.as_str()) {
                        cfg.c_takhrij = v.to_string();
                    }
                    if let Some(v) = obj.get("cSharh").and_then(|v| v.as_str()) {
                        cfg.c_sharh = v.to_string();
                    }
                    if let Some(v) = obj.get("theme").and_then(|v| v.as_str()) {
                        cfg.theme = Theme::from_str(v);
                    }
                    if let Some(v) = obj.get("index").and_then(|v| v.as_i64()) {
                        cfg.index = v;
                    }
                    if let Some(v) = obj.get("firstRun").and_then(|v| v.as_bool()) {
                        cfg.first_run = v;
                    }
                    if let Some(v) = obj.get("autoLaunch").and_then(|v| v.as_bool()) {
                        cfg.auto_launch = v;
                    }
                    if let Some(v) = obj.get("appMode").and_then(|v| v.as_str()) {
                        cfg.app_mode = AppMode::from_str(v);
                    }
                    if let Some(v) = obj.get("widgetW").and_then(|v| v.as_f64()) {
                        cfg.widget_w = v;
                    }
                    if let Some(v) = obj.get("widgetH").and_then(|v| v.as_f64()) {
                        cfg.widget_h = v;
                    }
                    if let Some(v) = obj.get("widgetX").and_then(|v| v.as_f64()) {
                        cfg.widget_x = Some(v);
                    }
                    if let Some(v) = obj.get("widgetY").and_then(|v| v.as_f64()) {
                        cfg.widget_y = Some(v);
                    }
                    if let Some(v) = obj.get("hReviewEnabled").and_then(|v| v.as_bool()) {
                        cfg.h_review_enabled = v;
                    }
                    if let Some(v) = obj.get("hReviewDays").and_then(|v| v.as_i64()) {
                        cfg.h_review_days = v;
                    }
                    if let Some(v) = obj.get("hReviews").and_then(|v| v.as_object()) {
                        cfg.h_reviews = v
                            .iter()
                            .filter_map(|(k, v)| v.as_i64().map(|val| (k.clone(), val)))
                            .collect();
                    }
                    log::info!("Loaded AppConfig with {} custom values", obj.len());
                }
            } else {
                log::error!("Failed to parse config file as JSON: {}", path.display());
            }
        } else {
            log::warn!("Config file not found, using defaults: {}", path.display());
        }
    }

    /// Save the hadith config to disk atomically (write to temp file, then rename).
    pub fn save_cfg(&self, app: &AppHandle) {
        let path = match store_path(app) {
            Ok(p) => p,
            Err(e) => {
                log::error!("Failed to get store path for saving: {}", e);
                return;
            }
        };
        
        let snapshot = self.cfg.lock().clone();
        match serde_json::to_string(&snapshot) {
            Ok(s) => {
                let tmp_path = path.with_extension("json.tmp");
                if let Err(e) = std::fs::write(&tmp_path, &s) {
                    log::error!("Failed to write temp config to {}: {}", tmp_path.display(), e);
                    return;
                }
                if let Err(e) = std::fs::rename(&tmp_path, &path) {
                    log::error!("Failed to rename temp config {} -> {}: {}", tmp_path.display(), path.display(), e);
                    // Fallback: try direct write
                    let _ = std::fs::write(&path, &s);
                } else {
                    log::debug!("Config saved atomically to {}", path.display());
                }
            }
            Err(e) => {
                log::error!("Failed to serialize config: {}", e);
            }
        }
    }

    pub fn load_quran_cfg(&self, app: &AppHandle) {
        let path = match q_store_path(app) {
            Ok(p) => p,
            Err(e) => {
                log::error!("Failed to get quran store path: {}", e);
                return;
            }
        };
        
        if let Ok(raw) = std::fs::read_to_string(&path) {
            // Try strongly-typed parsing first
            if let Ok(parsed) = serde_json::from_str::<QuranConfig>(&raw) {
                log::info!("Successfully loaded strongly-typed QuranConfig");
                *self.quran.lock() = parsed;
                return;
            }
            
            // Fallback to Value parsing
            log::warn!("QuranConfig not strongly-typed, falling back to Value parsing");
            if let Ok(parsed_value) = serde_json::from_str::<Value>(&raw) {
                self.quran_update(&parsed_value);
                log::info!("Loaded QuranConfig via fallback update");
            } else {
                log::error!("Failed to parse quran config file as JSON: {}", path.display());
            }
        } else {
            log::warn!("Quran config file not found, using defaults: {}", path.display());
        }
    }

    pub fn save_quran_cfg(&self, app: &AppHandle) {
        let path = match q_store_path(app) {
            Ok(p) => p,
            Err(e) => {
                log::error!("Failed to get quran store path for saving: {}", e);
                return;
            }
        };
        
        let snapshot = self.quran.lock().clone();
        match serde_json::to_string(&snapshot) {
            Ok(s) => {
                let tmp_path = path.with_extension("json.tmp");
                if let Err(e) = std::fs::write(&tmp_path, &s) {
                    log::error!("Failed to write temp quran config to {}: {}", tmp_path.display(), e);
                    return;
                }
                if let Err(e) = std::fs::rename(&tmp_path, &path) {
                    log::error!("Failed to rename temp quran config {} -> {}: {}", tmp_path.display(), path.display(), e);
                    let _ = std::fs::write(&path, &s);
                } else {
                    log::debug!("Quran config saved atomically to {}", path.display());
                }
            }
            Err(e) => {
                log::error!("Failed to serialize quran config: {}", e);
            }
        }
    }

    pub fn cfg_get(&self) -> Value {
        let cfg = self.cfg.lock();
        serde_json::to_value(cfg.clone()).unwrap_or_else(|e| {
            log::error!("Failed to serialize AppConfig to Value: {}", e);
            json!({})
        })
    }

    pub fn cfg_set(&self, key: &str, value: Value) {
        let mut cfg = self.cfg.lock();
        match key {
            "interval" => cfg.interval = value.as_i64().unwrap_or(cfg.interval),
            "fontSize" => cfg.font_size = value.as_i64().unwrap_or(cfg.font_size),
            "fontFamily" => cfg.font_family = value.as_str().unwrap_or(&cfg.font_family).to_string(),
            "cSanad" => cfg.c_sanad = value.as_str().unwrap_or(&cfg.c_sanad).to_string(),
            "cMatn" => cfg.c_matn = value.as_str().unwrap_or(&cfg.c_matn).to_string(),
            "cTakhrij" => cfg.c_takhrij = value.as_str().unwrap_or(&cfg.c_takhrij).to_string(),
            "cSharh" => cfg.c_sharh = value.as_str().unwrap_or(&cfg.c_sharh).to_string(),
            "theme" => cfg.theme = value.as_str().map_or(cfg.theme.clone(), Theme::from_str),
            "index" => cfg.index = value.as_i64().unwrap_or(cfg.index),
            "firstRun" => cfg.first_run = value.as_bool().unwrap_or(cfg.first_run),
            "autoLaunch" => cfg.auto_launch = value.as_bool().unwrap_or(cfg.auto_launch),
            "appMode" => cfg.app_mode = value.as_str().map_or(cfg.app_mode.clone(), AppMode::from_str),
            "widgetW" => cfg.widget_w = value.as_f64().unwrap_or(cfg.widget_w),
            "widgetH" => cfg.widget_h = value.as_f64().unwrap_or(cfg.widget_h),
            "widgetX" => cfg.widget_x = value.as_f64(),
            "widgetY" => cfg.widget_y = value.as_f64(),
            "hReviewEnabled" => cfg.h_review_enabled = value.as_bool().unwrap_or(cfg.h_review_enabled),
            "hReviewDays" => cfg.h_review_days = value.as_i64().unwrap_or(cfg.h_review_days),
            "hReviews" => {
                if let Some(obj) = value.as_object() {
                    cfg.h_reviews = obj
                        .iter()
                        .filter_map(|(k, v)| v.as_i64().map(|val| (k.clone(), val)))
                        .collect();
                }
            }
            _ => log::warn!("Unknown config key: {}", key),
        }
    }

    pub fn cfg_update(&self, partial: &Value) {
        let mut cfg = self.cfg.lock();
        if let Some(map) = partial.as_object() {
            for (k, v) in map {
                match k.as_str() {
                    "interval" => cfg.interval = v.as_i64().unwrap_or(cfg.interval),
                    "fontSize" => cfg.font_size = v.as_i64().unwrap_or(cfg.font_size),
                    "fontFamily" => cfg.font_family = v.as_str().unwrap_or(&cfg.font_family).to_string(),
                    "cSanad" => cfg.c_sanad = v.as_str().unwrap_or(&cfg.c_sanad).to_string(),
                    "cMatn" => cfg.c_matn = v.as_str().unwrap_or(&cfg.c_matn).to_string(),
                    "cTakhrij" => cfg.c_takhrij = v.as_str().unwrap_or(&cfg.c_takhrij).to_string(),
                    "cSharh" => cfg.c_sharh = v.as_str().unwrap_or(&cfg.c_sharh).to_string(),
                    "theme" => cfg.theme = v.as_str().map_or(cfg.theme.clone(), Theme::from_str),
                    "index" => cfg.index = v.as_i64().unwrap_or(cfg.index),
                    "firstRun" => cfg.first_run = v.as_bool().unwrap_or(cfg.first_run),
                    "autoLaunch" => cfg.auto_launch = v.as_bool().unwrap_or(cfg.auto_launch),
                    "appMode" => cfg.app_mode = v.as_str().map_or(cfg.app_mode.clone(), AppMode::from_str),
                    "widgetW" => cfg.widget_w = v.as_f64().unwrap_or(cfg.widget_w),
                    "widgetH" => cfg.widget_h = v.as_f64().unwrap_or(cfg.widget_h),
                    "widgetX" => cfg.widget_x = v.as_f64(),
                    "widgetY" => cfg.widget_y = v.as_f64(),
                    "hReviewEnabled" => cfg.h_review_enabled = v.as_bool().unwrap_or(cfg.h_review_enabled),
                    "hReviewDays" => cfg.h_review_days = v.as_i64().unwrap_or(cfg.h_review_days),
                    "hReviews" => {
                        if let Some(obj) = v.as_object() {
                            cfg.h_reviews = obj
                                .iter()
                                .filter_map(|(k, val)| val.as_i64().map(|v| (k.clone(), v)))
                                .collect();
                        }
                    }
                    _ => log::warn!("Unknown config key in update: {}", k),
                }
            }
        }
    }

    pub fn cfg_value(&self, key: &str) -> Option<Value> {
        let cfg = self.cfg.lock();
        match key {
            "interval" => Some(json!(cfg.interval)),
            "fontSize" => Some(json!(cfg.font_size)),
            "fontFamily" => Some(json!(cfg.font_family)),
            "cSanad" => Some(json!(cfg.c_sanad)),
            "cMatn" => Some(json!(cfg.c_matn)),
            "cTakhrij" => Some(json!(cfg.c_takhrij)),
            "cSharh" => Some(json!(cfg.c_sharh)),
            "theme" => Some(json!(cfg.theme.as_str())),
            "index" => Some(json!(cfg.index)),
            "firstRun" => Some(json!(cfg.first_run)),
            "autoLaunch" => Some(json!(cfg.auto_launch)),
            "appMode" => Some(json!(cfg.app_mode.as_str())),
            "widgetW" => Some(json!(cfg.widget_w)),
            "widgetH" => Some(json!(cfg.widget_h)),
            "widgetX" => cfg.widget_x.map(|v| json!(v)),
            "widgetY" => cfg.widget_y.map(|v| json!(v)),
            "hReviewEnabled" => Some(json!(cfg.h_review_enabled)),
            "hReviewDays" => Some(json!(cfg.h_review_days)),
            "hReviews" => Some(json!(cfg.h_reviews)),
            _ => {
                log::warn!("Unknown config key: {}", key);
                None
            }
        }
    }

    // Strongly-typed getters for internal use
    pub fn get_app_config(&self) -> AppConfig {
        self.cfg.lock().clone()
    }

    pub fn get_quran_config(&self) -> QuranConfig {
        self.quran.lock().clone()
    }

    pub fn quran_get(&self) -> Value {
        let q = self.quran.lock();
        let mut val = serde_json::to_value(q.clone()).unwrap_or_else(|e| {
            log::error!("Failed to serialize QuranConfig to Value: {}", e);
            json!({})
        });
        if let Some(obj) = val.as_object_mut() {
            if let Some(h) = q.hide_header {
                obj.insert("hideHeader".to_string(), json!(h));
                obj.insert("hide_header".to_string(), json!(h));
            }
        }
        val
    }

    pub fn quran_set(&self, key: &str, value: Value) {
        let mut q = self.quran.lock();
        match key {
            "currentQuranPage" => q.current_quran_page = value.as_i64(),
            "dailyGoal" => q.daily_goal = value.as_i64(),
            "memorizationInterval" => q.memorization_interval = value.as_i64(),
            "reviewIndex" => q.review_index = value.as_i64(),
            "recentReviewIndex" => q.recent_review_index = value.as_i64(),
            "pausedUntil" => q.paused_until = value.as_i64(),
            "widgetX" => q.widget_x = value.as_f64(),
            "widgetY" => q.widget_y = value.as_f64(),
            "widgetCustomWidth" => q.widget_custom_width = value.as_f64(),
            "widgetCustomHeight" => q.widget_custom_height = value.as_f64(),
            "recentReadings" => {
                if let Some(arr) = value.as_array() {
                    q.recent_readings = Some(
                        arr.iter()
                            .filter_map(|r| serde_json::from_value::<ReadingEntry>(r.clone()).ok())
                            .collect()
                    );
                }
            }
            "widgetSize" => q.widget_size = value.as_str().map(|s| s.to_string()),
            "fontSizePx" => q.font_size_px = value.as_i64(),
            "reviewEnabled" => q.review_enabled = to_bool(&value),
            "recentReviewEnabled" => q.recent_review_enabled = to_bool(&value),
            "reviewDays" => q.review_days = value.as_i64(),
            "reviewPagesPerSession" => q.review_pages_per_session = value.as_i64(),
            "hideHeader" | "hide_header" => q.hide_header = to_bool(&value),
            "memorizedPages" => {
                if let Some(arr) = value.as_array() {
                    q.memorized_pages = Some(arr.iter().filter_map(|v| v.as_i64()).collect());
                }
            }
            "preloadedPages" => {
                if let Some(arr) = value.as_array() {
                    q.preloaded_pages = Some(arr.iter().filter_map(|v| v.as_i64()).collect());
                }
            }
            "dailyStreak" => q.daily_streak = value.as_i64(),
            "lastCompletedDate" => q.last_completed_date = value.as_str().map(|s| s.to_string()),
            "lastCompletedTime" => q.last_completed_time = value.as_i64(),
            "totalReadCount" => q.total_read_count = value.as_i64(),
            "completedPages" => q.completed_pages = Some(value),
            "recentPagesPerSession" => q.recent_pages_per_session = value.as_i64(),
            "dayStartHour" => q.day_start_hour = value.as_i64(),
            "lastReviewDate" => q.last_review_date = value.as_str().map(|s| s.to_string()),
            "reviewCycleStartDate" => q.review_cycle_start_date = value.as_str().map(|s| s.to_string()),
            "reviewSessionStart" => q.review_session_start = value.as_i64(),
            "reviewRetryPages" => {
                if let Some(arr) = value.as_array() {
                    q.review_retry_pages = Some(arr.iter().filter_map(|v| v.as_i64()).collect());
                }
            }
            "lastRecentReviewDate" => q.last_recent_review_date = value.as_str().map(|s| s.to_string()),
            "recentRetryPages" => {
                if let Some(arr) = value.as_array() {
                    q.recent_retry_pages = Some(arr.iter().filter_map(|v| v.as_i64()).collect());
                }
            }
            "testModeEnabled" => q.test_mode_enabled = to_bool(&value),
            "widgetShownAt" => q.widget_shown_at = value.as_i64(),
            "weakPages" => {
                if let Some(arr) = value.as_array() {
                    q.weak_pages = Some(arr.iter().filter_map(|v| v.as_i64()).collect());
                }
            }
            "progressiveModeEnabled" => q.progressive_mode_enabled = to_bool(&value),
            "progressiveLevel" => q.progressive_level = value.as_i64(),
            "progressiveChunk" => q.progressive_chunk = value.as_i64(),
            "prepModeEnabled" => q.prep_mode_enabled = to_bool(&value),
            "prepPagesCount" => q.prep_pages_count = value.as_i64(),
            "prepPagesPerSession" => q.prep_pages_per_session = value.as_i64(),
            "prepCyclesCount" => q.prep_cycles_count = value.as_i64(),
            "prepSessionIndex" => q.prep_session_index = value.as_i64(),
            "prepCurrentCycle" => q.prep_current_cycle = value.as_i64(),
            "prepLastDate" => q.prep_last_date = value.as_str().map(|s| s.to_string()),
            "audioBasePath" => q.audio_base_path = value.as_str().map(|s| s.to_string()),
            "audioReciter" => q.audio_reciter = value.as_str().map(|s| s.to_string()),
            "audioRepeatCount" => q.audio_repeat_count = value.as_i64(),
            "audioAutoPlay" => q.audio_auto_play = to_bool(&value),
            _ => log::warn!("Unknown quran config key: {}", key),
        }
    }

    pub fn quran_update(&self, partial: &Value) {
        let mut q = self.quran.lock();
        if let Some(map) = partial.as_object() {
            for (k, v) in map {
                match k.as_str() {
                    "currentQuranPage" => q.current_quran_page = v.as_i64(),
                    "dailyGoal" => q.daily_goal = v.as_i64(),
                    "memorizationInterval" => q.memorization_interval = v.as_i64(),
                    "reviewIndex" => q.review_index = v.as_i64(),
                    "recentReviewIndex" => q.recent_review_index = v.as_i64(),
                    "pausedUntil" => q.paused_until = v.as_i64(),
                    "widgetX" => q.widget_x = v.as_f64(),
                    "widgetY" => q.widget_y = v.as_f64(),
                    "widgetCustomWidth" => q.widget_custom_width = v.as_f64(),
                    "widgetCustomHeight" => q.widget_custom_height = v.as_f64(),
                    "recentReadings" => {
                        if let Some(arr) = v.as_array() {
                            q.recent_readings = Some(
                                arr.iter()
                                    .filter_map(|r| serde_json::from_value::<ReadingEntry>(r.clone()).ok())
                                    .collect()
                            );
                        }
                    }
                    "widgetSize" => q.widget_size = v.as_str().map(|s| s.to_string()),
                    "fontSizePx" => q.font_size_px = v.as_i64(),
                    "reviewEnabled" => q.review_enabled = to_bool(v),
                    "recentReviewEnabled" => q.recent_review_enabled = to_bool(v),
                    "reviewDays" => q.review_days = v.as_i64(),
                    "reviewPagesPerSession" => q.review_pages_per_session = v.as_i64(),
                    "hideHeader" | "hide_header" => q.hide_header = to_bool(v),
                    "memorizedPages" => {
                        if let Some(arr) = v.as_array() {
                            q.memorized_pages = Some(arr.iter().filter_map(|x| x.as_i64()).collect());
                        }
                    }
                    "preloadedPages" => {
                        if let Some(arr) = v.as_array() {
                            q.preloaded_pages = Some(arr.iter().filter_map(|x| x.as_i64()).collect());
                        }
                    }
                    "dailyStreak" => q.daily_streak = v.as_i64(),
                    "lastCompletedDate" => q.last_completed_date = v.as_str().map(|s| s.to_string()),
                    "lastCompletedTime" => q.last_completed_time = v.as_i64(),
                    "totalReadCount" => q.total_read_count = v.as_i64(),
                    "completedPages" => q.completed_pages = Some(v.clone()),
                    "recentPagesPerSession" => q.recent_pages_per_session = v.as_i64(),
                    "dayStartHour" => q.day_start_hour = v.as_i64(),
                    "lastReviewDate" => q.last_review_date = v.as_str().map(|s| s.to_string()),
                    "reviewCycleStartDate" => q.review_cycle_start_date = v.as_str().map(|s| s.to_string()),
                    "reviewSessionStart" => q.review_session_start = v.as_i64(),
                    "reviewRetryPages" => {
                        if let Some(arr) = v.as_array() {
                            q.review_retry_pages = Some(arr.iter().filter_map(|v| v.as_i64()).collect());
                        }
                    }
                    "lastRecentReviewDate" => q.last_recent_review_date = v.as_str().map(|s| s.to_string()),
                    "recentRetryPages" => {
                        if let Some(arr) = v.as_array() {
                            q.recent_retry_pages = Some(arr.iter().filter_map(|v| v.as_i64()).collect());
                        }
                    }
                    "testModeEnabled" => q.test_mode_enabled = to_bool(v),
                    "widgetShownAt" => q.widget_shown_at = v.as_i64(),
                    "weakPages" => {
                        if let Some(arr) = v.as_array() {
                            q.weak_pages = Some(arr.iter().filter_map(|v| v.as_i64()).collect());
                        }
                    }
                    "progressiveModeEnabled" => q.progressive_mode_enabled = to_bool(v),
                    "progressiveLevel" => q.progressive_level = v.as_i64(),
                    "progressiveChunk" => q.progressive_chunk = v.as_i64(),
                    "prepModeEnabled" => q.prep_mode_enabled = to_bool(v),
                    "prepPagesCount" => q.prep_pages_count = v.as_i64(),
                    "prepPagesPerSession" => q.prep_pages_per_session = v.as_i64(),
                    "prepCyclesCount" => q.prep_cycles_count = v.as_i64(),
                    "prepSessionIndex" => q.prep_session_index = v.as_i64(),
                    "prepCurrentCycle" => q.prep_current_cycle = v.as_i64(),
                    "prepLastDate" => q.prep_last_date = v.as_str().map(|s| s.to_string()),
                    "audioBasePath" => q.audio_base_path = v.as_str().map(|s| s.to_string()),
                    "audioReciter" => q.audio_reciter = v.as_str().map(|s| s.to_string()),
                    "audioRepeatCount" => q.audio_repeat_count = v.as_i64(),
                    "audioAutoPlay" => q.audio_auto_play = to_bool(v),
                    _ => log::warn!("Unknown quran config key in update: {}", k),
                }
            }
        }
    }

    pub fn quran_remove(&self, key: &str) -> bool {
        let mut q = self.quran.lock();
        match key {
            "currentQuranPage" => q.current_quran_page.take().is_some(),
            "dailyGoal" => q.daily_goal.take().is_some(),
            "memorizationInterval" => q.memorization_interval.take().is_some(),
            "reviewIndex" => q.review_index.take().is_some(),
            "recentReviewIndex" => q.recent_review_index.take().is_some(),
            "pausedUntil" => q.paused_until.take().is_some(),
            "widgetX" => q.widget_x.take().is_some(),
            "widgetY" => q.widget_y.take().is_some(),
            "widgetCustomWidth" => q.widget_custom_width.take().is_some(),
            "widgetCustomHeight" => q.widget_custom_height.take().is_some(),
            "recentReadings" => q.recent_readings.take().is_some(),
            "widgetSize" => q.widget_size.take().is_some(),
            "fontSizePx" => q.font_size_px.take().is_some(),
            "reviewEnabled" => q.review_enabled.take().is_some(),
            "recentReviewEnabled" => q.recent_review_enabled.take().is_some(),
            "reviewDays" => q.review_days.take().is_some(),
            "reviewPagesPerSession" => q.review_pages_per_session.take().is_some(),
            "hideHeader" | "hide_header" => q.hide_header.take().is_some(),
            "memorizedPages" => q.memorized_pages.take().is_some(),
            "preloadedPages" => q.preloaded_pages.take().is_some(),
            "dailyStreak" => q.daily_streak.take().is_some(),
            "lastCompletedDate" => q.last_completed_date.take().is_some(),
            "lastCompletedTime" => q.last_completed_time.take().is_some(),
            "totalReadCount" => q.total_read_count.take().is_some(),
            "completedPages" => q.completed_pages.take().is_some(),
            "recentPagesPerSession" => q.recent_pages_per_session.take().is_some(),
            "dayStartHour" => q.day_start_hour.take().is_some(),
            "lastReviewDate" => q.last_review_date.take().is_some(),
            "reviewCycleStartDate" => q.review_cycle_start_date.take().is_some(),
            "reviewSessionStart" => q.review_session_start.take().is_some(),
            "reviewRetryPages" => q.review_retry_pages.take().is_some(),
            "lastRecentReviewDate" => q.last_recent_review_date.take().is_some(),
            "recentRetryPages" => q.recent_retry_pages.take().is_some(),
            "testModeEnabled" => q.test_mode_enabled.take().is_some(),
            "widgetShownAt" => q.widget_shown_at.take().is_some(),
            "weakPages" => q.weak_pages.take().is_some(),
            "progressiveModeEnabled" => q.progressive_mode_enabled.take().is_some(),
            "progressiveLevel" => q.progressive_level.take().is_some(),
            "progressiveChunk" => q.progressive_chunk.take().is_some(),
            "prepModeEnabled" => q.prep_mode_enabled.take().is_some(),
            "prepPagesCount" => q.prep_pages_count.take().is_some(),
            "prepPagesPerSession" => q.prep_pages_per_session.take().is_some(),
            "prepCyclesCount" => q.prep_cycles_count.take().is_some(),
            "prepSessionIndex" => q.prep_session_index.take().is_some(),
            "prepCurrentCycle" => q.prep_current_cycle.take().is_some(),
            "prepLastDate" => q.prep_last_date.take().is_some(),
            "audioBasePath" => q.audio_base_path.take().is_some(),
            "audioReciter" => q.audio_reciter.take().is_some(),
            "audioRepeatCount" => q.audio_repeat_count.take().is_some(),
            "audioAutoPlay" => q.audio_auto_play.take().is_some(),
            _ => {
                log::warn!("Unknown quran config key to remove: {}", key);
                false
            }
        }
    }

    pub fn quran_clear(&self) -> Vec<String> {
        let mut q = self.quran.lock();
        let keys: Vec<String> = vec![
            "currentQuranPage".to_string(),
            "dailyGoal".to_string(),
            "memorizationInterval".to_string(),
            "reviewIndex".to_string(),
            "recentReviewIndex".to_string(),
            "pausedUntil".to_string(),
            "widgetX".to_string(),
            "widgetY".to_string(),
            "widgetCustomWidth".to_string(),
            "widgetCustomHeight".to_string(),
            "recentReadings".to_string(),
            "widgetSize".to_string(),
            "fontSizePx".to_string(),
            "reviewEnabled".to_string(),
            "recentReviewEnabled".to_string(),
            "reviewDays".to_string(),
            "reviewPagesPerSession".to_string(),
            "hideHeader".to_string(),
            "memorizedPages".to_string(),
            "preloadedPages".to_string(),
            "dailyStreak".to_string(),
            "lastCompletedDate".to_string(),
            "lastCompletedTime".to_string(),
            "totalReadCount".to_string(),
            "completedPages".to_string(),
            "recentPagesPerSession".to_string(),
            "dayStartHour".to_string(),
            "lastReviewDate".to_string(),
            "reviewCycleStartDate".to_string(),
            "reviewSessionStart".to_string(),
            "reviewRetryPages".to_string(),
            "lastRecentReviewDate".to_string(),
            "recentRetryPages".to_string(),
            "testModeEnabled".to_string(),
            "widgetShownAt".to_string(),
            "weakPages".to_string(),
            "progressiveModeEnabled".to_string(),
            "progressiveLevel".to_string(),
            "progressiveChunk".to_string(),
            "prepModeEnabled".to_string(),
            "prepPagesCount".to_string(),
            "prepPagesPerSession".to_string(),
            "prepCyclesCount".to_string(),
            "prepSessionIndex".to_string(),
            "prepCurrentCycle".to_string(),
            "prepLastDate".to_string(),
            "audioBasePath".to_string(),
            "audioReciter".to_string(),
            "audioRepeatCount".to_string(),
            "audioAutoPlay".to_string(),
        ];
        *q = QuranConfig::default();
        keys
    }
}

impl Default for ConfigStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_have_expected_keys() {
        let d = defaults();
        assert_eq!(d.get("interval").unwrap().as_i64().unwrap(), 30);
        assert_eq!(d.get("fontSize").unwrap().as_i64().unwrap(), 22);
        assert_eq!(d.get("appMode").unwrap().as_str().unwrap(), "sequential");
        assert!(d.get("firstRun").unwrap().as_bool().unwrap());
        assert_eq!(d.get("hReviewDays").unwrap().as_i64().unwrap(), 7);
    }

    #[test]
    fn cfg_set_and_update() {
        let s = ConfigStore::new();
        s.cfg_set("index", json!(42));
        assert_eq!(s.cfg_value("index").unwrap().as_i64().unwrap(), 42);
        s.cfg_update(&json!({"index": 7, "theme": "dark"}));
        assert_eq!(s.cfg_value("index").unwrap().as_i64().unwrap(), 7);
        assert_eq!(s.cfg_value("theme").unwrap().as_str().unwrap(), "dark");
    }

    #[test]
    fn quran_remove_and_clear() {
        let s = ConfigStore::new();
        s.quran_update(&json!({"fontSizePx": 24, "dailyGoal": 2}));
        assert!(s.quran_remove("fontSizePx"));
        assert!(!s.quran_remove("fontSizePx"));
        let cleared = s.quran_clear();
        assert!(cleared.contains(&"dailyGoal".to_string()));
        assert!(s.quran_get().as_object().unwrap().values().all(|v| v.is_null()));
    }

    #[test]
    fn quran_hide_header_persistence() {
        let s = ConfigStore::new();
        s.quran_set("hideHeader", json!(true));
        assert_eq!(s.quran_get().get("hideHeader").unwrap(), &json!(true));
        let snapshot = s.quran.lock().clone();
        let serialized = serde_json::to_string(&snapshot).unwrap();
        let deserialized: QuranConfig = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized.hide_header, Some(true));

        // Test snake_case deserialization alias
        let from_snake: QuranConfig = serde_json::from_str(r#"{"hide_header": true}"#).unwrap();
        assert_eq!(from_snake.hide_header, Some(true));

        // Test snake_case quran_set
        s.quran_set("hide_header", json!(false));
        assert_eq!(s.quran.lock().hide_header, Some(false));
    }
}
