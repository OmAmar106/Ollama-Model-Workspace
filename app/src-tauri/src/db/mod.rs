use rusqlite::{Connection, Result};
use std::fs;
use std::path::Path;
pub mod generations;
pub mod messages;
pub mod sessions;
pub fn initialize_database(path: &Path) -> Result<Connection> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
    }

    let connection = Connection::open(path)?;
    connection.execute_batch(
        "
        PRAGMA foreign_keys = ON;

        CREATE TABLE IF NOT EXISTS sessions (
            id TEXT PRIMARY KEY,
            title TEXT,
            model_name TEXT NOT NULL,
            think INTEGER CHECK (think IN (0, 1) OR think IS NULL),
            system_prompt TEXT,
            options_json TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS messages (
            id TEXT PRIMARY KEY,
            session_id TEXT NOT NULL,
            role TEXT NOT NULL CHECK (role IN ('user', 'assistant', 'system')),
            content TEXT NOT NULL,
            thinking_content TEXT,
            status TEXT NOT NULL CHECK (
                status IN ('completed', 'generating', 'cancelled', 'failed', 'interrupted')
            ),
            created_at TEXT NOT NULL,
            completed_at TEXT,
            FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS generations (
            id TEXT PRIMARY KEY,
            message_id TEXT NOT NULL,
            model_name TEXT NOT NULL,
            status TEXT NOT NULL CHECK (
                status IN ('running', 'completed', 'cancelled', 'failed', 'interrupted')
            ),
            started_at TEXT NOT NULL,
            finished_at TEXT,
            time_to_first_token_ms INTEGER,
            duration_ms INTEGER,
            prompt_tokens INTEGER,
            completion_tokens INTEGER,
            tokens_per_second REAL,
            error_code TEXT,
            error_message TEXT,
            FOREIGN KEY (message_id) REFERENCES messages(id) ON DELETE CASCADE
        );

        CREATE INDEX IF NOT EXISTS idx_messages_session_created
            ON messages(session_id, created_at, id);

        CREATE INDEX IF NOT EXISTS idx_generations_message
            ON generations(message_id);

        CREATE INDEX IF NOT EXISTS idx_sessions_updated
            ON sessions(updated_at DESC);
        ",
    )?;

    Ok(connection)
}
