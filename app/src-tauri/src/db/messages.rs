use chrono::Utc;
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Message {
    pub id: String,
    pub session_id: String,
    pub role: String,
    pub content: String,
    pub thinking_content: Option<String>,
    pub status: String,
    pub created_at: String,
    pub completed_at: Option<String>,
}

/// Insert a message into an existing session.
pub fn create_message(
    conn: &Connection,
    session_id: &str,
    role: &str,
    content: &str,
    thinking_content: Option<&str>,
    status: &str,
) -> rusqlite::Result<Message> {
    let id = Uuid::new_v4().to_string();
    let created_at = Utc::now().to_rfc3339();

    conn.execute(
        "INSERT INTO messages (
            id, session_id, role, content, thinking_content,
            status, created_at, completed_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, NULL)",
        params![
            id,
            session_id,
            role,
            content,
            thinking_content,
            status,
            created_at,
        ],
    )?;

    Ok(Message {
        id,
        session_id: session_id.to_owned(),
        role: role.to_owned(),
        content: content.to_owned(),
        thinking_content: thinking_content.map(str::to_owned),
        status: status.to_owned(),
        created_at,
        completed_at: None,
    })
}

/// Retrieve a single message by its ID.
pub fn get_message(conn: &Connection, message_id: &str) -> rusqlite::Result<Option<Message>> {
    conn.query_row(
        "SELECT id, session_id, role, content, thinking_content,
                status, created_at, completed_at
         FROM messages
         WHERE id = ?1",
        params![message_id],
        |row| {
            Ok(Message {
                id: row.get(0)?,
                session_id: row.get(1)?,
                role: row.get(2)?,
                content: row.get(3)?,
                thinking_content: row.get(4)?,
                status: row.get(5)?,
                created_at: row.get(6)?,
                completed_at: row.get(7)?,
            })
        },
    )
    .optional()
}

/// Retrieve a session's messages in chronological order.
pub fn get_session_messages(conn: &Connection, session_id: &str) -> rusqlite::Result<Vec<Message>> {
    let mut stmt = conn.prepare(
        "SELECT id, session_id, role, content, thinking_content,
                status, created_at, completed_at
         FROM messages
         WHERE session_id = ?1
         ORDER BY created_at ASC, id ASC",
    )?;

    let rows = stmt.query_map(params![session_id], |row| {
        Ok(Message {
            id: row.get(0)?,
            session_id: row.get(1)?,
            role: row.get(2)?,
            content: row.get(3)?,
            thinking_content: row.get(4)?,
            status: row.get(5)?,
            created_at: row.get(6)?,
            completed_at: row.get(7)?,
        })
    })?;

    rows.collect()
}

/// Update a message after generation finishes or fails.
pub fn finish_message(
    conn: &Connection,
    message_id: &str,
    content: &str,
    thinking_content: Option<&str>,
    status: &str,
) -> rusqlite::Result<bool> {
    let completed_at = Utc::now().to_rfc3339();

    let updated = conn.execute(
        "UPDATE messages
         SET content = ?1,
             thinking_content = ?2,
             status = ?3,
             completed_at = ?4
         WHERE id = ?5",
        params![content, thinking_content, status, completed_at, message_id,],
    )?;

    Ok(updated == 1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::initialize_database;
    use std::path::Path;
    use tempfile::tempdir;

    fn setup_database() -> (tempfile::TempDir, Connection, String) {
        let dir = tempdir().expect("failed to create temporary directory");
        let db_path = dir.path().join("test.db");
        let conn =
            initialize_database(Path::new(&db_path)).expect("failed to initialize test database");

        let session_id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();

        conn.execute(
            "INSERT INTO sessions (
                id, title, model_name, think, system_prompt,
                options_json, created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                session_id,
                "Test session",
                "test-model",
                Option::<i32>::None,
                Option::<String>::None,
                Option::<String>::None,
                now,
                now,
            ],
        )
        .expect("failed to insert test session");

        (dir, conn, session_id)
    }

    #[test]
    fn creates_and_retrieves_message() {
        let (_dir, conn, session_id) = setup_database();

        let created = create_message(
            &conn,
            &session_id,
            "user",
            "Hello, Ollama!",
            None,
            "completed",
        )
        .expect("failed to create message");

        let retrieved = get_message(&conn, &created.id)
            .expect("failed to retrieve message")
            .expect("message should exist");

        assert_eq!(retrieved.id, created.id);
        assert_eq!(retrieved.session_id, session_id);
        assert_eq!(retrieved.role, "user");
        assert_eq!(retrieved.content, "Hello, Ollama!");
        assert_eq!(retrieved.status, "completed");
        assert_eq!(retrieved.thinking_content, None);
        assert_eq!(retrieved.completed_at, None);
    }

    #[test]
    fn retrieves_session_history() {
        let (_dir, conn, session_id) = setup_database();

        create_message(
            &conn,
            &session_id,
            "user",
            "What is Rust?",
            None,
            "completed",
        )
        .expect("failed to create user message");

        create_message(
            &conn,
            &session_id,
            "assistant",
            "Rust is a programming language.",
            Some("I should explain Rust concisely."),
            "completed",
        )
        .expect("failed to create assistant message");

        let messages =
            get_session_messages(&conn, &session_id).expect("failed to retrieve history");

        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].role, "user");
        assert_eq!(messages[0].content, "What is Rust?");
        assert_eq!(messages[1].role, "assistant");
        assert_eq!(
            messages[1].thinking_content.as_deref(),
            Some("I should explain Rust concisely.")
        );
    }

    #[test]
    fn finishes_generating_message() {
        let (_dir, conn, session_id) = setup_database();

        let message = create_message(&conn, &session_id, "assistant", "", None, "generating")
            .expect("failed to create placeholder");

        let updated = finish_message(
            &conn,
            &message.id,
            "Generation finished.",
            Some("Reasoning content"),
            "completed",
        )
        .expect("failed to finish message");

        assert!(updated);

        let saved = get_message(&conn, &message.id)
            .expect("failed to retrieve message")
            .expect("message should exist");

        assert_eq!(saved.content, "Generation finished.");
        assert_eq!(saved.thinking_content.as_deref(), Some("Reasoning content"));
        assert_eq!(saved.status, "completed");
        assert!(saved.completed_at.is_some());
    }

    #[test]
    fn finishing_unknown_message_returns_false() {
        let (_dir, conn, _session_id) = setup_database();

        let updated = finish_message(&conn, "missing-message-id", "Response", None, "completed")
            .expect("update query should succeed");

        assert!(!updated);
    }

    #[test]
    fn rejects_message_for_unknown_session() {
        let (_dir, conn, _session_id) = setup_database();

        let result = create_message(
            &conn,
            "missing-session-id",
            "user",
            "Hello",
            None,
            "completed",
        );

        assert!(result.is_err());
    }
}
