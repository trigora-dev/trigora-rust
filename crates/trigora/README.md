# trigora

Authoring crate for Trigora Rust programs. Import `effect`, `sleep`, `wait_for_event`, `invoke`, `join`, and `race`.

The program entry is `pub async fn main`. The functions typecheck under `cargo check` and do not run. `trigora dev` compiles the program and supplies effect results. Use `trigora-client` to start an execution.
