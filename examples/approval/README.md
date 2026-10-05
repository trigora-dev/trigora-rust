# Approval

```bash
cargo install trigora-cli --version 1.0.1
trigora dev
```

`trigora dev` stays running. In another shell, `cargo run --bin run` starts the program, sends `approved`, and prints the result. The `generate` effect receives `{ "amount": 42.0 }` because the closure captures `amount`.
