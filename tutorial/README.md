# uProtocol Tutorial — From Raw Sockets to Zenoh Fan-Out

## Prologue

The story begins when I had been exploring the world of Eclipse-SDV out of curiosity. This was an 
area hitherto completely unknown to me. Yet, I was drawn towards it. Why? I have captured the reasons in [this blog post](https://nsengupta.github.io/blog/why-explore-software-defined-vehicle/).

One of the technologies that had captured my interest was [uProtocol](https://github.com/eclipse-uprotocol). I am familiar with the problem it was trying to solve (I have worked in the area of location-agnostic, multi-machine-architecture-friendly, network-carried, multiplex-able middleware for a good part of my career), but the domain was different. 

My aim was to understand the landscape well, and Eclipse SDV sites helped; so did the uProtocol repo, blogs (viz., Pete Le Vasseur's [Articles | plog](https://petelevasseur.com/articles/index.html)), and YouTube videos — but what I didn't find was a classical tutorial; a tutorial which helped a software developer to lay her/his hands on the code to solidify the understanding along with the specifications and examples, and helped create a mental map of _what was what_.

So, I decided to write one myself. This tutorial follows how I approached learning uProtocol; hopefully, this will be useful for you too.

- We start with **raw Unix Domain Sockets** and manually frame uProtocol `UMessage` bytes.
- Then we refactor uProtocol's own L1 (`UTransport` / `UListener`) and L2 (`SimplePublisher` / `CallOptions`) abstractions — still on the same Unix Domain Socket wire. 
- Then, we **replace the transport with Zenoh**, add a second subscriber (the thermal logger mentioned in Phase 1), and prove that business logic survives the change of transport (from *Unix Domain Socket* to *Zenoh*).

## What we will learn

- How uProtocol's `UUri`, `UAttributes`, `UPayload`, and `UMessage` map to the wire.
- Why stream transports (Unix Domain Sockets) need explicit length-prefix framing.
- How uProtocol's L1 (`UTransport` / `UListener`) separates message moving from message handling.
- How uProtocol's L2 (`SimplePublisher` / `CallOptions`) separates publishing intent from envelope construction.
- Why Unix Domain Sockets fails for multi-process fan-out **even on one host** — and why a 
  data-space transport (Zenoh) fills that gap.
- How swapping the L1 transport plugin leaves publisher and subscriber business logic unchanged.
- How `UUri` metadata becomes first-class routing information in Zenoh (vs opaque bytes on a Unix Domain Socket).
- How L3 PUBLISH registration lets independent processes subscribe to the same resource URI.


## Repository layout

Each tutorial chapter is a self-contained Cargo workspace under `phases/` — no git checkout needed to run a given phase. Narrative drafts live in `tutorial-text/`.

| Phase directory                  | Chapter | Code                                                                                                                       |
| -------------------------------- | ------- | -------------------------------------------------------------------------------------------------------------------------- |
| `phases/01_raw_sockets/`         | Phase 1 | `up-frame-codec`, one publisher / one subscriber — Unix Domain Socket transport, length-framed `UMessage`                                 |
| `phases/02_uprotocol_semantics/` | Phase 2 | `up-bms-proto`, `up-unix-domain-socket-transport`, refactored one publisher / one subscriber — uProtocol L1/L2 over (still) Unix Domain Socket transport |
| `phases/03_zenoh_topology/`      | Phase 3 | Use of `up-transport-zenoh`, one publisher / two subscribers - uProtocol L1/L2/L3 over Zenoh transport                     |

```
├── phases/
│   ├── 01_raw_sockets/           # Phase 1 – Unix Domain Socket + length-prefix framing
│   ├── 02_uprotocol_semantics/   # Phase 2 – uP-L1/L2 over the same Unix Domain Socket wire
│   └── 03_zenoh_topology/        # Phase 3 – Zenoh data-space transport, multi-subscriber
├── tutorial-text/
│   ├── Tutorial-Phase-1.md    # Phase 1 narrative
│   ├── Tutorial-Phase-2.md    # Phase 2 narrative
│   └── Tutorial-Phase-3.md    # Phase 3 narrative — Zenoh, fan-out, L3
└── Notes/                     # Maintainer notes and feedback
```

Phase 3 **retires** `up-unix-domain-socket-transport` and `up-frame-codec` (frozen in Phase 2 only) and **carries forward** `up-bms-proto`, the publisher/subscriber logic, and the protobuf schema unchanged.

## The tutorial documents

Follow the tutorials for each phase, kept under [`tutorial-text/`](./tutorial-text/):

| File                                                         | What it covers                                                                                                                                                                                                                                                                                                                                                                                                         |
| ------------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [`Tutorial-Phase-1.md`](./tutorial-text/Tutorial-Phase-1.md) | **Phase 1 — Raw Unix Domain Sockets.** Build `UMessage` envelopes by hand, frame them with a 4-byte length prefix, and send over a Unix Domain Socket transport (but raw socket calls). Two processes — a publisher and a subscriber — exchange battery telemetry (SoC, temperature) as raw packed bytes. One sees every layer of the wire with no library hiding the details.                                                        |
| [`Tutorial-Phase-2.md`](./tutorial-text/Tutorial-Phase-2.md) | **Phase 2 — uProtocol semantics.** A Unix Domain Socket-based transport is wrapped behind uProtocol's L1 (`UTransport`, `UListener`) and L2 (`SimplePublisher`, `CallOptions`). The raw CAN-frame packing is replaced by a protobuf schema (`bms_telemetry.proto`). The transport crate handcrafted (`up-unix-domain-socket-transport`) centralizes framing and dispatch; application code no longer touches sockets or byte headers. |
| [`Tutorial-Phase-3.md`](./tutorial-text/Tutorial-Phase-3.md) | **Phase 3 — Zenoh topology.** Unix Domain Sockets retire; Zenoh (`up-transport-zenoh`) becomes the L1 plugin. Same `SimplePublisher` and `UListener` bodies, but a second process (`up-thermal-logging-subscriber`) receives the same stream independently — the fan-out payoff Phase 1 promised and Phase 2 documented but could not deliver. All on one Linux host (Zenoh peer mode).                                       |

## Quick start — run the demo

Each phase is an independent Cargo workspace. Run all commands from the repo root.

**Start order (all phases):** start every **subscriber first**, then the **publisher**. The publisher sends a short burst and exits; if no subscriber is listening yet, those messages are missed (especially visible in Phase 3’s multi-subscriber demo).

### Phase 1 — Raw sockets

Start the subscriber, then the publisher (two terminals):

```bash
cargo build --manifest-path phases/01_raw_sockets/Cargo.toml

# Terminal 1 — subscriber (start this first)
cargo run --manifest-path phases/01_raw_sockets/Cargo.toml -p up-telemetry-subscriber

# Terminal 2 — publisher (sends 5 messages, then exits)
cargo run --manifest-path phases/01_raw_sockets/Cargo.toml -p up-battery-telemetry-publisher
```

### Phase 2 — uProtocol semantics

Start the subscriber, then the publisher (two terminals):

```bash
cargo build --manifest-path phases/02_uprotocol_semantics/Cargo.toml

# Terminal 1 — subscriber (start this first)
cargo run --manifest-path phases/02_uprotocol_semantics/Cargo.toml -p up-telemetry-subscriber

# Terminal 2 — publisher (sends 5 messages, then exits)
cargo run --manifest-path phases/02_uprotocol_semantics/Cargo.toml -p up-battery-telemetry-publisher
```

Optional — enable `RUST_LOG=trace` on both terminals to see `up-unix-domain-socket-transport` dispatch logs.

Run the Phase 2 transport crate tests:

```bash
cargo test --manifest-path phases/02_uprotocol_semantics/Cargo.toml -p up-unix-domain-socket-transport
```

### Phase 3 — Zenoh topology (multi-subscriber, single producer)

**Start both subscribers before the publisher** (three terminals). If the publisher runs first, its five messages can finish before either subscriber has registered interest. Background on Zenoh peer mode and why this demo does not use `zenohd`: [`tutorial-text/Tutorial-Phase-3.md`](./tutorial-text/Tutorial-Phase-3.md) (Chapter 6).

```bash
cargo build --manifest-path phases/03_zenoh_topology/Cargo.toml
```

```bash
# Terminal 1 — battery telemetry subscriber (start first)
cargo run --manifest-path phases/03_zenoh_topology/Cargo.toml -p up-telemetry-subscriber

# Terminal 2 — thermal logging subscriber (start second; new in Phase 3)
cargo run --manifest-path phases/03_zenoh_topology/Cargo.toml -p up-thermal-logging-subscriber

# Terminal 3 — publisher last (sends 5 messages, then exits)
cargo run --manifest-path phases/03_zenoh_topology/Cargo.toml -p up-battery-telemetry-publisher
```

Expected: both subscribers receive all five messages independently — no shared socket path, no broker in application code. The publisher and battery subscriber `on_receive` bodies are **identical** to Phase 2; only transport construction changed.

### Prerequisites

- Rust toolchain (edition 2024, in my set-up)
- Linux (Unix Domain Sockets for Phases 1–2; Phase 3 demo also runs on Linux)
- **Phase 3 only:** [Zenoh](https://zenoh.io/) via the `up-transport-zenoh` dependency (pulled by Cargo); no separate `zenohd` install for this demo
- No prior uProtocol knowledge assumed

### Declaration

I indeed have taken some help from [Cursor](https://cursor.com) and [Ralph](https://ralphy-server.fly.dev/) for writing draft code, but 
the concept behind this tutorial, and the choice of the problem and solutions as well the final 
documentation/code-structure/code are entirely mine.

## License

This project is licensed under the [Apache License, Version 2.0](./LICENSE.txt).

The entire tutorial text, notes, and sample Rust code in `phases/` are covered by that license 
unless noted otherwise.