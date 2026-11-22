use tauri::{AppHandle, LogicalSize, Manager, PhysicalPosition, Position, Size, WebviewUrl};
use tauri_nspanel::{CollectionBehavior, PanelBuilder, PanelLevel, StyleMask, tauri_panel};

use objc2_app_kit::{NSApplication, NSRunningApplication, NSWorkspace};

tauri_panel! {
    panel!(FocusPill {
        config: {
            can_become_key_window: true,
            can_become_main_window: false,
            becomes_key_only_if_needed: true,
            is_floating_panel: true
        }
    })
}

pub fn install_pill(app: &AppHandle, position: PhysicalPosition<i32>) -> tauri::Result<()> {
    let panel = PanelBuilder::<_, FocusPill>::new(app, "main")
        .url(WebviewUrl::App("index.html".into()))
        .title("nudge")
        .size(Size::Logical(LogicalSize::new(240.0, 30.0)))
        .level(PanelLevel::Floating)
        .collection_behavior(
            CollectionBehavior::new()
                .can_join_all_spaces()
                .full_screen_auxiliary()
                .ignores_cycle(),
        )
        .has_shadow(false)
        .hides_on_deactivate(false)
        .no_activate(true)
        .transparent(true)
        .corner_radius(15.0)
        .add_style_mask(StyleMask::empty().nonactivating_panel())
        .with_window(|window| {
            window
                .decorations(false)
                .resizable(false)
                .always_on_top(true)
                .visible_on_all_workspaces(true)
                .focused(false)
                .visible(false)
                .skip_taskbar(true)
        })
        .build()?;

    if let Some(window) = app.get_webview_window("main") {
        let _ = window.set_position(Position::Physical(position));
        let _ = window_vibrancy::apply_vibrancy(
            &window,
            window_vibrancy::NSVisualEffectMaterial::HudWindow,
            Some(window_vibrancy::NSVisualEffectState::Active),
            Some(15.0),
        );
    }

    panel.order_front_regardless();
    Ok(())
}

/// PID of the app that was frontmost before nudge took over.
pub fn frontmost_pid() -> Option<i32> {
    NSWorkspace::sharedWorkspace()
        .frontmostApplication()
        .map(|app| app.processIdentifier())
}

/// Bring nudge to the front so the input can take keystrokes.
pub fn activate_self() {
    if let Some(mtm) = MainThreadMarker::new() {
        NSApplication::sharedApplication(mtm).activate();
    }
}

pub fn deactivate_self() {
    if let Some(mtm) = MainThreadMarker::new() {
        NSApplication::sharedApplication(mtm).deactivate();
    }
}

pub fn activate_process(pid: i32) {
    if let Some(app) = NSRunningApplication::runningApplicationWithProcessIdentifier(pid) {
        let _ = app.activateWithOptions(objc2_app_kit::NSApplicationActivationOptions::empty());
    }
}
