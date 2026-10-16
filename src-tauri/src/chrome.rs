// SPDX-License-Identifier: GPL-3.0-or-later
//! Native window chrome: the translucent Windows 11 title bar (Mica) and the
//! macOS menu bar. The title bar itself is drawn by the web layer (see
//! `tauri.windows.conf.json` / `tauri.macos.conf.json`); this module only
//! applies the parts the webview cannot.

use std::sync::atomic::{AtomicBool, Ordering};

use serde::Serialize;
use tauri::menu::{Menu, MenuItem, SubmenuBuilder};
#[cfg(target_os = "macos")]
use tauri::Emitter;
use tauri::{AppHandle, Runtime, State};

/// Whether the system backdrop (Mica) is active, so the web layer knows to
/// leave its toolbar see-through instead of painting an opaque one.
#[derive(Default)]
pub struct ChromeState {
    mica: AtomicBool,
}

#[derive(Serialize)]
pub struct ChromeInfo {
    mica: bool,
}

#[tauri::command]
pub fn chrome_info(state: State<ChromeState>) -> ChromeInfo {
    ChromeInfo {
        mica: state.mica.load(Ordering::Relaxed),
    }
}

/// The Event name the web layer listens on for native-menu clicks.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub const MENU_EVENT: &str = "menu://action";

/// File / Edit / Window menus with Amber's own commands. Menu ids are the
/// action names the web layer understands.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub fn build_menu<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Menu<R>> {
    let open = MenuItem::with_id(app, "open", "Open Archive…", true, Some("CmdOrCtrl+O"))?;
    let import = MenuItem::with_id(
        app,
        "import",
        "Import from iPhone…",
        true,
        Some("CmdOrCtrl+I"),
    )?;
    let export = MenuItem::with_id(
        app,
        "export",
        "Export Conversation…",
        true,
        Some("CmdOrCtrl+E"),
    )?;
    let find = MenuItem::with_id(
        app,
        "find",
        "Find in Conversation",
        true,
        Some("CmdOrCtrl+F"),
    )?;

    let app_menu = SubmenuBuilder::new(app, "Amber")
        .about(None)
        .separator()
        .services()
        .separator()
        .hide()
        .hide_others()
        .show_all()
        .separator()
        .quit()
        .build()?;
    let file = SubmenuBuilder::new(app, "File")
        .item(&open)
        .item(&import)
        .separator()
        .item(&export)
        .separator()
        .close_window()
        .build()?;
    let edit = SubmenuBuilder::new(app, "Edit")
        .undo()
        .redo()
        .separator()
        .cut()
        .copy()
        .paste()
        .select_all()
        .separator()
        .item(&find)
        .build()?;
    let window = SubmenuBuilder::new(app, "Window")
        .minimize()
        .maximize()
        .fullscreen()
        .build()?;

    Menu::with_items(app, &[&app_menu, &file, &edit, &window])
}

pub fn setup(app: &mut tauri::App) -> tauri::Result<()> {
    #[cfg(windows)]
    {
        use tauri::window::{Effect, EffectsBuilder};
        use tauri::Manager;
        if let Some(window) = app.get_webview_window("main") {
            // Fails on Windows 10, where the toolbar just stays opaque.
            let applied = window
                .set_effects(EffectsBuilder::new().effect(Effect::Mica).build())
                .is_ok();
            app.state::<ChromeState>()
                .mica
                .store(applied, Ordering::Relaxed);
        }
    }

    #[cfg(target_os = "macos")]
    {
        app.set_menu(build_menu(app.handle())?)?;
        app.on_menu_event(|app, event| {
            let _ = app.emit(MENU_EVENT, event.id().as_ref());
        });
    }

    #[cfg(not(any(windows, target_os = "macos")))]
    let _ = app;
    Ok(())
}
