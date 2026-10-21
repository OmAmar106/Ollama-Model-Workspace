use chrono::Utc;
use rusqlite::{Connection, OptionalExtension, Result, params};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub title: Option<String>,
    pub model_name: String,
    pub think: Option<bool>,
    pub system_prompt: Option<String>,
    pub options_json: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

pub fn create_session(
    connection: &Connection,
    model_name: &str,
    think: Option<bool>,
) -> Result<Session> {
    let now = Utc::now().to_rfc3339();
    let session = Session {
        id: Uuid::new_v4().to_string(),
        title: None,
        model_name: model_name.to_string(),
        think,
        system_prompt: None,
        options_json: None,
        created_at: now.clone(),
        updated_at: now,
    };

    connection.execute(
        "INSERT INTO sessions
        (id, title, model_name, think, system_prompt, options_json, created_at, updated_at)
     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            session.id,
            session.title,
            session.model_name,
            session.think.map(i64::from),
            session.system_prompt,
            session.options_json,
            session.created_at,
            session.updated_at,
        ],
    )?;

    Ok(session)
}

pub fn get_session(connection: &Connection, session_id: &str) -> Result<Option<Session>> {
    connection
        .query_row(
            "SELECT
                id,
                title,
                model_name,
                think,
                system_prompt,
                options_json,
                created_at,
                updated_at
             FROM sessions
             WHERE id = ?1",
            [session_id],
            |row| {
                let think: Option<i64> = row.get(3)?;

                Ok(Session {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    model_name: row.get(2)?,
                    think: think.map(|value| value != 0),
                    system_prompt: row.get(4)?,
                    options_json: row.get(5)?,
                    created_at: row.get(6)?,
                    updated_at: row.get(7)?,
                })
            },
        )
        .optional()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::initialize_database;
    use tempfile::tempdir;

    fn setup_database() -> (tempfile::TempDir, Connection) {
        let directory = tempdir().expect("failed to create temporary directory");
        let path = directory.path().join("test.db");
        let connection = initialize_database(&path).expect("failed to initialize test database");

        (directory, connection)
    }

    #[test]
    fn creates_and_retrieves_session() {
        let (_directory, connection) = setup_database();

        let created =
            create_session(&connection, "llama3.2", Some(false)).expect("failed to create session");

        assert!(!created.id.is_empty());
        assert_eq!(created.model_name, "llama3.2");
        assert_eq!(created.think, Some(false));
        assert!(created.title.is_none());
        assert_eq!(created.created_at, created.updated_at);

        let retrieved = get_session(&connection, &created.id)
            .expect("failed to retrieve session")
            .expect("session should exist");

        assert_eq!(retrieved.id, created.id);
        assert_eq!(retrieved.model_name, "llama3.2");
        assert_eq!(retrieved.think, Some(false));
        assert_eq!(retrieved.created_at, created.created_at);
    }

    #[test]
    fn supports_true_and_unspecified_think_values() {
        let (_directory, connection) = setup_database();

        let thinking_enabled = create_session(&connection, "model-a", Some(true))
            .expect("failed to create session with think=true");

        let thinking_unspecified = create_session(&connection, "model-b", None)
            .expect("failed to create session with unspecified think");

        assert_eq!(
            get_session(&connection, &thinking_enabled.id)
                .unwrap()
                .unwrap()
                .think,
            Some(true)
        );

        assert_eq!(
            get_session(&connection, &thinking_unspecified.id)
                .unwrap()
                .unwrap()
                .think,
            None
        );
    }

    #[test]
    fn returns_none_for_unknown_session_id() {
        let (_directory, connection) = setup_database();

        let result =
            get_session(&connection, "session-that-does-not-exist").expect("query should succeed");

        assert!(result.is_none());
    }
}
