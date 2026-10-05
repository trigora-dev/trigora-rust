# trigora

Rust authoring SDK for Trigora durable programs.

Build long-running programs that can call external systems, wait for events, sleep durably, invoke child executions, and recover from committed continuation state.

## Install

```bash
cargo add trigora
```

## Quick example

```rust
use trigora::{effect, wait_for_event};

pub async fn main() -> Result<String, String> {
    let greeting: String = effect("generate", || String::from("hello")).await?;
    let review: String = wait_for_event("approved").await?;
    Ok(format!("{greeting} {review}"))
}
```

Run it locally with:

```bash
trigora dev
trigora start approval
```

## Program entry

Trigora discovers programs from the paths configured in `trigora.toml`:

```toml
[project]
name = "my-project"
programs = ["src/**/*.rs"]
```

Each program file has one public async entry. The program id is the file name, so `src/approval.rs` starts with `trigora start approval`. A program in `src/lib.rs` or `src/main.rs` uses the Cargo package name.

```rust
pub async fn main(topic: String) -> Result<String, String> {
    Ok(topic)
}
```

The Rust frontend implements `rust.subset.v1`.

Parameters are plain owned values. The start arguments must match that parameter list. Defaults are not supported.

A private `async fn main` is not a program entry.

## Durable primitives

- `effect(name, fn)` — run a durable external effect
- `sleep(ms)` — suspend on a durable timer
- `wait_for_event(name)` — wait for an external event
- `invoke(program, input)` — invoke a durable child execution
- `join(left, right)` — wait for both durable branches
- `race(left, right)` — wait for the first durable branch

Effect keys and event names are string literals in the current Rust subset.

## Local development

```bash
trigora init
trigora dev
trigora start approval
trigora send <execution> approved --payload '"ok"'
trigora result <execution>
```

Programs execute through the Trigora runtime rather than by running the source file directly. `cargo check` typechecks the authoring crate. It does not run the program.

## Learn more

- [Quickstart](https://trigora.dev/docs/quickstart)
- [Trigora documentation](https://trigora.dev/docs)
- [TCC Rust semantics](https://github.com/trigora-dev/tcc-engine/blob/main/spec/rust-subset.md)

## License

MIT © 2026 Trigora, Inc.
