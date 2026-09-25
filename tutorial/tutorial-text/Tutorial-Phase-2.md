### Prologue

This is the second phase of the tutorial where we will build on everything that was established in Phase-1 (the previous chapter) and then some. So, if we
have not read the previous tutorial [Phase 1 tutorial](Tutorial-Phase-1.md), please do so. The code 
corresponding to that (previous) tutorial is in `phases/01_raw_sockets/`. This chapter's 
code lives in `phases/02_uprotocol_semantics/`.


In this chapter, we are going to:

- See where the Phase-1 architecture falls short
- Introduce _uProtocol_'s built-in layering and how it helps
- Replace the hand-rolled socket code with one L1 type, `UnixDomainSocketTransport` (`connect` /
  `bind`) — uProtocol's `UTransport` over a Unix Domain Socket
- Introduce `UPayload` properly at L2 (deferred from Phase 1) and replace RAW CAN bytes with typed
  Protobuf messages
- Replace Phase 1's `UMessageBuilder` publish loop with `SimplePublisher` + `CallOptions` (L2)
- Replace the ad-hoc subscriber decode loop with `UListener` callbacks
- Be honest about what still doesn't work

Let's dive in.

----

### Chapter 1: Where Phase-1 left us

In Phase-1, we successfully sent a `UMessage` from the battery-telemetry-publisher to the 
telemetry-subscriber over a Unix Domain Socket. We packed SoC and temperature into 8 RAW bytes 
using `pack_bms_can_frame` and unpacked with `unpack_bms_can_frame`. It worked.

But, let's re-read the subscriber code from Phase-1:

```rust
// Phase-1 subscriber — a transport program in disguise
loop {
    let (mut stream, _) = listener.accept().await?;
    tokio::spawn(async move {
        let mut len_bytes = [0u8; 4];
        if stream.read_exact(&mut len_bytes).await.is_err() { return; }
        let body_len = u32::from_be_bytes(len_bytes) as usize;
        let mut body_bytes = vec![0u8; body_len];
        if stream.read_exact(&mut body_bytes).await.is_err() { return; }
        // ... decode, unpack CAN frame, print ...
    });
}
```
And the publisher:

```rust
// Phase-1 publisher — builder + raw socket (no L1/L2 helpers yet)
let source_uri = UUri::try_from_parts("my_own_car", 0x1010, 1, 0x8001)?;
let message = UMessageBuilder::publish(source_uri)
    .with_ttl(5000)
    .build_with_payload(
        pack_bms_can_frame(battery_pct, temp_c).to_vec(),
        UPayloadFormat::UPAYLOAD_FORMAT_RAW,
    )?;
let framed = serialize_for_unix_socket(&message)?;
let socket_path = up_frame_codec::socket_path()?;
let mut stream = UnixStream::connect(&socket_path).await?;
stream.write_all(&framed).await?;
```
```
┌─────────────────────────────────────────────────────────────────────────┐
│                                                                         │
│  Publisher (main.rs)                             Subscriber (main.rs)   │
│  ┌───────────────────┐                           ┌───────────────────┐  │
│  │ 1. Build UMessage │                           │ 1. Accept socket  │  │
│  │ 2. Serialize      │   Unix Domain Socket      │ 2. Read + decode  │  │
│  │ 3. Connect + send │  ───────────────────────▶ │ 3. Unpack CAN     │  │
│  │                   │                           │ 4. Print telemetry│  │
│  └───────────────────┘                           └───────────────────┘  │
│                                                                         │
│  -- Everything in one binary — no separation of transport logic from    │
│  from business logic.                                                   │
│  -- CAN byte layout is a shared secret.                                 │
│  -- No way to add a second consumer.                                    │
└─────────────────────────────────────────────────────────────────────────┘
```
### Something is not quite right there

Both binaries embed transport details that do not belong in battery telemetry code.

- The **publisher** does three jobs: build a `UMessage`, frame it, and connect+write to a socket.
- The **subscriber** does three jobs: accept a connection, read+decode a frame, interpret the payload
- If we want a second consumer for the same telemetry (say, a thermal management logging engine),
  we cannot give it access to the subscriber's socket — there is only one accept loop.
- The **payload** is RAW bytes with a secret CAN-byte layout. If we change the layout, then both 
  sides must be updated in lockstep.

Phase-2 fixes these using uProtocol's own facilities: L1 Transport, L2 Communication patterns,
and typed Protobuf payloads.

----

