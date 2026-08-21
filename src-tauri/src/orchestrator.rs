use crate::config::ConfigStore;
use chrono::{Datelike, Local, Timelike};
use parking_lot::Mutex;
use serde_json::Value;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

/// Mirrors orchestrator.js modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppMode {
    Sequential,
    Alternating,
    Both,
    QuranOnly,
    HadithOnly,
}

impl AppMode {
    pub fn parse(s: &str) -> Self {
        match s {
            "alternating" => Self::Alternating,
            "both" => Self::Both,
            "quranOnly" => Self::QuranOnly,
            "hadithOnly" => Self::HadithOnly,
            _ => Self::Sequential,
        }
    }
}

/// Minutes -> milliseconds.
pub fn minutes_to_ms(m: i64) -> i64 {
    m * 60 * 1000
}

/// Today as `YYYY-MM-DD` (local time), respecting `dayStartHour`.
/// If the current hour is before `day_start_hour`, the effective date is yesterday.
fn today_string_with_offset(day_start_hour: u32) -> String {
    let mut now = Local::now();
    if (now.hour()) < day_start_hour {
        now = now - chrono::Duration::days(1);
    }
    format!("{:04}-{:02}-{:02}", now.year(), now.month(), now.day())
}

/// Whether the configured Quran daily goal has been met (in either appMode).
pub fn is_quran_goal_met(quran_cfg: &Value) -> bool {
    let daily_goal = quran_cfg
        .get("dailyGoal")
        .and_then(|v| v.as_i64())
        .filter(|n| *n > 0)
        .unwrap_or(1);
    let day_start_hour = quran_cfg
        .get("dayStartHour")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u32;
    let today = today_string_with_offset(day_start_hour);
    let recent = quran_cfg
        .get("recentReadings")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let completed_today = recent
        .iter()
        .filter(|r| r.get("date").and_then(|v| v.as_str()) == Some(today.as_str()))
        .count() as i64;
    completed_today >= daily_goal
}

/// Active interval in milliseconds based on the current mode.
pub fn get_active_interval_ms(cfg: &Value, quran_cfg: &Value) -> i64 {
    let mode = AppMode::parse(
        cfg.get("appMode")
            .and_then(|v| v.as_str())
            .unwrap_or("sequential"),
    );
    let hadith_iv_min = cfg.get("interval").and_then(|v| v.as_i64()).unwrap_or(30);
    let quran_iv_min = quran_cfg
        .get("memorizationInterval")
        .and_then(|v| v.as_i64())
        .filter(|n| *n > 0)
        .unwrap_or(10);

    match mode {
        AppMode::QuranOnly => minutes_to_ms(quran_iv_min),
        AppMode::HadithOnly => minutes_to_ms(hadith_iv_min),
        AppMode::Sequential if !is_quran_goal_met(quran_cfg) => minutes_to_ms(quran_iv_min),
        _ => minutes_to_ms(hadith_iv_min),
    }
}

/// Outcome of a tick — used by the integration code to actually show/hide windows.
/// Lets the orchestrator stay testable without depending on Tauri windows directly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TickAction {
    ShowHadith,
    ShowQuran,
    ShowBoth,
    HideQuranShowHadith,
}

#[derive(Default)]
struct OrchestratorInner {
    last_was_quran: AtomicBool,
}

/// The orchestrator exposes:
///  - `decide_tick_action` (pure logic, mirrors `tick()` in JS),
///  - `start`/`stop`/`restart` (timer wiring).
#[derive(Clone)]
pub struct Orchestrator {
    inner: Arc<OrchestratorInner>,
    /// Bumped on every restart, used to invalidate stale timer tasks.
    generation: Arc<AtomicU64>,
    /// Holds the JoinHandle of the currently running timer task (if any).
    handle: Arc<Mutex<Option<tauri::async_runtime::JoinHandle<()>>>>,
}

