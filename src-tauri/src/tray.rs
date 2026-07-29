use crate::config::ConfigStore;
use crate::data_loader::DataLoader;
use parking_lot::Mutex;
use std::sync::Arc;
use tauri::{
    menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager,
};

const ID_SHOW_QURAN: &str = "tray.show_quran";
const ID_HIDE_QURAN: &str = "tray.hide_quran";
const ID_SHOW_HADITH: &str = "tray.show_hadith";
const ID_NEXT_HADITH: &str = "tray.next_hadith";
const ID_PREV_HADITH: &str = "tray.prev_hadith";
const ID_OPEN_SETTINGS: &str = "tray.open_settings";
const ID_QUIT: &str = "tray.quit";

/// State carried through the menu/tray event handler. We use the default
/// (Wry) runtime here because Tauri stores trays per-runtime.
#[derive(Clone)]
pub struct TrayState {
    pub tray_icon: Arc<Mutex<Option<TrayIcon<tauri::Wry>>>>,
}

impl TrayState {
    pub fn new() -> Self {
        Self {
            tray_icon: Arc::new(Mutex::new(None)),
        }
    }
}

impl Default for TrayState {
    fn default() -> Self {
        Self::new()
    }
}

fn build_menu(
    app: &AppHandle,
    store: &ConfigStore,
    data: &DataLoader,
) -> tauri::Result<Menu<tauri::Wry>> {
    let cfg = store.cfg_get();
    let total = data.hadiths_len().max(1);
    let idx = cfg.get("index").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
    let idx_display = (idx + 1).min(total);

    let header = MenuItem::with_id(
        app,
        "tray.header",
        "زاد المسلم - رفيقك اليومي",
        false,
        None::<&str>,
    )?;
    let sep1 = PredefinedMenuItem::separator(app)?;
    let q_label = MenuItem::with_id(
        app,
        "tray.q_label",
        "القرآن الكريم",
        false,
        None::<&str>,
    )?;
    let q_show = MenuItem::with_id(
        app,
        ID_SHOW_QURAN,
        "عرض نافذة القرآن",
        true,
        None::<&str>,
    )?;
    let q_hide = MenuItem::with_id(
        app,
        ID_HIDE_QURAN,
        "إخفاء نافذة القرآن",
        true,
        None::<&str>,
    )?;
    let sep2 = PredefinedMenuItem::separator(app)?;
    let h_label = MenuItem::with_id(
        app,
        "tray.h_label",
        format!("الأحاديث - الحديث {} / {}", idx_display, total),
        false,
        None::<&str>,
    )?;
    let h_show = MenuItem::with_id(
        app,
        ID_SHOW_HADITH,
        "عرض الحديث الآن",
        true,
        None::<&str>,
    )?;
    let h_next = MenuItem::with_id(app, ID_NEXT_HADITH, "الحديث التالي", true, None::<&str>)?;
    let h_prev = MenuItem::with_id(app, ID_PREV_HADITH, "الحديث السابق", true, None::<&str>)?;
    let sep3 = PredefinedMenuItem::separator(app)?;
    let settings = MenuItem::with_id(app, ID_OPEN_SETTINGS, "الإعدادات", true, None::<&str>)?;
    let sep4 = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, ID_QUIT, "إغلاق", true, None::<&str>)?;

    Menu::with_items(
        app,
        &[
            &header, &sep1, &q_label, &q_show, &q_hide, &sep2, &h_label, &h_show, &h_next,
            &h_prev, &sep3, &settings, &sep4, &quit,
        ],
    )
}

