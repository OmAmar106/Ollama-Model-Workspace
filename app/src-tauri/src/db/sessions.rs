use chrono::{Duration, Utc};
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

pub fn list_sessions(connection: &Connection) -> Result<Vec<Session>> {
    let mut stmt = connection.prepare(
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
         ORDER BY updated_at DESC, created_at DESC",
    )?;

    let rows = stmt.query_map([], |row| {
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
    })?;

    rows.collect()
}

pub fn update_session(connection: &Connection, session: &Session) -> Result<bool> {
    let updated_at = Utc::now().to_rfc3339();
    let updated = connection.execute(
        "UPDATE sessions
         SET title = ?1,
             model_name = ?2,
             think = ?3,
             system_prompt = ?4,
             options_json = ?5,
             updated_at = ?6
         WHERE id = ?7",
        params![
            session.title,
            session.model_name,
            session.think.map(i64::from),
            session.system_prompt,
            session.options_json,
            updated_at,
            session.id,
        ],
    )?;

    Ok(updated == 1)
}

pub fn update_session_timestamp(connection: &Connection, session_id: &str) -> Result<bool> {
    let updated_at = Utc::now().to_rfc3339();
    let updated = connection.execute(
        "UPDATE sessions SET updated_at = ?1 WHERE id = ?2",
        params![updated_at, session_id],
    )?;
    Ok(updated == 1)
}

pub fn delete_session(connection: &Connection, session_id: &str) -> Result<bool> {
    let deleted = connection.execute("DELETE FROM sessions WHERE id = ?1", params![session_id])?;
    Ok(deleted == 1)
}

/// Delete sessions whose `updated_at` is older than `days`, returning the
/// deleted session IDs. Messages and generations are removed by the
/// `ON DELETE CASCADE` foreign keys.
pub fn delete_sessions_older_than(connection: &Connection, days: i64) -> Result<Vec<String>> {
    let cutoff = (Utc::now() - Duration::days(days)).to_rfc3339();

    let ids = {
        let mut statement = connection.prepare("SELECT id FROM sessions WHERE updated_at < ?1")?;
        let rows = statement.query_map([&cutoff], |row| row.get::<_, String>(0))?;
        rows.collect::<Result<Vec<_>>>()?
    };

    if !ids.is_empty() {
        connection.execute("DELETE FROM sessions WHERE updated_at < ?1", [&cutoff])?;
    }

    Ok(ids)
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

    #[test]
    fn lists_updates_and_deletes_sessions() {
        let (_directory, connection) = setup_database();

        let s1 = create_session(&connection, "model-1", None).unwrap();
        let s2 = create_session(&connection, "model-2", Some(true)).unwrap();

        let list = list_sessions(&connection).unwrap();
        assert_eq!(list.len(), 2);

        let mut updated_s1 = s1.clone();
        updated_s1.title = Some("New Title".to_string());
        updated_s1.system_prompt = Some("Act as an assistant".to_string());
        let updated = update_session(&connection, &updated_s1).unwrap();
        assert!(updated);

        let retrieved = get_session(&connection, &s1.id).unwrap().unwrap();
        assert_eq!(retrieved.title.as_deref(), Some("New Title"));
        assert_eq!(
            retrieved.system_prompt.as_deref(),
            Some("Act as an assistant")
        );

        let deleted = delete_session(&connection, &s2.id).unwrap();
        assert!(deleted);

        let after_delete = list_sessions(&connection).unwrap();
        assert_eq!(after_delete.len(), 1);
        assert_eq!(after_delete[0].id, s1.id);
    }

    #[test]
    fn deletes_sessions_older_than_retention_window_with_cascade() {
        let (_directory, connection) = setup_database();

        let old = create_session(&connection, "old-model", None).unwrap();
        let recent = create_session(&connection, "recent-model", None).unwrap();

        crate::db::messages::create_message(
            &connection,
            &old.id,
            "user",
            "Old message",
            None,
            "completed",
        )
        .unwrap();

        let old_timestamp = (Utc::now() - Duration::days(31)).to_rfc3339();
        connection
            .execute(
                "UPDATE sessions SET updated_at = ?1 WHERE id = ?2",
                params![old_timestamp, old.id],
            )
            .unwrap();

        let deleted = delete_sessions_older_than(&connection, 30).unwrap();
        assert_eq!(deleted, vec![old.id.clone()]);

        assert!(get_session(&connection, &old.id).unwrap().is_none());
        assert!(get_session(&connection, &recent.id).unwrap().is_some());

        let remaining_messages: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM messages WHERE session_id = ?1",
                [&old.id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(remaining_messages, 0);
    }
}
