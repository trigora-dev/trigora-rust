# trigora-client

HTTP client for Trigora programs and executions.

```rust
let client = trigora_client::Client::from_env();
let run = client.executions().start("approval", None)?;
run.send("approved", serde_json::json!(true))?;
let result = run.result()?;
```

`TRIGORA_TOKEN` selects Trigora Cloud (`TRIGORA_API_BASE_URL`, default `https://api.trigora.dev`). Otherwise the client uses the local runtime at `http://127.0.0.1:3477`.

The same client can call `whoami`, `projects`, `deploy`, and `programs`, including `programs().versions`. `programs().list`, `programs().versions`, and `executions().list` take an optional `Page` with `limit` and `cursor`. Triggers are configured in `trigora.toml` and deployed with the Trigora CLI. They are not SDK or client APIs.
