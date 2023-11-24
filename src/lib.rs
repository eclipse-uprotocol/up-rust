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

//! # uProtocol-SDK-Rust
//!
//! The purpose of this crate is to provide Rust specific code that builds the various data types
//! defined in the [uProtocol Specifications](https://github.com/eclipse-uprotocol/uprotocol-spec/tree/main).
//!
//! The crate contains factory methods and validators for all data types used in uProtocol.
//!
//! For the time being, usage examples can be seen in the test cases that are provided for almost every type.
//! More bespoke examples will be provided asap, once uProtocol runtime components are available.
//!
//! ## This crate includes:
//!
//! - the [`cloudevent`] module that offers a common way to represent uProtocol messages using the CloudEvent data model
//! - the [`rpc`] module which offers wrappers for dealing with uProtocol payload in the context of RPC method invokation
//! - the [`transport`] module as a set of abstractions for various transport-level concerns like status representation and serialization
//! - the [`uri`] module, providing convenience wrappers for creation and validation of uProtocol-style resource identifiers
//! - the [`uuid`] module which generates and validates UUIDs as per the uProtocol specification
//!
//! ## References
//! - [Eclipse-uProtocol Specification](https://github.com/eclipse-uprotocol/uprotocol-spec/tree/main)

mod types {
    pub mod ustatus;
    pub mod validationresult;

    pub use validationresult::*;
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

        pub use cloudeventjsonserializer::*;
        pub use cloudeventprotobufserializer::*;
        pub use cloudeventserializer::*;
    }
    pub mod validator {
        mod cloudeventvalidator;

        pub use cloudeventvalidator::*;
    }
}

pub mod uri {
    pub mod builder {
        pub mod resourcebuilder;
    }
    pub mod validator {
        mod urivalidator;

        pub use urivalidator::*;
    }
    pub mod serializer {
        mod longuriserializer;
        mod microuriserializer;
        mod uriserializer;

        pub use longuriserializer::*;
        pub use microuriserializer::*;
        pub use uriserializer::*;
    }
}

pub mod uuid {
    pub mod builder {
        mod uuidbuilder;
        mod uuidutils;

        pub use uuidbuilder::*;
        pub use uuidutils::*;
    }
    pub mod serializer {
        mod longuuidserializer;
        mod microuuidserializer;
        mod uuidserializer;

        pub use longuuidserializer::*;
        pub use microuuidserializer::*;
        pub use uuidserializer::*;
    }
    pub mod validator {
        mod uuidvalidator;

        pub use crate::types::ustatus::*;
        pub use crate::types::validationresult::*;
    }
}

pub mod transport {
    pub mod builder {
        mod uattributesbuilder;

        pub use uattributesbuilder::*;
    }
    pub mod datamodel {
        mod ulistener;
        mod utransport;

        pub use ulistener::*;
        pub use utransport::*;

        pub use crate::types::ustatus::*;
    }
    pub mod validator {
        mod uattributesvalidator;

        pub use uattributesvalidator::*;
    }
}

pub mod rpc {
    mod calloptions;
    mod rpcclient;
    mod rpcmapper;
    mod rpcresult;

    pub use calloptions::*;
    pub use rpcclient::*;
    pub use rpcmapper::*;
    pub use rpcresult::*;
}

pub mod uprotocol {
    include!(concat!(env!("OUT_DIR"), "/uprotocol.v1.rs"));

    pub use crate::proto::uprotocol::umessagetype;
    pub use crate::proto::uprotocol::upayload;
    pub use crate::proto::uprotocol::uresource;
    pub use crate::proto::uprotocol::uuid;
    pub use crate::proto::uprotocol::uuri;

    pub use u_authority::Remote;
    pub use u_payload::Data;
}

#[allow(non_snake_case)]
pub mod proto {
    include!(concat!(env!("OUT_DIR"), "/io.cloudevents.v1.rs"));
    pub mod cloudevents {
        pub mod protocloudevent;
    }

    include!(concat!(env!("OUT_DIR"), "/google.rpc.rs"));
    pub mod google {
        pub mod protostatus;
    }

    pub mod uprotocol {
        pub mod umessagetype;
        pub mod upayload;
        pub mod uresource;
        pub mod uuid;
        pub mod uuri;
    }
}
