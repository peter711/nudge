mod macos;
mod store;
mod timer;

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use global_hotkey::hotkey::{Code, HotKey, Modifiers};
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, State};
use tauri_nspanel::ManagerExt;

use store::SavedPos;

const INTERVALS: [(u32, &str); 5] = [
    (0, "Off"),
    (15, "Every 15 minutes"),
    (30, "Every 30 minutes"),
    (45, "Every 45 minutes"),
    (60, "Every 60 minutes"),
];

pub struct Shared {
    pub dir: PathBuf,
    pub data: Mutex<store::Saved>,
    /// Bumped on every reschedule; stale timer tasks compare and bail out.
    pub generation: AtomicU64,
    pub visible: AtomicBool,
    pub prev_pid: Mutex<Option<i32>>,
    pub shortcut_ok: AtomicBool,
}

#[derive(serde::Serialize, Clone)]
struct UiState {
    task: String,
    interval_minutes: u32,
}

fn ui_state_of(shared: &Shared) -> UiState {
    let data = shared.data.lock().unwrap();
    UiState {
        task: data.task.clone(),
        interval_minutes: data.interval_minutes,
    }
}

fn shared_of(app: &AppHandle) -> Arc<Shared> {
    app.state::<Arc<Shared>>().inner().clone()
}

fn persist(shared: &Shared) {
    let data = shared.data.lock().unwrap().clone();
    store::save(&shared.dir, &data);
}

fn broadcast(app: &AppHandle, shared: &Shared) {
    let _ = app.emit("task-changed", ui_state_of(shared));
    rebuild_tray(app, shared);
}

// ---- window visibility ----

fn show_pill(app: &AppHandle, shared: &Shared) {
    if let Ok(panel) = app.get_webview_panel("main") {
        panel.order_front_regardless();
    }
    if !shared.visible.swap(true, Ordering::SeqCst) {
        let _ = app.emit("visibility-changed", true);
        rebuild_tray(app, shared);
    }
}

fn hide_pill(app: &AppHandle, shared: &Shared) {
    if let Ok(panel) = app.get_webview_panel("main") {
        panel.hide();
    }
    if shared.visible.swap(false, Ordering::SeqCst) {
        let _ = app.emit("visibility-changed", false);
        rebuild_tray(app, shared);
    }
}

fn toggle_pill(app: &AppHandle, shared: &Shared) {
    if shared.visible.load(Ordering::SeqCst) {
        hide_pill(app, shared);
    } else {
        show_pill(app, shared);
    }
}

// ---- edit mode / focus handoff ----

fn open_editor(app: &AppHandle) {
    let shared = shared_of(app);

    if let Some(pid) = macos::frontmost_pid() {
        if pid != std::process::id() as i32 {
            *shared.prev_pid.lock().unwrap() = Some(pid);
        }
    }

    show_pill(app, &shared);
    macos::activate_self();
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.set_focus();
    }
    let _ = app.emit("edit-requested", ());
}

// ---- tray ----

fn tray_preview(task: &str) -> String {
    if task.is_empty() {
        return "No focus set".into();
    }
    let mut chars = task.chars();
    let short: String = chars.by_ref().take(34).collect();
    if chars.next().is_some() {
        format!("Focus: {short}…")
    } else {
        format!("Focus: {short}")
    }
}

