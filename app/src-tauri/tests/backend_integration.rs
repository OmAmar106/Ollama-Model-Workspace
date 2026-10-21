use app_lib::{
    db::{
        self,
        generations::{
            create_generation, finish_generation, get_generation, recover_interrupted_generations,
        },
        messages::{create_message, finish_message, get_message, get_session_messages},
        sessions::{create_session, delete_session, get_session, list_sessions, update_session},
    },
    generation_manager::GenerationManager,
    ollama::{GenerateChunk, GenerateRequest, OllamaClient, StreamOutcome},
    session_cache::SessionCache,
};
use rusqlite::Connection;
use std::{
    sync::{Arc, atomic::Ordering},
    thread,
    time::{Duration, Instant},
};
use tempfile::tempdir;
use tiny_http::{Response, Server};

fn setup_test_db() -> (tempfile::TempDir, Connection) {
    let dir = tempdir().expect("failed to create temp dir");
    let db_path = dir.path().join("integration_test.db");
    let conn = db::initialize_database(&db_path).expect("failed to initialize db");
    (dir, conn)
}

fn start_mock_ollama_server(
    status: u16,
    chunks: Vec<&'static str>,
) -> (String, thread::JoinHandle<()>) {
    let server = Server::http("127.0.0.1:0").expect("failed to bind mock server");
    let url = format!("http://{}", server.server_addr());

    let handle = thread::spawn(move || {
        let request = server.recv().expect("mock server recv");
        let mut body = String::new();
        for chunk in chunks {
            body.push_str(chunk);
        }

        let response = Response::from_string(body)
            .with_status_code(status)
            .with_header(
                tiny_http::Header::from_bytes("Content-Type", "application/x-ndjson").unwrap(),
            );

        request.respond(response).expect("respond to mock request");
    });

    (url, handle)
}

#[test]
fn test_session_and_message_persistence_and_cascading() {
    let (_dir, conn) = setup_test_db();

    // 1. Create sessions
    let s1 = create_session(&conn, "llama3.2", Some(false)).unwrap();
    let s2 = create_session(&conn, "qwen2.5", Some(true)).unwrap();

    let all_sessions = list_sessions(&conn).unwrap();
    assert_eq!(all_sessions.len(), 2);

    // 2. Add messages to s1
    let u1 = create_message(&conn, &s1.id, "user", "What is Tauri?", None, "completed").unwrap();
    let a1 = create_message(
        &conn,
        &s1.id,
        "assistant",
        "Tauri is a framework.",
        Some("Thinking about Tauri"),
        "completed",
    )
    .unwrap();

    let s1_messages = get_session_messages(&conn, &s1.id).unwrap();
    assert_eq!(s1_messages.len(), 2);
    assert_eq!(s1_messages[0].id, u1.id);
    assert_eq!(s1_messages[1].id, a1.id);

    // 3. Update session
    let mut s1_mod = s1.clone();
    s1_mod.title = Some("Tauri Chat".to_string());
    update_session(&conn, &s1_mod).unwrap();
    let s1_retrieved = get_session(&conn, &s1.id).unwrap().unwrap();
    assert_eq!(s1_retrieved.title.as_deref(), Some("Tauri Chat"));

    // 4. Cascade delete: deleting s1 deletes its messages
    delete_session(&conn, &s1.id).unwrap();
    assert!(get_session(&conn, &s1.id).unwrap().is_none());
    assert!(get_message(&conn, &u1.id).unwrap().is_none());
    assert!(get_message(&conn, &a1.id).unwrap().is_none());

    // s2 remains
    assert!(get_session(&conn, &s2.id).unwrap().is_some());
}

