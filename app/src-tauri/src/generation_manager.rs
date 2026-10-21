use std::{
    collections::HashMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};

#[derive(Debug, Clone)]
pub struct ActiveGeneration {
    pub session_id: String,
    pub generation_id: String,
    pub cancel_flag: Arc<AtomicBool>,
}

#[derive(Default)]
pub struct GenerationManager {
    active: Mutex<HashMap<String, ActiveGeneration>>,
}

impl GenerationManager {
    pub fn new() -> Self {
        Self {
            active: Mutex::new(HashMap::new()),
        }
    }

    /// Register an active generation. Returns the cancellation flag.
    pub fn register(&self, session_id: &str, generation_id: &str) -> Arc<AtomicBool> {
        let flag = Arc::new(AtomicBool::new(false));
        let mut map = self.active.lock().expect("lock poisoned");
        map.insert(
            generation_id.to_string(),
            ActiveGeneration {
                session_id: session_id.to_string(),
                generation_id: generation_id.to_string(),
                cancel_flag: Arc::clone(&flag),
            },
        );
        flag
    }

    /// Cancel a specific generation by ID.
    pub fn cancel(&self, generation_id: &str) -> bool {
        let map = self.active.lock().expect("lock poisoned");
        if let Some(active) = map.get(generation_id) {
            active.cancel_flag.store(true, Ordering::Relaxed);
            true
        } else {
            false
        }
    }

    /// Cancel any active generation belonging to a specific session.
    pub fn cancel_session(&self, session_id: &str) -> bool {
        let map = self.active.lock().expect("lock poisoned");
        let mut cancelled = false;
        for active in map.values() {
            if active.session_id == session_id {
                active.cancel_flag.store(true, Ordering::Relaxed);
                cancelled = true;
            }
        }
        cancelled
    }

    /// Cancel all active generations (used on application shutdown).
    pub fn cancel_all(&self) -> usize {
        let map = self.active.lock().expect("lock poisoned");
        let count = map.len();
        for active in map.values() {
            active.cancel_flag.store(true, Ordering::Relaxed);
        }
        count
    }

    /// Unregister a generation once complete, cancelled, or failed.
    pub fn unregister(&self, generation_id: &str) -> Option<ActiveGeneration> {
        let mut map = self.active.lock().expect("lock poisoned");
        map.remove(generation_id)
    }

    pub fn is_active(&self, generation_id: &str) -> bool {
        let map = self.active.lock().expect("lock poisoned");
        map.contains_key(generation_id)
    }

    pub fn active_count(&self) -> usize {
        let map = self.active.lock().expect("lock poisoned");
        map.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registers_and_cancels_generation() {
        let manager = GenerationManager::new();
        let flag = manager.register("session-1", "gen-1");

        assert!(manager.is_active("gen-1"));
        assert!(!flag.load(Ordering::Relaxed));

        // Unknown generation cancellation returns false
        assert!(!manager.cancel("gen-unknown"));

        // Cancel gen-1
        assert!(manager.cancel("gen-1"));
        assert!(flag.load(Ordering::Relaxed));

        // Unregister
        let removed = manager.unregister("gen-1");
        assert!(removed.is_some());
        assert!(!manager.is_active("gen-1"));
    }

    #[test]
    fn cancels_session_generations() {
        let manager = GenerationManager::new();
        let flag1 = manager.register("session-1", "gen-1");
        let flag2 = manager.register("session-2", "gen-2");

        assert!(manager.cancel_session("session-1"));
        assert!(flag1.load(Ordering::Relaxed));
        assert!(!flag2.load(Ordering::Relaxed));
    }

    #[test]
    fn cancels_all_active_generations() {
        let manager = GenerationManager::new();
        let flag1 = manager.register("session-1", "gen-1");
        let flag2 = manager.register("session-2", "gen-2");

        let count = manager.cancel_all();
        assert_eq!(count, 2);
        assert!(flag1.load(Ordering::Relaxed));
        assert!(flag2.load(Ordering::Relaxed));
    }
}
