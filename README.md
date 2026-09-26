# trigora-rust

Author a durable program with the `trigora` crate. Control executions with `trigora-client`.

```rust
use trigora::{effect, wait_for_event};

pub struct Approval {
    pub result: f64,
    pub review: bool,
}

pub async fn main() -> Result<Approval, String> {
    let amount: f64 = 42.0;
    let result = effect("generate", move || amount).await?;
    let review: bool = wait_for_event("approved").await?;
    Ok(Approval {
        result: result,
        review: review,
    })
}
```

The program entry is `pub async fn main`. `pub async fn run` is not an entry.

Triggers are configured in `trigora.toml` and deployed with the Trigora CLI. They are not SDK or client APIs.

`effect` and `wait_for_event` are durable boundaries. The closure body does not run inside the program. Trigora calls it later with the captured bindings as JSON (`{"amount": 42.0}`). A capture uses `move`. Names are string literals. Give generic results a type ascription (`let review: bool = ...`).

`trigora` only exists so `cargo check` can typecheck that subset. It does not run the program. Start the CLI with `trigora dev`, then:

```rust
let run = trigora_client::start("approval", None)?;
run.send("approved", serde_json::json!(true))?;
let result = run.result()?;
```

## rust.subset.v1

Owned values only: `f64`, `bool`, `String`, structs, enums, `Option`, `Result`, and `Vec`. `f64` and `bool` copy. Everything else moves. Integer types, references, methods, macros, and iterators are outside this subset. The compiler rejects them.

See `examples/approval`.
