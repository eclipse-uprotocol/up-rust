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
up-rust is the Rust language library for [Eclipse uProtocol&trade;](https://github.com/eclipse-uprotocol/up-spec/blob/v1.6.0-alpha.7/README.adoc) —
a protocol that lets software components (in vehicles and beyond) publish
data, subscribe to it, and call each other's services over whatever
messaging technology a deployment happens to use: MQTT, Zenoh, DDS,
shared memory, and more.

The idea in one sentence: **you write against one small messaging API, and
the technology underneath stays swappable.**

## Your first message

```rust,no_run
use up_rust::{UMessageBuilder, UPayloadFormat, UTransport, UUri};

async fn publish_engine_temp(transport: &dyn UTransport) -> Result<(), Box<dyn std::error::Error>> {
    // Every message has a source address: authority / entity / version / resource.
    let topic = UUri::try_from_parts("my-vehicle", 0x1_0001, 1, 0x8001)?;

    let message = UMessageBuilder::publish(topic)
        .build_with_payload("92.5", UPayloadFormat::Text)?;

    transport.send(message).await?;
    Ok(())
}
```

That is the whole application-side model: build a [`UMessage`], hand it to
a [`UTransport`] someone has configured, done. Receiving is the mirror
image — register a [`UListener`] for the addresses you care about.

## Which reader are you?

| You want to… | Start at | Cargo features |
| --- | --- | --- |
| **Write an application** — publish, subscribe, or call/serve RPC | The role traits in [`communication`]: `Publisher`, `Subscriber`, `Notifier`, `RpcClient`, `RpcServer` — ready-made on top of any transport | `communication` |
| **Connect a messaging technology** — make uProtocol run over your broker/bus | [`UTransport`] (start here), then the [`guide`]'s transport chapter for the richer frame-based options | `transport-implementer-api` |
| **Add a payload encoding** — carry a new serialization format efficiently | The [`guide`]'s wire chapter and [`wire_implementer_api`] | `wire-implementer-api` |
| **Move large payloads without copies** — camera frames, LiDAR scans, big tables | The typed path in the [transport chapter](crate::guide::applications::transport) (apps) or the [zero-copy tutorial](crate::guide::transports::zero_copy) (transports) | `zero-copy-transport` (+ your side's feature above) |

The [`guide`] walks each path end to end with code.

## Five words you'll meet everywhere

* **uEntity** — any software component that talks uProtocol (an app, a
  service, a sensor feed).
* **Transport Layer ("up-L1")** — the layer that physically moves message
  bytes; a *transport* is one implementation of it (Zenoh, MQTT, DDS, …).
* **Communication Layer ("up-L2")** — the friendly role API
  (publish/subscribe/RPC) built on top of any transport.
* **Wire format** — how a payload is encoded into bytes (protobuf, CDR,
  Arrow, …). Wires and transports are independent: any wire can ride any
  capable transport.
* **Zero-copy loan** — for large payloads, a transport can *lend* you its
  own transmit buffer so you write data exactly once, directly where it
  will be sent from. The receive side hands payloads out the same way, as
  read-only *leases*.

## Module map

* [`communication`] — the up-L2 roles most applications use.
* [`umessage`](UMessage), [`uri`](UUri), [`uattributes`](UAttributes) — the
  core message model.
* [`utransport`](UTransport) — the up-L1 contract transports implement.
* [`wire`] and [`payload`] — wire identities and payload codecs.
* [`frame`] — the validated frame model advanced transports exchange.
* [`guide`] — tutorials for every audience above.

## Features

No feature is enabled by default; enable what your role needs. Each
feature below silently enables everything it requires. Constrained
publish-only builds need **no feature at all**: use the Transport Layer
directly — it's the first example above, and it carries no protobuf and
no tokio.

*/
#![cfg_attr(feature = "document-features", doc = document_features::document_features!())]
#![doc = "## References"]
#![doc = ""]
#![doc = "* [uProtocol Specification](https://github.com/eclipse-uprotocol/up-spec/tree/v1.6.0-alpha.7)"]

// The stable-payload derives expand absolute ::up_rust paths; alias the
// crate to its external name so they resolve in-crate too.
extern crate self as up_rust;

#[cfg(feature = "cloudevents")]
mod cloudevents;
#[cfg(feature = "cloudevents")]
pub use cloudevents::{CloudEvent, CONTENT_TYPE_CLOUDEVENTS_PROTOBUF};

// [impl->dsn~communication-layer-api-namespace~1]
#[cfg(feature = "communication-api")]
pub mod communication;

#[cfg(any(feature = "udiscovery", feature = "usubscription"))]
pub mod core;

#[cfg(feature = "util")]
pub mod local_transport;
#[cfg(feature = "util")]
pub use local_transport::LocalTransport;

#[cfg(feature = "symphony")]
pub mod symphony;

mod uattributes;
pub use uattributes::{
    NotificationValidator, PublishValidator, RequestValidator, ResponseValidator, UAttributes,
    UAttributesError, UAttributesValidator, UAttributesValidators, UMessageType, UPayloadFormat,
    UPriority,
};

mod umessage;
pub use umessage::{
    BuilderState, NotificationBuilderState, PublishBuilderState, RequestBuilderState,
    ResponseBuilderState, UMessage, UMessageBuilder, UMessageError,
};

mod uri;
pub use uri::{ExactUUri, UUri, UUriError};

mod ustatus;
pub use ustatus::{UAny, UCode, UStatus};

mod utransport;
pub use utransport::{
    verify_filter_criteria, ComparableListener, LocalUriProvider, StaticUriProvider, UListener,
    UTransport,
};
#[cfg(feature = "test-util")]
pub use utransport::{MockLocalUriProvider, MockTransport, MockUListener};

mod uuid;
pub use uuid::UUID;

#[cfg(feature = "up-core-api")]
// protoc-generated types, see build.rs
pub(crate) mod up_core_api {
    include!(concat!(env!("OUT_DIR"), "/uprotocol/mod.rs"));
}

#[cfg(feature = "protobuf-support")]
mod protobuf_mappable;
#[cfg(feature = "protobuf-support")]
pub use protobuf_mappable::ProtobufMappable;

mod serialization_error;
pub use serialization_error::SerializationError;

// ---- native frame model ----

mod validation_state;
pub use validation_state::{Unvalidated, Validated};

/// The native frame data model: metadata, canonical field-block codec, and
/// the read-side frame view contract.
pub mod frame;
pub use frame::metadata::{
    FrameMessageKind, FramePriority, PayloadEncoding, UFrameMetadata, UFrameMetadataError,
};
pub use frame::{validate_frame_view_for_transport, UFrameView};

/// Payload codec contracts shared by the frame families.
pub mod payload;
pub use payload::UWireError;

// ---- owned-frame transport family ----

#[cfg(feature = "owned-frame-transport")]
pub use frame::envelope::{UFrameWireError, UFrameWireFormat};

#[cfg(feature = "owned-frame-transport")]
mod owned_frame;
#[cfg(all(feature = "owned-frame-transport", any(test, feature = "test-util")))]
pub use owned_frame::InMemoryOwnedTransport;
#[cfg(feature = "owned-frame-transport")]
pub use owned_frame::UOwnedFrame;

#[cfg(feature = "owned-frame-transport")]
pub use utransport::{UOwnedListener, UOwnedTransport, UOwnedTransportImpl};

// ---- zero-copy transport family ----

pub use payload::codec::{
    DecodePayload, EncodePayload, PayloadCodec, PayloadFormat, PayloadLayout, ProtobufPayload,
    ReadDecodePayload,
};
pub use payload::loan::LoanPayload;
#[cfg(feature = "zero-copy-transport")]
pub use payload::loan::{LoanUninitPayload, LoanedInitPayload, LoanedUninitPayload};
pub use payload::stable::{
    assert_stable_payload_byte_backed_uninit, stable_payload_supports_byte_backed_uninit,
    ByteBackedStablePayload, InitializedStablePayload, StableContainerPayload,
    StableContainerPayloadInfo, StablePayload, StablePayloadInit, StablePayloadInitContext,
};

#[cfg(feature = "zero-copy-transport")]
mod zero_copy;
#[cfg(all(feature = "test-util", feature = "zero-copy-transport"))]
pub use zero_copy::InMemoryZeroCopyTransport;
#[cfg(feature = "perf-diagnostics")]
#[doc(hidden)]
pub use zero_copy::UninitStableSendPhases;
#[cfg(feature = "zero-copy-transport")]
pub use zero_copy::{
    validate_tx_buffer_for_transport, verify_tx_buffer_payload_layout,
    verify_uninit_tx_buffer_payload_layout, LoanedPayload, LoanedPayloadUninitMut,
    PayloadAlignment, PayloadLoanProvenance, ULoanedContiguousZeroCopyRxFrame, UTxBuffer,
    UTxLoanSpec, UTxPayloadSpec, UUninitTxBuffer, UVecRxLease, UVecTxBuffer, UVecUninitTxBuffer,
    UZeroCopyListener, UZeroCopyRxLease, UZeroCopyTransport, UZeroCopyTransportExt,
    UZeroCopyTransportImpl, UZeroCopyUninitTransportImpl,
};
#[cfg(feature = "zero-copy-transport")]
pub use zero_copy::{UZeroCopyUninitTransport, UZeroCopyUninitTransportExt};

#[cfg(any(test, feature = "test-util", feature = "payload-contract-fixtures"))]
pub mod test_support;

/// Derives for stable payload types.
pub use up_rust_macros::{ByteBackedStablePayload, StablePayload, StablePayloadInit};

#[doc(hidden)]
pub mod __derive_support {
    pub use crate::payload::stable::{
        ByteBackedStablePayloadField, StablePayloadInitSet, StablePayloadInitSlot,
        StablePayloadInitUnset,
    };
}

// ---- wire formats ----

#[cfg(feature = "wire-implementer-api")]
pub mod wire;
#[cfg(not(feature = "wire-implementer-api"))]
mod wire;
#[cfg(any(feature = "selected-wire-user-api", feature = "wire-implementer-api"))]
pub use wire::NativePrefixFrameMetadataCodec;
#[cfg(any(feature = "selected-wire-user-api", feature = "wire-implementer-api"))]
pub use wire::{
    ProtobufWire, StableContainerWireFormat, UProtocolNativeWire, WireCompatibility, WireIdentity,
    WireIdentityRef, NATIVE_EXPLICIT_PAYLOAD_FAMILY_ID, NATIVE_PREFIX_METADATA_LAYOUT_ID,
    PROTOBUF_PAYLOAD_FAMILY_ID, PROTOBUF_WIRE_ID, STABLE_CONTAINER_PAYLOAD_FAMILY_ID,
    STABLE_CONTAINER_WIRE_ID, UFRAME_FIELDS_METADATA_LAYOUT_ID, UPROTOCOL_NATIVE_WIRE_ID,
    XCDR_V2_PAYLOAD_FAMILY_ID, XCDR_V2_WIRE_ID,
};
#[cfg(feature = "wire-implementer-api")]
pub use wire::{UWire, UWireDecode, UWireEncode, UWirePayload, UWireReadDecode};
#[cfg(feature = "wire-implementer-api")]
pub use wire::{
    UWireMetadataCodec, UWireMetadataCodecFor, UWireMetadataContext, UWireMetadataError,
};

/// Compatibility import surface for wire-format implementers.
///
/// The crate root remains canonical; this role-oriented module groups the same
/// public contracts for existing wire crates and focused imports.
#[cfg(feature = "wire-implementer-api")]
pub mod wire_implementer_api {
    pub use crate::{
        NativePrefixFrameMetadataCodec, ProtobufWire, StableContainerWireFormat,
        UProtocolNativeWire, UWire, UWireDecode, UWireEncode, UWireMetadataCodec,
        UWireMetadataCodecFor, UWireMetadataContext, UWireMetadataError, UWirePayload,
        UWireReadDecode, WireCompatibility, WireIdentity, WireIdentityRef,
        NATIVE_EXPLICIT_PAYLOAD_FAMILY_ID, NATIVE_PREFIX_METADATA_LAYOUT_ID,
        PROTOBUF_PAYLOAD_FAMILY_ID, PROTOBUF_WIRE_ID, STABLE_CONTAINER_PAYLOAD_FAMILY_ID,
        STABLE_CONTAINER_WIRE_ID, UFRAME_FIELDS_METADATA_LAYOUT_ID, UPROTOCOL_NATIVE_WIRE_ID,
        XCDR_V2_PAYLOAD_FAMILY_ID, XCDR_V2_WIRE_ID,
    };
}

// ---- selected-wire transport ----

#[cfg(all(
    feature = "selected-wire-transport-core",
    feature = "transport-implementer-api"
))]
pub mod wire_transport;
#[cfg(all(
    feature = "selected-wire-transport-core",
    not(feature = "transport-implementer-api")
))]
mod wire_transport;

