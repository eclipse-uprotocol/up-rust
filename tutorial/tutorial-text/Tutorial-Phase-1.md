
### Prologue
The story began when I was exploring the world of Eclipse-SDV ([here](https://eclipsesdv.org/)) out of curiosity. This 
was an area hitherto completely unknown to me. Yet, I was drawn towards it. Why? I have captured 
the reasons [here](https://nsengupta.github.io/blog/why-explore-software-defined-vehicle/).
One of the technologies that captured my interest was [uProtocol](https://github.com/eclipse-uprotocol). I am familiar with 
the problem it was trying to solve (I have worked in the area of location-agnostic, 
multi-machine-architecture-friendly, network-carried, multiplex-able middleware for a 
good part of my career), but the domain was different. My aim was to understand the landscape 
well, and Eclipse SDV sites helped; so did uProtocol REPO, blogs (Pete Le Vasseur, 
chief maintainer of [uProtocol](https://petelevasseur.com/articles/index.html)) and Youtube videos but what I didn't find was a classical 
tutorial; a tutorial which helped a software developer to lay her/his hands on the code to 
solidify the understanding along with the specifications and examples, and helped create a mental 
map of _what was what_. 

So, I decided to write one myself. This tutrial follows how I approached learning uProtocol; 
hopefully, this will be useful for us too.

----

### Chapter 1

### What shall we build

We will build a small application using uProtocol's up-rust ([here](https://github.com/eclipse-uprotocol/up-rust)) implementation. Along the 
way, we will clarify the _what_-s and the _why_-s. I believe this is good way to let things fall 
in the appropriate _conceptual_ place. Every crate under `phases/01_raw_sockets/` pins 
`up-rust = "0.9.0"`; API links in this chapter point at that version on docs.rs.

The application is simple. There exists:

-   An application that can send out ( _publish_ ) two pieces of information from a car's battery 
    telemetry: 
    (a) State-of-charge and (b) Temperature; in the code, this is `up-battery-telemetry-publisher`
-   An application that can read these values and print; in the code, this is `up-telemetry-subscriber`

Each application runs in its own address-space (two different processes, run from separate 
shells on my Linux machine). 

This tutorial is **Phase-1**. The code lives in `phases/01_raw_sockets/`:

```
phases/01_raw_sockets/
├── Cargo.toml
└── crates/
    ├── up-frame-codec/                  # length-prefix serialize + deserialize
    ├── up-battery-telemetry-publisher   # sends UMessage over Unix Domain Socket
    └── up-telemetry-subscriber          # receives UMessage over Unix Domain Socket
```
A later phase (Phase-2, in `phases/02_uprotocol_semantics/`) will refactor this same
functionality using uProtocol's own L1 transport and L2 communication helpers.

For any such application where two (or more, but that's later) processes converse between 
themselves, two aspects are important (obvious, one may quip):

1. What is shared
2. How is it shared

The 'what' part needs a little more understanding. One application doesn't 'know' the other 
application. Therefore, both the sides have to have complete knowledge of 'what' - the _structure_ 
and the mechanism to _interpret_ the structure. One side cannot change the 'what' without the 
other side being aware of it. They are bound by this pre-condition. 

The 'how' is the aspect of transportation: either side must be equipped to hook itself with and 
collect from the same transportation facility or 'channels'. This is the other binding factor. 
It is important to note here, that each such 'channel' comes with its own facilities and 
limitations. Again, both the sides have to agree to adjust to and/or abide by these. We will 
revisit this later in this tutorial.

Let's focus on the *what* part first. 2 pieces of data are sent and received:
1) Percent of charge remaining in battery (a `float` value) 
2) Temperature of battery (again, a `float` value)

We assume that these two are packed in a CAN-frame (we don't care where do these originate; not 
important for the tutorial).

```rust
fn pack_bms_can_frame(battery_level_pct: f32, temperature_c: i8) -> [u8; 8] {
    let mut can_data = [0u8; 8];
    // Convert using a DBC scale of 0.5 (e.g. 75.0 / 0.5 = 150)
    let raw_soc = (battery_level_pct / 0.5) as u8;
    can_data[0] = raw_soc;
    can_data[1] = temperature_c as u8;
    can_data
}
```

This array is the application _payload_ (raw bytes) that is transported and collected at the other 
end. We want to put those bytes in a uProtocol envelope: a [UMessage](https://docs.rs/up-rust/0.9.0/up_rust/struct.UMessage.html).

Before we build the message, the envelope needs a **source** identity — who/what this PUBLISH is 
from. That identity is a [UUri](https://docs.rs/up-rust/0.9.0/up_rust/struct.UUri.html).

A UUri address is always:

**authority + numeric entity id + version + numeric resource id.**

Our **battery telemetry topic** — State-of-Charge and temperature from the battery publisher — is:

```text
up://my_own_car/1010/1/8001

authority   my_own_car
entity      1010   (0x1010)   ← our battery publisher entity
version     1
resource    8001   (0x8001)   ← SoC / temperature telemetry resource
```

The labels on the right are for us as readers; they are **not** URI path segments. In code we build 
the same address with `UUri::try_from_parts` (Chapter 2 also shows parsing from a string with 
`UUri::from_str`):

```rust
let source_uri = UUri::try_from_parts("my_own_car", 0x1010, 1, 0x8001)?;
```

We will revisit `UUri` in detail in Chapter 2.

Application code should not assemble `UAttributes` / `UMessage` structs by hand. This tutorial 
follows current recommended `up-rust` practice: build messages with 
[UMessageBuilder](https://docs.rs/up-rust/0.9.0/up_rust/struct.UMessageBuilder.html), which sets 
the fields required for a **PUBLISH** correctly (and keeps the same pattern for other message 
types when we meet them later).

> **Side note:** In an upcoming `up-rust` release, the constructors for `UAttributes` and 
> `UMessage` will no longer be public — another reason to learn the builder now rather than 
> struct literals.

```rust
let message = UMessageBuilder::publish(source_uri)
    .with_ttl(5000)
    .build_with_payload(
        pack_bms_can_frame(battery_pct, temp_c).to_vec(),
        UPayloadFormat::UPAYLOAD_FORMAT_RAW,
    )?;
```

- `publish(source_uri)` marks the message as **PUBLISH** and sets **source** to our battery 
  telemetry topic URI (`up://my_own_car/1010/1/8001`).
- `with_ttl(5000)` sets a 5-second usefulness window.
- `build_with_payload(bytes, format)` attaches the raw CAN bytes and a 
  [UPayloadFormat](https://docs.rs/up-rust/0.9.0/up_rust/enum.UPayloadFormat.html) hint (`RAW` here).

L1 is concerned with **`UMessage` instances**. The L2 helper type `UPayload` is useful with 
communication APIs such as `SimplePublisher`; we defer it to Phase 2. Here we only need bytes + 
format on the builder.

Now that the 'what' part is done (more or less; Chapter 2 goes deeper), the next part to deal with 
is 'how'.

These two applications are running on a single Linux host. One of the easiest ways to connect 
them is to use a **Unix Domain Socket** (not to be confused with automotive Unified Diagnostic
Services). It is quite easy to send a byte-stream through a Unix Domain Socket. However, 
the socket behaves as if it is forwarding a stream; the start and end of a message is not 
interpreted. So, we have to arrange for marking these two. One standard way to achieve this is to 
prefix the buffer with its length. The length is an integer (4 bytes); so the receiving 
application can read the length (first 4 bytes), and then read _that many_ bytes from the buffer 
that arrived. 

In the publisher side
```rust
pub fn serialize_for_unix_socket(msg: &UMessage) -> Result<Vec<u8>, anyhow::Error> {
    let envelope_bytes = msg.write_to_bytes()?;
    let msg_len = envelope_bytes.len() as u32;
    let mut framed_buffer = msg_len.to_be_bytes().to_vec();
    framed_buffer.append(&mut envelope_bytes.to_vec()); // Length is prefixed

    Ok(framed_buffer)
}
```
Conversely, in the subscriber side:
```rust
pub fn deserialize_for_unix_socket(framed: &[u8]) -> Result<UMessage, anyhow::Error> {
    if framed.len() < 4 {
        anyhow::bail!("framed buffer too short for length prefix");
    }

    let body_len = u32::from_be_bytes(framed[0..4].try_into()?) as usize;
    let end = 4 + body_len;
    if framed.len() < end {
        anyhow::bail!("framed buffer too short for declared body length");
    }

    Ok(UMessage::parse_from_bytes(&framed[4..end])?)
}
```
[Jump to 'How to run' for a sample output](#how-to-run)

So, there we are. A uProtocol `UMessage` has been shared between two applications using an 
OS-provided transport facility. 

One important inference that we can draw is that uProtocol's types and operations are **completely 
independent** of the transport facility that is used to  move the messages. We will revisit this 
aspect in the next sections.

----

### Chapter 2

Let's take a good look at types that uProtocol gives us.

This is what we have seen:

- We identified the publish topic with a `UUri`.
- We built a PUBLISH `UMessage` with `UMessageBuilder` (raw bytes + `UPayloadFormat`).
- Message **attributes** describe purpose, source, lifetime, and identity; the builder fills them in.
- Payload **bytes** ride inside the `UMessage`.

On the uProtocol layer map (the same stack Phases 2–3 will reuse), Phase 1 works at the bottom
two bands only:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  Application / uEntity                                                      │
│  publisher · subscriber                                                     │
├─────────────────────────────────────────────────────────────────────────────┤
│  L2 — Communication          (Phase 2: SimplePublisher, UPayload, …)        │
├─────────────────────────────────────────────────────────────────────────────┤
│  L1 — Transport              (Phase 2: UTransport, UListener, …)            │
├─────────────────────────────────────────────────────────────────────────────┤
│  Envelope (every message)    ← Phase 1                                      │
│  UMessage · UAttributes (incl. source/sink UUri, payload format, …)         │
├─────────────────────────────────────────────────────────────────────────────┤
│  Wire                        ← Phase 1                                      │
│  Unix Domain Socket + length-prefix framing (`up-frame-codec`)              │
└─────────────────────────────────────────────────────────────────────────────┘
```

```
╔═══════════╗
║   UUri    ║
╚═══════════╝
```
uProtocol needs a way to name a software entity / resource anywhere in the vehicle. In this 
tutorial, that includes our **battery telemetry topic** (`up://my_own_car/1010/1/8001`) — 
State-of-Charge and temperature from the battery publisher.

A valid address for that topic is:

```text
up://my_own_car/1010/1/8001
```

After the authority, the path segments are **hex encodings of unsigned integers** (entity id, 
version, resource id) — not free-form names. See the current 
[UUri specification](https://github.com/eclipse-uprotocol/up-spec/blob/main/basics/uri.adoc).

```text
┌──────────┬──────────────────┬────────────────┬────────────────────────────────┐
│ Field    │ Code property    │ Value in URI   │ Our label (commentary only)    │
├──────────┼──────────────────┼────────────────┼────────────────────────────────┤
│ Scheme   │ up               │ up             │ uProtocol                      │
│ Authority│ authority_name   │ my_own_car     │ deployment namespace           │
│ uEntity  │ ue_id            │ 1010 (0x1010)  │ battery telemetry publisher    │
│ Version  │ ue_version_major │ 1              │ API major version              │
│ uResource│ resource_id      │ 8001 (0x8001)  │ SoC / temperature telemetry    │
└──────────┴──────────────────┴────────────────┴────────────────────────────────┘
```

The last column is for us, humans. It is **not** part of the URI.

Note: A _[uEntity](https://github.com/eclipse-uprotocol/up-spec/blob/main/basics/README.adoc#uentity)_ 
(uProtocol software entity) is a piece of software deployed somewhere on a network host (in our 
case, the car itself). uEntities are uniquely identified within a system by means of the type and 
version of the service interface that they implement — expressed as numeric ids, not dotted names 
like `powertrain.battery`.

Names such as “battery SoC” are useful in docs and conversation. The **address** on the wire is 
numeric so it stays compact, stable, and machine-checkable; not to be put in the URI path.

In application code we use the `UUri` type, which carries the same four fields. Create instances 
with the supported constructors — not struct literals:

```rust
// When we already know the numeric parts:
let source_uri = UUri::try_from_parts("my_own_car", 0x1010, 1, 0x8001)?;

// When we have a URI string:
let source_uri = UUri::from_str("up://my_own_car/1010/1/8001")?;
```

**Are those IDs arbitrary?** No. They are _car-level constants_ assigned for this system (and, in 
this tutorial, chosen demo constants — see the registry below).

Let's start with _Authority_. It is the root of a tree holding specific resources, laying out a 
_namespace_; just like 'github.com' defines the root of all resources held on that server. In the 
seemingly endless Internet, if I want to reach a particular source code of mine, I can type 
'www.github.com/nsengupta/personal_project/...' in the browser. The DNS finds where that 
'github.com' physically is and hands the HTTP request to it. In the same vein, in an SDV-run car, 
the _Authority_ can hold values like `my_own_car`, `central_compute`, or `front_left_zone`. In the 
entire car there can be only one `my_own_car` or `front_left_zone` — just as on the Internet there 
can be only one 'github.com'.

**Authority is a name; entity and resource under it are numeric IDs.**

Inside the authority there can be more than one uEntity. For example, a battery publisher, a 
head-lamp service, and a tyre service each get their own `ue_id`. A given uEntity may expose one 
or more **resources** (topics or methods) — each with its own `resource_id`.

Remember: **uEntity** = the software component; **resource** = one part of that entity's interface. 
For tyre pressure, *tyre* is the entity and *pressure* is a resource of that entity.

Here is a second topic — head-lamp is-on — **same four-part shape**, distinct IDs:

```text
up://my_own_car/1020/1/8002

authority   my_own_car
entity      1020   (0x1020)   ← head-lamp entity
version     1
resource    8002   (0x8002)   ← is-on
```

```rust
let head_lamp_uri = UUri::try_from_parts("my_own_car", 0x1020, 1, 0x8002)?;
```

And a third card so the pattern sticks — tyre / pressure:

```text
up://my_own_car/101F/1/A010

authority   my_own_car
entity      101F   (0x101F)   ← tyre entity
version     1
resource    A010   (0xA010)   ← pressure resource
```

```rust
let tyre_pressure_uri = UUri::try_from_parts("my_own_car", 0x101F, 1, 0xA010)?;
```

Different topic, **same four-part shape**.

| Topic | Authority | Entity | Resource | Labels (commentary only) |
|-------|-----------|--------|----------|---------------------------|
| Battery telemetry (this phase's runnable source) | `my_own_car` | `0x1010` | `0x8001` | battery publisher / SoC |
| Head-lamp is-on | `my_own_car` | `0x1020` | `0x8002` | head-lamp / is-on |
| Tyre pressure | `my_own_car` | `0x101F` | `0xA010` | tyre / pressure |

```text
+--
|
| Rules exist to assign uEntity IDs. Read more about assignment of IDs to uEntities
| [here](https://github.com/eclipse-uprotocol/up-spec/blob/main/up-l3/README.adoc#31-uentity-id-ranges).
|
+--
```

With a `source_uri` in hand, Chapter 1 built a PUBLISH `UMessage` via `UMessageBuilder`. We now 
look at that envelope more carefully.

```
╔══════════════╗
║   UMessage   ║
╚══════════════╝
```

`UMessage` is the L1 unit we send and receive. Application code creates a PUBLISH message like 
this (same path as the publisher crate):

```rust
let message = UMessageBuilder::publish(source_uri)
    .with_ttl(5000)
    .build_with_payload(
        pack_bms_can_frame(battery_pct, temp_c).to_vec(),
        UPayloadFormat::UPAYLOAD_FORMAT_RAW,
    )?;
```

```
╔══════════════════╗
║   UAttributes    ║
╚══════════════════╝
```

_UAttributes_ play a very important role in _uProtocol_'s design. They

- tell the world what the purpose of the message is,
- hint at how to interpret the contents,
- augment upstream routing logic of the transporters.

We do **not** fill an `UAttributes { ... }` struct ourselves. The builder populates attributes for 
a PUBLISH. Conceptually we care about:

- **type** — `UMESSAGE_TYPE_PUBLISH`: interested parties may consume; this is not an RPC request.
- **source** — our battery telemetry topic (`up://my_own_car/1010/1/8001`).
- **ttl** — usefulness window (we set 5000 ms).
- **id** — unique message id (assigned by the builder).

The complete data-model for _UAttributes_ is [here](https://docs.rs/up-rust/0.9.0/up_rust/struct.UAttributes.html). 
For other values that message `type` can have, see [here](https://docs.rs/up-rust/0.9.0/up_rust/enum.UMessageType.html).

After the builder returns, we can **inspect** those attributes on the `UMessage`:

```rust
assert_eq!(message.type_unchecked(), UMessageType::UMESSAGE_TYPE_PUBLISH);
assert_eq!(message.source_unchecked(), &source_uri);
assert_eq!(message.ttl_unchecked(), 5000);
```

The `source` helps transporters and consumers reason about who is interested in this topic — some 
wait for tyre pressure (`0x101F` / `0xA010`), others for head-lamp is-on (`0x1020` / `0x8002`). The 
`ttl` helps decide if a late message is still useful.

```
╔══════════════════╗
║  UPayloadFormat  ║
╚══════════════════╝
```

Receivers need a hint for how to interpret the payload bytes. Phase 1 uses 
`UPayloadFormat::UPAYLOAD_FORMAT_RAW` because we still pack a hand-rolled CAN layout. The L2 type 
`UPayload` (bytes + format as a communication helper) arrives in Phase 2 together with protobuf 
and `SimplePublisher`.

Once we have the `UMessage`, we flatten it to a stream of bytes so it can be transported. 
`UMessage.write_to_bytes()` gives us the series of octets that is transportation-ready; our 
length-prefix framing then wraps that buffer for the Unix Domain Socket.

----

### How to run

Both processes use a Unix Domain Socket at **`{current working directory}/tmp/uprotocol_twin.sock`**
(defined in `up_frame_codec::socket_path`). Start them from the **same directory** (the repo root
below) so they share that path. The subscriber creates `tmp/` if needed.

```shell
# From the repo root
cargo build --manifest-path phases/01_raw_sockets/Cargo.toml

# Terminal 1 — subscriber
cargo run --manifest-path phases/01_raw_sockets/Cargo.toml -p up-telemetry-subscriber

# Terminal 2 — publisher
cargo run --manifest-path phases/01_raw_sockets/Cargo.toml -p up-battery-telemetry-publisher
```
Expected output:

```shell
--- Battery telemetry publisher starting ---
Message 1: SoC = 76.6%, Temp = 24°C
   Sent 67 bytes.

Message 2: SoC = 77.9%, Temp = 23°C
   Sent 67 bytes.

Message 3: SoC = 76.6%, Temp = 21°C
   Sent 67 bytes.

Message 4: SoC = 75.3%, Temp = 20°C
   Sent 67 bytes.

Message 5: SoC = 78.3%, Temp = 22°C
   Sent 67 bytes.

```
```shell
Battery telemetry subscriber listening on: …/tmp/uprotocol_twin.sock
[Battery telemetry subscriber] Processing incoming CAN telemetry...
-> State of Charge: 76.5%
-> Cell Temp: 24 °C
[Battery telemetry subscriber] Processing incoming CAN telemetry...
-> State of Charge: 77.5%
-> Cell Temp: 23 °C
[Battery telemetry subscriber] Processing incoming CAN telemetry...
-> State of Charge: 76.5%
-> Cell Temp: 21 °C
[Battery telemetry subscriber] Processing incoming CAN telemetry...
-> State of Charge: 75.0%
-> Cell Temp: 20 °C
[Battery telemetry subscriber] Processing incoming CAN telemetry...
-> State of Charge: 78.0%
-> Cell Temp: 22 °C

```
