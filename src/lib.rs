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

pub mod cloudevent {
    pub mod datamodel {
        pub mod ucloudeventattributes;
        pub mod ucloudeventtype;
    }
    pub mod serializer {
        pub mod cloudeventjsonserializer;
        pub mod cloudeventprotobufserializer;
        pub mod cloudeventserializer;
    }
    pub mod validator {
        pub mod cloudeventvalidator;
        pub mod validationresult;
    }
    pub mod ucloudevent;
    pub mod ucloudeventbuilder;
}

pub mod uri {
    pub mod datamodel {
        pub mod uauthority;
        pub mod uentity;
        pub mod uresource;
        pub mod uuri;
    }
    pub mod validator {
        pub mod urivalidator;
    }
    pub mod serializer {
        pub mod longuriserializer;
        pub mod microuriserializer;
        pub mod uriserializer;
    }
}
pub mod uuid {
    pub mod uuidbuilder;
    pub mod uuidutils;
}

pub mod transport {
    pub mod datamodel {
        pub mod uattributes;
        pub mod ulistener;
        pub mod umessagetype;
        pub mod upayload;
        pub mod upriority;
        pub mod userializationhint;
        pub mod ustatus;
    }
    pub mod validator {
        pub mod uattributesvalidator;
    }
    pub mod utransport;
}

pub mod rpc {
    pub mod calloptions;
    pub mod rpcclient;
    pub mod rpcmapper;
    pub mod rpcresult;
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
}
