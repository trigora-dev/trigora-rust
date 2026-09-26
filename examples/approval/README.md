# Approval

```bash
cargo run --bin run
```

From the Trigora CLI repo, in this directory:

```bash
trigora dev
```

In another shell, `cargo run --bin run` starts the program, sends `approved`, and prints the result. The `generate` effect receives `{ "amount": 42.0 }` because the closure captures `amount`.