#[tokio::test]
async fn test_full_chat_generation_flow() {
    let (_dir, conn) = setup_test_db();
    let session = create_session(&conn, "test-model", Some(true)).unwrap();

    // Setup mock Ollama streaming response
    let (base_url, server_handle) = start_mock_ollama_server(
        200,
        vec![
            "{\"response\":\"Thinking...\",\"thinking\":\"Hmm\",\"done\":false}\n",
            "{\"response\":\"Here is the answer.\",\"done\":true,\"prompt_eval_count\":15,\"eval_count\":8}\n",
        ],
    );

    let client = OllamaClient::with_base_url(base_url);

    // 1. User message
    let user_msg =
        create_message(&conn, &session.id, "user", "Help me", None, "completed").unwrap();

    // 2. Assistant placeholder
    let asst_msg = create_message(&conn, &session.id, "assistant", "", None, "generating").unwrap();

    // 3. Generation record
    let gen_record = create_generation(&conn, &asst_msg.id, &session.model_name).unwrap();
    assert_eq!(gen_record.status, "running");

    // 4. Stream chunks
    let started = Instant::now();
    let mut full_response = String::new();
    let mut full_thinking = String::new();
    let mut first_token_ms: Option<i64> = None;
    let mut prompt_tokens: Option<i64> = None;
    let mut completion_tokens: Option<i64> = None;

    let outcome = client
        .generate_stream(
            GenerateRequest {
                model: session.model_name.clone(),
                prompt: format!("User: {}\n\nAssistant: ", user_msg.content),
                think: session.think,
            },
            |chunk: GenerateChunk| {
                if first_token_ms.is_none()
                    && (!chunk.response.is_empty() || chunk.thinking.is_some())
                {
                    first_token_ms = Some(started.elapsed().as_millis() as i64);
                }
                if chunk.prompt_eval_count.is_some() {
                    prompt_tokens = chunk.prompt_eval_count;
                }
                if chunk.eval_count.is_some() {
                    completion_tokens = chunk.eval_count;
                }
                full_response.push_str(&chunk.response);
                if let Some(t) = chunk.thinking {
                    full_thinking.push_str(&t);
                }
            },
        )
        .await
        .unwrap();

    server_handle.join().unwrap();
    assert_eq!(outcome, StreamOutcome::Completed);

    let duration_ms = started.elapsed().as_millis() as i64;

    // 5. Finalize assistant message and generation
    finish_message(
        &conn,
        &asst_msg.id,
        &full_response,
        Some(&full_thinking),
        "completed",
    )
    .unwrap();

    finish_generation(
        &conn,
        &gen_record.id,
        "completed",
        first_token_ms,
        Some(duration_ms.max(1)),
        prompt_tokens,
        completion_tokens,
        None,
        None,
    )
    .unwrap();

    // Verify persisted state
    let saved_msg = get_message(&conn, &asst_msg.id).unwrap().unwrap();
    assert_eq!(saved_msg.status, "completed");
    assert_eq!(saved_msg.content, "Thinking...Here is the answer.");
    assert_eq!(saved_msg.thinking_content.as_deref(), Some("Hmm"));
    assert!(saved_msg.completed_at.is_some());

    let saved_gen = get_generation(&conn, &gen_record.id).unwrap().unwrap();
    assert_eq!(saved_gen.status, "completed");
    assert_eq!(saved_gen.prompt_tokens, Some(15));
    assert_eq!(saved_gen.completion_tokens, Some(8));
    assert!(saved_gen.time_to_first_token_ms.is_some());
    assert!(saved_gen.duration_ms.is_some());
    assert!(saved_gen.tokens_per_second.is_some());
}

#[tokio::test]
async fn test_failed_stream_preserves_partial_response() {
    let (_dir, conn) = setup_test_db();
    let session = create_session(&conn, "test-model", None).unwrap();

    // Mock server returns 1 chunk then incomplete stream
    let (base_url, server_handle) = start_mock_ollama_server(
        200,
        vec!["{\"response\":\"Partial start\",\"done\":false}\n"],
    );

    let client = OllamaClient::with_base_url(base_url);
    let asst_msg = create_message(&conn, &session.id, "assistant", "", None, "generating").unwrap();
    let gen_record = create_generation(&conn, &asst_msg.id, &session.model_name).unwrap();

    let mut partial = String::new();
    let result = client
        .generate_stream(
            GenerateRequest {
                model: session.model_name.clone(),
                prompt: "Hello".to_string(),
                think: None,
            },
            |chunk| {
                partial.push_str(&chunk.response);
            },
        )
        .await;

    server_handle.join().unwrap();
    // Stream ended without done: true -> error
    assert!(result.is_err());
    assert_eq!(partial, "Partial start");

    // Persist as failed
    finish_message(&conn, &asst_msg.id, &partial, None, "failed").unwrap();
    finish_generation(
        &conn,
        &gen_record.id,
        "failed",
        Some(10),
        Some(100),
        None,
        None,
        Some("STREAM_INTERRUPTED"),
        Some("Stream ended prematurely"),
    )
    .unwrap();

    let saved_msg = get_message(&conn, &asst_msg.id).unwrap().unwrap();
    assert_eq!(saved_msg.status, "failed");
    assert_eq!(saved_msg.content, "Partial start");

    let saved_gen = get_generation(&conn, &gen_record.id).unwrap().unwrap();
    assert_eq!(saved_gen.status, "failed");
    assert_eq!(saved_gen.error_code.as_deref(), Some("STREAM_INTERRUPTED"));
}

