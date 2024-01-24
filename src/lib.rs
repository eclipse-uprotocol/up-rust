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
//! - the [`cloudevent`] module that offers a common way to represent uProtocol messages using the `CloudEvent` data model
//! - the [`rpc`] module which offers wrappers for dealing with uProtocol payload in the context of RPC method invokation
//! - the [`transport`] module as a set of abstractions for various transport-level concerns like status representation and serialization
//! - the [`uri`] module, providing convenience wrappers for creation and validation of uProtocol-style resource identifiers
//! - the [`uuid`] module which generates and validates UUIDs as per the uProtocol specification
//!
//! ## References
//! - [Eclipse-uProtocol Specification](https://github.com/eclipse-uprotocol/uprotocol-spec/tree/main)

mod types {
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
        mod longuriserializer;
        mod microuriserializer;
        mod uriserializer;

        pub use crate::types::serializationerror::*;
        pub use longuriserializer::*;
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
    include!(concat!(env!("OUT_DIR"), "/uprotocol.v1.rs"));

    pub use crate::proto::uprotocol::uauthority;
    pub use crate::proto::uprotocol::uentity;
    pub use crate::proto::uprotocol::umessagetype;
    pub use crate::proto::uprotocol::upayload;
    pub use crate::proto::uprotocol::uresource;
    pub use crate::proto::uprotocol::ustatus;
    pub use crate::proto::uprotocol::uuid;
    pub use crate::proto::uprotocol::uuri;

    pub use u_authority::Remote;
    pub use u_payload::Data;

    // This is to make the more specialized uprotocol types available within the SDK scope;
    // not required by the code, so not sure whether it'll stay in.
    mod v1 {
        // this re-export is necessary to accomodate the package/reference structure of the uprotocol uproto files (included below)
        pub(crate) use crate::uprotocol::{UCode, UMessage, UStatus, UUri, UUriBatch};
    }
    pub mod core {
        pub mod udiscovery {
            pub mod v3 {
                include!(concat!(env!("OUT_DIR"), "/uprotocol.core.udiscovery.v3.rs"));
            }
        }
        pub mod usubscription {
            pub mod v3 {
                include!(concat!(
                    env!("OUT_DIR"),
                    "/uprotocol.core.usubscription.v3.rs"
                ));
            }
        }
        pub mod utwin {
            pub mod v1 {
                include!(concat!(env!("OUT_DIR"), "/uprotocol.core.utwin.v1.rs"));
            }
        }
    }
}

#[allow(non_snake_case)]
pub mod proto {
    // protoc-generated stubs, see build.rs
    include!(concat!(env!("OUT_DIR"), "/io.cloudevents.v1.rs"));

    pub mod cloudevents {
        pub mod protocloudevent;
    }

    pub mod uprotocol {
        pub mod uauthority;
        pub mod uentity;
        pub mod umessagetype;
        pub mod upayload;
        pub mod uresource;
        pub mod ustatus;
        pub mod uuid;
        pub mod uuri;
    }
}
