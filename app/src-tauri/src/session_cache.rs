use crate::db::{
    messages::{self, Message},
    sessions::{self, Session},
};
use rusqlite::Connection;
use std::{
    collections::HashMap,
    sync::RwLock,
    time::{Duration, Instant},
};

pub const DEFAULT_CACHE_TTL: Duration = Duration::from_secs(5 * 60); // 5 minutes

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionRuntimeState {
    pub is_generating: bool,
    pub active_generation_id: Option<String>,
}

impl Default for SessionRuntimeState {
    fn default() -> Self {
        Self {
            is_generating: false,
            active_generation_id: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CachedSessionEntry {
    pub session: Session,
    pub last_activity: Instant,
    pub runtime: SessionRuntimeState,
}

pub struct SessionCache {
    entries: RwLock<HashMap<String, CachedSessionEntry>>,
    ttl: Duration,
}

impl Default for SessionCache {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionCache {
    pub fn new() -> Self {
        Self::with_ttl(DEFAULT_CACHE_TTL)
    }

    pub fn with_ttl(ttl: Duration) -> Self {
        Self {
            entries: RwLock::new(HashMap::new()),
            ttl,
        }
    }

    pub fn ttl(&self) -> Duration {
        self.ttl
    }

    pub fn insert(&self, session: Session) {
        let mut map = self.entries.write().expect("lock poisoned");
        map.insert(
            session.id.clone(),
            CachedSessionEntry {
                session,
                last_activity: Instant::now(),
                runtime: SessionRuntimeState::default(),
            },
        );
    }

    pub fn update(&self, session: Session) {
        let mut map = self.entries.write().expect("lock poisoned");
        if let Some(entry) = map.get_mut(&session.id) {
            entry.session = session;
            entry.last_activity = Instant::now();
        } else {
            map.insert(
                session.id.clone(),
                CachedSessionEntry {
                    session,
                    last_activity: Instant::now(),
                    runtime: SessionRuntimeState::default(),
                },
            );
        }
    }

    pub fn remove(&self, session_id: &str) -> Option<CachedSessionEntry> {
        let mut map = self.entries.write().expect("lock poisoned");
        map.remove(session_id)
    }

    pub fn is_cached(&self, session_id: &str) -> bool {
        let map = self.entries.read().expect("lock poisoned");
        if let Some(entry) = map.get(session_id) {
            entry.last_activity.elapsed() <= self.ttl
        } else {
            false
        }
    }

    pub fn cached_count(&self) -> usize {
        let map = self.entries.read().expect("lock poisoned");
        map.len()
    }

    pub fn touch(&self, session_id: &str) -> bool {
        let mut map = self.entries.write().expect("lock poisoned");
        if let Some(entry) = map.get_mut(session_id) {
            entry.last_activity = Instant::now();
            true
        } else {
            false
        }
    }

    pub fn set_last_activity(&self, session_id: &str, time: Instant) -> bool {
        let mut map = self.entries.write().expect("lock poisoned");
        if let Some(entry) = map.get_mut(session_id) {
            entry.last_activity = time;
            true
        } else {
            false
        }
    }

    pub fn evict_expired(&self) -> usize {
        self.evict_inactive(self.ttl)
    }

    pub fn evict_inactive(&self, max_idle: Duration) -> usize {
        let mut map = self.entries.write().expect("lock poisoned");
        let initial_count = map.len();
        map.retain(|_, entry| entry.last_activity.elapsed() <= max_idle);
        initial_count - map.len()
    }

    pub fn get_or_restore(
        &self,
        connection: &Connection,
        session_id: &str,
    ) -> rusqlite::Result<Option<Session>> {
        // Fast path: check read lock.
        {
            let map = self.entries.read().expect("lock poisoned");
            if let Some(entry) = map.get(session_id) {
                if entry.last_activity.elapsed() <= self.ttl {
                    // Still fresh. Return clone.
                    let session = entry.session.clone();
                    drop(map);
                    self.touch(session_id);
                    return Ok(Some(session));
                }
            }
        }

        // Expired or missing: acquire write lock to evict or restore from SQLite.
        let mut map = self.entries.write().expect("lock poisoned");

        // If it was expired, remove it.
        if let Some(entry) = map.get(session_id) {
            if entry.last_activity.elapsed() > self.ttl {
                map.remove(session_id);
            } else {
                return Ok(Some(entry.session.clone()));
            }
        }

        // Restore from SQLite.
        if let Some(session) = sessions::get_session(connection, session_id)? {
            map.insert(
                session.id.clone(),
                CachedSessionEntry {
                    session: session.clone(),
                    last_activity: Instant::now(),
                    runtime: SessionRuntimeState::default(),
                },
            );
            Ok(Some(session))
        } else {
            Ok(None)
        }
    }

    pub fn get_or_restore_with_history(
        &self,
        connection: &Connection,
        session_id: &str,
    ) -> rusqlite::Result<Option<(Session, Vec<Message>)>> {
        let session = match self.get_or_restore(connection, session_id)? {
            Some(s) => s,
            None => return Ok(None),
        };

        let messages = messages::get_session_messages(connection, session_id)?;
        self.touch(session_id);

        Ok(Some((session, messages)))
    }

    pub fn try_acquire_generation(
        &self,
        session_id: &str,
        generation_id: &str,
    ) -> Result<(), String> {
        let mut map = self.entries.write().expect("lock poisoned");
        if let Some(entry) = map.get_mut(session_id) {
            if entry.runtime.is_generating {
                return Err(format!(
                    "A generation is already in progress for session {session_id}"
                ));
            }
            entry.runtime.is_generating = true;
            entry.runtime.active_generation_id = Some(generation_id.to_string());
            entry.last_activity = Instant::now();
            Ok(())
        } else {
            Err(format!("Session {session_id} not found in cache"))
        }
    }

    pub fn release_generation(&self, session_id: &str, generation_id: &str) {
        let mut map = self.entries.write().expect("lock poisoned");
        if let Some(entry) = map.get_mut(session_id) {
            if entry.runtime.active_generation_id.as_deref() == Some(generation_id) {
                entry.runtime.is_generating = false;
                entry.runtime.active_generation_id = None;
                entry.last_activity = Instant::now();
            }
        }
    }

    pub fn is_generating(&self, session_id: &str) -> bool {
        let map = self.entries.read().expect("lock poisoned");
        map.get(session_id)
            .map(|e| e.runtime.is_generating)
            .unwrap_or(false)
    }

    pub fn get_runtime_state(&self, session_id: &str) -> Option<SessionRuntimeState> {
        let map = self.entries.read().expect("lock poisoned");
        map.get(session_id).map(|e| e.runtime.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{
        initialize_database,
        messages::create_message,
        sessions::{create_session, get_session},
    };
    use std::sync::Arc;
    use std::thread;
    use tempfile::tempdir;

    fn setup_database() -> (tempfile::TempDir, Connection) {
        let dir = tempdir().expect("failed to create temporary directory");
        let path = dir.path().join("test_cache.db");
        let conn = initialize_database(&path).expect("failed to initialize test database");
        (dir, conn)
    }

    #[test]
    fn maps_session_and_runtime_state() {
        let (_dir, conn) = setup_database();
        let session = create_session(&conn, "llama3.2", Some(false)).unwrap();

        let cache = SessionCache::new();
        cache.insert(session.clone());

        assert!(cache.is_cached(&session.id));
        assert!(!cache.is_generating(&session.id));

        // Acquire generation
        assert!(cache.try_acquire_generation(&session.id, "gen-1").is_ok());
        assert!(cache.is_generating(&session.id));

        // Conflicting generation is prevented
        let conflict = cache.try_acquire_generation(&session.id, "gen-2");
        assert!(conflict.is_err());

        // Release generation
        cache.release_generation(&session.id, "gen-1");
        assert!(!cache.is_generating(&session.id));
    }

    #[test]
    fn deterministic_eviction_after_inactivity_without_deleting_sqlite_data() {
        let (_dir, conn) = setup_database();
        let session = create_session(&conn, "llama3.2", None).unwrap();

        let cache = SessionCache::with_ttl(Duration::from_secs(300));
        cache.insert(session.clone());
        assert!(cache.is_cached(&session.id));
        assert_eq!(cache.cached_count(), 1);

        // Deterministically simulate 301 seconds of inactivity
        let past = Instant::now() - Duration::from_secs(301);
        cache.set_last_activity(&session.id, past);

        // Session should be considered expired
        assert!(!cache.is_cached(&session.id));

        // Evict expired entries
        let evicted = cache.evict_expired();
        assert_eq!(evicted, 1);
        assert_eq!(cache.cached_count(), 0);

        // Verify SQLite data was NOT deleted
        let in_db = get_session(&conn, &session.id).unwrap();
        assert!(in_db.is_some());
        assert_eq!(in_db.unwrap().id, session.id);
    }

    #[test]
    fn restores_session_and_message_history_from_sqlite_after_eviction() {
        let (_dir, conn) = setup_database();
        let session = create_session(&conn, "qwen2.5", Some(true)).unwrap();

        create_message(&conn, &session.id, "user", "Hello", None, "completed").unwrap();
        create_message(
            &conn,
            &session.id,
            "assistant",
            "Hi there!",
            None,
            "completed",
        )
        .unwrap();

        let cache = SessionCache::with_ttl(Duration::from_secs(300));
        cache.insert(session.clone());

        // Evict from cache
        cache.set_last_activity(&session.id, Instant::now() - Duration::from_secs(305));
        cache.evict_expired();
        assert_eq!(cache.cached_count(), 0);

        // Now restore session with message history from SQLite
        let restored = cache
            .get_or_restore_with_history(&conn, &session.id)
            .unwrap()
            .expect("should restore from SQLite");

        assert_eq!(restored.0.id, session.id);
        assert_eq!(restored.0.model_name, "qwen2.5");
        assert_eq!(restored.1.len(), 2);
        assert_eq!(restored.1[0].content, "Hello");
        assert_eq!(restored.1[1].content, "Hi there!");

        // Cache now holds the session again
        assert!(cache.is_cached(&session.id));
        assert_eq!(cache.cached_count(), 1);
    }

    #[test]
    fn keeps_cached_config_consistent_with_persisted() {
        let (_dir, conn) = setup_database();
        let session = create_session(&conn, "old-model", None).unwrap();

        let cache = SessionCache::new();
        cache.insert(session.clone());

        // Update session in SQLite and cache
        let mut updated = session.clone();
        updated.model_name = "new-model".to_string();
        updated.title = Some("New Title".to_string());
        sessions::update_session(&conn, &updated).unwrap();
        cache.update(updated.clone());

        let from_cache = cache.get_or_restore(&conn, &session.id).unwrap().unwrap();
        assert_eq!(from_cache.model_name, "new-model");
        assert_eq!(from_cache.title.as_deref(), Some("New Title"));

        let from_db = get_session(&conn, &session.id).unwrap().unwrap();
        assert_eq!(from_db.model_name, "new-model");
        assert_eq!(from_db.title.as_deref(), Some("New Title"));
    }

    #[test]
    fn concurrency_safe_multi_threaded_access() {
        let (_dir, conn) = setup_database();
        let session = create_session(&conn, "llama3.2", None).unwrap();

        let cache = Arc::new(SessionCache::new());
        cache.insert(session.clone());

        let mut handles = Vec::new();
        for _ in 0..10 {
            let cache_clone = Arc::clone(&cache);
            let session_id = session.id.clone();
            handles.push(thread::spawn(move || {
                for _ in 0..100 {
                    assert!(cache_clone.is_cached(&session_id));
                    cache_clone.touch(&session_id);
                }
            }));
        }

        for handle in handles {
            handle.join().expect("thread failed");
        }

        assert!(cache.is_cached(&session.id));
    }
}
