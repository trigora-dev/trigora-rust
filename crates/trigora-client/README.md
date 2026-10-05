# trigora-client

Rust client for starting and controlling Trigora durable executions.

Use the same API locally with `trigora dev` or remotely with Trigora Cloud.

## Install

```bash
cargo add trigora-client
```

## Quick example

```rust
use trigora_client::{Client, ClientOptions};

let client = Client::from_env()?;

let run = client
    .executions()
    .start("approval", Some(serde_json::json!({ "request": "example" })))?;
run.send("approved", serde_json::json!("ok"))?;
let result = run.result()?;

println!("{result}");
```

## Local and Cloud

By default, the client connects to the local runtime started by `trigora dev`.

```rust
let client = Client::from_env()?;
```

To connect to Trigora Cloud:

```rust
let client = Client::new(ClientOptions {
    remote: true,
    ..ClientOptions::default()
})?;
```

Cloud requires a token. You can pass one directly or set `TRIGORA_TOKEN`.

```rust
let client = Client::new(ClientOptions {
    remote: true,
    token: std::env::var("TRIGORA_TOKEN").ok(),
    ..ClientOptions::default()
})?;
```

You can also connect to a custom endpoint:

```rust
let client = Client::new(ClientOptions {
    url: Some("http://127.0.0.1:3477".into()),
    ..ClientOptions::default()
})?;
```

Endpoint defaults:

- Local runtime: `TRIGORA_RUNTIME_URL` or `http://127.0.0.1:3477`
- Trigora Cloud: `TRIGORA_API_BASE_URL` or `https://api.trigora.dev`

An explicit `url` overrides the default endpoint. An explicit `token` is sent with requests to that endpoint.

## API

Workspace and project operations:

- `whoami()`
- `projects().list()`
- `projects().create()`

Programs:

- `deploy()`
- `programs().list()`
- `programs().get()`
- `programs().versions()`

Executions:

- `executions().start(program, input)`
- `executions().list()`
- `executions().get(id)`

Execution handle:

- `run.id`
- `run.send(event, payload)`
- `run.cancel()`
- `run.result()`

`programs().list`, `programs().versions`, and `executions().list` accept an optional `Page` with `limit` and `cursor`.

`program` is a program id string.

`event` is an event name string.

## Learn more

- [Trigora documentation](https://trigora.dev/docs)
- [Client documentation](https://trigora.dev/docs/client)
- [Trigora Cloud](https://cloud.trigora.dev)

## License

MIT © 2026 Trigora, Inc.
