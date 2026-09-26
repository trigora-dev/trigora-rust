//! HTTP client for the Trigora `/v1` API.
//!
//! A token selects Trigora Cloud. Otherwise the client talks to the local
//! runtime (`trigora dev`). This crate does not compile or author programs.

use std::env;
use std::io::Read;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use serde_json::Value;

const DEFAULT_RUNTIME_URL: &str = "http://127.0.0.1:3477";
const DEFAULT_CLOUD_API_URL: &str = "https://api.trigora.dev";
const PROJECT_HEADER: &str = "X-Trigora-Project-Id";
const RESULT_POLL: Duration = Duration::from_millis(50);

#[derive(Debug)]
pub struct TrigoraError {
    message: String,
    pub status: u16,
    pub code: Option<String>,
}

impl TrigoraError {
    fn new(message: impl Into<String>, status: u16, code: Option<String>) -> Self {
        Self {
            message: message.into(),
            status,
            code,
        }
    }
}

impl std::fmt::Display for TrigoraError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for TrigoraError {}

pub struct ClientOptions {
    pub url: Option<String>,
    pub token: Option<String>,
    pub project_id: Option<String>,
}

impl Default for ClientOptions {
    fn default() -> Self {
        Self {
            url: None,
            token: None,
            project_id: None,
        }
    }
}

#[derive(Clone)]
pub struct Client {
    inner: Arc<Inner>,
}

struct Inner {
    url: String,
    token: Option<String>,
    project_id: Option<String>,
    agent: ureq::Agent,
}

impl Client {
    pub fn new(options: ClientOptions) -> Self {
        let token = options.token.or_else(env_token);
        let url = options
            .url
            .map(|value| value.trim_end_matches('/').to_string())
            .unwrap_or_else(|| base_url(token.as_deref()));
        Self {
            inner: Arc::new(Inner {
                url,
                token,
                project_id: options.project_id,
                agent: ureq::AgentBuilder::new()
                    .timeout(Duration::from_secs(30))
                    .build(),
            }),
        }
    }

    pub fn from_env() -> Self {
        Self::new(ClientOptions::default())
    }

    pub fn whoami(&self) -> Result<Value, TrigoraError> {
        self.request("GET", "/v1/whoami", None)
    }

    pub fn deploy(&self, body: Value) -> Result<Value, TrigoraError> {
        self.request("POST", "/v1/programs/deploy", Some(body))
    }

    pub fn projects(&self) -> Projects {
        Projects {
            client: self.clone(),
        }
    }

    pub fn programs(&self) -> Programs {
        Programs {
            client: self.clone(),
        }
    }

    pub fn executions(&self) -> Executions {
        Executions {
            client: self.clone(),
        }
    }

    fn request(
        &self,
        method: &str,
        path: &str,
        body: Option<Value>,
    ) -> Result<Value, TrigoraError> {
        let url = format!("{}{path}", self.inner.url);
        let mut request = self
            .inner
            .agent
            .request(method, &url)
            .set("Accept", "application/json");
        if let Some(token) = &self.inner.token {
            request = request.set("Authorization", &format!("Bearer {token}"));
        }
        if let Some(project_id) = &self.inner.project_id {
            request = request.set(PROJECT_HEADER, project_id);
        }

        let result = if let Some(body) = body {
            request.send_json(body)
        } else {
            request.call()
        };

        match result {
            Ok(response) => read_json(response.into_reader()),
            Err(ureq::Error::Status(status, response)) => {
                Err(error_from_response(status, response))
            }
            Err(ureq::Error::Transport(error)) => Err(TrigoraError::new(
                format!("Could not reach Trigora at {url}. {error}"),
                0,
                None,
            )),
        }
    }
}

#[derive(Clone)]
pub struct Page {
    pub limit: Option<u64>,
    pub cursor: Option<String>,
}

fn query(page: Option<&Page>) -> String {
    let Some(page) = page else {
        return String::new();
    };
    let mut params = Vec::new();
    if let Some(limit) = page.limit {
        params.push(format!("limit={limit}"));
    }
    if let Some(cursor) = &page.cursor {
        params.push(format!("cursor={}", encode(cursor)));
    }
    if params.is_empty() {
        String::new()
    } else {
        format!("?{}", params.join("&"))
    }
}

pub struct Projects {
    client: Client,
}

impl Projects {
    pub fn list(&self) -> Result<Value, TrigoraError> {
        self.client.request("GET", "/v1/projects", None)
    }

    pub fn create(&self, body: Value) -> Result<Value, TrigoraError> {
        self.client.request("POST", "/v1/projects", Some(body))
    }
}

pub struct Programs {
    client: Client,
}

impl Programs {
    pub fn list(&self, page: Option<Page>) -> Result<Value, TrigoraError> {
        self.client.request(
            "GET",
            &format!("/v1/programs{}", query(page.as_ref())),
            None,
        )
    }

    pub fn get(&self, program_id: &str) -> Result<Value, TrigoraError> {
        self.client
            .request("GET", &format!("/v1/programs/{}", encode(program_id)), None)
    }

    pub fn versions(&self, program_id: &str, page: Option<Page>) -> Result<Value, TrigoraError> {
        self.client.request(
            "GET",
            &format!(
                "/v1/programs/{}/versions{}",
                encode(program_id),
                query(page.as_ref())
            ),
            None,
        )
    }
}