### Chapter 2: The layers of uProtocol (a map)

uProtocol is organized in layers. This diagram is the **canonical layer map** for the tutorial —
Phases 1 and 3 use the same stack; only which bands are active changes.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  Application / uEntity                                                      │
│  up-battery-telemetry-publisher · up-telemetry-subscriber                   │
├─────────────────────────────────────────────────────────────────────────────┤
│  L2 — Communication                                                         │
│  SimplePublisher · CallOptions · UPayload                                   │
│  "PUBLISH this resource; attach typed payload + options"                    │
├─────────────────────────────────────────────────────────────────────────────┤
│  L1 — Transport                                                             │
│  UTransport · UListener · UnixDomainSocketTransport                         │
│  send · register_listener                                                   │
│  "Move this UMessage; invoke matching listeners when it arrives"            │
├─────────────────────────────────────────────────────────────────────────────┤
│  Envelope (every message)                                                   │
│  UMessage · UAttributes (incl. source/sink UUri, payload format, …)         │
│  (UUri is a field inside UAttributes — not a separate metadata layer)       │
├─────────────────────────────────────────────────────────────────────────────┤
│  Wire                                                                       │
│  socket_path() → {cwd}/tmp/uprotocol_twin.sock                              │
│  serialize_for_unix_socket / deserialize_for_unix_socket                    │
└─────────────────────────────────────────────────────────────────────────────┘
```

Phase 1 lived mainly at the **envelope** and **wire** layers (builder → `UMessage` → framed
socket bytes). Phase 2 introduces L1 (`UTransport`, `UListener`, `UnixDomainSocketTransport`) and
L2 (`SimplePublisher`, `CallOptions`, `UPayload`). The wire stays a Unix Domain Socket with
length-prefix framing — wrapped behind those abstractions. L3 application services
(uSubscription, uDiscovery, …) are not part of this phase. Specs for those layers live in
[uP-L1 (Transport)](https://github.com/eclipse-uprotocol/up-spec/tree/main/up-l1),
[uP-L2 (Communication)](https://github.com/eclipse-uprotocol/up-spec/tree/main/up-l2), and
[uP-L3 (Application)](https://github.com/eclipse-uprotocol/up-spec/tree/main/up-l3).
Crates under `phases/02_uprotocol_semantics/` keep the same pin as Phase 1: `up-rust = "0.9.0"`.

Both processes attach the **same** L1 type. Socket bind vs connect is wire setup, not a Client/Server
split in the application model. L2 appears only on the publish path in this demo (`SimplePublisher`);
the subscriber registers an L1 `UListener` directly — uProtocol also defines an L2 `Subscriber`, but
we do not use it here (same-transport interest does not need it).

```
┌────────────────────────────────────────────────────────────────────────────────────┐
│  Phase 2 — peer apps; same L1 transport on both sides                              │
│                                                                                    │
│  Publisher app                         Subscriber app                              │
│  ┌───────────────────────────┐         ┌────────────────────────────────┐          
│  │ BatteryTelemetry          │         │ BatteryTelemetryListener       │          │
│  │ + UPayload                │         │ (impl UListener)               │          │
│  └─────────────┬─────────────┘         └─────────────▲──────────────────┘          │
│                │ publish()                           │ on_receive()                │
│  ┌─────────────▼─────────────┐                       │                             │
│  │ SimplePublisher (L2)      │         (no L2 Subscriber in this demo)             │
│  │ CallOptions               │                             │                       │
│  └─────────────┬─────────────┘                             │                       │
│                │ UTransport::send                          │                       │
│  ┌─────────────▼─────────────┐           ┌─────────────────┴──────────────────┐    │
│  │ UnixDomainSocketTransport │ ──────▶   │ UnixDomainSocketTransport::bind    │    │
│  │ ::connect                 │  UMessage │                                    │    │
│  │                           │           │ register_listener(...)             │    │
│  └───────────────────────────┘           └────────────────────────────────────┘    │
│                         Unix Domain Socket                                         │
│                                                                                    │
│       Shared: up-bms-proto · up-frame-codec · up-unix-domain-socket-transport      │
└────────────────────────────────────────────────────────────────────────────────────┘
```
----

### Chapter 3: What changed in the workspace

Phase-2 lives in `phases/02_uprotocol_semantics/`. The workspace grew from 3 to 5 crates:

```
phases/02_uprotocol_semantics/
├── Cargo.toml
└── crates/
    ├── up-frame-codec/                     # length-prefix serialize (aligned with Phase 1)
    ├── up-bms-proto/                       # NEW: shared protobuf schema + demo constants
    ├── up-unix-domain-socket-transport/    # NEW: UTransport over a Unix Domain Socket
    ├── up-battery-telemetry-publisher/     # REFACTORED: SimplePublisher + UPayload
    └── up-telemetry-subscriber/            # REFACTORED: UListener