fn build_tray_menu(app: &AppHandle, shared: &Shared) -> tauri::Result<Menu<tauri::Wry>> {
    let (task, interval, visible, shortcut_ok) = {
        let data = shared.data.lock().unwrap();
        (
            data.task.clone(),
            data.interval_minutes,
            shared.visible.load(Ordering::SeqCst),
            shared.shortcut_ok.load(Ordering::SeqCst),
        )
    };

    let menu = Menu::new(app)?;

    let preview = MenuItem::with_id(app, "preview", tray_preview(&task), false, None::<&str>)?;
    menu.append(&preview)?;
    menu.append(&PredefinedMenuItem::separator(app)?)?;

    let change = MenuItem::with_id(app, "change", "Change task…", true, None::<&str>)?;
    menu.append(&change)?;

    let toggle = MenuItem::with_id(
        app,
        "toggle",
        if visible { "Hide pill" } else { "Show pill" },
        true,
        None::<&str>,
    )?;
    menu.append(&toggle)?;

    let interval_menu = Submenu::with_id(app, "interval", "Remind me", true)?;
    for (minutes, label) in INTERVALS {
        let item = CheckMenuItem::with_id(
            app,
            format!("interval:{minutes}"),
            label,
            true,
            interval == minutes,
            None::<&str>,
        )?;
        interval_menu.append(&item)?;
    }
    menu.append(&interval_menu)?;

    menu.append(&PredefinedMenuItem::separator(app)?)?;

    if !shortcut_ok {
        let unavailable =
            MenuItem::with_id(app, "shortcut", "Shortcut unavailable", false, None::<&str>)?;
        menu.append(&unavailable)?;
        menu.append(&PredefinedMenuItem::separator(app)?)?;
    }

    let quit = MenuItem::with_id(app, "quit", "Quit nudge", true, None::<&str>)?;
    menu.append(&quit)?;

    Ok(menu)
}

fn rebuild_tray(app: &AppHandle, shared: &Shared) {
    let Ok(menu) = build_tray_menu(app, shared) else {
        return;
    };
    if let Some(tray) = app.tray_by_id("main") {
        let _ = tray.set_menu(Some(menu));
    }
}

fn on_menu_event(app: &AppHandle, event: tauri::menu::MenuEvent) {
    let id: &str = event.id().as_ref();
    let shared = shared_of(app);

    match id {
        "change" => open_editor(app),
        "toggle" => toggle_pill(app, &shared),
        "quit" => app.exit(0),
        _ => {
            if let Some(minutes) = id.strip_prefix("interval:") {
                if let Ok(minutes) = minutes.parse::<u32>() {
                    apply_interval(app, &shared, minutes);
                }
            }
        }
    }
}

fn apply_interval(app: &AppHandle, shared: &Arc<Shared>, minutes: u32) {
    let minutes = store::valid_interval(minutes);
    {
        let mut data = shared.data.lock().unwrap();
        if data.interval_minutes == minutes {
            return;
        }
        data.interval_minutes = minutes;
        if !data.task.is_empty() && minutes > 0 {
            data.task_set_at_ms = store::now_ms();
        }
    }
    persist(shared);
    broadcast(app, shared);
    timer::reschedule(app, shared);
}

fn setup_tray(app: &AppHandle, shared: &Shared) -> tauri::Result<()> {
    let menu = build_tray_menu(app, shared)?;
    let icon = tauri::image::Image::from_bytes(include_bytes!("../icons/tray.png"))?;

    TrayIconBuilder::with_id("main")
        .icon(icon)
        .icon_as_template(true)
        .tooltip("nudge")
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(on_menu_event)
        .build(app)?;

    Ok(())
}

// ---- global shortcut ----

