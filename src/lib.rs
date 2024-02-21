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
//! - the [`cloudevent`] module that offers a common way to represent uProtocol messages using the `CloudEvent` data model
//! - the [`rpc`] module which offers wrappers for dealing with uProtocol payload in the context of RPC method invokation
//! - the [`transport`] module as a set of abstractions for various transport-level concerns like status representation and serialization
//! - the [`uri`] module, providing convenience wrappers for creation and validation of uProtocol-style resource identifiers
//! - the [`uuid`] module which generates and validates UUIDs as per the uProtocol specification
//!
//! ## References
//! - [Eclipse-uProtocol Specification](https://github.com/eclipse-uprotocol/up-spec)
//! - [Eclipse-uProtocol Core API types](https://github.com/eclipse-uprotocol/up-core-api)

mod types {
    pub mod parsingerror;
    pub mod serializationerror;
    pub mod validationerror;
}

pub mod cloudevent {
    pub mod builder {
        mod ucloudeventbuilder;
        mod ucloudeventutils;

        pub use ucloudeventbuilder::*;
        pub use ucloudeventutils::*;
    }
    pub mod datamodel {
        mod ucloudeventattributes;

        pub use ucloudeventattributes::*;
    }
    pub mod serializer {
        mod cloudeventjsonserializer;
        mod cloudeventprotobufserializer;
        mod cloudeventserializer;

        pub use crate::types::serializationerror::*;
        pub use cloudeventjsonserializer::*;
        pub use cloudeventprotobufserializer::*;
        pub use cloudeventserializer::*;
    }
    pub mod validator {
        mod cloudeventvalidator;

        pub use crate::types::validationerror::*;
        pub use cloudeventvalidator::*;
    }
}

// protoc-generated stubs, see build.rs
include!(concat!(env!("OUT_DIR"), "/cloudevents/mod.rs"));

pub mod rpc {
    mod calloptions;
    mod rpcclient;
    mod rpcmapper;
    mod rpcresult;
    mod rpcserver;

    pub use calloptions::*;
    pub use rpcclient::*;
    pub use rpcmapper::*;
    pub use rpcresult::*;
    pub use rpcserver::*;
}

pub mod transport {
    pub mod builder {
        mod uattributesbuilder;

        pub use uattributesbuilder::*;
    }
    pub mod datamodel {
        mod utransport;

        pub use utransport::*;
    }
    pub mod validator {
        mod uattributesvalidator;

        pub use crate::types::validationerror::*;
        pub use uattributesvalidator::*;
    }
}

pub mod uri {
    pub mod builder {
        pub mod resourcebuilder;
    }
    pub mod validator {
        mod urivalidator;

        pub use crate::types::validationerror::*;
        pub use urivalidator::*;
    }
    pub mod serializer {
        mod microuriserializer;
        mod uriserializer;

        pub use crate::types::serializationerror::*;
        pub use microuriserializer::*;
        pub use uriserializer::*;
    }
}

pub mod uuid {
    pub mod builder {
        mod uuidbuilder;

        pub use uuidbuilder::*;
    }
}

pub mod uprotocol {
    // protoc-generated stubs, see build.rs
    include!(concat!(env!("OUT_DIR"), "/uprotocol/mod.rs"));

    pub use self::uuid::UUID;
    pub use crate::proto::uprotocol::upayload::*;
    pub use crate::proto::uprotocol::uuid::*;
    pub use uattributes::{UAttributes, UMessageType, UPriority};
    pub use umessage::UMessage;
    pub use upayload::{upayload::Data, UPayload, UPayloadFormat};
    pub use uri::{UAuthority, UEntity, UResource, UUri};
    pub use ustatus::{UCode, UStatus};
}

#[allow(non_snake_case)]
pub(crate) mod proto {

    pub(crate) mod cloudevents {
        pub(crate) mod protocloudevent;
    }

    pub(crate) mod uprotocol {
        pub(crate) mod uauthority;
        pub(crate) mod uentity;
        pub(crate) mod umessagetype;
        pub(crate) mod upayload;
        pub(crate) mod upriority;
        pub(crate) mod uresource;
        pub(crate) mod ustatus;
        pub(crate) mod uuid;
        pub(crate) mod uuri;
    }
}
