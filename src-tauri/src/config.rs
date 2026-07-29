use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, Manager};

/// Strongly-typed app mode enum
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum AppMode {
    Sequential,
    Alternating,
    Both,
    QuranOnly,
    HadithOnly,
}

impl Default for AppMode {
    fn default() -> Self {
        Self::Sequential
    }
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
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    Light,
    Dark,
}

impl Default for Theme {
    fn default() -> Self {
        Self::Light
    }
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadingEntry {
    pub date: String,
    pub page: Option<i64>,
}

impl Default for ReadingEntry {
    fn default() -> Self {
        Self {
            date: String::new(),
            page: None,
        }
    }
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
        
        let mut merged = AppConfig::default();
        
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

    /// Save the hadith config to disk (best effort).
    pub fn save_cfg(&self, app: &AppHandle) {
        let path = match store_path(app) {
            Ok(p) => p,
            Err(e) => {
                log::error!("Failed to get store path for saving: {}", e);
                return;
            }
        };
        
        let snapshot = self.cfg.lock().clone();
        match serde_json::to_string_pretty(&snapshot) {
            Ok(s) => {
                if let Err(e) = std::fs::write(&path, s) {
                    log::error!("Failed to save config to {}: {}", path.display(), e);
                } else {
                    log::debug!("Config saved successfully to {}", path.display());
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
                if let Some(obj) = parsed_value.as_object() {
                    let mut q = self.quran.lock();
                    if let Some(v) = obj.get("currentQuranPage").and_then(|v| v.as_i64()) {
                        q.current_quran_page = Some(v);
                    }
                    if let Some(v) = obj.get("dailyGoal").and_then(|v| v.as_i64()) {
                        q.daily_goal = Some(v);
                    }
                    if let Some(v) = obj.get("memorizationInterval").and_then(|v| v.as_i64()) {
                        q.memorization_interval = Some(v);
                    }
                    if let Some(v) = obj.get("reviewIndex").and_then(|v| v.as_i64()) {
                        q.review_index = Some(v);
                    }
                    if let Some(v) = obj.get("recentReviewIndex").and_then(|v| v.as_i64()) {
                        q.recent_review_index = Some(v);
                    }
                    if let Some(v) = obj.get("pausedUntil").and_then(|v| v.as_i64()) {
                        q.paused_until = Some(v);
                    }
                    if let Some(v) = obj.get("widgetX").and_then(|v| v.as_f64()) {
                        q.widget_x = Some(v);
                    }
                    if let Some(v) = obj.get("widgetY").and_then(|v| v.as_f64()) {
                        q.widget_y = Some(v);
                    }
                    if let Some(v) = obj.get("widgetCustomWidth").and_then(|v| v.as_f64()) {
                        q.widget_custom_width = Some(v);
                    }
                    if let Some(v) = obj.get("widgetCustomHeight").and_then(|v| v.as_f64()) {
                        q.widget_custom_height = Some(v);
                    }
                    if let Some(v) = obj.get("recentReadings").and_then(|v| v.as_array()) {
                        q.recent_readings = Some(
                            v.iter()
                                .filter_map(|r| {
                                    serde_json::from_value::<ReadingEntry>(r.clone()).ok()
                                })
                                .collect()
                        );
                    }
                    log::info!("Loaded QuranConfig with {} custom values", obj.len());
                }
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
        match serde_json::to_string_pretty(&snapshot) {
            Ok(s) => {
                if let Err(e) = std::fs::write(&path, s) {
                    log::error!("Failed to save quran config to {}: {}", path.display(), e);
                } else {
                    log::debug!("Quran config saved successfully to {}", path.display());
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
            "theme" => cfg.theme = value.as_str().map_or(cfg.theme.clone(), |s| Theme::from_str(s)),
            "index" => cfg.index = value.as_i64().unwrap_or(cfg.index),
            "firstRun" => cfg.first_run = value.as_bool().unwrap_or(cfg.first_run),
            "autoLaunch" => cfg.auto_launch = value.as_bool().unwrap_or(cfg.auto_launch),
            "appMode" => cfg.app_mode = value.as_str().map_or(cfg.app_mode.clone(), |s| AppMode::from_str(s)),
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
                    "theme" => cfg.theme = v.as_str().map_or(cfg.theme.clone(), |s| Theme::from_str(s)),
                    "index" => cfg.index = v.as_i64().unwrap_or(cfg.index),
                    "firstRun" => cfg.first_run = v.as_bool().unwrap_or(cfg.first_run),
                    "autoLaunch" => cfg.auto_launch = v.as_bool().unwrap_or(cfg.auto_launch),
                    "appMode" => cfg.app_mode = v.as_str().map_or(cfg.app_mode.clone(), |s| AppMode::from_str(s)),
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
        serde_json::to_value(q.clone()).unwrap_or_else(|e| {
            log::error!("Failed to serialize QuranConfig to Value: {}", e);
            json!({})
        })
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
                    _ => log::warn!("Unknown quran config key in update: {}", k),
                }
            }
        }
    }

    pub fn quran_remove(&self, key: &str) -> bool {
        let mut q = self.quran.lock();
        match key {
            "currentQuranPage" => { q.current_quran_page = None; true }
            "dailyGoal" => { q.daily_goal = None; true }
            "memorizationInterval" => { q.memorization_interval = None; true }
            "reviewIndex" => { q.review_index = None; true }
            "recentReviewIndex" => { q.recent_review_index = None; true }
            "pausedUntil" => { q.paused_until = None; true }
            "widgetX" => { q.widget_x = None; true }
            "widgetY" => { q.widget_y = None; true }
            "widgetCustomWidth" => { q.widget_custom_width = None; true }
            "widgetCustomHeight" => { q.widget_custom_height = None; true }
            "recentReadings" => { q.recent_readings = None; true }
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
        s.quran_update(&json!({"a": 1, "b": 2}));
        assert!(s.quran_remove("a"));
        assert!(!s.quran_remove("a"));
        let cleared = s.quran_clear();
        assert!(cleared.contains(&"b".to_string()));
        assert!(s.quran_get().as_object().unwrap().is_empty());
    }
}