struct HotKeyGuard(#[allow(dead_code)] GlobalHotKeyManager);

fn setup_shortcut(app: &AppHandle, shared: Arc<Shared>) -> tauri::Result<()> {
    let manager = match GlobalHotKeyManager::new() {
        Ok(manager) => manager,
        Err(err) => {
            eprintln!("nudge: global shortcut manager unavailable: {err}");
            shared.shortcut_ok.store(false, Ordering::SeqCst);
            return Ok(());
        }
    };

    let hotkey = HotKey::new(Some(Modifiers::META | Modifiers::SHIFT), Code::Space);
    match manager.register(hotkey) {
        Ok(()) => shared.shortcut_ok.store(true, Ordering::SeqCst),
        Err(err) => {
            eprintln!("nudge: Cmd+Shift+Space unavailable: {err}");
            shared.shortcut_ok.store(false, Ordering::SeqCst);
        }
    }

    app.manage(HotKeyGuard(manager));

    let handle = app.clone();
    GlobalHotKeyEvent::set_event_handler(Some(move |event: GlobalHotKeyEvent| {
        if event.state == HotKeyState::Pressed {
            let app = handle.clone();
            let _ = app.clone().run_on_main_thread(move || open_editor(&app));
        }
    }));

    Ok(())
}

// ---- initial window placement ----

fn initial_position(app: &AppHandle, saved: &Option<SavedPos>) -> PhysicalPosition<i32> {
    let primary = app.primary_monitor().ok().flatten();
    let scale = primary.as_ref().map(|m| m.scale_factor()).unwrap_or(2.0);
    let pill_w = (240.0 * scale) as i32;
    let pill_h = (30.0 * scale) as i32;

    if let Some(pos) = saved {
        if let Ok(monitors) = app.available_monitors() {
            if let Some(monitor) = monitors.iter().find(|m| m.name() == pos.monitor.as_ref()) {
                let area = monitor.work_area();
                let min_x = area.position.x;
                let min_y = area.position.y;
                let max_x = (area.position.x + area.size.width as i32 - pill_w).max(min_x);
                let max_y = (area.position.y + area.size.height as i32 - pill_h).max(min_y);
                return PhysicalPosition::new(
                    (area.position.x + pos.dx).clamp(min_x, max_x),
                    (area.position.y + pos.dy).clamp(min_y, max_y),
                );
            }
        }
    }

    if let Some(monitor) = primary {
        let area = monitor.work_area();
        let max_x = (area.position.x + area.size.width as i32 - pill_w).max(area.position.x);
        let x =
            (area.position.x + (area.size.width as i32 - pill_w) / 2).clamp(area.position.x, max_x);
        let y = area.position.y + (36.0 * scale) as i32;
        return PhysicalPosition::new(x, y);
    }

    PhysicalPosition::new(400, 80)
}

// ---- commands ----

#[tauri::command]
fn ui_state(state: State<'_, Arc<Shared>>) -> UiState {
    ui_state_of(&state)
}

#[tauri::command]
fn set_task(app: AppHandle, state: State<'_, Arc<Shared>>, task: String) {
    let shared = state.inner().clone();
    let task = store::sanitize(&task);

    {
        let mut data = shared.data.lock().unwrap();
        if data.task == task {
            return;
        }
        data.task = task;
        data.task_set_at_ms = store::now_ms();
    }

    persist(&shared);
    broadcast(&app, &shared);
    timer::reschedule(&app, &shared);
}

#[tauri::command]
fn begin_edit(app: AppHandle) {
    open_editor(&app);
}

#[tauri::command]
fn end_edit(state: State<'_, Arc<Shared>>) {
    let pid = state.prev_pid.lock().unwrap().take();
    match pid {
        Some(pid) => macos::activate_process(pid),
        None => macos::deactivate_self(),
    }
}

// ---- app ----

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_nspanel::init())
        .plugin(tauri_plugin_notification::init())
        .invoke_handler(tauri::generate_handler![
            ui_state, set_task, begin_edit, end_edit
        ])
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::Moved(_) = event {
                let app = window.app_handle().clone();
                tauri::async_runtime::spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                    let shared = shared_of(&app);
                    let Some(window) = app.get_webview_window("main") else {
                        return;
                    };
                    let (Ok(position), Ok(Some(monitor))) =
                        (window.outer_position(), window.current_monitor())
                    else {
                        return;
                    };
                    {
                        let mut data = shared.data.lock().unwrap();
                        data.pos = Some(SavedPos {
                            monitor: monitor.name().cloned(),
                            dx: position.x - monitor.position().x,
                            dy: position.y - monitor.position().y,
                        });
                    }
                    persist(&shared);
                });
            }
        })
        .setup(|app| {
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let handle = app.handle().clone();
            let dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&dir)?;
            let saved = store::load(&dir);
            let position = initial_position(&handle, &saved.pos);

            let shared = Arc::new(Shared {
                dir,
                data: Mutex::new(saved),
                generation: AtomicU64::new(0),
                visible: AtomicBool::new(true),
                prev_pid: Mutex::new(None),
                shortcut_ok: AtomicBool::new(true),
            });
            app.manage(shared.clone());

            macos::install_pill(&handle, position)?;
            setup_shortcut(&handle, shared.clone())?;
            setup_tray(&handle, &shared)?;
            timer::reschedule(&handle, &shared);

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running nudge");
}
