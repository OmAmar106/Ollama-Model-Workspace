use crate::{
    AppDatabase, AppGenerationManager, AppOllamaClient, AppSessionCache,
    db::{
        generations::{self, Generation},
        messages::{self, Message},
        sessions::{self, Session},
    },
    ollama::{GenerateChunk, GenerateRequest, HealthStatus, ModelInfo, StreamOutcome},
};
use serde::{Deserialize, Serialize};
use std::time::Instant;
use tauri::{AppHandle, Emitter, State};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChatStreamEvent {
    pub session_id: String,
    pub user_message_id: String,
    pub assistant_message_id: String,
    pub generation_id: String,
    pub content: String,
    pub thinking: Option<String>,
    pub done: bool,
    pub status: String, // "generating", "completed", "cancelled", "failed"
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatStart {
    pub session: Session,
    pub user_message: Message,
    pub assistant_message: Message,
    pub generation: Generation,
}

#[tauri::command]
pub fn create_session(
    database: State<'_, AppDatabase>,
    cache: State<'_, AppSessionCache>,
    model_name: String,
    think: Option<bool>,
    title: Option<String>,
    system_prompt: Option<String>,
) -> Result<Session, String> {
    let trimmed_model = model_name.trim();
    if trimmed_model.is_empty() {
        return Err("Model name cannot be empty".to_string());
    }

    let connection = database
        .0
        .lock()
        .map_err(|error| format!("Database lock failed: {error}"))?;

    let mut session = sessions::create_session(&connection, trimmed_model, think)
        .map_err(|error| format!("Failed to create session: {error}"))?;

    if title.is_some() || system_prompt.is_some() {
        session.title = title;
        session.system_prompt = system_prompt;
        sessions::update_session(&connection, &session)
            .map_err(|error| format!("Failed to set session details: {error}"))?;
    }

    cache.0.insert(session.clone());

    Ok(session)
}

#[tauri::command]
pub fn get_session(
    database: State<'_, AppDatabase>,
    cache: State<'_, AppSessionCache>,
    session_id: String,
) -> Result<Option<Session>, String> {
    let connection = database
        .0
        .lock()
        .map_err(|error| format!("Database lock failed: {error}"))?;

    cache
        .0
        .get_or_restore(&connection, &session_id)
        .map_err(|error| format!("Failed to retrieve session: {error}"))
}

#[tauri::command]
pub fn list_sessions(database: State<'_, AppDatabase>) -> Result<Vec<Session>, String> {
    let connection = database
        .0
        .lock()
        .map_err(|error| format!("Database lock failed: {error}"))?;

    sessions::list_sessions(&connection)
        .map_err(|error| format!("Failed to list sessions: {error}"))
}

#[tauri::command]
pub fn update_session(
    database: State<'_, AppDatabase>,
    cache: State<'_, AppSessionCache>,
    session_id: String,
    title: Option<String>,
    model_name: Option<String>,
    think: Option<bool>,
    system_prompt: Option<String>,
) -> Result<Session, String> {
    let connection = database
        .0
        .lock()
        .map_err(|error| format!("Database lock failed: {error}"))?;

    let mut session = cache
        .0
        .get_or_restore(&connection, &session_id)
        .map_err(|error| format!("Database error: {error}"))?
        .ok_or_else(|| format!("Session not found: {session_id}"))?;

    if let Some(m) = model_name {
        let trimmed = m.trim();
        if trimmed.is_empty() {
            return Err("Model name cannot be empty".to_string());
        }
        session.model_name = trimmed.to_string();
    }
    if title.is_some() {
        session.title = title;
    }
    if think.is_some() {
        session.think = think;
    }
    if system_prompt.is_some() {
        session.system_prompt = system_prompt;
    }

    sessions::update_session(&connection, &session)
        .map_err(|error| format!("Failed to update session: {error}"))?;

    cache.0.update(session.clone());

    Ok(session)
}

#[tauri::command]
pub fn create_branch_session(
    database: State<'_, AppDatabase>,
    cache: State<'_, AppSessionCache>,
    parent_session_id: String,
    message_id: String,
    title: Option<String>,
) -> Result<Session, String> {
    let connection = database
        .0
        .lock()
        .map_err(|error| format!("Database lock failed: {error}"))?;

    let parent_session = cache
        .0
        .get_or_restore(&connection, &parent_session_id)
        .map_err(|error| format!("Database error: {error}"))?
        .ok_or_else(|| format!("Parent session not found: {parent_session_id}"))?;

    let parent_messages = messages::get_session_messages(&connection, &parent_session_id)
        .map_err(|error| format!("Failed to load parent messages: {error}"))?;

    let mut branch_messages = Vec::new();
    let mut found = false;
    for msg in parent_messages {
        branch_messages.push(msg.clone());
        if msg.id == message_id {
            found = true;
            break;
        }
    }

    if !found {
        return Err(format!("Message not found in session: {message_id}"));
    }

    let branch_title = title.unwrap_or_else(|| {
        format!(
            "Branch: {}",
            parent_session.title.as_deref().unwrap_or("Chat")
        )
    });

    let branch_metadata = serde_json::json!({
        "parent_session_id": parent_session_id,
        "branched_from_message_id": message_id,
        "is_branch": true,
    })
    .to_string();

    let mut new_session = sessions::create_session(
        &connection,
        &parent_session.model_name,
        parent_session.think,
    )
    .map_err(|error| format!("Failed to create branch session: {error}"))?;

    new_session.title = Some(branch_title);
    new_session.system_prompt = parent_session.system_prompt.clone();
    new_session.options_json = Some(branch_metadata);

    sessions::update_session(&connection, &new_session)
        .map_err(|error| format!("Failed to set branch details: {error}"))?;

    for msg in branch_messages {
        messages::create_message(
            &connection,
            &new_session.id,
            &msg.role,
            &msg.content,
            msg.thinking_content.as_deref(),
            &msg.status,
        )
        .map_err(|error| format!("Failed to copy message into branch: {error}"))?;
    }

    cache.0.insert(new_session.clone());

    Ok(new_session)
}

#[tauri::command]
pub fn delete_session(
    database: State<'_, AppDatabase>,
    cache: State<'_, AppSessionCache>,
    gen_manager: State<'_, AppGenerationManager>,
    session_id: String,
) -> Result<bool, String> {
    // Cancel any running generation for this session
    gen_manager.0.cancel_session(&session_id);

    let connection = database
        .0
        .lock()
        .map_err(|error| format!("Database lock failed: {error}"))?;

    cache.0.remove(&session_id);

    sessions::delete_session(&connection, &session_id)
        .map_err(|error| format!("Failed to delete session: {error}"))
}

#[tauri::command]
pub fn get_session_messages(
    database: State<'_, AppDatabase>,
    cache: State<'_, AppSessionCache>,
    session_id: String,
) -> Result<Vec<Message>, String> {
    let connection = database
        .0
        .lock()
        .map_err(|error| format!("Database lock failed: {error}"))?;

    cache.0.touch(&session_id);

    messages::get_session_messages(&connection, &session_id)
        .map_err(|error| format!("Failed to retrieve messages: {error}"))
}

#[tauri::command]
pub fn get_generation(
    database: State<'_, AppDatabase>,
    generation_id: String,
) -> Result<Option<Generation>, String> {
    let connection = database
        .0
        .lock()
        .map_err(|error| format!("Database lock failed: {error}"))?;

    generations::get_generation(&connection, &generation_id)
        .map_err(|error| format!("Failed to retrieve generation: {error}"))
}

#[tauri::command]
pub async fn check_ollama_health(
    client: State<'_, AppOllamaClient>,
) -> Result<HealthStatus, String> {
    Ok(client.0.check_health().await)
}

#[tauri::command]
pub async fn list_ollama_models(
    client: State<'_, AppOllamaClient>,
) -> Result<Vec<ModelInfo>, String> {
    client.0.list_models().await.map_err(|err| err.to_string())
}

#[tauri::command]
pub fn cancel_generation(
    gen_manager: State<'_, AppGenerationManager>,
    generation_id: String,
) -> Result<bool, String> {
    Ok(gen_manager.0.cancel(&generation_id))
}

#[tauri::command]
pub fn cancel_session(
    gen_manager: State<'_, AppGenerationManager>,
    session_id: String,
) -> Result<bool, String> {
    Ok(gen_manager.0.cancel_session(&session_id))
}

#[tauri::command]
pub async fn generate_chat(
    app: AppHandle,
    database: State<'_, AppDatabase>,
    cache: State<'_, AppSessionCache>,
    gen_manager: State<'_, AppGenerationManager>,
    client: State<'_, AppOllamaClient>,
    model_name: String,
    prompt: String,
    session_id: Option<String>,
    think: Option<bool>,
) -> Result<ChatStart, String> {
    let trimmed_model = model_name.trim();
    if trimmed_model.is_empty() {
        return Err("Model name cannot be empty".to_string());
    }

    let trimmed_prompt = prompt.trim();
    if trimmed_prompt.is_empty() {
        return Err("Prompt cannot be empty".to_string());
    }

    // Step 1: Session resolution, message creation, and concurrency check
    let (session, history, user_message, assistant_message, generation, cancel_flag) = {
        let connection = database
            .0
            .lock()
            .map_err(|error| format!("Database lock failed: {error}"))?;

        let session = match &session_id {
            Some(id) => cache
                .0
                .get_or_restore(&connection, id)
                .map_err(|error| format!("Failed to retrieve session: {error}"))?
                .ok_or_else(|| format!("Session not found: {id}"))?,
            None => {
                let s = sessions::create_session(&connection, trimmed_model, think)
                    .map_err(|error| format!("Failed to create session: {error}"))?;
                cache.0.insert(s.clone());
                s
            }
        };

        let placeholder_user = messages::create_message(
            &connection,
            &session.id,
            "user",
            trimmed_prompt,
            None,
            "completed",
        )
        .map_err(|error| format!("Failed to save user message: {error}"))?;

        let placeholder_assistant = messages::create_message(
            &connection,
            &session.id,
            "assistant",
            "",
            None,
            "generating",
        )
        .map_err(|error| format!("Failed to create assistant message: {error}"))?;

        let generation = generations::create_generation(
            &connection,
            &placeholder_assistant.id,
            &session.model_name,
        )
        .map_err(|error| format!("Failed to create generation record: {error}"))?;

        // Prevent conflicting simultaneous generations in the same session
        cache
            .0
            .try_acquire_generation(&session.id, &generation.id)
            .map_err(|err| {
                // Clean up placeholder records on conflict
                let _ =
                    connection.execute("DELETE FROM generations WHERE id = ?1", [&generation.id]);
                let _ = connection.execute(
                    "DELETE FROM messages WHERE id = ?1",
                    [&placeholder_assistant.id],
                );
                let _ = connection
                    .execute("DELETE FROM messages WHERE id = ?1", [&placeholder_user.id]);
                err
            })?;

        let cancel_flag = gen_manager.0.register(&session.id, &generation.id);

        let history = messages::get_session_messages(&connection, &session.id)
            .map_err(|error| format!("Failed to load history: {error}"))?;

        (
            session,
            history,
            placeholder_user,
            placeholder_assistant,
            generation,
            cancel_flag,
        )
    };

    // Step 2: Reconstruct conversation prompt
    let mut full_prompt = String::new();

    if let Some(system_prompt) = &session.system_prompt {
        if !system_prompt.trim().is_empty() {
            full_prompt.push_str("System: ");
            full_prompt.push_str(system_prompt);
            full_prompt.push_str("\n\n");
        }
    }

    for message in &history {
        // Exclude unfinished or current placeholder messages
        if message.status != "completed" || message.id == assistant_message.id {
            continue;
        }

        let role = match message.role.as_str() {
            "user" => "User",
            "assistant" => "Assistant",
            "system" => "System",
            _ => continue,
        };

        full_prompt.push_str(role);
        full_prompt.push_str(": ");
        full_prompt.push_str(&message.content);
        full_prompt.push_str("\n\n");
    }

    if !full_prompt.ends_with(&format!("User: {trimmed_prompt}\n\n")) {
        full_prompt.push_str("User: ");
        full_prompt.push_str(trimmed_prompt);
        full_prompt.push_str("\n\n");
    }
    full_prompt.push_str("Assistant: ");

    let effective_think = think.or(session.think);
    let started = Instant::now();

    let mut response_text = String::new();
    let mut thinking_text = String::new();
    let mut first_token_ms: Option<i64> = None;
    let mut prompt_tokens: Option<i64> = None;
    let mut completion_tokens: Option<i64> = None;

    // Step 3: Stream chunks from Ollama with cancellation support
    let stream_result = client
        .0
        .generate_stream_cancellable(
            GenerateRequest {
                model: session.model_name.clone(),
                prompt: full_prompt,
                think: effective_think,
            },
            Some(cancel_flag),
            |chunk: GenerateChunk| {
                if first_token_ms.is_none()
                    && (!chunk.response.is_empty()
                        || chunk.thinking.as_deref().is_some_and(|s| !s.is_empty()))
                {
                    first_token_ms = Some(started.elapsed().as_millis() as i64);
                }
                if chunk.prompt_eval_count.is_some() {
                    prompt_tokens = chunk.prompt_eval_count;
                }
                if chunk.eval_count.is_some() {
                    completion_tokens = chunk.eval_count;
                }

                response_text.push_str(&chunk.response);

                if let Some(thinking) = &chunk.thinking {
                    thinking_text.push_str(thinking);
                }

                // Intermediate stream event: done is always false.
                // The single terminal event is emitted after DB persistence.
                let _ = app.emit(
                    "chat-stream",
                    ChatStreamEvent {
                        session_id: session.id.clone(),
                        user_message_id: user_message.id.clone(),
                        assistant_message_id: assistant_message.id.clone(),
                        generation_id: generation.id.clone(),
                        content: chunk.response,
                        thinking: chunk.thinking,
                        done: false,
                        status: "generating".to_string(),
                        error: None,
                    },
                );
            },
        )
        .await;

    let duration_ms = started.elapsed().as_millis() as i64;

    // Step 4: Cleanup active generation & release session concurrency lock
    gen_manager.0.unregister(&generation.id);
    cache.0.release_generation(&session.id, &generation.id);
    cache.0.touch(&session.id);

    // Step 5: Final state persistence and single terminal event emission
    let connection = database
        .0
        .lock()
        .map_err(|error| format!("Database lock failed: {error}"))?;

    let final_thinking = if thinking_text.is_empty() {
        None
    } else {
        Some(thinking_text.as_str())
    };

    match stream_result {
        Ok(StreamOutcome::Completed) => {
            messages::finish_message(
                &connection,
                &assistant_message.id,
                &response_text,
                final_thinking,
                "completed",
            )
            .map_err(|error| format!("Failed to save assistant response: {error}"))?;

            generations::finish_generation(
                &connection,
                &generation.id,
                "completed",
                first_token_ms,
                Some(duration_ms),
                prompt_tokens,
                completion_tokens,
                None,
                None,
            )
            .map_err(|error| format!("Failed to save generation metrics: {error}"))?;

            sessions::update_session_timestamp(&connection, &session.id).ok();

            // Emit the single terminal event
            let _ = app.emit(
                "chat-stream",
                ChatStreamEvent {
                    session_id: session.id.clone(),
                    user_message_id: user_message.id.clone(),
                    assistant_message_id: assistant_message.id.clone(),
                    generation_id: generation.id.clone(),
                    content: String::new(),
                    thinking: None,
                    done: true,
                    status: "completed".to_string(),
                    error: None,
                },
            );

            Ok(ChatStart {
                session,
                user_message,
                assistant_message,
                generation,
            })
        }
        Ok(StreamOutcome::Cancelled) => {
            messages::finish_message(
                &connection,
                &assistant_message.id,
                &response_text,
                final_thinking,
                "cancelled",
            )
            .map_err(|error| format!("Failed to update cancelled message: {error}"))?;

            generations::finish_generation(
                &connection,
                &generation.id,
                "cancelled",
                first_token_ms,
                Some(duration_ms),
                prompt_tokens,
                completion_tokens,
                Some("CANCELLED"),
                Some("Generation was cancelled"),
            )
            .map_err(|error| format!("Failed to update cancelled generation: {error}"))?;

            sessions::update_session_timestamp(&connection, &session.id).ok();

            // Emit the single terminal event
            let _ = app.emit(
                "chat-stream",
                ChatStreamEvent {
                    session_id: session.id.clone(),
                    user_message_id: user_message.id.clone(),
                    assistant_message_id: assistant_message.id.clone(),
                    generation_id: generation.id.clone(),
                    content: String::new(),
                    thinking: None,
                    done: true,
                    status: "cancelled".to_string(),
                    error: Some("Generation was cancelled".to_string()),
                },
            );

            Err("Generation was cancelled".to_string())
        }
        Err(err) => {
            let error_message = err.to_string();

            messages::finish_message(
                &connection,
                &assistant_message.id,
                &response_text,
                final_thinking,
                "failed",
            )
            .map_err(|db_error| {
                format!("Generation failed ({error_message}); failed to save message: {db_error}")
            })?;

            generations::finish_generation(
                &connection,
                &generation.id,
                "failed",
                first_token_ms,
                Some(duration_ms),
                prompt_tokens,
                completion_tokens,
                Some("OLLAMA_ERROR"),
                Some(&error_message),
            )
            .map_err(|db_error| {
                format!("Generation failed ({error_message}); failed to save metrics: {db_error}")
            })?;

            sessions::update_session_timestamp(&connection, &session.id).ok();

            // Emit the single terminal event
            let _ = app.emit(
                "chat-stream",
                ChatStreamEvent {
                    session_id: session.id.clone(),
                    user_message_id: user_message.id.clone(),
                    assistant_message_id: assistant_message.id.clone(),
                    generation_id: generation.id.clone(),
                    content: String::new(),
                    thinking: None,
                    done: true,
                    status: "failed".to_string(),
                    error: Some(error_message.clone()),
                },
            );

            Err(error_message)
        }
    }
}