pub struct Executions {
    client: Client,
}

impl Executions {
    pub fn start(
        &self,
        program_id: &str,
        input: Option<Value>,
    ) -> Result<ExecutionHandle, TrigoraError> {
        let body = self.client.request(
            "POST",
            "/v1/executions",
            Some(serde_json::json!({
                "programId": program_id,
                "input": input.unwrap_or(Value::Object(Default::default())),
            })),
        )?;
        let id = body
            .pointer("/execution/id")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                TrigoraError::new("Start response did not include an execution id.", 0, None)
            })?
            .to_string();
        Ok(ExecutionHandle {
            client: self.client.clone(),
            id,
        })
    }

    pub fn get(&self, execution_id: &str) -> Result<Value, TrigoraError> {
        let body = self.client.request(
            "GET",
            &format!("/v1/executions/{}", encode(execution_id)),
            None,
        )?;
        Ok(body.get("execution").cloned().unwrap_or(body))
    }

    pub fn list(&self, page: Option<Page>) -> Result<Value, TrigoraError> {
        self.client.request(
            "GET",
            &format!("/v1/executions{}", query(page.as_ref())),
            None,
        )
    }

    pub fn send(
        &self,
        execution_id: &str,
        name: &str,
        payload: Value,
    ) -> Result<Value, TrigoraError> {
        self.client.request(
            "POST",
            &format!("/v1/executions/{}/events", encode(execution_id)),
            Some(serde_json::json!({ "name": name, "payload": payload })),
        )
    }

    pub fn cancel(&self, execution_id: &str) -> Result<Value, TrigoraError> {
        self.client.request(
            "POST",
            &format!("/v1/executions/{}/cancel", encode(execution_id)),
            Some(serde_json::json!({})),
        )
    }

    pub fn result(&self, execution_id: &str) -> Result<Value, TrigoraError> {
        ExecutionHandle {
            client: self.client.clone(),
            id: execution_id.to_string(),
        }
        .result()
    }
}

pub struct ExecutionHandle {
    client: Client,
    pub id: String,
}

impl ExecutionHandle {
    pub fn send(&self, name: &str, payload: Value) -> Result<(), TrigoraError> {
        self.client.executions().send(&self.id, name, payload)?;
        Ok(())
    }

    pub fn cancel(&self) -> Result<(), TrigoraError> {
        self.client.executions().cancel(&self.id)?;
        Ok(())
    }

    pub fn result(&self) -> Result<Value, TrigoraError> {
        loop {
            let body = self.client.request(
                "GET",
                &format!("/v1/executions/{}/result", encode(&self.id)),
                None,
            )?;
            let result = body.get("result").unwrap_or(&body);
            match result.get("status").and_then(Value::as_str) {
                Some("completed") => {
                    return Ok(result.get("result").cloned().unwrap_or(Value::Null))
                }
                Some("failed") => {
                    let message = result
                        .pointer("/error/message")
                        .and_then(Value::as_str)
                        .unwrap_or("Execution failed.");
                    return Err(TrigoraError::new(message, 0, None));
                }
                Some("cancelled") => {
                    return Err(TrigoraError::new(
                        format!("Execution \"{}\" was cancelled.", self.id),
                        409,
                        None,
                    ));
                }
                _ => thread::sleep(RESULT_POLL),
            }
        }
    }
}

pub fn start(program_id: &str, input: Option<Value>) -> Result<ExecutionHandle, TrigoraError> {
    Client::from_env().executions().start(program_id, input)
}

fn env_token() -> Option<String> {
    env::var("TRIGORA_TOKEN")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn base_url(token: Option<&str>) -> String {
    if token.is_some() {
        return env::var("TRIGORA_API_BASE_URL")
            .unwrap_or_else(|_| DEFAULT_CLOUD_API_URL.to_string())
            .trim_end_matches('/')
            .to_string();
    }
    env::var("TRIGORA_RUNTIME_URL")
        .unwrap_or_else(|_| DEFAULT_RUNTIME_URL.to_string())
        .trim_end_matches('/')
        .to_string()
}

fn read_json(mut reader: impl Read) -> Result<Value, TrigoraError> {
    let mut raw = String::new();
    reader
        .read_to_string(&mut raw)
        .map_err(|error| TrigoraError::new(error.to_string(), 0, None))?;
    if raw.trim().is_empty() {
        return Ok(Value::Null);
    }
    serde_json::from_str(&raw).map_err(|error| TrigoraError::new(error.to_string(), 0, None))
}

fn error_from_response(status: u16, response: ureq::Response) -> TrigoraError {
    let parsed = read_json(response.into_reader()).unwrap_or(Value::Null);
    let message = parsed
        .pointer("/error/message")
        .and_then(Value::as_str)
        .unwrap_or("Request failed.")
        .to_string();
    let code = parsed
        .pointer("/error/code")
        .and_then(Value::as_str)
        .map(str::to_string);
    TrigoraError::new(message, status, code)
}

fn encode(value: &str) -> String {
    let mut encoded = String::new();
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(byte as char);
            }
            _ => encoded.push_str(&format!("%{byte:02X}")),
        }
    }
    encoded
}