```
----

### Chapter 4: `up-bms-proto` — why typed protobuf payloads

Phase 1 deliberately avoided `UPayload`. The L1 path builds a `UMessage` with
`UMessageBuilder::build_with_payload(raw_bytes, UPAYLOAD_FORMAT_RAW)`. The application still had
to invent a byte layout:

```rust
// Phase 1 — RAW bytes in the envelope; no UPayload type
.build_with_payload(
    pack_bms_can_frame(battery_pct, temp_c).to_vec(),
    UPayloadFormat::UPAYLOAD_FORMAT_RAW,
)
```
And on the subscriber side:

```rust
let payload = msg.payload.unwrap();
let (pct, temp) = unpack_bms_can_frame(&payload);
```
`pack_bms_can_frame` and `unpack_bms_can_frame` are **C code in Rust clothing**. They exist
because the publisher and subscriber need a way to agree on how SoC and temperature are laid
out in a byte buffer. In C-based automotive systems, this agreement comes from a DBC file (CAN
database). Here, it comes from a pair of functions.

Let's look at the actual functions from Phase-1:

```rust
// 1 LSB = 0.5% SoC — this is a DBC convention in disguise
fn pack_bms_can_frame(soc_pct: f32, temp_c: f32) -> [u8; 8] {
    let soc_encoded = (soc_pct / 0.5) as u16;
    let temp_encoded = (temp_c * 10.0) as i16;
    let mut buf = [0u8; 8];
    buf[0..2].copy_from_slice(&soc_encoded.to_be_bytes());
    buf[2..4].copy_from_slice(&temp_encoded.to_be_bytes());
    buf
}
```
We can spot the problems:

1. **Scale factors are buried in code.** `0.5` and `10.0` are kind of, magic numbers. If we 
   change the SoC resolution from 0.5% to 0.1%, we must find and update both pack *and* unpack functions
   in lockstep. If we miss one, the subscriber prints garbage.

2. **Byte layout is implicit.** `buf[0..2]` for SoC, `buf[2..4]` for temperature. These offsets
   are invisible to anyone reading the code unless they carefully trace both functions.

3. **There is no schema.** Nothing tells us "this buffer contains two fields named
   `soc_percent` and `temp_celsius`". The field names exist only in the programmer's head and
   in Rust's variable names. A DBC file at least names the signals.

4. **Adding a third field is error-prone.** Double the buffer? Shift offsets? Every change
   requires a coordinated deploy of both publisher and subscriber.

**Protobuf solves all four problems in one move.**

Instead of a pair of pack/unpack functions with hidden offsets, we write a schema file:

```protobuf
// proto/bms_telemetry.proto
syntax = "proto3";
package tutorial.bms.v1;

