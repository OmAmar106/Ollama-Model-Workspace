pub mod commands;
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub mod db;
pub mod generation_manager;
pub mod ollama;
pub mod session_cache;

use rusqlite::Connection;
use std::{sync::Mutex, time::Duration};
use tauri::Manager;

pub const SESSION_RETENTION_DAYS: i64 = 30;
const SESSION_CLEANUP_INTERVAL: Duration = Duration::from_secs(24 * 60 * 60);

pub struct AppDatabase(pub Mutex<Connection>);
pub struct AppSessionCache(pub session_cache::SessionCache);
pub struct AppGenerationManager(pub generation_manager::GenerationManager);
pub struct AppOllamaClient(pub ollama::OllamaClient);

fn prune_old_sessions(database: &AppDatabase, cache: &AppSessionCache) -> Result<usize, String> {
    let deleted_ids = {
        let connection = database
            .0
            .lock()
            .map_err(|error| format!("Database lock failed: {error}"))?;
        db::sessions::delete_sessions_older_than(&connection, SESSION_RETENTION_DAYS)
            .map_err(|error| format!("Failed to delete old sessions: {error}"))?
    };

    for session_id in &deleted_ids {
        cache.0.remove(session_id);
    }

    Ok(deleted_ids.len())
}

pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::create_session,
            commands::get_session,
            commands::list_sessions,
            commands::update_session,
            commands::delete_session,
            commands::get_session_messages,
            commands::get_generation,
            commands::check_ollama_health,
            commands::list_ollama_models,
            commands::generate_chat,
            commands::cancel_generation,
            commands::cancel_session,
            commands::create_branch_session
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

            // Phase 5: Startup recovery of running generations
            match db::generations::recover_interrupted_generations(&connection) {
                Ok(count) if count > 0 => {
                    log::warn!("Recovered {count} interrupted generation(s) from previous run");
                }
                Ok(_) => {}
                Err(err) => {
                    log::error!("Failed to run startup generation recovery: {err}");
                }
            }

            let generation_manager = generation_manager::GenerationManager::new();
            let database = AppDatabase(Mutex::new(connection));
            let cache = AppSessionCache(session_cache::SessionCache::new());

            match prune_old_sessions(&database, &cache) {
                Ok(count) if count > 0 => {
                    log::info!(
                        "Deleted {count} session(s) older than {SESSION_RETENTION_DAYS} days"
                    );
                }
                Ok(_) => {}
                Err(err) => {
                    log::error!("Failed to prune old sessions: {err}");
                }
            }

            app.manage(database);
            app.manage(cache);
            app.manage(AppGenerationManager(generation_manager));
            app.manage(AppOllamaClient(ollama::OllamaClient::new()));

            log::info!("SQLite database initialized at {}", database_path.display());

            let cleanup_handle = app.handle().clone();
            std::thread::spawn(move || loop {
                std::thread::sleep(SESSION_CLEANUP_INTERVAL);

                let Some(database) = cleanup_handle.try_state::<AppDatabase>() else {
                    continue;
                };
                let Some(cache) = cleanup_handle.try_state::<AppSessionCache>() else {
                    continue;
                };

                match prune_old_sessions(&database, &cache) {
                    Ok(count) if count > 0 => {
                        log::info!(
                            "Deleted {count} session(s) older than {SESSION_RETENTION_DAYS} days"
                        );
                    }
                    Ok(_) => {}
                    Err(err) => {
                        log::error!("Failed to prune old sessions: {err}");
                    }
                }
            });

            Ok(())
        })
        .on_window_event(|window, event| {
            // Phase 5: Clean up on application shutdown
            if let tauri::WindowEvent::CloseRequested { .. } | tauri::WindowEvent::Destroyed = event
            {
                if let Some(gen_manager) = window.try_state::<AppGenerationManager>() {
                    let cancelled = gen_manager.0.cancel_all();
                    if cancelled > 0 {
                        log::info!("Cancelled {cancelled} active generation(s) on window shutdown");
                    }
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while building tauri application");
}