#[cfg(feature = "transport-implementer-api")]
pub use wire_transport::UEncodedRxFrame;
#[cfg(all(feature = "selected-wire-user-api", feature = "zero-copy-transport"))]
pub use wire_transport::USelectedWireZeroCopyTransport;
#[cfg(all(
    feature = "selected-wire-transport-core",
    any(
        feature = "transport-implementer-api",
        feature = "wire-implementer-api"
    )
))]
pub use wire_transport::UWireTransport;
#[cfg(all(
    feature = "owned-frame-transport",
    feature = "transport-implementer-api"
))]
pub use wire_transport::{
    EncodedOwnedFrame, PreparedOwnedFrame, UEncodedOwnedListener, UOwnedTransportCore,
};
#[cfg(all(feature = "transport-implementer-api", feature = "zero-copy-transport"))]
pub use wire_transport::{
    PreparedTxLoanSpec, UEncodedLoanedRxFrame, UEncodedZeroCopyListener, UZeroCopyTransportCore,
    UZeroCopyUninitTransportCore,
};
#[cfg(feature = "selected-wire-user-api")]
pub use wire_transport::{
    ProtobufWireTransport, StableContainerWireTransport, UNativePrefixWireTransport,
    UWithNativePrefixWire,
};
#[cfg(feature = "selected-wire-user-api")]
pub use wire_transport::{UHasWire, UWireRx};

