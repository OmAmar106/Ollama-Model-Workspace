use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

pub const DEFAULT_OLLAMA_URL: &str = "http://127.0.0.1:11434";

pub fn resolve_base_url() -> String {
    if let Ok(url) = std::env::var("OLLAMA_BASE_URL") {
        let trimmed = url.trim();
        if !trimmed.is_empty() {
            return trimmed.trim_end_matches('/').to_string();
        }
    }
    if let Ok(host) = std::env::var("OLLAMA_HOST") {
        let trimmed = host.trim();
        if !trimmed.is_empty() {
            if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
                return trimmed.trim_end_matches('/').to_string();
            } else {
                return format!("http://{}", trimmed.trim_end_matches('/'));
            }
        }
    }
    DEFAULT_OLLAMA_URL.to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum OllamaErrorKind {
    ConnectionFailed,
    Timeout,
    HttpError,
    MalformedResponse,
    ModelNotFound,
    StreamError,
    InvalidRequest,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OllamaError {
    pub kind: OllamaErrorKind,
    pub message: String,
    pub status_code: Option<u16>,
}

impl OllamaError {
    pub fn new(kind: OllamaErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            status_code: None,
        }
    }

    pub fn http(status_code: u16, message: impl Into<String>) -> Self {
        Self {
            kind: OllamaErrorKind::HttpError,
            message: message.into(),
            status_code: Some(status_code),
        }
    }

    pub fn cancelled() -> Self {
        Self {
            kind: OllamaErrorKind::Cancelled,
            message: "Generation was cancelled".to_string(),
            status_code: None,
        }
    }
}