impl Orchestrator {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(OrchestratorInner::default()),
            generation: Arc::new(AtomicU64::new(0)),
            handle: Arc::new(Mutex::new(None)),
        }
    }

    /// Returns the action that the integration layer should perform for this tick,
    /// updating internal alternating state as a side effect.
    pub fn decide_tick_action(&self, cfg: &Value, quran_cfg: &Value) -> TickAction {
        let mode = AppMode::parse(
            cfg.get("appMode")
                .and_then(|v| v.as_str())
                .unwrap_or("sequential"),
        );
        match mode {
            AppMode::HadithOnly => TickAction::ShowHadith,
            AppMode::QuranOnly => TickAction::ShowQuran,
            AppMode::Both => TickAction::ShowBoth,
            AppMode::Alternating => {
                let last = self.inner.last_was_quran.load(Ordering::Relaxed);
                if !last {
                    self.inner.last_was_quran.store(true, Ordering::Relaxed);
                    TickAction::ShowQuran
                } else {
                    self.inner.last_was_quran.store(false, Ordering::Relaxed);
                    TickAction::ShowHadith
                }
            }
            AppMode::Sequential => {
                if is_quran_goal_met(quran_cfg) {
                    TickAction::HideQuranShowHadith
                } else {
                    TickAction::ShowQuran
                }
            }
        }
    }

    /// Schedule a recurring tick that calls `on_tick` every `interval_ms` until `stop` is called.
    pub fn start<F>(&self, store: ConfigStore, on_tick: F)
    where
        F: Fn(TickAction) + Send + Sync + 'static,
    {
        self.stop();
        let gen_now = self.generation.fetch_add(1, Ordering::SeqCst).wrapping_add(1);
        let me = self.clone();
        let on_tick = Arc::new(on_tick);
        let handle = tauri::async_runtime::spawn(async move {
            loop {
                let cfg_snapshot = store.cfg_get();
                let q_snapshot = store.quran_get();
                let interval_ms = get_active_interval_ms(&cfg_snapshot, &q_snapshot);
                let interval_ms = interval_ms.max(1000) as u64;
                let mut elapsed = 0u64;
                let chunk = 500u64;
                while elapsed < interval_ms {
                    if me.generation.load(Ordering::SeqCst) != gen_now {
                        log::debug!("Orchestrator tick cancelled (generation changed)");
                        return;
                    }
                    let sleep_duration = chunk.min(interval_ms - elapsed);
                    tokio::time::sleep(Duration::from_millis(sleep_duration)).await;
                    elapsed = elapsed.saturating_add(sleep_duration);
                }
                if me.generation.load(Ordering::SeqCst) != gen_now {
                    return;
                }
                let cfg = store.cfg_get();
                let q = store.quran_get();
                if is_paused(&q) {
                    // Skip this tick; the next iteration will re-evaluate.
                    continue;
                }
                let action = me.decide_tick_action(&cfg, &q);
                on_tick(action);
            }
        });
        *self.handle.lock() = Some(handle);
    }

    pub fn stop(&self) {
        // Bumping the generation invalidates the running task at its next check.
        self.generation.fetch_add(1, Ordering::SeqCst);
        if let Some(h) = self.handle.lock().take() {
            h.abort();
        }
    }

    /// Equivalent to JS `restart()` — re-arm the timer with the current interval.
    pub fn restart<F>(&self, store: ConfigStore, on_tick: F)
    where
        F: Fn(TickAction) + Send + Sync + 'static,
    {
        self.start(store, on_tick);
    }

    /// Trigger a single tick immediately (mirrors orchestrator.tick() in JS).
    pub fn tick_once(&self, cfg: &Value, quran_cfg: &Value) -> TickAction {
        self.decide_tick_action(cfg, quran_cfg)
    }
}

/// Returns true when the user has set a `pausedUntil` timestamp (ms since epoch)
/// in the quran cfg that is still in the future.
pub fn is_paused(quran_cfg: &Value) -> bool {
    let until = quran_cfg
        .get("pausedUntil")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);
    if until <= 0 {
        return false;
    }
    let now = chrono::Utc::now().timestamp_millis();
    now < until
}

impl Default for Orchestrator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn quran_goal_met_when_recent_readings_today_meets_goal() {
        let q = json!({"dailyGoal": 2, "recentReadings": [
            {"date": today_string_with_offset(0)},
            {"date": today_string_with_offset(0)},
            {"date": "1970-01-01"}
        ]});
        assert!(is_quran_goal_met(&q));
    }

    #[test]
    fn quran_goal_not_met_when_recent_below_goal() {
        let q = json!({"dailyGoal": 3, "recentReadings": [
            {"date": today_string_with_offset(0)}
        ]});
        assert!(!is_quran_goal_met(&q));
    }

    #[test]
    fn quran_only_uses_memorization_interval() {
        let cfg = json!({"appMode": "quranOnly", "interval": 33});
        let q = json!({"memorizationInterval": 11});
        assert_eq!(get_active_interval_ms(&cfg, &q), 11 * 60 * 1000);
    }

    #[test]
    fn hadith_only_uses_hadith_interval() {
        let cfg = json!({"appMode": "hadithOnly", "interval": 17});
        let q = json!({});
        assert_eq!(get_active_interval_ms(&cfg, &q), 17 * 60 * 1000);
    }

    #[test]
    fn sequential_uses_quran_interval_until_goal_met() {
        let cfg = json!({"appMode": "sequential", "interval": 30});
        let q = json!({"memorizationInterval": 10, "dailyGoal": 1, "recentReadings": []});
        assert_eq!(get_active_interval_ms(&cfg, &q), 10 * 60 * 1000);

        let q2 = json!({"memorizationInterval": 10, "dailyGoal": 1, "recentReadings": [
            {"date": today_string_with_offset(0)}
        ]});
        assert_eq!(get_active_interval_ms(&cfg, &q2), 30 * 60 * 1000);
    }

    #[test]
    fn alternating_alternates_between_quran_and_hadith() {
        let o = Orchestrator::new();
        let cfg = json!({"appMode": "alternating"});
        let q = json!({});
        assert_eq!(o.decide_tick_action(&cfg, &q), TickAction::ShowQuran);
        assert_eq!(o.decide_tick_action(&cfg, &q), TickAction::ShowHadith);
        assert_eq!(o.decide_tick_action(&cfg, &q), TickAction::ShowQuran);
    }

    #[test]
    fn sequential_shows_hadith_after_goal_met() {
        let o = Orchestrator::new();
        let cfg = json!({"appMode": "sequential"});
        let q = json!({"dailyGoal": 1, "recentReadings": [{"date": today_string_with_offset(0)}]});
        assert_eq!(
            o.decide_tick_action(&cfg, &q),
            TickAction::HideQuranShowHadith
        );
    }
}