message BatteryTelemetry {
    float soc_percent = 1;
    int32 temp_celsius = 2;
}
```
- **No more bit-level encoding.** The raw CAN buffer required dividing SoC by 0.5 and
  multiplying temperature by 10 to fit into `u16`/`i16` — then the reverse on the subscriber
  side. Protobuf lets us express `soc_percent` as `float` and `temp_celsius` as `int32`
  directly. The scale conversion (how the sensor's ADC count maps to a physical value) is
  still an application concern — protobuf cannot annotate units — but the *wire-level*
  encode/decode arithmetic is gone.
- **Byte layout is gone.** Protobuf field numbers (`= 1`, `= 2`) replace byte offsets. The
  encoding is handled by the protobuf library.
- **The schema is the contract.** Both publisher and subscriber compile from the same `.proto`
  file. Any change to the schema is immediately visible at compile time — not at 2 AM when a
  subscriber prints -273.15°C because of a scale mismatch.
- **Adding a third field is safe.** Appending `optional int32 cell_voltage = 3;` to the message
  does not break existing publishers or subscribers — protobuf skips unknown fields at deserialisation.

The `up-bms-proto` crate wraps this proto file in a Cargo crate:

```rust
// build.rs
fn main() -> Result<(), Box<dyn std::error::Error>> {
    protobuf_codegen::Codegen::new()
        .protoc()
        .protoc_path(&protoc_bin_vendored::protoc_bin_path().unwrap())
        .include("proto")
        .input("proto/bms_telemetry.proto")
        .cargo_out_dir("gen")
        .run_from_script();
    Ok(())
}
```
It also exports demo URI constants (authority aligned with Phase 1; the socket path lives in
`up-frame-codec`, not here):

```rust
// up-bms-proto/src/lib.rs
pub mod constants {
    pub const AUTHORITY_NAME: &str = "my_own_car";
    pub const PUBLISHER_UE_ID: u32 = 0x1010;
    pub const PUBLISHER_UE_VERSION: u8 = 0x01;
    pub const BATTERY_TELEMETRY_RESOURCE_ID: u16 = 0x8001;
    pub const EXPECTED_MESSAGE_COUNT: u32 = 5;
}
```
Both the publisher and subscriber depend on `up-bms-proto`. There is no more DBC secret — the
`.proto` file is the **contract**.

----

A quick note on what this *does not* solve: protobuf gives us a typed data contract, but the
URI conventions (`PUBLISHER_UE_ID = 0x1010`, `BATTERY_TELEMETRY_RESOURCE_ID = 0x8001`) and the
socket path from `up_frame_codec::socket_path()` (`{cwd}/tmp/uprotocol_twin.sock`) are still
shared out of band. Those will move to configuration or discovery in later phases. For now, the
constants crate plus the shared frame-codec helpers keep the demo honest.

### Chapter 5: What does the Phase-1 subscriber *actually* do?

Before we look at `up-unix-domain-socket-transport`, let's visit Phase-1 subscriber code 
and trace what it does for every incoming message. The key section here:

```rust
// Phase-1 subscriber (simplified accept loop)
loop {
    let (mut stream, _) = listener.accept().await?;
    tokio::spawn(async move {
        // Step 1 — read the 4-byte length prefix
        let mut len_bytes = [0u8; 4];
        if stream.read_exact(&mut len_bytes).await.is_err() { return; }
        let body_len = u32::from_be_bytes(len_bytes) as usize;

        // Step 2 — read that many bytes
        let mut body_bytes = vec![0u8; body_len];
        if stream.read_exact(&mut body_bytes).await.is_err() { return; }

        // Step 3 — deserialise the protobuf UMessage
        let msg = UMessage::parse_from_bytes(&body_bytes[..]).ok()?;

        // Step 4 — extract the CAN payload and interpret
        let payload = msg.payload?;
        let (pct, temp) = unpack_bms_can_frame(&payload);
        println!("-> State of Charge: {:.1}%", pct);
        println!("-> Cell Temp: {} °C", temp);
    });
}
```
Let's count the implicit responsibilities in that one spawned closure:

| Step | Responsibility | Who should own it |
|---|---|---|
| 1–2 | Read framed bytes from a Unix socket | **Transport layer** — the socket code |
| 3 | Deserialise a protobuf `UMessage` | **Transport layer** — `read_framed_message` |
| 4 | Extract the typed payload and print | **Application** — the subscriber |

Three responsibilities. Two of them have nothing to do with battery telemetry.

If we wanted to add a second consumer — say, a thermal management logging engine — we would have
to copy the socket-reading code. If we wanted to switch from Unix Domain Socket to another 
transport - say, Zenoh - we would rewrite the socket-reading code. If we wanted to run two 
listeners inside the same process (one for SoC, one for temperature), we would need to duplicate the accept loop or build our own dispatch table.

**This is where a transport crate enters the story.**

The transport crate's job is to own steps 1–3 so the application code (step 4) does not need to
know about sockets, length prefixes, or protobuf envelope parsing. uProtocol gives us a trait
for this:

```
╔═══════════════════════════════════════════════╗
║   UTransport (trait)                          ║
╠═══════════════════════════════════════════════╣
║  send(message)                                ║
║  register_listener(filter, listener)          ║
║  unregister_listener(filter, listener)        ║
╚═══════════════════════════════════════════════╝
```
`UTransport` says: "I know how to move a `UMessage` from point A to point B. I know how to
accept registrations from interested parties. I know when to call those parties. We tell it
_who_ is interested and _what_ we want to send; the transport handles the rest."

This is not an abstraction for the joy of abstraction. It is a **responsibility boundary**.
The transport code lives in `up-unix-domain-socket-transport`. The subscriber code lives in
`up-telemetry-subscriber`. They depend on each other through the `UTransport` + `UListener`
traits, not through a shared socket path embedded in application code.

In the same way, the publisher should not know about socket I/O either. The publisher says
"publish this data"; the *transport* says "I will frame it, connect to the socket, and write
it." Both sides use the **same** type — `UnixDomainSocketTransport` — with `connect` on the
publisher and `bind` on the subscriber. That is wire setup, not a Client/Server split in the
uProtocol model.

Let's look at `up-unix-domain-socket-transport` carefully.

----

### Chapter 6: `up-unix-domain-socket-transport` — the transport crate

`up-unix-domain-socket-transport` lives at
`phases/02_uprotocol_semantics/crates/up-unix-domain-socket-transport/`. It exposes **one**
`UTransport` type:

**`UnixDomainSocketTransport::bind`** — binds a socket path, spawns an accept loop, reads framed
messages, and dispatches to registered listeners:

```rust
let socket_path = up_frame_codec::ensure_socket_dir()?;
let transport = UnixDomainSocketTransport::bind(&socket_path).await?;
transport
    .register_listener(&source_filter, None, listener)
    .await?;