/// Create the tray icon and install it into the app.
#[allow(clippy::too_many_arguments)]
pub fn create<F1, F2, F3, F4, F5, F6, F7>(
    app: &AppHandle,
    store: ConfigStore,
    data: DataLoader,
    state: TrayState,
    on_show_quran: F1,
    on_hide_quran: F2,
    on_show_hadith: F3,
    on_next: F4,
    on_prev: F5,
    on_settings: F6,
    on_left_click: F7,
) -> tauri::Result<()>
where
    F1: Fn(AppHandle) + Send + Sync + 'static,
    F2: Fn(AppHandle) + Send + Sync + 'static,
    F3: Fn(AppHandle) + Send + Sync + 'static,
    F4: Fn(AppHandle) + Send + Sync + 'static,
    F5: Fn(AppHandle) + Send + Sync + 'static,
    F6: Fn(AppHandle) + Send + Sync + 'static,
    F7: Fn(AppHandle) + Send + Sync + 'static,
{
    log::debug!("Creating tray menu");
    let menu = match build_menu(app, &store, &data) {
        Ok(m) => {
            log::debug!("Tray menu built successfully");
            m
        }
        Err(e) => {
            log::error!("Failed to build tray menu: {}", e);
            return Err(e);
        }
    };

    let on_show_quran = Arc::new(on_show_quran);
    let on_hide_quran = Arc::new(on_hide_quran);
    let on_show_hadith = Arc::new(on_show_hadith);
    let on_next = Arc::new(on_next);
    let on_prev = Arc::new(on_prev);
    let on_settings = Arc::new(on_settings);
    let on_left_click = Arc::new(on_left_click);

    let mh_show_quran = on_show_quran.clone();
    let mh_hide_quran = on_hide_quran.clone();
    let mh_show_hadith = on_show_hadith.clone();
    let mh_next = on_next.clone();
    let mh_prev = on_prev.clone();
    let mh_settings = on_settings.clone();

    let lc = on_left_click.clone();

    let icon = match app.default_window_icon() {
        Some(i) => i.clone(),
        None => {
            log::error!("No default window icon available for tray");
            return Err(tauri::Error::Anyhow(anyhow::anyhow!("default window icon missing")));
        }
    };

    log::debug!("Building tray icon with icon data length: {:?}", icon.rgba().len());

    let tray = TrayIconBuilder::with_id("zad-tray")
        .icon(icon)
        .tooltip("زاد المسلم")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(move |app, event: MenuEvent| {
            log::debug!("Tray menu event: {}", event.id.as_ref());
            match event.id.as_ref() {
                ID_SHOW_QURAN => {
                    log::info!("Tray: Show Quran requested");
                    mh_show_quran(app.clone());
                }
                ID_HIDE_QURAN => {
                    log::info!("Tray: Hide Quran requested");
                    mh_hide_quran(app.clone());
                }
                ID_SHOW_HADITH => {
                    log::info!("Tray: Show Hadith requested");
                    mh_show_hadith(app.clone());
                }
                ID_NEXT_HADITH => {
                    log::info!("Tray: Next hadith requested");
                    mh_next(app.clone());
                }
                ID_PREV_HADITH => {
                    log::info!("Tray: Previous hadith requested");
                    mh_prev(app.clone());
                }
                ID_OPEN_SETTINGS => {
                    log::info!("Tray: Open settings requested");
                    mh_settings(app.clone());
                }
                ID_QUIT => {
                    log::info!("Tray: Quit requested");
                    app.exit(0);
                }
                _ => {}
            }
        })
        .on_tray_icon_event(move |tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                log::debug!("Tray icon left-clicked");
                lc(tray.app_handle().clone());
            }
        })
        .build(app);

    match tray {
        Ok(tray) => {
            log::info!("Tray icon created successfully");
            *state.tray_icon.lock() = Some(tray);
            Ok(())
        }
        Err(e) => {
            log::error!("Failed to create tray icon: {}", e);
            Err(e)
        }
    }
}

/// Refresh the tray menu (mirrors `tray.refresh()` in JS — rebuilds the menu
/// so the current hadith index is reflected).
pub fn refresh(app: &AppHandle, store: &ConfigStore, data: &DataLoader) {
    if let Some(state) = app.try_state::<TrayState>() {
        if let Some(tray) = state.tray_icon.lock().as_ref() {
            match build_menu(app, store, data) {
                Ok(menu) => {
                    if let Err(e) = tray.set_menu(Some(menu)) {
                        log::error!("Failed to refresh tray menu: {}", e);
                    } else {
                        log::debug!("Tray menu refreshed");
                    }
                }
                Err(e) => {
                    log::error!("Failed to build tray menu for refresh: {}", e);
                }
            }
        }
    }
}