#[tokio::test]
async fn test_cancellation_and_concurrency_control() {
    let (_dir, conn) = setup_test_db();
    let session = create_session(&conn, "test-model", None).unwrap();

    let cache = SessionCache::new();
    cache.insert(session.clone());

    let gen_manager = GenerationManager::new();

    // Acquire generation lock
    cache.try_acquire_generation(&session.id, "gen-1").unwrap();
    let cancel_flag = gen_manager.register(&session.id, "gen-1");

    // Conflicting simultaneous generation in same session MUST fail
    let conflict = cache.try_acquire_generation(&session.id, "gen-2");
    assert!(conflict.is_err());

    // Different session can acquire generation concurrently
    let session2 = create_session(&conn, "test-model-2", None).unwrap();
    cache.insert(session2.clone());
    assert!(cache.try_acquire_generation(&session2.id, "gen-3").is_ok());

    // Mock streaming server that sends chunks until cancelled
    let (base_url, server_handle) = start_mock_ollama_server(
        200,
        vec![
            "{\"response\":\"First \",\"done\":false}\n",
            "{\"response\":\"Second \",\"done\":false}\n",
            "{\"response\":\"Third \",\"done\":true}\n",
        ],
    );

    let client = OllamaClient::with_base_url(base_url);
    let cancel_clone = Arc::clone(&cancel_flag);
    let mut captured = String::new();

    let outcome = client
        .generate_stream_cancellable(
            GenerateRequest {
                model: session.model_name.clone(),
                prompt: "Tell a story".to_string(),
                think: None,
            },
            Some(cancel_flag),
            |chunk| {
                captured.push_str(&chunk.response);
                // Trigger cancellation on first chunk
                cancel_clone.store(true, Ordering::Relaxed);
            },
        )
        .await
        .unwrap();

    server_handle.join().unwrap();
    assert_eq!(outcome, StreamOutcome::Cancelled);
    assert!(!captured.is_empty());

    // Release generation
    cache.release_generation(&session.id, "gen-1");
    gen_manager.unregister("gen-1");
    assert!(!cache.is_generating(&session.id));

    // Now a new generation on session 1 is allowed
    assert!(cache.try_acquire_generation(&session.id, "gen-4").is_ok());
}

#[test]
fn test_cache_eviction_and_restoration_from_sqlite() {
    let (_dir, conn) = setup_test_db();
    let s = create_session(&conn, "mistral", Some(false)).unwrap();
    create_message(&conn, &s.id, "user", "Ping", None, "completed").unwrap();
    create_message(&conn, &s.id, "assistant", "Pong", None, "completed").unwrap();

    let cache = SessionCache::with_ttl(Duration::from_secs(300));
    cache.insert(s.clone());
    assert!(cache.is_cached(&s.id));

    // Simulate 301 seconds of inactivity
    cache.set_last_activity(&s.id, Instant::now() - Duration::from_secs(301));
    assert_eq!(cache.evict_expired(), 1);
    assert!(!cache.is_cached(&s.id));

    // Restore from SQLite
    let (restored_session, restored_history) = cache
        .get_or_restore_with_history(&conn, &s.id)
        .unwrap()
        .expect("should restore");

    assert_eq!(restored_session.id, s.id);
    assert_eq!(restored_history.len(), 2);
    assert_eq!(restored_history[0].content, "Ping");
    assert_eq!(restored_history[1].content, "Pong");
}

#[test]
fn test_startup_recovery_is_idempotent() {
    let (_dir, conn) = setup_test_db();
    let s = create_session(&conn, "llama3", None).unwrap();

    let m1 = create_message(
        &conn,
        &s.id,
        "assistant",
        "Incomplete text...",
        None,
        "generating",
    )
    .unwrap();
    let g1 = create_generation(&conn, &m1.id, "llama3").unwrap();
    assert_eq!(g1.status, "running");

    // Startup recovery marks them interrupted
    let recovered = recover_interrupted_generations(&conn).unwrap();
    assert_eq!(recovered, 1);

    let saved_gen = get_generation(&conn, &g1.id).unwrap().unwrap();
    assert_eq!(saved_gen.status, "interrupted");
    assert_eq!(saved_gen.error_code.as_deref(), Some("INTERRUPTED"));

    let saved_msg = get_message(&conn, &m1.id).unwrap().unwrap();
    assert_eq!(saved_msg.status, "interrupted");
    assert_eq!(saved_msg.content, "Incomplete text..."); // Partial text preserved!

    // Second recovery run is idempotent
    assert_eq!(recover_interrupted_generations(&conn).unwrap(), 0);
}

#[test]
fn test_database_integrity_and_check_constraints() {
    let (_dir, conn) = setup_test_db();

    // 1. Foreign key failure: insert message with non-existent session
    let res = create_message(
        &conn,
        "non-existent-session-id",
        "user",
        "Hi",
        None,
        "completed",
    );
    assert!(res.is_err());

    // 2. CHECK constraint failure: invalid message role
    let s = create_session(&conn, "model", None).unwrap();
    let invalid_role = conn.execute(
        "INSERT INTO messages (id, session_id, role, content, status, created_at)
         VALUES ('m-id', ?1, 'invalid_role', 'test', 'completed', '2026-01-01')",
        [&s.id],
    );
    assert!(invalid_role.is_err());

    // 3. CHECK constraint failure: invalid generation status
    let valid_msg = create_message(&conn, &s.id, "assistant", "", None, "generating").unwrap();
    let invalid_gen_status = conn.execute(
        "INSERT INTO generations (id, message_id, model_name, status, started_at)
         VALUES ('g-id', ?1, 'model', 'not_a_valid_status', '2026-01-01')",
        [&valid_msg.id],
    );
    assert!(invalid_gen_status.is_err());
}
