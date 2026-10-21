use chrono::Utc;
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Generation {
    pub id: String,
    pub message_id: String,
    pub model_name: String,
    pub status: String,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub time_to_first_token_ms: Option<i64>,
    pub duration_ms: Option<i64>,
    pub prompt_tokens: Option<i64>,
    pub completion_tokens: Option<i64>,
    pub tokens_per_second: Option<f64>,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
}

/// Create a generation record before streaming begins.
pub fn create_generation(
    conn: &Connection,
    message_id: &str,
    model_name: &str,
) -> rusqlite::Result<Generation> {
    let id = Uuid::new_v4().to_string();
    let started_at = Utc::now().to_rfc3339();

    conn.execute(
        "INSERT INTO generations (
            id, message_id, model_name, status, started_at
        ) VALUES (?1, ?2, ?3, 'running', ?4)",
        params![id, message_id, model_name, started_at],
    )?;

    Ok(Generation {
        id,
        message_id: message_id.to_owned(),
        model_name: model_name.to_owned(),
        status: "running".to_owned(),
        started_at,
        finished_at: None,
        time_to_first_token_ms: None,
        duration_ms: None,
        prompt_tokens: None,
        completion_tokens: None,
        tokens_per_second: None,
        error_code: None,
        error_message: None,
    })
}

/// Finish a generation and save the metrics that are available.
#[allow(clippy::too_many_arguments)]
pub fn finish_generation(
    conn: &Connection,
    generation_id: &str,
    status: &str,
    time_to_first_token_ms: Option<i64>,
    duration_ms: Option<i64>,
    prompt_tokens: Option<i64>,
    completion_tokens: Option<i64>,
    error_code: Option<&str>,
    error_message: Option<&str>,
) -> rusqlite::Result<bool> {
    let finished_at = Utc::now().to_rfc3339();

    let tokens_per_second = match (completion_tokens, duration_ms) {
        (Some(tokens), Some(duration)) if duration > 0 => {
            Some(tokens as f64 * 1000.0 / duration as f64)
        }
        _ => None,
    };

    let updated = conn.execute(
        "UPDATE generations
         SET status = ?1,
             finished_at = ?2,
             time_to_first_token_ms = ?3,
             duration_ms = ?4,
             prompt_tokens = ?5,
             completion_tokens = ?6,
             tokens_per_second = ?7,
             error_code = ?8,
             error_message = ?9
         WHERE id = ?10",
        params![
            status,
            finished_at,
            time_to_first_token_ms,
            duration_ms,
            prompt_tokens,
            completion_tokens,
            tokens_per_second,
            error_code,
            error_message,
            generation_id,
        ],
    )?;

    Ok(updated == 1)
}

/// Retrieve a generation by its ID.
pub fn get_generation(
    conn: &Connection,
    generation_id: &str,
) -> rusqlite::Result<Option<Generation>> {
    conn.query_row(
        "SELECT id, message_id, model_name, status, started_at,
                finished_at, time_to_first_token_ms, duration_ms,
                prompt_tokens, completion_tokens, tokens_per_second,
                error_code, error_message
         FROM generations
         WHERE id = ?1",
        params![generation_id],
        |row| {
            Ok(Generation {
                id: row.get(0)?,
                message_id: row.get(1)?,
                model_name: row.get(2)?,
                status: row.get(3)?,
                started_at: row.get(4)?,
                finished_at: row.get(5)?,
                time_to_first_token_ms: row.get(6)?,
                duration_ms: row.get(7)?,
                prompt_tokens: row.get(8)?,
                completion_tokens: row.get(9)?,
                tokens_per_second: row.get(10)?,
                error_code: row.get(11)?,
                error_message: row.get(12)?,
            })
        },
    )
    .optional()
}

