use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::json;

const DEFAULT_OLLAMA_URL: &str = "http://localhost:11434";

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

impl OllamaClient {
    pub fn new() -> Self {
        Self {
            base_url: DEFAULT_OLLAMA_URL.to_string(),
            http: reqwest::Client::new(),
        }
    }

    pub async fn generate_stream<F>(
        &self,
        request: GenerateRequest,
        mut on_chunk: F,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
    where
        F: FnMut(GenerateChunk) + Send,
    {
        let url = format!("{}/api/generate", self.base_url);

        let mut body = json!({
            "model": request.model,
            "prompt": request.prompt,
            "stream": true
        });

        if let Some(think) = request.think {
            body["think"] = json!(think);
        }

        let response = self.http.post(url).json(&body).send().await?;
        let status = response.status();

        if !status.is_success() {
            let error_body = response.text().await?;
            return Err(format!("Ollama returned HTTP {status}: {error_body}").into());
        }

        let mut stream = response.bytes_stream();
        let mut decoder = StreamDecoder::default();

        while let Some(result) = stream.next().await {
            let bytes = result?;

            for chunk in decoder.push(&bytes)? {
                if let Some(error) = &chunk.error {
                    return Err(error.clone().into());
                }

                let done = chunk.done;
                on_chunk(chunk);

                if done {
                    return Ok(());
                }
            }
        }

        if let Some(chunk) = decoder.finish()? {
            if let Some(error) = &chunk.error {
                return Err(error.clone().into());
            }

            let done = chunk.done;
            on_chunk(chunk);

            if done {
                return Ok(());
            }
        }

        Err("Ollama stream ended before a completion signal was received".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;
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

        // No newline yet, so the JSON object is still buffered.
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
            let mut reader = request.as_reader();
            reader
                .read_to_string(&mut body)
                .expect("failed to read request body");

            let response = Response::from_string(response_body)
                .with_status_code(response_status)
                .with_header(
                    tiny_http::Header::from_bytes("Content-Type", "application/x-ndjson")
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

        let client = OllamaClient {
            base_url: address,
            http: reqwest::Client::new(),
        };

        let mut received = Vec::new();

        client
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

        let client = OllamaClient {
            base_url: address,
            http: reqwest::Client::new(),
        };

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
        assert!(error.to_string().contains("500"));
        assert!(error.to_string().contains("mock server failure"));
    }
}