/// Compatibility import surface for selected-wire users.
///
/// The crate root remains canonical; this role-oriented module groups the same
/// public contracts for existing applications and focused imports.
#[cfg(feature = "selected-wire-user-api")]
pub mod selected_wire_user_api {
    #[cfg(feature = "zero-copy-transport")]
    pub use crate::USelectedWireZeroCopyTransport;
    pub use crate::{
        ProtobufWire, ProtobufWireTransport, StableContainerWireFormat,
        StableContainerWireTransport, UHasWire, UNativePrefixWireTransport, UProtocolNativeWire,
        UWireRx, UWithNativePrefixWire, WireCompatibility, WireIdentity, WireIdentityRef,
    };
    #[cfg(feature = "zero-copy-transport")]
    pub use crate::{UZeroCopyUninitTransport, UZeroCopyUninitTransportExt};
}

/// Compatibility import surface for transport implementers.
///
/// The crate root remains canonical; this role-oriented module groups the same
/// encoded-core contracts for existing transports and focused imports.
#[cfg(feature = "transport-implementer-api")]
pub mod transport_implementer_api {
    #[cfg(feature = "owned-frame-transport")]
    pub use crate::{
        EncodedOwnedFrame, PreparedOwnedFrame, UEncodedOwnedListener, UOwnedTransportCore,
    };
    #[cfg(feature = "zero-copy-transport")]
    pub use crate::{
        PreparedTxLoanSpec, UEncodedLoanedRxFrame, UEncodedZeroCopyListener,
        UZeroCopyTransportCore, UZeroCopyUninitTransportCore,
    };
    pub use crate::{UEncodedRxFrame, UWireTransport};
}

