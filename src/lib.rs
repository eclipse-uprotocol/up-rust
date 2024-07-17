/********************************************************************************
 * Copyright (c) 2023 Contributors to the Eclipse Foundation
 *
 * See the NOTICE file(s) distributed with this work for additional
 * information regarding copyright ownership.
 *
 * This program and the accompanying materials are made available under the
 * terms of the Apache License Version 2.0 which is available at
 * https://www.apache.org/licenses/LICENSE-2.0
 *
 * SPDX-License-Identifier: Apache-2.0
 ********************************************************************************/

/*!
up-rust is the [Eclipse uProtocol&trade; Language Library](https://github.com/eclipse-uprotocol/up-spec/blob/v1.6.0-alpha.2/languages.adoc) for the
Rust programming language.

This crate can be used to

* implement uEntities that communicate with each other using the uProtocol [Communication Layer API](https://github.com/eclipse-uprotocol/up-spec/blob/v1.6.0-alpha.2/up-l2/api.adoc)
  over one of the supported transport protocols.
* implement support for an additional transport protocol by means of implementing the [Transport Layer API](https://github.com/eclipse-uprotocol/up-spec/blob/v1.6.0-alpha.2/up-l1/README.adoc).

## Library contents

* `communication` module, which defines uProtocol's Communication Layer API for publishing and subscribing to topics and invoking RPC methods.
  It also contains a default implementation employing the Transport Layer API.
* `uattributes` module, with uProtocol message attribute types and validators
* `umessage` module, which defines the uProtocol core message type and provides related convenience functionality
* `upayload` module, which defines payload representation for uProtocol messages
* `uri` module, providing convenience wrappers for creation and validation of uProtocol-style resource identifiers
* `ustatus` module, which provices uProtocol types for representing status and status codes
* `utransport` module, as an interface contract between uProtocol and specific transport protocol implementations
* `uuid` module, which generates and validates UUIDs as per the uProtocol specification

For user convenience, all of these modules export their types on up_rust top-level, except for (future) optional features.

## Features

* `communication` enables support for the [Communication Layer API](https://github.com/eclipse-uprotocol/up-spec/blob/v1.6.0-alpha.2/up-l2/api.adoc) and its
  default implementation on top of the [Transport Layer API](https://github.com/eclipse-uprotocol/up-spec/blob/v1.6.0-alpha.2/up-l1/README.adoc).
  Enabled by default.
* `udiscovery` enables support for types required to interact with [uDiscovery service](https://raw.githubusercontent.com/eclipse-uprotocol/up-spec/v1.6.0-alpha.2/up-l3/udiscovery/v3/README.adoc)
  implementations.
* `usubscription` enables support for types required to interact with [uSubscription service](https://raw.githubusercontent.com/eclipse-uprotocol/up-spec/v1.6.0-alpha.2/up-l3/usubscription/v3/README.adoc)
  implementations. Enabled by default.
* `utwin` enables support for types required to interact with [uTwin service](https://raw.githubusercontent.com/eclipse-uprotocol/up-spec/v1.6.0-alpha.2/up-l3/utwin/v3/README.adoc)
  implementations.

## References

* [uProtocol Specification](https://github.com/eclipse-uprotocol/up-spec/tree/v1.6.0-alpha.2)

*/

// up_core_api types used and augmented by up_rust - symbols re-exported to toplevel, errors are module-specific
#[cfg(feature = "communication")]
pub mod communication;
mod uattributes;
pub use uattributes::{
    NotificationValidator, PublishValidator, RequestValidator, ResponseValidator,
    UAttributesValidator, UAttributesValidators,
};
pub use uattributes::{UAttributes, UAttributesError, UMessageType, UPayloadFormat, UPriority};

mod umessage;
pub use umessage::{UMessage, UMessageBuilder, UMessageError};

mod uri;
pub use uri::{UUri, UUriError};

mod ustatus;
pub use ustatus::{UCode, UStatus};

mod utransport;
pub use utransport::{ComparableListener, LocalUriProvider, UListener, UTransport};
mod uuid;
pub use uuid::UUID;

// protoc-generated stubs, see build.rs
mod up_core_api {
    include!(concat!(env!("OUT_DIR"), "/uprotocol/mod.rs"));
}

// Types from up_core_api that we're not re-exporting for now (might change if need arises)
// pub use up_core_api::file;
// pub use up_core_api::uprotocol_options;
pub mod core;