```
The dispatch logic (simplified from the crate):

```rust
for registered in listeners.iter() {
    if registered.matches_msg(&message) {
        registered.on_receive(message.clone()).await;
    }
}
```
Where `matches_msg` uses `up-rust`'s `UUri::matches` — the same URI matching rules that
`LocalTransport` uses. A listener fires when the source filter matches the message's source URI.

**`UnixDomainSocketTransport::connect`** — send-only attachment: connect per `send`, frame via
`up-frame-codec`, write bytes:

```rust
let socket_path = up_frame_codec::socket_path()?;
let transport = UnixDomainSocketTransport::connect(&socket_path);
transport.send(message).await?;
```
`register_listener` after `connect` returns `UNIMPLEMENTED` — listeners live on the bind side.

#### How does this compare to Phase-1?

Phase-1's subscriber was doing the transport layer's job. It read raw bytes from a socket,
decoded them, and interpreted the payload — all in one spawned task. The transport crate now
owns the read-decode-dispatch pipeline. The subscriber only implements `UListener::on_receive`.

```
Phase-1:  socket → read_exact(4) → read_exact(N) → parse → application logic
Phase-2:  socket → [UnixDomainSocketTransport] → filter match → UListener::on_receive
                                     ↑
                            (centralised, reusable)
```
----

### Chapter 7: Refactored publisher — `SimplePublisher` + L2 `UPayload`

Phase 1 put raw bytes into the envelope with `UMessageBuilder::build_with_payload`. Phase 2
introduces **`UPayload` as an L2 handle**: bytes plus format hint that `SimplePublisher` folds
into `UAttributes` / `UMessage`. L1 still only sees `UMessage`; applications talk L2 with
`UPayload::try_from_protobuf(...)`.

Let's look at the publisher code. Compare it with the Phase-1 version from the previous chapter.

```rust
// up-battery-telemetry-publisher/src/main.rs (Phase 2)

