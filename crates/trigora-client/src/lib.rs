//! HTTP client for the Trigora `/v1` API.
//!
//! The client is local unless `ClientOptions::remote` is set. An explicit URL
//! wins over `remote`. `TRIGORA_TOKEN` is a credential and is read only when
//! `remote` selects Trigora Cloud. This crate does not compile or author programs.

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
    pub remote: bool,
    pub token: Option<String>,
    pub project_id: Option<String>,
}

impl Default for ClientOptions {
    fn default() -> Self {
        Self {
            url: None,
            remote: false,
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
    cloud: bool,
    project_id: Option<String>,
    agent: ureq::Agent,
}

struct ResolvedTarget {
    url: String,
    token: Option<String>,
    cloud: bool,
}

impl Client {
    pub fn new(options: ClientOptions) -> Result<Self, TrigoraError> {
        let resolved = resolve_target(options.url, options.remote, options.token)?;
        Ok(Self {
            inner: Arc::new(Inner {
                url: resolved.url,
                token: resolved.token,
                cloud: resolved.cloud,
                project_id: options.project_id,
                agent: ureq::AgentBuilder::new()
                    .timeout(Duration::from_secs(30))
                    .build(),
            }),
        })
    }

    pub fn from_env() -> Result<Self, TrigoraError> {
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
            Err(ureq::Error::Transport(error)) => {
                let message = if self.inner.cloud {
                    format!("Could not reach Trigora Cloud at {url}. {error}")
                } else {
                    format!(
                        "Could not reach the local Trigora runtime at {url}. Is `trigora dev` running? {error}"
                    )
                };
                Err(TrigoraError::new(message, 0, None))
            }
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
    Client::from_env()?.executions().start(program_id, input)
}

fn present(value: String) -> Option<String> {
    let trimmed = value.trim().to_string();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

fn env_token() -> Option<String> {
    env::var("TRIGORA_TOKEN").ok().and_then(present)
}

fn env_url(name: &str, fallback: &str) -> String {
    env::var(name)
        .ok()
        .map(|value| value.trim().trim_end_matches('/').to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| fallback.to_string())
}

fn resolve_target(
    url: Option<String>,
    remote: bool,
    token: Option<String>,
) -> Result<ResolvedTarget, TrigoraError> {
    let explicit_token = token.and_then(present);
    if let Some(url) = url {
        return Ok(ResolvedTarget {
            url: url.trim_end_matches('/').to_string(),
            token: explicit_token,
            cloud: false,
        });
    }
    if remote {
        let Some(token) = explicit_token.or_else(env_token) else {
            return Err(TrigoraError::new("TRIGORA_TOKEN is not set.", 0, None));
        };
        return Ok(ResolvedTarget {
            url: env_url("TRIGORA_API_BASE_URL", DEFAULT_CLOUD_API_URL),
            token: Some(token),
            cloud: true,
        });
    }
    Ok(ResolvedTarget {
        url: env_url("TRIGORA_RUNTIME_URL", DEFAULT_RUNTIME_URL),
        token: explicit_token,
        cloud: false,
    })
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

#[cfg(test)]
mod target_tests {
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::{Mutex, OnceLock};

    use super::*;

    fn env_lock() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(())).lock().unwrap()
    }

    struct EnvGuard {
        token: Option<String>,
        runtime: Option<String>,
        api: Option<String>,
    }

    impl EnvGuard {
        fn capture() -> Self {
            Self {
                token: env::var("TRIGORA_TOKEN").ok(),
                runtime: env::var("TRIGORA_RUNTIME_URL").ok(),
                api: env::var("TRIGORA_API_BASE_URL").ok(),
            }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            restore("TRIGORA_TOKEN", self.token.take());
            restore("TRIGORA_RUNTIME_URL", self.runtime.take());
            restore("TRIGORA_API_BASE_URL", self.api.take());
        }
    }

    fn restore(key: &str, value: Option<String>) {
        match value {
            Some(value) => env::set_var(key, value),
            None => env::remove_var(key),
        }
    }

    fn authorization(head: &str) -> Option<String> {
        head.lines().find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case("authorization")
                .then(|| value.trim().to_string())
        })
    }

    fn serve_one() -> (String, thread::JoinHandle<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let handle = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut buffer = Vec::new();
            let mut chunk = [0u8; 1024];
            loop {
                let read = stream.read(&mut chunk).unwrap();
                buffer.extend_from_slice(&chunk[..read]);
                if buffer.windows(4).any(|window| window == b"\r\n\r\n") {
                    break;
                }
            }
            let _ = stream.write_all(
                b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}",
            );
            String::from_utf8(buffer).unwrap()
        });
        (format!("http://{address}"), handle)
    }

    #[test]
    fn env_token_stays_local_without_authorization() {
        let _lock = env_lock();
        let _guard = EnvGuard::capture();
        env::set_var("TRIGORA_TOKEN", "cloud-token");
        env::remove_var("TRIGORA_RUNTIME_URL");
        let client = Client::from_env().unwrap();
        assert_eq!(client.inner.url, "http://127.0.0.1:3477");
        assert!(client.inner.token.is_none());

        let (url, handle) = serve_one();
        env::set_var("TRIGORA_RUNTIME_URL", &url);
        Client::from_env().unwrap().whoami().unwrap();
        assert!(authorization(&handle.join().unwrap()).is_none());
    }

    #[test]
    fn remote_selects_cloud_and_sends_token() {
        let _lock = env_lock();
        let _guard = EnvGuard::capture();
        env::set_var("TRIGORA_TOKEN", "cloud-token");
        env::remove_var("TRIGORA_API_BASE_URL");
        let client = Client::new(ClientOptions {
            remote: true,
            ..ClientOptions::default()
        })
        .unwrap();
        assert_eq!(client.inner.url, "https://api.trigora.dev");
        assert_eq!(client.inner.token.as_deref(), Some("cloud-token"));

        let (url, handle) = serve_one();
        env::set_var("TRIGORA_API_BASE_URL", &url);
        Client::new(ClientOptions {
            remote: true,
            ..ClientOptions::default()
        })
        .unwrap()
        .whoami()
        .unwrap();
        assert_eq!(
            authorization(&handle.join().unwrap()).as_deref(),
            Some("Bearer cloud-token")
        );
    }

    #[test]
    fn remote_without_token_fails_at_construction() {
        let _lock = env_lock();
        let _guard = EnvGuard::capture();
        env::remove_var("TRIGORA_TOKEN");
        let Err(error) = Client::new(ClientOptions {
            remote: true,
            ..ClientOptions::default()
        }) else {
            panic!("Cloud construction succeeded without a token");
        };
        assert_eq!(error.to_string(), "TRIGORA_TOKEN is not set.");
    }

    #[test]
    fn explicit_url_wins_over_remote_without_a_token() {
        let _lock = env_lock();
        let _guard = EnvGuard::capture();
        env::remove_var("TRIGORA_TOKEN");
        let (url, handle) = serve_one();
        Client::new(ClientOptions {
            url: Some(url),
            remote: true,
            ..ClientOptions::default()
        })
        .unwrap()
        .whoami()
        .unwrap();
        assert!(authorization(&handle.join().unwrap()).is_none());
    }

    #[test]
    fn explicit_token_is_sent_to_a_custom_local_url() {
        let _lock = env_lock();
        let _guard = EnvGuard::capture();
        env::set_var("TRIGORA_TOKEN", "env-token");
        let (url, handle) = serve_one();
        Client::new(ClientOptions {
            url: Some(url),
            remote: false,
            token: Some("explicit-token".into()),
            ..ClientOptions::default()
        })
        .unwrap()
        .whoami()
        .unwrap();
        assert_eq!(
            authorization(&handle.join().unwrap()).as_deref(),
            Some("Bearer explicit-token")
        );
    }
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
