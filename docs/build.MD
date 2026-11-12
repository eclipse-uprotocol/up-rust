# 🏗️ Build Process

## 📘 Overview

The [`build.rs`](./../build.rs) file is a special build script that is executed automatically by Cargo during the following commands:

- `cargo build`
- `cargo test`
- `cargo run`

---

## 🔁 Execution Flow

```text
cargo build / test / run
        ⬇️
Cargo checks for a build.rs file in the root directory
        ⬇️
If found, it runs the main() function inside build.rs
        ⬇️
Once build.rs finishes, Cargo proceeds with:
   - Running main() if it's a binary (`bin`) package
   - Running tests if it's a library (`lib`) package
```

## 🔧 What’s Inside `build.rs`

The `build.rs` file is a Rust build script that runs **before** your crate is compiled.

Here’s what it does at a high level:

- ✅ **Specifies `.proto` files** that define the uProtocol data models (e.g., `uuid.proto`, `uri.proto`, `umessage.proto`, etc.)

- ⚙️ **Uses `protobuf_codegen`** to generate Rust code from those `.proto` files

- 📦 **Outputs** the generated code to the crate’s output directory (`OUT_DIR`)

- 🔄 Uses a **vendored `protoc` compiler** (via `protoc-bin-vendored`) instead of relying on a system-installed version

- 🧩 Conditionally includes optional features like:
  - `udiscovery`
  - `usubscription`
  - `utwin`
  - `cloudevents` (generates separately)

This script ensures that whenever you build the crate, the Protobuf definitions are always up-to-date and compiled into Rust.

---

## 🧭 build.rs Execution Flow Diagram

```bash
cargo build / test / run
        ⬇️
Cargo looks for build.rs in the root directory
        ⬇️
If found, it executes build.rs
        ⬇️
build.rs:
   - Lists .proto files to compile
   - Sets up protoc compiler (vendored)
   - Runs protobuf_codegen
        ⬇️
Protobuf Rust code is generated into:
   📂 $OUT_DIR/uprotocol
   📂 $OUT_DIR/cloudevents (if feature enabled)
        ⬇️
Rust code uses generated types at compile time
```