use up_bms_proto::constants::*;
use up_bms_proto::BatteryTelemetry;
use up_rust::communication::{CallOptions, Publisher, SimplePublisher, UPayload};
use up_rust::{StaticUriProvider, UTransport};
use up_unix_domain_socket_transport::UnixDomainSocketTransport;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    env_logger::init();
    println!("--- Battery telemetry publisher starting ---");

    let uri_provider = Arc::new(StaticUriProvider::new(
        AUTHORITY_NAME,
        PUBLISHER_UE_ID,
        PUBLISHER_UE_VERSION,
    ));
    let socket_path = up_frame_codec::socket_path()?;
    let transport: Arc<dyn UTransport> =
        UnixDomainSocketTransport::connect(&socket_path);
    let publisher = SimplePublisher::new(transport, uri_provider);

    let mut rng = rand::rng();

    for i in 1..=EXPECTED_MESSAGE_COUNT {
        let telemetry = BatteryTelemetry {
            soc_percent: rng.random_range(75.0..78.9),
            temp_celsius: rng.random_range(20..=25),
            ..Default::default()
        };

        println!("Message {}: SoC = {:.1}%, Temp = {}°C",
            i, telemetry.soc_percent, telemetry.temp_celsius);

        // L2: typed protobuf + format → SimplePublisher builds the UMessage
        let payload = UPayload::try_from_protobuf(telemetry)?;
        publisher
            .publish(
                BATTERY_TELEMETRY_RESOURCE_ID,
                // Same 5 s usefulness window as Phase 1's with_ttl(5000):
                // the *application* still chooses TTL; CallOptions carries it.
                CallOptions::for_publish(Some(5000), None, None),
                Some(payload),
            )
            .await
            .map_err(|err| anyhow::anyhow!("publish failed: {err}"))?;
        println!();
    }
    Ok(())
}
```

**Where did `with_ttl(5000)` go?** It did not disappear. Phase 1 set TTL on the builder
(`UMessageBuilder::publish(...).with_ttl(5000)`). Phase 2 still has the application choose that
value; it passes it through L2 as `CallOptions::for_publish(Some(5000), ...)`.
`SimplePublisher` copies options into `UAttributes.ttl` when it builds the `UMessage` — it does
not invent a default TTL for us. Omit `Some(5000)` (pass `None`) and the published attributes
have no TTL.

#### What did we manage to get rid of?

| Phase-1 code | Phase-2 equivalent |
|---|---|
| `UUri::try_from_parts(...)` | `StaticUriProvider::new(`<br>`authority, ue_id, version)` |
| `UMessageBuilder::publish(...)`<br>`.with_ttl(...)`<br>`.build_with_payload(raw, RAW)` | `SimplePublisher::publish`<br>+ `CallOptions` (TTL, …)<br>+ `UPayload` |
| RAW CAN `pack_bms_can_frame` | `UPayload::try_from_protobuf(`<br>`telemetry)` |
| `serialize_for_unix_socket`<br>+ `UnixStream::connect`<br>+ `write_all` | `UnixDomainSocketTransport`<br>`::connect` → `UTransport::send` |

**We no longer create `UMessage` by hand.** The `SimplePublisher` (from `up-rust`'s L2 helpers)
assembles the envelope from three application-supplied pieces: source URI (`StaticUriProvider`),
options such as TTL (`CallOptions`), and payload bytes + format (`UPayload`). Message type
`UMESSAGE_TYPE_PUBLISH` and a fresh message ID are filled in by the helper.

**We no longer pack CAN bytes.** `UPayload::try_from_protobuf(telemetry)` serialises the
`BatteryTelemetry` protobuf message and wraps it with format hint
`UPAYLOAD_FORMAT_PROTOBUF_WRAPPED_IN_ANY`. No DBC offsets, no bit-shifting.

**We no longer connect and write to the socket.** That is inside
`UnixDomainSocketTransport::send` (after `connect`), which implements `UTransport::send`. The
publisher only says "publish this data"; the how of moving bytes lives in the transport layer.

----

### Chapter 8: Refactored subscriber — `UListener` + `UnixDomainSocketTransport::bind`

Now the subscriber. This is a bigger change. The entire socket loop is gone.

```
╔════════════════════════════════╗
║   UListener (trait)            ║
╠════════════════════════════════╣
║   async fn on_receive(         ║
║       &self,                   ║
║       msg: UMessage            ║
║   )                            ║
╚════════════════════════════════╝
```
We implement this trait:

```rust
struct BatteryTelemetryListener {
    received: Arc<AtomicU32>,
    shutdown: Arc<Notify>,
}