/// Recovers any generations and associated messages that were left in 'running' / 'generating' state.
/// Marks them as 'interrupted', sets finished_at/completed_at timestamps, and leaves partial output intact.
/// This operation is idempotent.
pub fn recover_interrupted_generations(conn: &Connection) -> rusqlite::Result<usize> {
    let now = Utc::now().to_rfc3339();

    let mut stmt =
        conn.prepare("SELECT id, message_id FROM generations WHERE status = 'running'")?;
    let running: Vec<(String, String)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect::<Result<Vec<_>, _>>()?;

    let count = running.len();

    for (gen_id, msg_id) in running {
        conn.execute(
            "UPDATE generations
             SET status = 'interrupted',
                 finished_at = ?1,
                 error_code = 'INTERRUPTED',
                 error_message = 'Generation interrupted by application shutdown'
             WHERE id = ?2 AND status = 'running'",
            params![now, gen_id],
        )?;

        conn.execute(
            "UPDATE messages
             SET status = 'interrupted',
                 completed_at = ?1
             WHERE id = ?2 AND status = 'generating'",
            params![now, msg_id],
        )?;
    }

    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{initialize_database, messages::create_message};
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

        let message = create_message(&conn, &session_id, "assistant", "", None, "generating")
            .expect("failed to create assistant message");

        (dir, conn, message.id)
    }

    #[test]
    fn creates_and_retrieves_generation() {
        let (_dir, conn, message_id) = setup_database();

        let created = create_generation(&conn, &message_id, "test-model")
            .expect("failed to create generation");

        let saved = get_generation(&conn, &created.id)
            .expect("failed to retrieve generation")
            .expect("generation should exist");

        assert_eq!(saved.id, created.id);
        assert_eq!(saved.message_id, message_id);
        assert_eq!(saved.model_name, "test-model");
        assert_eq!(saved.status, "running");
        assert!(saved.finished_at.is_none());
        assert!(saved.duration_ms.is_none());
    }

    #[test]
    fn finishes_generation_and_calculates_throughput() {
        let (_dir, conn, message_id) = setup_database();

        let generation = create_generation(&conn, &message_id, "test-model")
            .expect("failed to create generation");

        let updated = finish_generation(
            &conn,
            &generation.id,
            "completed",
            Some(250),
            Some(2000),
            Some(100),
            Some(40),
            None,
            None,
        )
        .expect("failed to finish generation");

        assert!(updated);

        let saved = get_generation(&conn, &generation.id)
            .expect("failed to retrieve generation")
            .expect("generation should exist");

        assert_eq!(saved.status, "completed");
        assert!(saved.finished_at.is_some());
        assert_eq!(saved.time_to_first_token_ms, Some(250));
        assert_eq!(saved.duration_ms, Some(2000));
        assert_eq!(saved.prompt_tokens, Some(100));
        assert_eq!(saved.completion_tokens, Some(40));
        assert_eq!(saved.tokens_per_second, Some(20.0));
        assert!(saved.error_code.is_none());
        assert!(saved.error_message.is_none());
    }

    #[test]
    fn records_generation_failure() {
        let (_dir, conn, message_id) = setup_database();

        let generation = create_generation(&conn, &message_id, "test-model")
            .expect("failed to create generation");

        finish_generation(
            &conn,
            &generation.id,
            "failed",
            None,
            Some(500),
            None,
            None,
            Some("OLLAMA_UNAVAILABLE"),
            Some("Could not connect to Ollama"),
        )
        .expect("failed to save failure");

        let saved = get_generation(&conn, &generation.id)
            .expect("failed to retrieve generation")
            .expect("generation should exist");

        assert_eq!(saved.status, "failed");
        assert_eq!(saved.error_code.as_deref(), Some("OLLAMA_UNAVAILABLE"));
        assert_eq!(
            saved.error_message.as_deref(),
            Some("Could not connect to Ollama")
        );
        assert!(saved.tokens_per_second.is_none());
    }

    #[test]
    fn finishing_unknown_generation_returns_false() {
        let (_dir, conn, _message_id) = setup_database();

        let updated = finish_generation(
            &conn,
            "missing-generation-id",
            "completed",
            None,
            Some(100),
            None,
            Some(10),
            None,
            None,
        )
        .expect("update query should succeed");

        assert!(!updated);
    }

    #[test]
    fn zero_duration_does_not_calculate_throughput() {
        let (_dir, conn, message_id) = setup_database();

        let generation = create_generation(&conn, &message_id, "test-model")
            .expect("failed to create generation");

        finish_generation(
            &conn,
            &generation.id,
            "completed",
            Some(0),
            Some(0),
            Some(10),
            Some(5),
            None,
            None,
        )
        .expect("failed to finish generation");

        let saved = get_generation(&conn, &generation.id)
            .expect("failed to retrieve generation")
            .expect("generation should exist");

        assert!(saved.tokens_per_second.is_none());
    }

    #[test]
    fn recovers_running_generations_and_is_idempotent() {
        let (_dir, conn, message_id) = setup_database();

        let gen1 = create_generation(&conn, &message_id, "test-model").unwrap();
        assert_eq!(gen1.status, "running");

        // Run recovery
        let count = recover_interrupted_generations(&conn).expect("recovery should succeed");
        assert_eq!(count, 1);

        // Verify generation is marked interrupted
        let saved_gen = get_generation(&conn, &gen1.id).unwrap().unwrap();
        assert_eq!(saved_gen.status, "interrupted");
        assert!(saved_gen.finished_at.is_some());
        assert_eq!(saved_gen.error_code.as_deref(), Some("INTERRUPTED"));

        // Verify associated message is marked interrupted
        let saved_msg = crate::db::messages::get_message(&conn, &message_id)
            .unwrap()
            .unwrap();
        assert_eq!(saved_msg.status, "interrupted");
        assert!(saved_msg.completed_at.is_some());

        // Recovery is idempotent: running again finds 0 running generations
        let count2 = recover_interrupted_generations(&conn).expect("second recovery");
        assert_eq!(count2, 0);
    }
}