// ---- the guide ----

#[doc = include_str!("guide/README.md")]
pub mod guide {
    #[cfg_attr(
        all(feature = "communication", feature = "util"),
        doc = include_str!("guide/applications.md")
    )]
    #[cfg_attr(
        not(all(feature = "communication", feature = "util")),
        doc = "Enable `communication` and `util` for the runnable application guide."
    )]
    pub mod applications {
        #[cfg_attr(
            all(
                feature = "communication",
                feature = "util",
                feature = "test-util",
                feature = "selected-wire-user-api",
                feature = "owned-frame-transport",
                feature = "zero-copy-transport"
            ),
            doc = include_str!("guide/communication.md")
        )]
        #[cfg_attr(
            not(all(
                feature = "communication",
                feature = "util",
                feature = "test-util",
                feature = "selected-wire-user-api",
                feature = "owned-frame-transport",
                feature = "zero-copy-transport"
            )),
            doc = "Enable the documented communication, transport-family and test features for the runnable communication guide."
        )]
        pub mod communication {}
        #[doc = include_str!("guide/transport.md")]
        pub mod transport {}
    }
    #[doc = include_str!("guide/transports.md")]
    pub mod transports {
        #[doc = include_str!("guide/utransport.md")]
        pub mod utransport {}
        #[cfg_attr(
            feature = "owned-frame-transport",
            doc = include_str!("guide/owned.md")
        )]
        #[cfg_attr(
            not(feature = "owned-frame-transport"),
            doc = "Enable `owned-frame-transport` for the runnable owned-frame guide."
        )]
        pub mod owned {}
        #[cfg_attr(
            all(
                feature = "zero-copy-transport",
                feature = "test-util",
                feature = "transport-implementer-api",
                feature = "selected-wire-user-api"
            ),
            doc = include_str!("guide/zero_copy.md")
        )]
        #[cfg_attr(
            not(all(
                feature = "zero-copy-transport",
                feature = "test-util",
                feature = "transport-implementer-api",
                feature = "selected-wire-user-api"
            )),
            doc = "Enable the documented zero-copy, selected-wire, transport-implementer and test features for the runnable zero-copy guide."
        )]
        pub mod zero_copy {}
    }
    #[doc = include_str!("guide/wires.md")]
    pub mod wires {}
    #[doc = include_str!("guide/trait_map.md")]
    pub mod trait_map {}
}

#[cfg(feature = "payload-contract-fixtures")]
pub mod bench_fixtures;
