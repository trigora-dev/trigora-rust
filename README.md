<p align="center">
  <a href="https://trigora.dev">
    <img src="https://trigora.dev/rust-banner.png" alt="Trigora / Rust — durable execution without history replay." width="100%" />
  </a>
</p>

# Trigora for Rust

**Durable execution for Rust without history replay.**

Build long-lived Rust programs that can suspend across effects, events, timers, and child executions, then recover from committed continuation state after failure.

Trigora is powered by **Transparent Continuation Checkpointing (TCC)**.

## Install

```sh
cargo add trigora trigora-client
cargo install trigora-cli
```

- `trigora` — Rust authoring crate
- `trigora-client` — Trigora Cloud API client
- `trigora-cli` — Trigora CLI

## Quickstart

Initialize:

```sh
trigora init
```

Run locally:

```sh
trigora dev
```

Deploy:

```sh
trigora deploy
```

See the [quickstart](https://trigora.dev/docs/quickstart).

## Durable programs

Trigora programs can:

- execute durable external effects;
- wait for events;
- sleep durably;
- invoke child executions;
- join or race concurrent branches;
- resume after worker or process failure.

The Rust frontend implements the declared `rust.subset.v1` semantics rather than arbitrary Rust execution.

See the [TCC Rust subset specification](https://github.com/trigora-dev/tcc-engine/blob/main/spec/rust-subset.md).

## Crates

### `trigora`

Authoring surface for durable Rust programs.

### `trigora-client`

Rust client for the Trigora Cloud API.

## Ecosystem

- [Trigora](https://github.com/trigora-dev/trigora)
- [Trigora for TypeScript](https://github.com/trigora-dev/trigora-typescript)
- [Trigora for Python](https://github.com/trigora-dev/trigora-python)
- [TCC Engine](https://github.com/trigora-dev/tcc-engine)

## Links

- [Website](https://trigora.dev)
- [Documentation](https://trigora.dev/docs)
- [Trigora Cloud](https://cloud.trigora.dev)
- [Research](https://trigora.dev/research)
- [Technical report](https://trigora.dev/research/whitepaper)

## License

MIT © 2026 Trigora, Inc.
