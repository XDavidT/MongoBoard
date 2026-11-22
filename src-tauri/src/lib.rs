mod commands;
mod error;
mod mongo;
mod profiles;
mod secrets;
mod shell_json;
mod tree;

use tauri::menu::{Menu, MenuItemBuilder, PredefinedMenuItem, Submenu};
use tauri::{Emitter, Manager};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let dir = app.path().app_config_dir()?;
            let path = dir.join("connections.json");
            profiles::migrate_legacy_file(&path);
            app.manage(profiles::ProfileStore::load(path));
            app.manage(mongo::Sessions::default());
            app.set_menu(build_menu(app.handle())?)?;
            Ok(())
        })
        .on_menu_event(|app, event| {
            let app = app.clone();
            match event.id().as_ref() {
                "connection-manager" => {
                    tauri::async_runtime::spawn(async move {
                        let _ = commands::open_connection_manager(app).await;
                    });
                }
                "new-connection" => {
                    tauri::async_runtime::spawn(async move {
                        let _ = commands::open_connection_editor(app, None).await;
                    });
                }
                id => {
                    // Other menu actions (run query, add document …) go to the focused window only.
                    let focused = app
                        .webview_windows()
                        .into_values()
                        .find(|w| w.is_focused().unwrap_or(false));
                    if let Some(window) = focused {
                        let _ = app.emit_to(window.label(), "menu", id.to_string());
                    }
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_profiles,
            commands::get_profile,
            commands::save_profile,
            commands::duplicate_profile,
            commands::delete_profile,
            commands::test_connection,
            commands::parse_connection_string,
            commands::open_connection_manager,
            commands::open_connection_editor,
            commands::open_connection,
            commands::list_open_connections,
            commands::close_connection,
            commands::reconnect,
            commands::list_databases,
            commands::list_collections,
            commands::find,
            commands::count_documents,
            commands::format_document,
            commands::insert_document,
            commands::replace_document,
            commands::delete_document,
        ])
        .run(tauri::generate_context!())
        .expect("error while running MongoBoard");
}

fn build_menu(app: &tauri::AppHandle) -> tauri::Result<Menu<tauri::Wry>> {
    let manager = MenuItemBuilder::with_id("connection-manager", "Connection Manager…")
        .accelerator("CmdOrCtrl+Shift+C")
        .build(app)?;
    let new_conn = MenuItemBuilder::with_id("new-connection", "New Connection…")
        .accelerator("CmdOrCtrl+N")
        .build(app)?;
    let run_query = MenuItemBuilder::with_id("run-query", "Run Query")
        .accelerator("CmdOrCtrl+Enter")
        .build(app)?;
    let add_doc = MenuItemBuilder::with_id("add-document", "Add Document…")
        .accelerator("CmdOrCtrl+Shift+N")
        .build(app)?;
    let edit_doc = MenuItemBuilder::with_id("edit-document", "Edit Document…")
        .accelerator("CmdOrCtrl+E")
        .build(app)?;
    let refresh = MenuItemBuilder::with_id("refresh", "Refresh")
        .accelerator("CmdOrCtrl+R")
        .build(app)?;

    let app_menu = Submenu::with_items(
        app,
        "MongoBoard",
        true,
        &[
            &PredefinedMenuItem::about(app, None, None)?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::hide(app, None)?,
            &PredefinedMenuItem::hide_others(app, None)?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::quit(app, None)?,
        ],
    )?;
    let file = Submenu::with_items(
        app,
        "File",
        true,
        &[
            &new_conn,
            &manager,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::close_window(app, None)?,
        ],
    )?;
    let edit = Submenu::with_items(
        app,
        "Edit",
        true,
        &[
            &PredefinedMenuItem::undo(app, None)?,
            &PredefinedMenuItem::redo(app, None)?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::cut(app, None)?,
            &PredefinedMenuItem::copy(app, None)?,
            &PredefinedMenuItem::paste(app, None)?,
            &PredefinedMenuItem::select_all(app, None)?,
        ],
    )?;
    let data = Submenu::with_items(
        app,
        "Data",
        true,
        &[
            &run_query,
            &refresh,
            &PredefinedMenuItem::separator(app)?,
            &add_doc,
            &edit_doc,
        ],
    )?;
    let window = Submenu::with_items(
        app,
        "Window",
        true,
        &[
            &PredefinedMenuItem::minimize(app, None)?,
            &PredefinedMenuItem::fullscreen(app, None)?,
        ],
    )?;
    Menu::with_items(app, &[&app_menu, &file, &edit, &data, &window])
}