impl std::fmt::Display for OllamaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for OllamaError {}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HealthStatus {
    pub is_healthy: bool,
    pub version: Option<String>,
    pub base_url: String,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct ModelDetails {
    #[serde(default)]
    pub parent_model: Option<String>,
    #[serde(default)]
    pub format: Option<String>,
    #[serde(default)]
    pub family: Option<String>,
    #[serde(default)]
    pub families: Option<Vec<String>>,
    #[serde(default)]
    pub parameter_size: Option<String>,
    #[serde(default)]
    pub quantization_level: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModelInfo {
    pub name: String,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub modified_at: Option<String>,
    #[serde(default)]
    pub size: Option<u64>,
    #[serde(default)]
    pub digest: Option<String>,
    #[serde(default)]
    pub details: Option<ModelDetails>,
}

#[derive(Debug, Clone, Deserialize, Default)]
struct TagsResponse {
    #[serde(default)]
    pub models: Vec<ModelInfo>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GenerateRequest {
    pub model: String,
    pub prompt: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub think: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GenerateChunk {
    #[serde(default)]
    pub response: String,
    #[serde(default)]
    pub thinking: Option<String>,
    #[serde(default)]
    pub done: bool,
    #[serde(default)]
    pub error: Option<String>,
    #[serde(default)]
    pub prompt_eval_count: Option<i64>,
    #[serde(default)]
    pub eval_count: Option<i64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamOutcome {
    Completed,
    Cancelled,
}

#[derive(Default)]
struct StreamDecoder {
    buffer: Vec<u8>,
}

impl StreamDecoder {
    fn push(&mut self, bytes: &[u8]) -> serde_json::Result<Vec<GenerateChunk>> {
        self.buffer.extend_from_slice(bytes);

        let mut chunks = Vec::new();

        while let Some(newline) = self.buffer.iter().position(|byte| *byte == b'\n') {
            let mut line: Vec<u8> = self.buffer.drain(..=newline).collect();
            line.pop(); // Remove newline.

            if line.last() == Some(&b'\r') {
                line.pop();
            }

            if line.iter().all(u8::is_ascii_whitespace) {
                continue;
            }

            chunks.push(serde_json::from_slice(&line)?);
        }

        Ok(chunks)
    }

    fn finish(&mut self) -> serde_json::Result<Option<GenerateChunk>> {
        if self.buffer.iter().all(u8::is_ascii_whitespace) {
            self.buffer.clear();
            return Ok(None);
        }

        let remaining = std::mem::take(&mut self.buffer);
        serde_json::from_slice(&remaining).map(Some)
    }
}

pub struct OllamaClient {
    base_url: String,
    http: reqwest::Client,
}

impl Default for OllamaClient {
    fn default() -> Self {
        Self::new()
    }
}

impl OllamaClient {
    pub fn new() -> Self {
        Self::with_base_url(resolve_base_url())
    }

    pub fn with_base_url(base_url: impl Into<String>) -> Self {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(120))
            .connect_timeout(Duration::from_secs(5))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self {
            base_url: base_url.into().trim_end_matches('/').to_string(),
            http,
        }
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    pub async fn check_health(&self) -> HealthStatus {
        let url = format!("{}/api/version", self.base_url);
        let request = self
            .http
            .get(&url)
            .timeout(Duration::from_secs(3))
            .send()
            .await;

        match request {
            Ok(response) => {
                let status = response.status();
                if status.is_success() {
                    #[derive(Deserialize)]
                    struct VersionResp {
                        version: Option<String>,
                    }
                    let version = response
                        .json::<VersionResp>()
                        .await
                        .ok()
                        .and_then(|v| v.version);
                    HealthStatus {
                        is_healthy: true,
                        version,
                        base_url: self.base_url.clone(),
                        error: None,
                    }
                } else {
                    HealthStatus {
                        is_healthy: false,
                        version: None,
                        base_url: self.base_url.clone(),
                        error: Some(format!("Ollama server returned HTTP {status}")),
                    }
                }
            }
            Err(err) => {
                // Try fallback to GET / in case /api/version isn't available
                let fallback = self
                    .http
                    .get(&self.base_url)
                    .timeout(Duration::from_secs(2))
                    .send()
                    .await;

                match fallback {
                    Ok(fb_resp) if fb_resp.status().is_success() => HealthStatus {
                        is_healthy: true,
                        version: None,
                        base_url: self.base_url.clone(),
                        error: None,
                    },
                    _ => HealthStatus {
                        is_healthy: false,
                        version: None,
                        base_url: self.base_url.clone(),
                        error: Some(format!(
                            "Could not connect to Ollama at {}: {err}",
                            self.base_url
                        )),
                    },
                }
            }
        }
    }

    pub async fn list_models(&self) -> Result<Vec<ModelInfo>, OllamaError> {
        let url = format!("{}/api/tags", self.base_url);
        let response = self
            .http
            .get(&url)
            .timeout(Duration::from_secs(10))
            .send()
            .await
            .map_err(|err| {
                if err.is_timeout() {
                    OllamaError::new(
                        OllamaErrorKind::Timeout,
                        format!(
                            "Timeout while listing models from Ollama at {}",
                            self.base_url
                        ),
                    )
                } else {
                    OllamaError::new(
                        OllamaErrorKind::ConnectionFailed,
                        format!("Failed to connect to Ollama at {}: {err}", self.base_url),
                    )
                }
            })?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(OllamaError::http(
                status.as_u16(),
                format!("Ollama returned HTTP {status}: {error_text}"),
            ));
        }

        let body_bytes = response.bytes().await.map_err(|err| {
            OllamaError::new(
                OllamaErrorKind::MalformedResponse,
                format!("Failed to read response body: {err}"),
            )
        })?;

        if body_bytes.is_empty() {
            return Ok(Vec::new());
        }

        let parsed: TagsResponse = serde_json::from_slice(&body_bytes).map_err(|err| {
            OllamaError::new(
                OllamaErrorKind::MalformedResponse,
                format!("Failed to parse models JSON response from Ollama: {err}"),
            )
        })?;

        Ok(parsed.models)
    }

    pub async fn validate_model(&self, model_name: &str) -> Result<bool, OllamaError> {
        let trimmed = model_name.trim();
        if trimmed.is_empty() {
            return Err(OllamaError::new(
                OllamaErrorKind::InvalidRequest,
                "Model name cannot be empty",
            ));
        }

        let models = self.list_models().await?;
        let exists = models.iter().any(|m| {
            m.name == trimmed
                || m.model.as_deref() == Some(trimmed)
                || m.name.starts_with(&format!("{trimmed}:"))
        });

        Ok(exists)
    }

    pub async fn generate_stream<F>(
        &self,
        request: GenerateRequest,
        on_chunk: F,
    ) -> Result<StreamOutcome, OllamaError>
    where
        F: FnMut(GenerateChunk) + Send,
    {
        self.generate_stream_cancellable(request, None, on_chunk)
            .await
    }

    pub async fn generate_stream_cancellable<F>(
        &self,
        request: GenerateRequest,
        cancel_flag: Option<Arc<AtomicBool>>,
        mut on_chunk: F,
    ) -> Result<StreamOutcome, OllamaError>
    where
        F: FnMut(GenerateChunk) + Send,
    {
        if request.model.trim().is_empty() {
            return Err(OllamaError::new(
                OllamaErrorKind::InvalidRequest,
                "Model name cannot be empty",
            ));
        }
        if request.prompt.trim().is_empty() {
            return Err(OllamaError::new(
                OllamaErrorKind::InvalidRequest,
                "Prompt cannot be empty",
            ));
        }

        let url = format!("{}/api/generate", self.base_url);
        let mut body = json!({
            "model": request.model,
            "prompt": request.prompt,
            "stream": true
        });

        if let Some(think) = request.think {
            body["think"] = json!(think);
        }

        let response = self
            .http
            .post(url)
            .json(&body)
            .send()
            .await
            .map_err(|err| {
                if err.is_timeout() {
                    OllamaError::new(
                        OllamaErrorKind::Timeout,
                        format!(
                            "Timeout starting generation with Ollama at {}",
                            self.base_url
                        ),
                    )
                } else {
                    OllamaError::new(
                        OllamaErrorKind::ConnectionFailed,
                        format!("Failed to connect to Ollama at {}: {err}", self.base_url),
                    )
                }
            })?;

        let status = response.status();
        if !status.is_success() {
            let error_body = response.text().await.unwrap_or_default();
            return Err(OllamaError::http(
                status.as_u16(),
                format!("Ollama returned HTTP {status}: {error_body}"),
            ));
        }

        let mut stream = response.bytes_stream();
        let mut decoder = StreamDecoder::default();

        while let Some(result) = stream.next().await {
            // Check cancellation before processing next chunk
            if let Some(cancel) = &cancel_flag {
                if cancel.load(Ordering::Relaxed) {
                    return Ok(StreamOutcome::Cancelled);
                }
            }

            let bytes = result.map_err(|err| {
                OllamaError::new(
                    OllamaErrorKind::StreamError,
                    format!("Network stream error: {err}"),
                )
            })?;

            let chunks = decoder.push(&bytes).map_err(|err| {
                OllamaError::new(
                    OllamaErrorKind::MalformedResponse,
                    format!("Malformed JSON chunk from Ollama: {err}"),
                )
            })?;

            for chunk in chunks {
                if let Some(cancel) = &cancel_flag {
                    if cancel.load(Ordering::Relaxed) {
                        return Ok(StreamOutcome::Cancelled);
                    }
                }

                if let Some(error) = &chunk.error {
                    return Err(OllamaError::new(
                        OllamaErrorKind::StreamError,
                        error.clone(),
                    ));
                }

                let done = chunk.done;
                on_chunk(chunk);

                if let Some(cancel) = &cancel_flag {
                    if cancel.load(Ordering::Relaxed) {
                        return Ok(StreamOutcome::Cancelled);
                    }
                }

                if done {
                    return Ok(StreamOutcome::Completed);
                }
            }

            if let Some(cancel) = &cancel_flag {
                if cancel.load(Ordering::Relaxed) {
                    return Ok(StreamOutcome::Cancelled);
                }
            }
        }

        if let Some(chunk) = decoder.finish().map_err(|err| {
            OllamaError::new(
                OllamaErrorKind::MalformedResponse,
                format!("Malformed JSON in final chunk: {err}"),
            )
        })? {
            if let Some(cancel) = &cancel_flag {
                if cancel.load(Ordering::Relaxed) {
                    return Ok(StreamOutcome::Cancelled);
                }
            }

            if let Some(error) = &chunk.error {
                return Err(OllamaError::new(
                    OllamaErrorKind::StreamError,
                    error.clone(),
                ));
            }

            let done = chunk.done;
            on_chunk(chunk);

            if let Some(cancel) = &cancel_flag {
                if cancel.load(Ordering::Relaxed) {
                    return Ok(StreamOutcome::Cancelled);
                }
            }

            if done {
                return Ok(StreamOutcome::Completed);
            }
        }

        // If cancelled at end of stream:
        if let Some(cancel) = &cancel_flag {
            if cancel.load(Ordering::Relaxed) {
                return Ok(StreamOutcome::Cancelled);
            }
        }

        Err(OllamaError::new(
            OllamaErrorKind::StreamError,
            "Ollama stream ended before a completion signal was received",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use tiny_http::{Response, Server};

    #[test]
    fn handles_json_split_across_network_chunks() {
        let mut decoder = StreamDecoder::default();

        let first = decoder
            .push(br#"{"response":"Hel"#)
            .expect("partial input should be accepted");

        assert!(first.is_empty());

        let second = decoder
            .push(br#"lo","done":false}"#)
            .expect("partial JSON should be completed");

        assert!(second.is_empty());

        let third = decoder.push(b"\n").expect("completed JSON should parse");

        assert_eq!(third.len(), 1);
        assert_eq!(third[0].response, "Hello");
        assert!(!third[0].done);
    }

    #[test]
    fn parses_final_json_without_trailing_newline() {
        let mut decoder = StreamDecoder::default();

        let chunks = decoder
            .push(br#"{"response":"Hello","done":true}"#)
            .expect("JSON should be buffered");

        assert!(chunks.is_empty());

        let final_chunk = decoder
            .finish()
            .expect("final JSON should parse")
            .expect("final chunk should exist");

        assert_eq!(final_chunk.response, "Hello");
        assert!(final_chunk.done);
    }

    #[test]
    fn parses_ollama_error_response() {
        let mut decoder = StreamDecoder::default();

        let chunks = decoder
            .push(br#"{"error":"model not found"}"#)
            .expect("JSON should be buffered");

        assert!(chunks.is_empty());

        let chunk = decoder
            .finish()
            .expect("error response JSON should parse")
            .expect("error response should exist");

        assert_eq!(chunk.error.as_deref(), Some("model not found"));
    }

    #[test]
    fn parses_multiple_json_lines_from_one_network_chunk() {
        let mut decoder = StreamDecoder::default();

        let chunks = decoder
            .push(
                b"{\"response\":\"A\",\"done\":false}\n\
                  {\"response\":\"B\",\"done\":true}\n",
            )
            .expect("both JSON lines should parse");

        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].response, "A");
        assert_eq!(chunks[1].response, "B");
        assert!(chunks[1].done);
    }

    fn mock_server(
        response_status: u16,
        response_body: &'static str,
    ) -> (String, thread::JoinHandle<String>) {
        let server = Server::http("127.0.0.1:0").expect("failed to start mock server");
        let address = format!("http://{}", server.server_addr());
        let handle = thread::spawn(move || {
            let mut request = server.recv().expect("server did not receive request");
            let mut body = String::new();
            let reader = request.as_reader();
            reader
                .read_to_string(&mut body)
                .expect("failed to read request body");

            let response = Response::from_string(response_body)
                .with_status_code(response_status)
                .with_header(
                    tiny_http::Header::from_bytes("Content-Type", "application/json")
                        .expect("valid header"),
                );

            request
                .respond(response)
                .expect("failed to send mock response");
            body
        });

        (address, handle)
    }

    #[tokio::test]
    async fn sends_generation_request_and_reads_stream() {
        let (address, server_handle) = mock_server(
            200,
            "{\"response\":\"Hello\",\"done\":false}\n\
             {\"response\":\" world\",\"done\":true}\n",
        );

        let client = OllamaClient::with_base_url(address);
        let mut received = Vec::new();

        let outcome = client
            .generate_stream(
                GenerateRequest {
                    model: "test-model".to_string(),
                    prompt: "Say hello".to_string(),
                    think: Some(false),
                },
                |chunk| received.push((chunk.response, chunk.done)),
            )
            .await
            .expect("generation should succeed");

        assert_eq!(outcome, StreamOutcome::Completed);

        let request_body = server_handle.join().expect("mock server thread failed");
        let json: serde_json::Value =
            serde_json::from_str(&request_body).expect("request body should be JSON");

        assert_eq!(json["model"], "test-model");
        assert_eq!(json["prompt"], "Say hello");
        assert_eq!(json["stream"], true);
        assert_eq!(json["think"], false);

        assert_eq!(received.len(), 2);
        assert_eq!(received[0], ("Hello".to_string(), false));
        assert_eq!(received[1], (" world".to_string(), true));
    }

    #[tokio::test]
    async fn reports_http_errors() {
        let (address, server_handle) = mock_server(500, "mock server failure");
        let client = OllamaClient::with_base_url(address);

        let result = client
            .generate_stream(
                GenerateRequest {
                    model: "test-model".to_string(),
                    prompt: "test".to_string(),
                    think: None,
                },
                |_| {},
            )
            .await;

        let _ = server_handle.join().expect("mock server thread failed");

        let error = result.expect_err("HTTP 500 should return an error");
        assert_eq!(error.kind, OllamaErrorKind::HttpError);
        assert_eq!(error.status_code, Some(500));
        assert!(error.to_string().contains("mock server failure"));
    }

    #[tokio::test]
    async fn checks_health_successfully() {
        let (address, _handle) = mock_server(200, "{\"version\":\"0.3.14\"}");
        let client = OllamaClient::with_base_url(address);

        let health = client.check_health().await;
        assert!(health.is_healthy);
        assert_eq!(health.version.as_deref(), Some("0.3.14"));
        assert!(health.error.is_none());
    }

    #[tokio::test]
    async fn checks_health_failure_on_non_200() {
        let (address, _handle) = mock_server(503, "Service Unavailable");
        let client = OllamaClient::with_base_url(address);

        let health = client.check_health().await;
        assert!(!health.is_healthy);
        assert!(health.error.is_some());
    }

    #[tokio::test]
    async fn lists_models_and_parses_metadata() {
        let json_body = r#"{
            "models": [
                {
                    "name": "llama3.2:latest",
                    "model": "llama3.2:latest",
                    "size": 2019393189,
                    "details": {
                        "format": "gguf",
                        "family": "llama",
                        "parameter_size": "3.2B",
                        "quantization_level": "Q4_K_M"
                    }
                }
            ]
        }"#;

        let (address, _handle) = mock_server(200, json_body);
        let client = OllamaClient::with_base_url(address);

        let models = client.list_models().await.expect("should list models");
        assert_eq!(models.len(), 1);
        assert_eq!(models[0].name, "llama3.2:latest");
        assert_eq!(models[0].size, Some(2019393189));

        let details = models[0].details.as_ref().unwrap();
        assert_eq!(details.family.as_deref(), Some("llama"));
        assert_eq!(details.parameter_size.as_deref(), Some("3.2B"));
        assert_eq!(details.quantization_level.as_deref(), Some("Q4_K_M"));
    }

    #[tokio::test]
    async fn handles_empty_or_malformed_model_responses() {
        // Empty JSON object
        let (address, _handle) = mock_server(200, "{}");
        let client = OllamaClient::with_base_url(address);
        let models = client.list_models().await.expect("empty models array");
        assert!(models.is_empty());

        // Malformed JSON
        let (address2, _handle2) = mock_server(200, "not valid json {[[");
        let client2 = OllamaClient::with_base_url(address2);
        let err = client2
            .list_models()
            .await
            .expect_err("should fail on malformed json");
        assert_eq!(err.kind, OllamaErrorKind::MalformedResponse);
    }

    #[tokio::test]
    async fn validates_requested_models() {
        let json_body = r#"{
            "models": [
                { "name": "llama3.2:latest" },
                { "name": "mistral:latest" }
            ]
        }"#;

        let (address, _handle) = mock_server(200, json_body);
        let client = OllamaClient::with_base_url(address);

        assert!(client.validate_model("llama3.2").await.unwrap());
        // Empty model name returns error
        assert!(client.validate_model("   ").await.is_err());
    }

    #[tokio::test]
    async fn supports_cancellation_during_generation() {
        let (address, _handle) = mock_server(
            200,
            "{\"response\":\"One\",\"done\":false}\n\
             {\"response\":\"Two\",\"done\":false}\n\
             {\"response\":\"Three\",\"done\":true}\n",
        );

        let client = OllamaClient::with_base_url(address);
        let cancel_flag = Arc::new(AtomicBool::new(false));
        let cancel_clone = Arc::clone(&cancel_flag);

        let mut chunks_received = 0;
        let outcome = client
            .generate_stream_cancellable(
                GenerateRequest {
                    model: "test-model".to_string(),
                    prompt: "hello".to_string(),
                    think: None,
                },
                Some(cancel_flag),
                |_| {
                    chunks_received += 1;
                    // Trigger cancellation after 1 chunk
                    cancel_clone.store(true, Ordering::Relaxed);
                },
            )
            .await
            .expect("should return cancellation outcome");

        assert_eq!(outcome, StreamOutcome::Cancelled);
        assert!(chunks_received >= 1);
    }
}
