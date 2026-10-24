# uProtocol Tutorial — From Raw Sockets to LocalTransport

## Prologue

The story begins when I had been exploring the world of Eclipse-SDV out of curiosity. This was an
area hitherto completely unknown to me. Yet, I was drawn towards it. Why? I have captured the reasons in [this blog post](https://nsengupta.github.io/blog/why-explore-software-defined-vehicle/).

One of the technologies that had captured my interest was [uProtocol](https://github.com/eclipse-uprotocol). I am familiar with the problem it was trying to solve (I have worked in the area of location-agnostic, multi-machine-architecture-friendly, network-carried, multiplex-able middleware for a good part of my career), but the domain was different.

My aim was to understand the landscape well, and Eclipse SDV sites helped; so did the uProtocol repo, blogs (viz., Pete Le Vasseur's [Articles | plog](https://petelevasseur.com/articles/index.html)), and YouTube videos — but what I didn't find was a classical tutorial; a tutorial which helped a software developer to lay her/his hands on the code to solidify the understanding along with the specifications and examples, and helped create a mental map of _what was what_.

So, I decided to write one myself. This tutorial follows how I approached learning uProtocol; hopefully, this will be useful for you too.

- We start with **raw Unix Domain Sockets** and manually frame uProtocol `UMessage` bytes (two processes).
- Then we refactor onto uProtocol's own L1 (`UTransport` / `UListener`) and L2 (`SimplePublisher` / `CallOptions`) abstractions — still on the same Unix Domain Socket wire (two processes).
- Then we **swap the L1 plugin** for `LocalTransport` from this crate: the same L1/L2 application types, two listeners, **one process**.

This copy of the tutorial lives **inside `up-rust`**. Every phase path-depends on the crate in this checkout (including SNAPSHOT / RC trees). Clone with `--recurse-submodules` and build the phases from this tree. Do not expect these workspaces to track a crates.io `up-rust` version on their own. CI builds the phases whenever the library or the tutorial changes so API drift cannot hide.

A longer walk with **independent processes** over Zenoh is not in this repository. See [nsengupta/tutorial_uprotocol](https://github.com/nsengupta/tutorial_uprotocol) and the official [up-transport-zenoh-rust](https://github.com/eclipse-uprotocol/up-transport-zenoh-rust) crate.

## What we will learn

- How uProtocol's `UUri`, `UAttributes`, `UPayload`, and `UMessage` map to the wire.
- Why stream transports (Unix Domain Sockets) need explicit length-prefix framing.
- How uProtocol's L1 (`UTransport` / `UListener`) separates message moving from message handling.
- How uProtocol's L2 (`SimplePublisher` / `CallOptions`) separates publishing intent from envelope construction.
- Why Unix Domain Sockets fail for multi-process fan-out **even on one host**.
- How swapping the L1 transport plugin leaves publisher and subscriber business logic unchanged.
- What `LocalTransport` can and cannot do (in-process URI-matched dispatch only).

## Repository layout

Each tutorial chapter is a self-contained Cargo workspace under `phases/`. Narrative lives in `tutorial-text/`.

| Phase directory                  | Chapter | Code |
| -------------------------------- | ------- | ---- |
| `phases/01_raw_sockets/`         | Phase 1 | `up-frame-codec`, one publisher / one subscriber — Unix Domain Socket, length-framed `UMessage` |
| `phases/02_uprotocol_semantics/` | Phase 2 | `up-bms-proto`, `up-unix-domain-socket-transport`, one publisher / one subscriber — uProtocol L1/L2 over the same Unix Domain Socket |
| `phases/03_local_transport/`     | Phase 3 | `up-bms-proto`, `up-bms-local-demo` — L1/L2 over `LocalTransport`, two listeners in one process |

```
├── phases/
│   ├── 01_raw_sockets/           # Phase 1 – Unix Domain Socket + length-prefix framing
│   ├── 02_uprotocol_semantics/   # Phase 2 – uP-L1/L2 over the same Unix Domain Socket wire
│   └── 03_local_transport/       # Phase 3 – LocalTransport, in-process listeners
└── tutorial-text/
    ├── Tutorial-Phase-1.md
    ├── Tutorial-Phase-2.md
    └── Tutorial-Phase-3.md
```

Phase 3 **retires** `up-unix-domain-socket-transport` and `up-frame-codec` and **carries forward** `up-bms-proto`, the listener/publish logic, and the protobuf schema.

## The tutorial documents

Follow the tutorials for each phase, kept under [`tutorial-text/`](./tutorial-text/):

| File                                                         | What it covers |
| ------------------------------------------------------------ | -------------- |
| [`Tutorial-Phase-1.md`](./tutorial-text/Tutorial-Phase-1.md) | **Phase 1 — Raw Unix Domain Sockets.** Build `UMessage` envelopes by hand, frame them with a 4-byte length prefix, and send over a Unix Domain Socket (raw socket calls). Two processes — a publisher and a subscriber — exchange battery telemetry (SoC, temperature) as raw packed bytes. |
| [`Tutorial-Phase-2.md`](./tutorial-text/Tutorial-Phase-2.md) | **Phase 2 — uProtocol semantics.** A Unix Domain Socket-based transport is wrapped behind uProtocol's L1 (`UTransport`, `UListener`) and L2 (`SimplePublisher`, `CallOptions`). Raw CAN-frame packing is replaced by a protobuf schema. Application code no longer touches sockets or byte headers. |
| [`Tutorial-Phase-3.md`](./tutorial-text/Tutorial-Phase-3.md) | **Phase 3 — LocalTransport.** Unix Domain Sockets retire; `LocalTransport` becomes the L1 plugin. Same `SimplePublisher` and `UListener` style, plus a thermal listener — both registered in **one process**. |

## Phase 3 constraint — read this before you run it

Phases 1 and 2 are **two processes**. You start the subscriber in one terminal and the publisher in another. They meet on a Unix Domain Socket file.

Phase 3 is **not** that arrangement.

- There is **one** `cargo run` and **one** process (`up-bms-local-demo`).
- The battery listener, the thermal listener, and `SimplePublisher` share a single `Arc<LocalTransport>`.
- Opening a second terminal and trying to run “another subscriber” **cannot work**. There is no socket path and no network. `LocalTransport` only delivers to listeners registered on **that** instance.
- `LocalTransport::send` runs matching listeners on the **sending thread** and returns after they finish (same as [`examples/simple_publish.rs`](../examples/simple_publish.rs)). There is no “start subscribers first” race.
- This is a limit of **this** L1 plugin, not of uProtocol. The same `UListener` / `SimplePublisher` code can talk across processes if you plug in a networked `UTransport` (Zenoh, MQTT, …).
- For that picture: [nsengupta/tutorial_uprotocol](https://github.com/nsengupta/tutorial_uprotocol) (three-process Zenoh walk) and [up-transport-zenoh-rust](https://github.com/eclipse-uprotocol/up-transport-zenoh-rust) (official transport; a distributed example may live there later).

## Quick start — run the demo

**All `cargo` commands in this file assume your shell is in the `tutorial/` directory** (the folder that contains this README). From the `up-rust` clone root:

```bash
cd tutorial
```

CI uses `--manifest-path tutorial/phases/<phase>/Cargo.toml` from the clone root. Those two path spellings are not interchangeable.

**Phases 1–2 start order:** start every **subscriber first**, then the **publisher**. The publisher sends a short burst and exits; if no subscriber is listening yet, those messages are missed. That order is an operational convention, not something `dispatch_to` enforces — see the Point of interest in [`Tutorial-Phase-2.md`](./tutorial-text/Tutorial-Phase-2.md) Chapter 6.

### Phase 1 — Raw sockets

```bash
cargo build --manifest-path phases/01_raw_sockets/Cargo.toml

# Terminal 1 — subscriber (start this first)
cargo run --manifest-path phases/01_raw_sockets/Cargo.toml -p up-telemetry-subscriber

# Terminal 2 — publisher (sends 5 messages, then exits)
cargo run --manifest-path phases/01_raw_sockets/Cargo.toml -p up-battery-telemetry-publisher
```

### Phase 2 — uProtocol semantics

```bash
cargo build --manifest-path phases/02_uprotocol_semantics/Cargo.toml

# Terminal 1 — subscriber (start this first)
cargo run --manifest-path phases/02_uprotocol_semantics/Cargo.toml -p up-telemetry-subscriber

# Terminal 2 — publisher (sends 5 messages, then exits)
cargo run --manifest-path phases/02_uprotocol_semantics/Cargo.toml -p up-battery-telemetry-publisher
```

Optional — enable `RUST_LOG=trace` on both terminals to see `up-unix-domain-socket-transport` dispatch logs.

Run the Phase 2 tests:

```bash
cargo test --manifest-path phases/02_uprotocol_semantics/Cargo.toml -p up-frame-codec
cargo test --manifest-path phases/02_uprotocol_semantics/Cargo.toml -p up-unix-domain-socket-transport
```

### Phase 3 — LocalTransport (one process, two listeners)

One terminal. Do not start a second subscriber process.

```bash
cargo run --manifest-path phases/03_local_transport/Cargo.toml -p up-bms-local-demo
```

Both listeners print for each of the five messages. The publisher and battery `on_receive` bodies are the same *kind* of code as Phase 2; only transport construction changed.

```bash
cargo test --manifest-path phases/03_local_transport/Cargo.toml -p up-bms-local-demo
```

### Prerequisites

- Rust toolchain (this crate’s MSRV; edition 2024 in the tutorial crates)
- Linux for Phases 1–2 (Unix Domain Sockets). Phase 3 is in-process and is not tied to a Unix socket.
- Clone this repository with submodules (`git clone --recurse-submodules` or `git submodule update --init --recursive`)
- No prior uProtocol knowledge assumed

### Declaration

I indeed have taken some help from [Cursor](https://cursor.com) and [Ralph](https://ralphy-server.fly.dev/) for writing draft code, but
the concept behind this tutorial, and the choice of the problem and solutions as well the final
documentation/code-structure/code are entirely mine.

## License

This project is licensed under the [Apache License, Version 2.0](../LICENSE).

The entire tutorial text and sample Rust code in `phases/` are covered by that license
unless noted otherwise.
