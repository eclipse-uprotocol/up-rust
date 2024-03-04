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

//! # up-rust  - uProtocol Rust library
//!
//! The purpose of this crate is to provide Rust specific code to work with the various
//! [uProtocol Core API types](https://github.com/eclipse-uprotocol/up-core-api). The crate contains trait definitions and
//! convenience functionality for building, converting, validating and serializing uProtocol data types.
//!
//! ## This crate includes:
//!
//! - the [cloudevents] module that offers a common way to represent uProtocol messages using the [cloudevents data model](https://cloudevents.io/)
//! - the rpc module which offers wrappers for dealing with uProtocol payload in the context of RPC method invocation
//! - the uattributes module with uProtocol message attribute types and validators
//! - the umessage module which defines the uProtocol core message type and provides related convenience functionality
//! - the upayload module which defines payload representation for uProtocol messages
//! - the uri module providing convenience wrappers for creation and validation of uProtocol-style resource identifiers
//! - the ustatus module which provices uProtocol types for representing status and status codes
//! - the utransport module as an interface contract between uProtocol and specific transport protocol implementations
//! - the uuid module which generates and validates UUIDs as per the uProtocol specification
//!
//! For user convenience, all of these modules export their types on up_rust top-level, except for (future) optional features.
//!
//! ## References
//! - [Eclipse-uProtocol Specification](https://github.com/eclipse-uprotocol/up-spec)
//! - [Eclipse-uProtocol Core API types](https://github.com/eclipse-uprotocol/up-core-api)

// up_core_api types used and augmented by up_rust - symbols re-exported to toplevel, errors are module-specific
mod rpc;
pub use rpc::{CallOptions, CallOptionsBuilder};
pub use rpc::{RpcClient, RpcClientResult};
pub use rpc::{RpcMapper, RpcMapperError};
pub use rpc::{RpcPayload, RpcPayloadResult, RpcResult};
pub use rpc::{RpcServer, RpcServerListener};

mod uattributes;
pub use uattributes::{
    PublishValidator, RequestValidator, ResponseValidator, UAttributesValidator,
    UAttributesValidators,
};
pub use uattributes::{UAttributes, UAttributesError, UMessageType, UPriority};

mod umessage;
pub use umessage::{UMessage, UMessageBuilder, UMessageBuilderError};

mod upayload;
pub use upayload::{Data, UPayload, UPayloadError, UPayloadFormat};

mod uri;
pub use uri::{AddressType, Number, UAuthority, UEntity, UResource, UResourceBuilder};
pub use uri::{UUri, UUriError, UriValidator};

mod ustatus;
pub use ustatus::{UCode, UStatus};

mod utransport;
pub use utransport::UTransport;

mod uuid;
pub use uuid::{UUIDBuilder, UUID};

// Types not used by up_rust, but re-exported to up_rust users, keeping them in their respective submodules
pub use up_core_api::udiscovery;
pub use up_core_api::usubscription;
pub use up_core_api::utwin;

// Types from up_core_api that we're not re-exporting for now (might change if need arises)
// pub use up_core_api::file;
// pub use up_core_api::uprotocol_options;

// protoc-generated stubs, see build.rs
mod up_core_api {
    include!(concat!(env!("OUT_DIR"), "/uprotocol/mod.rs"));
}

// cloudevent-proto, generated and augmented types
pub mod cloudevents;
mod proto_cloudevents {
    include!(concat!(env!("OUT_DIR"), "/cloudevents/mod.rs"));
    pub(crate) use self::cloudevents::cloud_event::*; // re-export for crate use, remove triple-redundant names
}
