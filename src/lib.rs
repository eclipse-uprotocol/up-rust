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
}

pub mod cloudevent {
    pub mod builder {
        mod ucloudeventbuilder;

        pub use ucloudeventbuilder::*;
    }
    pub mod datamodel {
        mod ucloudevent;
        mod ucloudeventattributes;
        mod ucloudeventtype;

        pub use ucloudevent::*;
        pub use ucloudeventattributes::*;
        pub use ucloudeventtype::*;
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

        pub use crate::types::validationresult::*;
        pub use cloudeventvalidator::*;
    }
}

pub mod uri {
    pub mod datamodel {
        mod uauthority;
        mod uentity;
        mod uresource;
        mod uuri;

        pub use uauthority::*;
        pub use uentity::*;
        pub use uresource::*;
        pub use uuri::*;
    }
    pub mod validator {
        mod urivalidator;

        pub use urivalidator::*;
    }
    pub mod serializer {
        mod longuriserializer;
        mod microuriserializer;
        mod shorturiserializer;
        mod uriserializer;

        pub use longuriserializer::*;
        pub use microuriserializer::*;
        pub use shorturiserializer::*;
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
        pub mod longuuidserializer;
        pub mod microuuidserializer;
        pub mod uuidserializer;
    }
    pub mod validator {
        mod uuidvalidator;

        pub use crate::types::ustatus::*;
        pub use crate::types::validationresult::*;
    }
}

pub mod transport {
    pub mod datamodel {
        mod uattributes;
        mod ulistener;
        mod umessagetype;
        mod upayload;
        mod upriority;
        mod userializationhint;
        mod utransport;

        pub use uattributes::*;
        pub use ulistener::*;
        pub use umessagetype::*;
        pub use upayload::*;
        pub use upriority::*;
        pub use userializationhint::*;
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

    pub use crate::proto::uprotocol::uuid;
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
        pub mod uuid;
    }
}
