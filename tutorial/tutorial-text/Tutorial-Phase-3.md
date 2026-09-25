### Phase 3: The Scaling Limit & The Zenoh Payoff

> **Prerequisites:** Phase 2 (`tutorial-text/Tutorial-Phase-2.md`, code in `phases/02_uprotocol_semantics/`).  
> **Active code:** `phases/03_zenoh_topology/` — copy-forward from Phase 2.

Phase 2 ended with an honest confession: Unix Domain Socket works for one publisher and one subscriber on a single Linux host, but it breaks the moment we need a second consumer — even on the same machine. Phase 3 swaps the wire for **Zenoh** and delivers the fan-out payoff that Phase 1's thermal logger narrative promised.

**Demo scope:** All processes run on **one Linux host** (separate terminals), same as Phases 1–2. The cross-ECU vehicle story is narrative only — we do not simulate multiple machines.

Let's dive in.

---

### Chapter 1: Why Unix Domain Sockets had to retire (recap)

Phase 2's `up-unix-domain-socket-transport` solved the right problem: separating business logic from socket I/O using `UTransport`. But the Unix Domain Socket **wire** still fails on one host when we need:

1. **Fan-out** — two subscriber *processes* cannot both receive the same stream (point-to-point; one bind owner).
2. **A logical address** — `{cwd}/tmp/uprotocol_twin.sock` is a filename, not a URI; only one process can bind it.
3. **Location transparency** — publisher and subscriber must share a kernel.

These limits bite us on a single laptop, which is why Phase 3 retires `up-unix-domain-socket-transport` even before we mention cross-ECU:

```
Same Linux host — Phase 2 Unix Domain Socket (still broken for fan-out)

  up-battery-telemetry-publisher          up-telemetry-subscriber
         │                                      ▲
         │  one Unix Domain Socket connection   │ UnixDomainSocketTransport::bind
         └──────────────► {cwd}/tmp/...sock ────┘ (one bind-side process)

  up-thermal-logging-subscriber (what we want in Phase 3)
         │
         └── cannot share the stream — second process has no socket to read
```

**We do not need a second machine to hit this wall.** Phase 1 introduced the thermal logging process as a second consumer; Phase 2 made URI interest declarative but did not deliver bytes to a second process. Zenoh's native pub/sub gives each process its own subscription without a manual forward loop.

> We are not exploring multiple Zone ECUs in this tutorial, intentionally. Networked Zone ECUs behave like apps on different Linux hosts — the same fan-out problem remains.

---

### Chapter 2: Zenoh on the same layer map

