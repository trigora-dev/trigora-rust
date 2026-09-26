use std::env;

use serde_json::Value;
use trigora_client::{Client, ClientOptions, TrigoraError};

#[test]
fn local_v1_parity() {
    let Ok(url) = env::var("TRIGORA_PARITY_URL") else {
        return;
    };
    let body: Value =
        serde_json::from_str(&env::var("TRIGORA_PARITY_BODY").expect("body")).expect("json");
    let client = Client::new(ClientOptions {
        url: Some(url),
        token: None,
        project_id: None,
    });
    let projects = client.projects().list().expect("projects");
    assert!(projects["projects"]
        .as_array()
        .unwrap()
        .iter()
        .any(|project| project["slug"] == "default"));
    let created = client
        .projects()
        .create(serde_json::json!({ "name": "parity-rust" }))
        .expect("create");
    assert_eq!(created["project"]["name"], "parity-rust");
    let deployed = client.deploy(body.clone()).expect("deploy");
    assert_eq!(deployed["program"]["name"], body["name"]);
    assert_eq!(
        deployed["version"]["artifactHash"],
        body["artifact"]["hash"]
    );
    let listed = client.programs().list(None).expect("list");
    assert!(listed["programs"]
        .as_array()
        .unwrap()
        .iter()
        .any(|program| program["name"] == body["name"]));
    let program = client
        .programs()
        .get(body["name"].as_str().unwrap())
        .expect("get");
    assert_eq!(
        program["program"]["currentVersion"]["artifactHash"],
        body["artifact"]["hash"]
    );
    let versions = client
        .programs()
        .versions(body["name"].as_str().unwrap(), None)
        .expect("versions");
    assert!(versions["versions"]
        .as_array()
        .unwrap()
        .iter()
        .any(|version| version["artifactHash"] == body["artifact"]["hash"]));
    let name = body["name"].as_str().unwrap();
    let first = client
        .executions()
        .start(name, Some(serde_json::json!({})))
        .expect("start");
    first
        .send("approved", serde_json::json!({ "ok": true }))
        .expect("send");
    assert_eq!(
        first.result().expect("result"),
        serde_json::json!({ "ok": true, "approval": { "ok": true } })
    );
    let second = client
        .executions()
        .start(name, Some(serde_json::json!({})))
        .expect("start again");
    second.cancel().expect("cancel");
    let error = second.result().expect_err("cancelled");
    assert!(
        error.to_string().contains("cancelled"),
        "{}",
        TrigoraError::to_string(&error)
    );
}
