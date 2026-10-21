use crate::{
    AppDatabase,
    db::{
        generations::{self, Generation},
        messages::{self, Message},
        sessions::{self, Session},
    },
    ollama::{GenerateChunk, GenerateRequest, OllamaClient},
};
use serde::Serialize;
use std::time::Instant;
use tauri::{AppHandle, Emitter, State};

#[derive(Debug, Clone, Serialize)]
pub struct ChatStreamEvent {
    pub session_id: String,
    pub user_message_id: String,
    pub assistant_message_id: String,
    pub generation_id: String,
    pub content: String,
    pub thinking: Option<String>,
    pub done: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChatStart {
    pub session: Session,
    pub user_message: Message,
    pub assistant_message: Message,
    pub generation: Generation,
}

#[tauri::command]
pub fn create_session(
    database: State<'_, AppDatabase>,
    model_name: String,
    think: Option<bool>,
) -> Result<Session, String> {
    let connection = database
        .0
        .lock()
        .map_err(|error| format!("Database lock failed: {error}"))?;

    sessions::create_session(&connection, &model_name, think)
        .map_err(|error| format!("Failed to create session: {error}"))
}

#[tauri::command]
pub fn get_session(
    database: State<'_, AppDatabase>,
    session_id: String,
) -> Result<Option<Session>, String> {
    let connection = database
        .0
        .lock()
        .map_err(|error| format!("Database lock failed: {error}"))?;

    sessions::get_session(&connection, &session_id)
        .map_err(|error| format!("Failed to retrieve session: {error}"))
}

#[tauri::command]
pub async fn generate_chat(
    app: AppHandle,
    database: State<'_, AppDatabase>,
    model_name: String,
    prompt: String,
    session_id: Option<String>,
    think: Option<bool>,
) -> Result<ChatStart, String> {
    if model_name.trim().is_empty() {
        return Err("Model name cannot be empty".to_string());
    }

    if prompt.trim().is_empty() {
        return Err("Prompt cannot be empty".to_string());
    }

    // Prepare the session, history, and database records while holding
    // the lock briefly. Never hold the database mutex during network I/O.
    let (session, history, user_message, assistant_message, generation) = {
        let connection = database
            .0
            .lock()
            .map_err(|error| format!("Database lock failed: {error}"))?;

        let session = match session_id {
            Some(id) => sessions::get_session(&connection, &id)
                .map_err(|error| format!("Failed to retrieve session: {error}"))?
                .ok_or_else(|| format!("Session not found: {id}"))?,
            None => sessions::create_session(&connection, &model_name, think)
                .map_err(|error| format!("Failed to create session: {error}"))?,
        };

        let history = messages::get_session_messages(&connection, &session.id)
            .map_err(|error| format!("Failed to load history: {error}"))?;

        let user_message =
            messages::create_message(&connection, &session.id, "user", &prompt, None, "completed")
                .map_err(|error| format!("Failed to save user message: {error}"))?;

        let assistant_message = messages::create_message(
            &connection,
            &session.id,
            "assistant",
            "",
            None,
            "generating",
        )
        .map_err(|error| format!("Failed to create assistant message: {error}"))?;

        let generation =
            generations::create_generation(&connection, &assistant_message.id, &session.model_name)
                .map_err(|error| format!("Failed to create generation record: {error}"))?;

        (
            session,
            history,
            user_message,
            assistant_message,
            generation,
        )
    };

    // Reconstruct the conversation because /api/generate accepts a prompt,
    // rather than a structured list of chat messages.
    let mut full_prompt = String::new();

    if let Some(system_prompt) = &session.system_prompt {
        if !system_prompt.trim().is_empty() {
            full_prompt.push_str("System: ");
            full_prompt.push_str(system_prompt);
            full_prompt.push_str("\n\n");
        }
    }

    for message in &history {
        if message.status != "completed" {
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

    full_prompt.push_str("User: ");
    full_prompt.push_str(&prompt);
    full_prompt.push_str("\n\nAssistant: ");

    let effective_think = think.or(session.think);
    let client = OllamaClient::new();
    let started = Instant::now();

    let mut response_text = String::new();
    let mut thinking_text = String::new();
    let mut first_token_ms: Option<i64> = None;
    let mut prompt_tokens: Option<i64> = None;
    let mut completion_tokens: Option<i64> = None;

    let stream_result = client
        .generate_stream(
            GenerateRequest {
                model: session.model_name.clone(),
                prompt: full_prompt,
                think: effective_think,
            },
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

                let _ = app.emit(
                    "chat-stream",
                    ChatStreamEvent {
                        session_id: session.id.clone(),
                        user_message_id: user_message.id.clone(),
                        assistant_message_id: assistant_message.id.clone(),
                        generation_id: generation.id.clone(),
                        content: chunk.response,
                        thinking: chunk.thinking,
                        done: chunk.done,
                        error: None,
                    },
                );
            },
        )
        .await;

    let duration_ms = started.elapsed().as_millis() as i64;

    // Persist the final state only after the streaming request finishes.
    let connection = database
        .0
        .lock()
        .map_err(|error| format!("Database lock failed: {error}"))?;

    match stream_result {
        Ok(()) => {
            messages::finish_message(
                &connection,
                &assistant_message.id,
                &response_text,
                if thinking_text.is_empty() {
                    None
                } else {
                    Some(&thinking_text)
                },
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
        }
        Err(error) => {
            let error_message = error.to_string();

            messages::finish_message(
                &connection,
                &assistant_message.id,
                &response_text,
                if thinking_text.is_empty() {
                    None
                } else {
                    Some(&thinking_text)
                },
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
                None,
                None,
                Some("OLLAMA_GENERATION_FAILED"),
                Some(&error_message),
            )
            .map_err(|db_error| {
                format!("Generation failed ({error_message}); failed to save metrics: {db_error}")
            })?;

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
                    error: Some(error_message.clone()),
                },
            );

            return Err(error_message);
        }
    }

    Ok(ChatStart {
        session,
        user_message,
        assistant_message,
        generation,
    })
}