#[async_trait]
impl UListener for BatteryTelemetryListener {
    async fn on_receive(&self, msg: UMessage) {
        match msg.extract_protobuf::<BatteryTelemetry>() {
            Ok(telemetry) => {
                let count = self.received.fetch_add(1, Ordering::SeqCst) + 1;
                println!(
                    "[Battery telemetry subscriber] Processing incoming telemetry...\n\
                     -> State of Charge: {:.1}%\n\
                     -> Cell Temp: {} °C",
                    telemetry.soc_percent, telemetry.temp_celsius,
                );
                if count >= EXPECTED_MESSAGE_COUNT {
                    self.shutdown.notify_one();
                }
            }
            Err(err) => eprintln!("Failed to decode BatteryTelemetry payload: {err}"),
        }
    }
}
```
And wire it into `main`:

```rust
#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    env_logger::init();

    let uri_provider = StaticUriProvider::new(
        AUTHORITY_NAME,
        PUBLISHER_UE_ID,
        PUBLISHER_UE_VERSION,
    );
    let source_filter = uri_provider.get_resource_uri(BATTERY_TELEMETRY_RESOURCE_ID);

    let received = Arc::new(AtomicU32::new(0));
    let shutdown = Arc::new(Notify::new());
    let listener = Arc::new(BatteryTelemetryListener {
        received,
        shutdown: shutdown.clone(),
    });

    let socket_path = up_frame_codec::ensure_socket_dir()?;
    let transport = UnixDomainSocketTransport::bind(&socket_path).await?;
    transport
        .register_listener(&source_filter, None, listener)
        .await?;

    println!(
        "Battery telemetry subscriber listening on: {} (expecting {} messages)",
        socket_path.display(),
        EXPECTED_MESSAGE_COUNT
    );

    shutdown.notified().await;
    println!("Received {EXPECTED_MESSAGE_COUNT} messages — exiting.");
    Ok(())
}
```
#### What changed from Phase-1?

Every line related to socket I/O is gone. In its place:

- `UnixDomainSocketTransport::bind(&socket_path)` — binds the socket and runs the accept loop
- `register_listener(&source_filter, None, listener)` — declares interest: "I want messages from
  resource 0x8001 from entity 0x1010"
- `UListener::on_receive` — receives an already-decoded `UMessage` with a typed protobuf payload

**The subscriber no longer reads raw bytes.** It does not know about length prefixes, protobuf
envelope decoding, or `read_exact`. It receives a `UMessage` and calls `extract_protobuf` to get
the typed `BatteryTelemetry` struct.

**Dispatch is declarative.** The transport crate's dispatch iterates registered listeners and
calls only those whose filters match. Phase-1's subscriber accepted every byte that arrived on
the socket. Phase-2's subscriber says "only nudge me for resource 0x8001 from the battery entity."

----

### Chapter 9: Build and run

```shell
# From the repo root
cargo build --manifest-path phases/02_uprotocol_semantics/Cargo.toml

# Terminal 1 — subscriber
cargo run --manifest-path phases/02_uprotocol_semantics/Cargo.toml -p up-telemetry-subscriber

# Terminal 2 — publisher
cargo run --manifest-path phases/02_uprotocol_semantics/Cargo.toml -p up-battery-telemetry-publisher
```
Expected output (subscriber):

```
Battery telemetry subscriber listening on: .../tmp/uprotocol_twin.sock (expecting 5 messages)
[Battery telemetry subscriber] Processing incoming telemetry...
-> State of Charge: 76.3%
-> Cell Temp: 22 °C
[Battery telemetry subscriber] Processing incoming telemetry...
-> State of Charge: 77.8%
-> Cell Temp: 24 °C
... (5 messages total)
Received 5 messages — exiting.
```
----

### Chapter 10: Looking back at what we have done

Let's look back at the architecture:

```
╔═════════════════════════════════════════════════════════════╗
║ Publisher (Phase 2)                                         ║
║                                                             ║
║   BatteryTelemetry { soc_percent, temp_celsius }            ║
║      ↓                                                      ║
║   UPayload::try_from_protobuf (L2)                          ║
║      ↓                                                      ║
║   SimplePublisher::publish(resource_id, CallOptions, ...)   ║
║      ↓                                                      ║
║   UnixDomainSocketTransport::connect → send                 ║
║      ↓                                                      ║
║   Unix Domain Socket                                        ║
╚═════════════════════════════════════════════════════════════╝

╔═════════════════════════════════════════════════════════════╗
║ Subscriber (Phase 2)                                        ║
║                                                             ║
║   Unix Domain Socket                                        ║
║      ↓                                                      ║
║   UnixDomainSocketTransport::bind → dispatch                ║
║      ↓                                                      ║
║   filter match → UListener::on_receive(msg)                 ║
║      ↓                                                      ║
║   msg.extract_protobuf::<BatteryTelemetry>()                ║
║      ↓                                                      ║
║   SoC: 76.3%, Temp: 22°C                                    ║
╚═════════════════════════════════════════════════════════════╝
```
The publisher does not know about sockets. The subscriber does not know about sockets. The
transport crate owns the wire; the application code owns the business logic.

**This is the uProtocol promise:** separate _what_ we want to happen (publish this telemetry,
listen for that resource) from _how_ bytes move (Unix Domain Socket, Zenoh, MQTT, ...).

----

### Chapter 11: Where Unix Domain Sockets still fall short

The refactor buys us cleaner APIs and production-parity interfaces. But the **execution path**
is still bottlenecked by a Unix Domain Socket. Let's be honest about this.

#### Limitation 1 — Point-to-point Unix Domain Socket (no fan-out)

`UnixDomainSocketTransport::bind` accepts a connection, reads one message, and dispatches to
listeners **inside that process**. A second process (say, a thermal management logging engine)
cannot independently subscribe to the same battery telemetry stream. The socket is consumed by
our subscriber's accept loop.

Phase 1's problem — "how do we add a second consumer?" — is **unresolved**. The URI filters help
within one process, but they do not clone bytes to arbitrary peers on the network.

```
Phase-1 wall:                    Phase-2:
1 publisher → 1 subscriber       Same socket, same topology
                                 UTransport + URI filters
                                 + listener dispatch in-process
                                 (still one process, one socket)
                                 Thermal engine still cannot tap the stream
