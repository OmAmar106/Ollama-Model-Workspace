mod commands;
#[cfg_attr(mobile, tauri::mobile_entry_point)]
mod db;
mod ollama;

use rusqlite::Connection;
use std::sync::Mutex;
use tauri::Manager;

pub struct AppDatabase(pub Mutex<Connection>);

pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::create_session,
            commands::get_session,
            commands::generate_chat
        ])
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            let database_file = if cfg!(debug_assertions) {
                "ollama_workspace_dev.db"
            } else {
                "ollama_workspace.db"
            };

            let database_path = app.path().app_data_dir()?.join(database_file);
            let connection = db::initialize_database(&database_path)?;

            app.manage(AppDatabase(Mutex::new(connection)));

            log::info!("SQLite database initialized at {}", database_path.display());

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while building tauri application");
}
