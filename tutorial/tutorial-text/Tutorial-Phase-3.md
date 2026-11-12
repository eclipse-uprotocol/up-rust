### Phase 3: The same L1/L2 APIs, `LocalTransport`

> **Prerequisites:** Phase 2 (`tutorial-text/Tutorial-Phase-2.md`, code in `phases/02_uprotocol_semantics/`).
> **Active code:** `phases/03_local_transport/` — one process, two listeners.

Phase 2 ended with an honest confession: Unix Domain Socket works for one publisher process and one subscriber process on a single Linux host, but it breaks the moment we need a second *process* to consume the same stream. This chapter does **not** introduce a networked transport. It swaps the L1 plugin for `LocalTransport` — the `UTransport` that ships in this `up-rust` crate — and shows that the Phase 2 publisher and listener bodies still compile against that plugin.

**Demo scope:** the demo runs in **one process**. Battery telemetry and thermal logging are two `UListener` implementations registered on the **same** `LocalTransport` instance. That is a limit of this plugin, not of uProtocol.

Let's dive in.

---

### Chapter 1: Why Unix Domain Sockets had to retire (recap)

Phase 2's `up-unix-domain-socket-transport` solved the right problem: separating business logic from socket I/O using `UTransport`. The Unix Domain Socket **wire** still fails when we need:

1. **Fan-out to a second process** — two subscriber processes cannot both receive the same stream (point-to-point; one bind owner).
2. **A logical address** — `{cwd}/tmp/uprotocol_twin.sock` is a filename, not a URI; only one process can bind it.
3. **Location transparency** — publisher and subscriber must share a kernel.

`LocalTransport` does not fix (1)–(3) across processes or machines. It *does* show in-process fan-out: several listeners, one URI filter, no socket path. Independent processes and hosts need a networked `UTransport` (for example [up-transport-zenoh-rust](https://github.com/eclipse-uprotocol/up-transport-zenoh-rust)). The three-process Zenoh walk that used to live here is kept at [nsengupta/tutorial_uprotocol](https://github.com/nsengupta/tutorial_uprotocol).

```
Phase 2 — Unix Domain Socket (one process owns the socket)

  Publisher process ───► UnixDomainSocketTransport::bind ───► one subscriber process

Phase 3 — LocalTransport (no wire)

  SimplePublisher ──send──► LocalTransport ──► BatteryTelemetryListener
                                         └──► ThermalLoggingListener
  (same process, same Arc<LocalTransport>)
```

---

### Chapter 2: The layer map, plugin swapped

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  Application / uEntity                                                      │
│  publisher loop · battery listener · thermal listener                       │
├─────────────────────────────────────────────────────────────────────────────┤
│  L2 — Communication                                                         │
│  SimplePublisher · CallOptions · UPayload          (unchanged from Phase 2) │
├─────────────────────────────────────────────────────────────────────────────┤
│  L1 — Transport                                                             │
│  UTransport · UListener · LocalTransport           (plugin swap)            │
│  send · register_listener                                                   │
├─────────────────────────────────────────────────────────────────────────────┤
│  Envelope                                                                   │
│  UMessage · UAttributes (source/sink UUri, payload format, …)               │
├─────────────────────────────────────────────────────────────────────────────┤
│  Wire                                                                       │
│  none — in-process dispatch on the sending thread                           │
└─────────────────────────────────────────────────────────────────────────────┘
```

Layer specs: [uP-L1](https://github.com/eclipse-uprotocol/up-spec/tree/main/up-l1),
[uP-L2](https://github.com/eclipse-uprotocol/up-spec/tree/main/up-l2).

The workspace (`phases/03_local_transport/`) copy-forwards `up-bms-proto`. It retires `up-unix-domain-socket-transport` and `up-frame-codec`. It depends on this checkout of `up-rust` with `communication`, `util`, and `protobuf-support`.

---

### Chapter 3: One process, two listeners

`LocalTransport` is documented in this crate as a transport for uEntities in the **same process**. `UTransport::send` dispatches to every registered listener whose source/sink filters match. Dispatch runs on the thread that called `send` — the same behaviour as [`examples/simple_publish.rs`](../../examples/simple_publish.rs). There is no “start the subscriber first” race: both listeners are registered in this process before `SimplePublisher` sends. (Phase 2's start-order advice is an operational convention around Unix Domain Socket `bind` vs `register_listener`, not an L1 invariant — see the Point of interest in [`Tutorial-Phase-2.md`](./Tutorial-Phase-2.md) Chapter 6.)

Opening a second terminal and running another binary cannot attach to this demo. There is no socket path and no network for a second process to join. A second `LocalTransport::default()` is a **different** instance and will not see the first process’s messages.

The thermal listener extracts the same `BatteryTelemetry` protobuf and applies thermal-specific logic (warn above 25°C). No new schema is introduced for thermal data: cell temperature is already a field of that message, and both listeners use the same resource URI.

This chapter still does **not** use L3 services such as uSubscription or uDiscovery.

---

### Chapter 4: Run the demo

From the `tutorial/` directory:

```bash
cd tutorial   # if you are not already here
cargo run --manifest-path phases/03_local_transport/Cargo.toml -p up-bms-local-demo
```

You should see five published messages. Both listeners print for each message. At least some thermal lines may show a warning (the publisher draws temperature from 20–28°C).

---

### Chapter 5: What this chapter does not show

- **Independent processes** — this chapter does not show them. Use a networked `UTransport`. See [nsengupta/tutorial_uprotocol](https://github.com/nsengupta/tutorial_uprotocol) and [up-transport-zenoh-rust](https://github.com/eclipse-uprotocol/up-transport-zenoh-rust).
- **Location transparency** — this chapter does not show it. `LocalTransport` never leaves the process.
- **L3 services** — uSubscription and uDiscovery are not implemented here.

Same-process dispatch is how *this* plugin is implemented. uProtocol’s L1/L2 types are not limited to one process.

---

### Appendix: Key takeaways

1. **Code to `UTransport` / `UListener` / `SimplePublisher`.** Phase 2 used a Unix Domain Socket plugin; Phase 3 uses `LocalTransport`. The listener and publish bodies stay the same kind of code.
2. **In-process fan-out is URI-filtered dispatch.** Two listeners registered on one resource URI both fire for each publish.
3. **`LocalTransport` is not a vehicle network.** For multiple processes or hosts, change the L1 plugin — do not rewrite the application types.
4. **This crate tracks the library in this checkout.** Tutorial crates path-depend on `up-rust`; CI builds them when the library API changes.

---

### Appendix B: Protobuf schema (bms_telemetry.proto)

```protobuf
syntax = "proto3";
package tutorial.bms.v1;

message BatteryTelemetry {
  float soc_percent = 1;
  int32 temp_celsius = 2;
}
```