```
#### Limitation 2 — Filesystem socket path (local only)

```text
{cwd}/tmp/uprotocol_twin.sock
```
This path exists only on one machine. Move the battery uEntity to a Zone ECU and the display to
a central compute node, and Unix Domain Socket breaks — there is no cross-machine Unix socket.

#### Limitation 3 — No location transparency

Both sides must agree on a literal path string and be on the same machine. There is
no way to spin up a subscriber on a different ECU without changing the transport entirely.

#### Limitation 4 — No vehicle-wide discovery

`register_listener` is a local, in-process registration on the bind-side transport. It is not
vehicle-wide topic discovery. A new uEntity cannot find our battery telemetry event without
out-of-band configuration. (Phase 3 will touch topology; L3 services such as uDiscovery are a
later story.)

----

### Chapter 12: Looking ahead — Phase 3

The four limitations above trace to one root cause: Unix Domain Socket is a local, point-to-point transport.
uProtocol's trait-based design lets us swap it without touching publisher or subscriber
business logic.

#### What Phase 3 will bring

| Aspect | Phase-2 (keep) | Phase-3 (replace wire) |
|---|---|---|
| Envelope | `UMessage`, `UAttributes` (UUri inside) | Unchanged |
| L2 patterns | `SimplePublisher`, `CallOptions`, `UPayload` | Unchanged |
| L1 API | `UTransport::send`, `register_listener`, `UListener::on_receive` | **Same trait** — different plugin |
| Physical transport | Unix Domain Socket + length framing | Zenoh (network-transparent) |
| Multi-subscriber | Blocked by one socket accept loop | Native pub/sub fan-out on the same transport |
| Demo scope | One battery subscriber | Thermal engine + fan-out payoff |

The key insight: **Phase-3 replaces transport execution; Phase-2 semantics stay.** We are
not relearning uProtocol in Phase-3 — we are unblocking topology.

----

### Appendix: Key takeaways

1. **L1 (`UTransport` / `UListener`) separates message moving from message handling.**
   Our business logic should implement `on_receive`, not `read_exact`. One type —
   `UnixDomainSocketTransport` — with `connect` / `bind` for wire setup.

2. **L2 (`SimplePublisher` / `CallOptions` / `UPayload`) separates intent from envelope construction.**
   We say "publish" with a typed `UPayload` and options — the library fills in
   `UMESSAGE_TYPE_PUBLISH` and message ID. **TTL stays the application's choice**, passed via
   `CallOptions::for_publish(Some(5000), ...)` (same 5 s window as Phase 1's `with_ttl(5000)`).
   Source URI comes from `StaticUriProvider`; payload format from `UPayload`.

3. **Typed protobuf payloads make the data contract explicit.**
   The `.proto` file is the source of truth, not a DBC offset comment.

4. **The Unix Domain Socket wire stays the same; the semantics around it change.**
   Phase-3 swaps the wire (Zenoh) but keeps these same L1/L2 APIs. That is the uProtocol
   promise — code to the trait, not the transport.

----

### Appendix B: Protobuf schema (bms_telemetry.proto)

The `up-bms-proto` crate in Phase-2 defines the payload contract using protobuf.
This replaces the ad-hoc `pack_bms_can_frame` used in Phase-1 with a schema that
both publisher and subscriber derive code from.

```protobuf
syntax = "proto3";
package tutorial.bms.v1;

// Battery management telemetry published by the demo publisher.
// SoC and cell temperature only.
message BatteryTelemetry {
  float soc_percent = 1;
  int32 temp_celsius = 2;
}
```
The publisher creates a `BatteryTelemetry` protobuf message and wraps it with
`UPayload::try_from_protobuf`. The subscriber uses `extract_protobuf` — the `.proto` file is
the single source of truth.

----