[Zenoh](https://zenoh.io/) is a data-space transport: publish/subscribe with location transparency and native fan-out. Eclipse uProtocol provides [`up-transport-zenoh`](https://github.com/eclipse-uprotocol/up-transport-zenoh-rust) — a `UTransport` implementation backed by Zenoh.

Phase 2 introduced the **canonical layer map**. Phase 3 keeps every band above the wire; we swap only the L1 plugin and the wire. Crates under `phases/03_zenoh_topology/` still pin `up-rust = "0.9.0"`.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  Application / uEntity                                                      │
│  publisher · battery subscriber · thermal subscriber                        │
├─────────────────────────────────────────────────────────────────────────────┤
│  L3 — Application services (uSubscription, uDiscovery, …)                   │
│  Not used in this demo — same authority (`my_own_car`) + same transport     │
├─────────────────────────────────────────────────────────────────────────────┤
│  L2 — Communication                                                         │
│  SimplePublisher · CallOptions · UPayload          (unchanged from Phase 2) │
├─────────────────────────────────────────────────────────────────────────────┤
│  L1 — Transport                                                             │
│  UTransport · UListener · UPTransportZenoh         (plugin swap)            │
│  send · register_listener                                                   │
├─────────────────────────────────────────────────────────────────────────────┤
│  Envelope (every message)                                                   │
│  UMessage · UAttributes (incl. source/sink UUri, payload format, …)         │
│  (UUri is a field inside UAttributes — not a separate metadata layer)       │
├─────────────────────────────────────────────────────────────────────────────┤
│  Wire                                                                       │
│  Zenoh data space (peer mode — see Chapter 6)                               │
└─────────────────────────────────────────────────────────────────────────────┘
```

Layer specs (same as Phase 2): [uP-L1](https://github.com/eclipse-uprotocol/up-spec/tree/main/up-l1),
[uP-L2](https://github.com/eclipse-uprotocol/up-spec/tree/main/up-l2),
[uP-L3](https://github.com/eclipse-uprotocol/up-spec/tree/main/up-l3).

```
Phase 2 — Unix Domain Socket (one process owns the socket)

  Publisher ───► UnixDomainSocketTransport::bind ───► one subscriber


Phase 3 — Zenoh (data space, no single-owner socket)

  Publisher ───► Zenoh data space ──┬──► subscriber 1 (battery telemetry)
                                    └──► subscriber 2 (thermal logging)
```

What the wire swap buys us (same L1/L2 APIs):

- **Fan-out** — N processes subscribe independently; no shared socket.
- **Address** — URI-derived Zenoh keys, not a filesystem path.
- **Listener ownership** — each process registers its own `UListener` via `UPTransportZenoh`.
- **Framing** — Zenoh owns message boundaries (`up-frame-codec` retires).
- **Cross-host readiness** — same APIs; this demo still runs on one host.

Every process opens a Zenoh session with the default **peer** config. How peers find each other — and why this demo does not run `zenohd` — is explained in Chapter 6.

#### What Zenoh is not

- **Not a replacement for uProtocol** — it is the *wire plugin* for L1.
- **Not a COVESA deep dive** — we use Zenoh as a data-space transport only.
- **Not a multi-host requirement** — the Phase 3 demo stays on one Linux host.
- **Not a mandatory broker** — see Chapter 6 (peer scouting vs `zenohd`).
- **Not L3 uSubscription** — same-transport fan-out stays on L1 `register_listener` (Chapter 5).

---

### Chapter 3: What uProtocol puts alongside the actual message

Before the code swap, recall what travels *with* each message. Zenoh uses parts of the envelope in a way Unix Domain Socket could not.

#### One metadata level: `UAttributes`

uProtocol has **one** metadata level on the wire: **`UAttributes`**. They hold everything needed for routing and for describing the payload — message type, TTL, priority, source/sink `UUri`, correlation IDs, and so on.

When we use the L2 API (e.g. `SimplePublisher`), those attributes are **assembled from three sources**:

- **`LocalUriProvider` / `StaticUriProvider`** — source `UUri` (authority, entity, version, resource)
- **`CallOptions`** — TTL, priority, …
- **`UPayload`** — payload format hint (plus the application bytes)

```rust
publisher.publish(
    BATTERY_TELEMETRY_RESOURCE_ID,
    CallOptions::for_publish(Some(5000), None, None), // app still chooses TTL
    Some(payload),
)
```

`SimplePublisher` builds a `UMessage` whose `UAttributes` combine those pieces (type=`PUBLISH`, TTL=5000 ms, source from the URI provider, format from `UPayload`). Conceptually:

```text
UMessage {
  attributes: UAttributes {   // ← the single metadata level
    type: PUBLISH,
    ttl: 5000,
    source: UUri { authority: "my_own_car", ue_id: 0x1010, ue_version: 1, resource_id: 0x8001 },
    // … other attribute fields as applicable …
  },
  payload: <protobuf bytes>,  // format recorded in attributes
}
```

`UUri` is not a separate metadata layer beside `UAttributes`. Source and sink are **fields inside**
`UAttributes` — composed into that single metadata level, not peers of it.

#### Why Unix Domain Sockets did not use this for routing

In Phase 2, `SimplePublisher` assembled the envelope, then `UnixDomainSocketTransport::send` treated it as **opaque bytes** on a socket path. Listener matching lived only in a local table inside the bind-side process. The `UUri` in `UAttributes` did not route on the wire.

> **Unix Domain Socket has no routing layer** — one socket, one connection, one direction.

#### How Zenoh makes the envelope useful

Zenoh is **content-aware**. When `UPTransportZenoh` receives a `UMessage` via `UTransport::send`:

1. It **reads the source `UUri`** from `UAttributes` and maps it to a Zenoh key expression (e.g. derived from `my_own_car` / `0x1010` / `1` / `0x8001`).
2. It **publishes** into the Zenoh data space under that key.
3. Each subscriber's `UPTransportZenoh` registers a **Zenoh subscriber** for its `source_filter`.
4. Arriving publications are reconstructed as `UMessage` and delivered to `UListener::on_receive`.

```text
Zenoh flow (Phase 3)

  Publisher              UPTransportZenoh         Zenoh data space        UPTransportZenoh          Subscriber
     │                          │                        │                         │                    │
     │──UMessage───────────────►│                        │                         │                    │
     │                          │──publish(key, …)──────►│──fan-out to matches────►│──reconstruct──────► on_receive
     │                          │                        │──fan-out to thermal────►│──reconstruct──────► on_receive
```

**What changed:** the `UUri` inside `UAttributes` is now **first-class routing information**. In Phase 2, matching lived only in a local `HashMap` inside the bind-side process. In Phase 3, matching spans **multiple processes** — and, if we deploy that way, **multiple hosts**.

---

### Chapter 4: Code — swapping Unix Domain Sockets for Zenoh

The Phase 3 workspace (`phases/03_zenoh_topology/`) copy-forwards `up-bms-proto`, the publisher, and the battery subscriber. It **retires** `up-unix-domain-socket-transport` and `up-frame-codec`, and **adds** `up-transport-zenoh` plus `up-thermal-logging-subscriber`.

The only application change in publisher and subscribers is **how the `UTransport` is constructed**. L2 (`SimplePublisher`, `CallOptions`, `UPayload`), URI filters, authority `my_own_car`, and the `BatteryTelemetry` schema stay as in Phase 2.

```rust
// Phase 2 (Unix Domain Socket) — retired
// let transport = UnixDomainSocketTransport::connect(&socket_path);

// Phase 3 (Zenoh) — same construction on every process
let transport: Arc<dyn UTransport> = Arc::new(
    UPTransportZenoh::builder(AUTHORITY_NAME)?
        .with_config(zenoh_config::Config::default())
        .build()
        .await?,
);
```

Everything after that — `SimplePublisher::publish`, `register_listener`, `on_receive` — is unchanged.

#### Publisher (same loop, Zenoh transport)

```rust
// up-battery-telemetry-publisher/src/main.rs — Phase 3
use up_transport_zenoh::{zenoh_config, UPTransportZenoh};

let transport: Arc<dyn UTransport> = Arc::new(
    UPTransportZenoh::builder(AUTHORITY_NAME)?
        .with_config(zenoh_config::Config::default())
        .build()
        .await?,
);
let publisher = SimplePublisher::new(transport, uri_provider);

for i in 1..=EXPECTED_MESSAGE_COUNT {
    let telemetry = BatteryTelemetry { /* … */ };
    let payload = UPayload::try_from_protobuf(telemetry)?;
    publisher
        .publish(
            BATTERY_TELEMETRY_RESOURCE_ID,
            CallOptions::for_publish(Some(5000), None, None), // TTL still the app's choice
            Some(payload),
        )
        .await?;
}
```

#### Subscriber (same on_receive, Zenoh transport)

```rust
// up-telemetry-subscriber/src/main.rs — Phase 3
let transport: Arc<dyn UTransport> = Arc::new(
    UPTransportZenoh::builder(AUTHORITY_NAME)?
        .with_config(zenoh_config::Config::default())
        .build()
        .await?,
);
transport
    .register_listener(&source_filter, None, listener)
    .await?;
```

The URI filter (`source_filter` with resource ID `0x8001`) works identically. Zenoh delivers matching publications to each subscriber process.

#### Transport config — socket path replaced

Phase 2 used `{cwd}/tmp/uprotocol_twin.sock`. Phase 3 uses `zenoh_config::Config::default()` — Zenoh **peer** mode with UDP multicast scouting, and no filesystem path. Chapter 6 explains what that mode means and why we do not start a `zenohd` router alongside the three binaries.

If we wanted an explicit remote endpoint (for example a router or a peer on another host):

```rust
let mut config = zenoh_config::Config::default();
config.connect.endpoints = vec!["tcp/192.168.1.100:7447".parse()?];
```

---

### Chapter 5: Fan-out payoff — and what is *not* L3 uSubscription

Phase 1 mentioned the **Thermal Management Logging Application** as a second consumer of the battery telemetry stream. It checks cell temperature and prints a warning if the temperature is over 25°C.

If we had implemented it on Unix Domain Socket, it would have had to live in the same process as the telemetry subscriber. Phase 3 delivers what Phase 1 only described: an **independent thermal logging subscriber** — a separate application that receives the same protobuf messages from the Zenoh data space.

#### The thermal subscriber

```rust
// up-thermal-logging-subscriber/src/main.rs — new in Phase 3
struct ThermalLoggingListener { /* … */ }

#[async_trait]
impl UListener for ThermalLoggingListener {
    async fn on_receive(&self, msg: UMessage) {
        if let Ok(telemetry) = msg.extract_protobuf::<BatteryTelemetry>() {
            let temp = telemetry.temp_celsius;
            if temp > 25 {
                println!("WARNING — cell temperature {temp}°C exceeds threshold");
            } else {
                println!("Cell temperature {temp}°C — OK");
            }
        }
    }
}

// main: same UPTransportZenoh::builder + register_listener as the battery subscriber
```

This listener extracts the same `BatteryTelemetry` struct, but applies **thermal-specific logic**. No new schema; no socket path; no shared process.

#### Transport-native pub/sub vs L3 uSubscription

Phase 2's `register_listener` was a **local** operation on the bind-side `UnixDomainSocketTransport`. Phase 3 still calls `register_listener` — but under Zenoh that interest is a **Zenoh subscription**. When the publisher sends resource `0x8001`, **every process** that registered a matching filter receives the message. That is **Zenoh's native pub/sub** (L1), not a special uProtocol L3 registration step.

uProtocol also defines an L3 application service named **[uSubscription](https://github.com/eclipse-uprotocol/up-spec/tree/main/up-l3/usubscription)**. That service matters when a
uEntity wants topics from a **different authority**, or when publisher and subscriber sit on
**different transports** (for example Zenoh on one side and MQTT5 on the other) and still need
transparent pub/sub. In those cases, transport-native interest alone is not enough; uSubscription
signals that interest across the uProtocol network.

**This tutorial does not use uSubscription.** Our publisher and both subscribers share one
authority (`my_own_car`) and one transport (Zenoh), so Zenoh's own pub/sub is sufficient. Fan-out
here is only `UTransport::register_listener` + Zenoh — not an L3 “PUBLISH registration” step.
Look back at the Chapter 2 layer map: the L3 band is present for orientation, and stays idle in this demo.

---

### Chapter 6: Running the multi-subscriber demo

#### Peer mode, routers, and why this demo skips `zenohd`

Zenoh can run processes in more than one role. Two that matter for reading our Phase 3 code:

1. **Peer** — a process that both sends and receives in the Zenoh data space. With `zenoh_config::Config::default()`, each of our binaries opens a **peer** session. Peers can discover one another using Zenoh’s **UDP multicast scouting**: on a network that allows multicast, they find each other without a central process.
2. **Router (`zenohd`)** — a long-running Zenoh process that **relays** traffic and helps clients that cannot (or should not) rely on multicast alone. Typical reasons to introduce a router: peers on different subnets or hosts where multicast does not cross the boundary; locked-down environments that block scouting; larger fleets where a stable rendezvous point is easier to operate than mesh discovery.

Our demo does **not** start `zenohd`. The publisher, battery subscriber, and thermal subscriber all run on **one Linux host**, all with the default peer config. In that layout, multicast scouting is enough: the three sessions form a small peer mesh and Zenoh delivers publications to every matching `register_listener`. Adding a router would not change the uProtocol lesson of this chapter (fan-out via the same L1/L2 APIs after a wire swap). We mention routers so the absence of `zenohd` is a deliberate choice, not an omission.

The optional snippet in Chapter 4 (`config.connect.endpoints = …`) is what we would use later if we *did* point at a router or a remote peer — it is not required for the run below.

#### Commands (subscribers first)

Start the two subscribers first, then the publisher (three terminals). The publisher sends a short burst and exits; if a subscriber is not yet registered, it can miss messages.

```bash
# Terminal 1 — battery telemetry subscriber (start first)
cargo run --manifest-path phases/03_zenoh_topology/Cargo.toml -p up-telemetry-subscriber

# Terminal 2 — thermal logging subscriber (start second)
cargo run --manifest-path phases/03_zenoh_topology/Cargo.toml -p up-thermal-logging-subscriber

# Terminal 3 — battery publisher (start last)
cargo run --manifest-path phases/03_zenoh_topology/Cargo.toml -p up-battery-telemetry-publisher
```

The publisher sends five messages (same as Phases 1–2). Both subscribers should receive all five. No process shares a socket, a listener table, or a filesystem path.

Build the whole workspace:

```bash
cargo build --manifest-path phases/03_zenoh_topology/Cargo.toml
```

---

### Chapter 7: A look at what we have done

```
╔═════════════════════════════════════════════════════════════╗
║ Publisher (Phase 3)                                         ║
║                                                             ║
║   BatteryTelemetry { soc_percent, temp_celsius }            ║
║      ↓                                                      ║
║   UPayload::try_from_protobuf (L2)                          ║
║      ↓                                                      ║
║   SimplePublisher::publish(resource_id, CallOptions, ...)   ║
║      ↓                                                      ║
║   UPTransportZenoh::send  →  Zenoh data space               ║
╚═════════════════════════════════════════════════════════════╝

╔═════════════════════════════════════════════════════════════╗
║ Battery Subscriber (Phase 3)                                ║
║                                                             ║
║   Zenoh data space  →  UPTransportZenoh                     ║
║      ↓                                                      ║
║   filter match → UListener::on_receive(msg)                 ║
║      ↓                                                      ║
║   msg.extract_protobuf::<BatteryTelemetry>()                ║
║      ↓                                                      ║
║   SoC: 76.3%, Temp: 22°C                                    ║
╚═════════════════════════════════════════════════════════════╝

╔═════════════════════════════════════════════════════════════╗
║ Thermal Subscriber (Phase 3 — new)                          ║
║                                                             ║
║   Zenoh data space  →  UPTransportZenoh                     ║
║      ↓                                                      ║
║   filter match → UListener::on_receive(msg)                 ║
║      ↓                                                      ║
║   msg.extract_protobuf::<BatteryTelemetry>()                ║
║      ↓                                                      ║
║   temp > 25°C? → WARNING  or  temp OK                       ║
╚═════════════════════════════════════════════════════════════╝
```

Notice the shape: both subscribers are **structurally identical**. They differ only in what they *do* with the telemetry data. That is the payoff.

---

### Chapter 8: What we learned

- **Zenoh replaces Unix Domain Sockets as the L1 transport** — same uProtocol API, different plugin.
- **Fan-out works on one host** — the thermal logger attaches without touching the battery subscriber.
- **Business logic survived the swap** — `SimplePublisher`, `UListener`, and `BatteryTelemetry` are unchanged.
- **`UAttributes` is the single metadata level** — L2 assembles it from URI provider + `CallOptions` + `UPayload` format; Zenoh uses the embedded `UUri` for keys.
- **Same-transport fan-out is Zenoh native pub/sub** — not L3 uSubscription (that service is for cross-authority / cross-transport interest).
- **Zenoh peer mode on one host — no `zenohd`** — Chapter 6: peers scout via UDP multicast; a router would not change this chapter’s fan-out lesson.
- **Phase 2 limits are resolved** — fan-out, address, listener ownership, framing, and cross-host readiness.

Phase 3 proves that uProtocol's layer design pays off: **swap the wire, keep the application**.

---

### Chapter 9: Where we still fall short

Phase 3 resolves the fan-out problem that Phase 1 identified and Phase 2 documented. Still open:

- **No L3 [uDiscovery](https://github.com/eclipse-uprotocol/up-spec/tree/main/up-l3/udiscovery)** — listeners are URI-filtered, but there is no registry of "who publishes what" (the service exists in the architecture; this tutorial has no implementation).
- **Hardcoded constants** — `AUTHORITY_NAME`, entity/resource IDs live in `up-bms-proto::constants`.
- **Single-topic demo** — only `BatteryTelemetry` (resource `0x8001`).
- **Cross-transport pub/sub** — would involve L3 **uSubscription**, unused while all peers share Zenoh.

These are configuration / later-service gaps, not a failure of the L1/L2 design we already have.

---

### Appendix: Key takeaways

1. **L1 (`UTransport` / `UListener`) absorbs the transport swap.** The binaries changed the transport construction block. The business logic (publish loop, `on_receive`) is untouched.

2. **Zenoh's data space enables fan-out where Unix Domain Socket could not.** A second subscriber attaches by running a new binary and registering the same URI filter — no socket sharing, no `SOCKET_PATH`. Chapter 6 covers peer mode vs `zenohd`.

3. **One metadata level: `UAttributes`.** Assembled at L2 from `LocalUriProvider` / `StaticUriProvider`, `CallOptions`, and `UPayload` format. Zenoh turns the source `UUri` into a key expression.

4. **Phase 1's thermal logger finally gets its own process.** Three phases of narrative resolve in one extra `cargo run`.

---

### Appendix B: Workspace diff — Phase 2 vs Phase 3

```
phases/02_uprotocol_semantics/           phases/03_zenoh_topology/
(frozen)                                 (active — Phase 3)
─────────────────────────────            ─────────────────────────────
up-bms-proto                       ──►   up-bms-proto (copy-forward)
up-battery-telemetry-publisher     ──►   up-battery-telemetry-publisher
up-telemetry-subscriber            ──►   up-telemetry-subscriber
up-unix-domain-socket-transport    ──X   (retired)
up-frame-codec                     ──X   (retired)
                                         up-thermal-logging-subscriber (new)
                                         + dependency: up-transport-zenoh
```

**Nothing was deleted from the repo** — Phase 2 code remains in `phases/02_uprotocol_semantics/` for side-by-side study.

---

### Appendix C: Protobuf schema (bms_telemetry.proto)

Identical to Phase 2. The `.proto` file is unchanged because the payload contract has not changed:

```protobuf
syntax = "proto3";

package tutorial.bms.v1;

message BatteryTelemetry {
  float soc_percent = 1;
  int32 temp_celsius = 2;
}
```

The publisher still creates `BatteryTelemetry`, wraps it with `UPayload::try_from_protobuf`, and publishes. Subscribers still `extract_protobuf`. Zenoh routes by `UUri`, not by schema.

---

### Appendix D: References

- [Zenoh documentation](https://zenoh.io/docs/) — data-space transport overview
- [Eclipse uProtocol — up-transport-zenoh-rust](https://github.com/eclipse-uprotocol/up-transport-zenoh-rust)
- [Vehicle Signal Specification (VSS)](https://covesa.github.io/vehicle_signal_specification/) — COVESA standard for vehicle data modelling
