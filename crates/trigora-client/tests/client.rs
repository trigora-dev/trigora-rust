use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;

use serde_json::{json, Value};
use trigora_client::Client;

fn serve() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    thread::spawn(move || {
        for stream in listener.incoming() {
            let mut stream = stream.unwrap();
            let mut buffer = Vec::new();
            let mut chunk = [0; 1024];
            let header_end = loop {
                let read = stream.read(&mut chunk).unwrap();
                if read == 0 {
                    return;
                }
                buffer.extend_from_slice(&chunk[..read]);
                if let Some(index) = buffer.windows(4).position(|window| window == b"\r\n\r\n") {
                    break index + 4;
                }
            };
            let head = String::from_utf8_lossy(&buffer[..header_end]).to_string();
            let length = head
                .lines()
                .find_map(|line| {
                    let (name, value) = line.split_once(':')?;
                    if name.eq_ignore_ascii_case("content-length") {
                        value.trim().parse::<usize>().ok()
                    } else {
                        None
                    }
                })
                .unwrap_or(0);
            while buffer.len() < header_end + length {
                let read = stream.read(&mut chunk).unwrap();
                if read == 0 {
                    break;
                }
                buffer.extend_from_slice(&chunk[..read]);
            }
            let path = head
                .lines()
                .next()
                .unwrap()
                .split_whitespace()
                .nth(1)
                .unwrap();
            let raw = &buffer[header_end..header_end + length];
            let body: Value = if raw.is_empty() {
                Value::Null
            } else {
                serde_json::from_slice(raw).unwrap_or(Value::Null)
            };
            let response = if path.starts_with("/v1/programs?")
                || path.starts_with("/v1/executions?")
                || path.starts_with("/v1/programs/approval/versions?")
            {
                json!({ "path": path })
            } else if path.starts_with("/v1/programs/approval/versions") {
                json!({"versions": [{"id": "ver_1", "artifactHash": "abc"}]})
            } else if path == "/v1/whoami" {
                json!({"actorType": "api_token"})
            } else if path == "/v1/executions" {
                let id = if body.get("input").is_none() {
                    "exec_omitted"
                } else {
                    "exec_1"
                };
                json!({"execution": {"id": id, "programId": body["programId"]}})
            } else if path.ends_with("/events") {
                json!({"ok": true, "name": body["name"]})
            } else if path == "/v1/executions/exec_1/result" {
                json!({"result": {"status": "completed", "result": {"ok": true}}})
            } else {
                json!({"error": {"message": "missing", "code": "not_found"}})
            };
            let encoded = response.to_string();
            let status = if path == "/v1/missing" { 404 } else { 200 };
            write!(
                stream,
                "HTTP/1.1 {status} OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{encoded}",
                encoded.len()
            )
            .unwrap();
        }
    });
    format!("http://{address}")
}

#[test]
fn start_send_result_and_whoami() {
    let url = serve();
    let client = Client::new(trigora_client::ClientOptions {
        url: Some(url),
        ..trigora_client::ClientOptions::default()
    })
    .unwrap();
    assert_eq!(client.whoami().unwrap()["actorType"], "api_token");
    assert_eq!(
        client.programs().versions("approval", None).unwrap()["versions"][0]["id"],
        "ver_1"
    );
    let page = trigora_client::Page {
        limit: Some(2),
        cursor: Some("a b".into()),
    };
    assert_eq!(
        client.programs().list(Some(page.clone())).unwrap()["path"],
        "/v1/programs?limit=2&cursor=a%20b"
    );
    assert_eq!(
        client
            .programs()
            .versions("approval", Some(page.clone()))
            .unwrap()["path"],
        "/v1/programs/approval/versions?limit=2&cursor=a%20b"
    );
    assert_eq!(
        client.executions().list(Some(page)).unwrap()["path"],
        "/v1/executions?limit=2&cursor=a%20b"
    );
    let omitted = client.executions().start("approval", None).unwrap();
    assert_eq!(omitted.id, "exec_omitted");
    let run = client
        .executions()
        .start("approval", Some(json!({"n": 1})))
        .unwrap();
    assert_eq!(run.id, "exec_1");
    run.send("approved", json!("ok")).unwrap();
    assert_eq!(run.result().unwrap(), json!({"ok": true}));
}
