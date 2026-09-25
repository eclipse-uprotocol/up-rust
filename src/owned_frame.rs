/********************************************************************************
 * Copyright (c) 2026 Contributors to the Eclipse Foundation
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

use bytes::Bytes;

use crate::{UFrameMetadata, UFrameMetadataError};

/// Owned, serialization-neutral native uProtocol frame.
///
/// `UOwnedFrame` carries Phase 01 frame metadata plus optional owned payload
/// bytes. It is available only with the `owned-frame-transport` feature and is
/// additive to the ordinary `UTransport`/`UMessage` compatibility path.
#[derive(Clone, Debug, PartialEq)]
pub struct UOwnedFrame<S = Validated> {
    metadata: UFrameMetadata,
    payload: Option<Bytes>,
    _state: core::marker::PhantomData<S>,
}

pub use crate::validation_state::{Unvalidated, Validated};

impl UOwnedFrame<Validated> {
    /// Creates a frame from metadata and optional payload bytes after validation.
    ///
    /// # Errors
    ///
    /// Returns an error if metadata validation fails or payload presence does
    /// not match metadata payload encoding presence.
    pub fn new(
        metadata: UFrameMetadata,
        payload: Option<Bytes>,
    ) -> Result<Self, UFrameMetadataError> {
        let frame = UOwnedFrame::<Validated> {
            metadata,
            payload,
            _state: core::marker::PhantomData,
        };
        frame.check()?;
        Ok(frame)
    }

    /// Creates a frame without validation.
    #[must_use]
    pub fn new_unchecked(
        metadata: UFrameMetadata,
        payload: Option<Bytes>,
    ) -> UOwnedFrame<Unvalidated> {
        UOwnedFrame {
            metadata,
            payload,
            _state: core::marker::PhantomData,
        }
    }

    /// Creates a payload-bearing frame after validation.
    ///
    /// # Errors
    ///
    /// Returns an error if metadata validation fails or the metadata has no
    /// payload encoding.
    pub fn with_payload(
        metadata: UFrameMetadata,
        payload: impl Into<Bytes>,
    ) -> Result<Self, UFrameMetadataError> {
        Self::new(metadata, Some(payload.into()))
    }

    /// Creates a payload-bearing frame without validation (test support:
    /// negative tests must be able to construct invalid frames).
    #[must_use]
    #[cfg(test)]
    pub(crate) fn with_payload_unchecked(
        metadata: UFrameMetadata,
        payload: impl Into<Bytes>,
    ) -> UOwnedFrame<Unvalidated> {
        Self::new_unchecked(metadata, Some(payload.into()))
    }

    /// Creates a no-payload frame after validation.
    ///
    /// # Errors
    ///
    /// Returns an error if metadata validation fails or the metadata still has
    /// a payload encoding.
    pub fn without_payload(metadata: UFrameMetadata) -> Result<Self, UFrameMetadataError> {
        Self::new(metadata, None)
    }
}

impl UOwnedFrame<Unvalidated> {
    /// Validates the frame, transitioning it to the [`Validated`] state.
    ///
    /// This is the only way from [`Unvalidated`] to [`Validated`]; the
    /// state transition IS the validation, and every transport API
    /// accepts only the validated state.
    ///
    /// # Errors
    ///
    /// Returns an error if metadata is invalid, if payload bytes are
    /// present without encoding metadata, or if encoding metadata is
    /// present without payload bytes.
    pub fn validate(self) -> Result<UOwnedFrame<Validated>, UFrameMetadataError> {
        self.check()?;
        Ok(UOwnedFrame {
            metadata: self.metadata,
            payload: self.payload,
            _state: core::marker::PhantomData,
        })
    }
}

impl UOwnedFrame<Validated> {
    /// Projects this frame into a `UMessage` — the compatibility bridge back
    /// to the `UTransport` family.
    ///
    /// # Errors
    ///
    /// Returns an error if the frame metadata cannot be represented as
    /// message attributes.
    pub fn to_umessage(&self) -> Result<crate::UMessage, crate::UFrameMetadataError> {
        crate::frame::metadata::try_project_frame_to_umessage(
            self.metadata().clone(),
            self.payload().cloned(),
        )
    }
}

impl<S> UOwnedFrame<S> {
    /// Validates metadata and payload presence consistency.
    ///
    /// # Errors
    ///
    /// Returns an error if metadata is invalid, if payload bytes are present
    /// without encoding metadata, or if encoding metadata is present without
    /// payload bytes.
    pub(crate) fn check(&self) -> Result<(), UFrameMetadataError> {
        self.metadata.validate()?;
        match (
            self.payload.is_some(),
            self.metadata.payload_encoding().is_some(),
        ) {
            (true, true) | (false, false) => Ok(()),
            (true, false) => Err(UFrameMetadataError::PayloadWithoutEncoding),
            (false, true) => Err(UFrameMetadataError::EncodingWithoutPayload),
        }
    }

    /// Returns the frame metadata.
    #[must_use]
    pub fn metadata(&self) -> &UFrameMetadata {
        &self.metadata
    }

    /// Returns the payload bytes when the frame carries a payload.
    #[must_use]
    pub fn payload(&self) -> Option<&Bytes> {
        self.payload.as_ref()
    }

    /// Returns the payload bytes, or an empty slice when payload is absent.
    ///
    /// Use [`Self::payload`] when distinguishing absent payload from present
    /// empty payload matters.
    #[must_use]
    pub fn payload_bytes(&self) -> &[u8] {
        self.payload.as_deref().unwrap_or_default()
    }

    /// Returns whether the frame carries payload bytes, including present empty payloads.
    #[must_use]
    pub fn has_payload(&self) -> bool {
        self.payload.is_some()
    }

    /// Consumes the frame and returns its metadata.
    #[must_use]
    pub fn into_metadata(self) -> UFrameMetadata {
        self.metadata
    }

    /// Consumes the frame and returns its optional payload bytes.
    #[must_use]
    pub fn into_payload(self) -> Option<Bytes> {
        self.payload
    }

    /// Consumes the frame and returns metadata plus optional payload bytes.
    #[must_use]
    pub fn into_parts(self) -> (UFrameMetadata, Option<Bytes>) {
        (self.metadata, self.payload)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{PayloadEncoding, UMessageBuilder, UUri};

    fn topic() -> UUri {
        UUri::try_from_parts("vehicle", 0x4210, 0x01, 0x9000).expect("failed to create topic")
    }

    fn metadata_without_encoding() -> UFrameMetadata {
        let message = UMessageBuilder::publish(topic()).build().expect("message");
        crate::frame::metadata::try_project_attributes_to_frame_metadata(message.attributes(), None)
            .expect("metadata")
    }

    fn metadata_with_encoding() -> UFrameMetadata {
        let message = UMessageBuilder::publish(topic()).build().expect("message");
        crate::frame::metadata::try_project_attributes_to_frame_metadata(
            message.attributes(),
            Some(PayloadEncoding::RAW),
        )
        .expect("metadata")
    }

    #[test]
    fn owned_frame_rejects_payload_without_encoding() {
        let error =
            UOwnedFrame::with_payload(metadata_without_encoding(), Bytes::from_static(b"payload"))
                .unwrap_err();

        assert_eq!(error, UFrameMetadataError::PayloadWithoutEncoding);
    }

    #[test]
    fn owned_frame_rejects_encoding_without_payload() {
        let error = UOwnedFrame::without_payload(metadata_with_encoding()).unwrap_err();

        assert_eq!(error, UFrameMetadataError::EncodingWithoutPayload);
    }

    #[test]
    fn owned_frame_accepts_absent_payload_without_encoding() {
        let frame = UOwnedFrame::without_payload(metadata_without_encoding()).unwrap();

        assert!(!frame.has_payload());
        assert_eq!(frame.payload_bytes(), &[] as &[u8]);
        assert!(frame.metadata().payload_encoding().is_none());
    }

    #[test]
    fn owned_frame_accepts_present_empty_payload_with_standard_encoding() {
        let frame = UOwnedFrame::with_payload(metadata_with_encoding(), Bytes::new()).unwrap();

        assert!(frame.has_payload());
        assert_eq!(frame.payload(), Some(&Bytes::new()));
        assert_eq!(frame.payload_bytes(), &[] as &[u8]);
        assert_eq!(
            frame.metadata().payload_encoding(),
            Some(&PayloadEncoding::RAW)
        );
    }
}

#[cfg(any(test, feature = "test-util"))]
type OwnedListenerRegistration = (
    crate::UUri,
    Option<crate::UUri>,
    std::sync::Arc<dyn crate::UOwnedListener>,
);

/// In-memory owned-frame transport for unit tests, examples, and guide
/// doctests.
///
/// *Role: test/proof support — a complete [`UOwnedTransportImpl`](crate::UOwnedTransportImpl)
/// loopback. Frames sent through it are dispatched to every registered
/// [`UOwnedListener`](crate::UOwnedListener) whose filters match.*
#[cfg(any(test, feature = "test-util"))]
#[derive(Default)]
pub struct InMemoryOwnedTransport {
    listeners: std::sync::RwLock<Vec<OwnedListenerRegistration>>,
}

#[cfg(any(test, feature = "test-util"))]
impl core::fmt::Debug for InMemoryOwnedTransport {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("InMemoryOwnedTransport")
            .finish_non_exhaustive()
    }
}

#[cfg(any(test, feature = "test-util"))]
#[async_trait::async_trait]
impl crate::UOwnedTransportImpl for InMemoryOwnedTransport {
    async fn send_validated_owned(&self, frame: crate::UOwnedFrame) -> Result<(), crate::UStatus> {
        let matching: Vec<_> = {
            let metadata = frame.metadata();
            let source = metadata.source();
            let sink = metadata.sink();
            self.listeners
                .read()
                .expect("lock")
                .iter()
                .filter(|(source_filter, sink_filter, _)| {
                    source_filter.matches(source)
                        && match (sink_filter, sink) {
                            (Some(pattern), Some(candidate)) => pattern.matches(candidate),
                            (None, None) => true,
                            _ => false,
                        }
                })
                .map(|(_, _, listener)| listener.clone())
                .collect()
        };
        for listener in matching {
            listener.on_receive_owned(frame.clone()).await;
        }
        Ok(())
    }

    async fn register_validated_owned_listener(
        &self,
        source_filter: &crate::UUri,
        sink_filter: Option<&crate::UUri>,
        listener: std::sync::Arc<dyn crate::UOwnedListener>,
    ) -> Result<(), crate::UStatus> {
        self.listeners.write().expect("lock").push((
            source_filter.to_owned(),
            sink_filter.map(ToOwned::to_owned),
            listener,
        ));
        Ok(())
    }

    async fn unregister_validated_owned_listener(
        &self,
        source_filter: &crate::UUri,
        sink_filter: Option<&crate::UUri>,
        listener: std::sync::Arc<dyn crate::UOwnedListener>,
    ) -> Result<(), crate::UStatus> {
        let target = std::sync::Arc::as_ptr(&listener) as *const () as usize;
        let mut listeners = self.listeners.write().expect("lock");
        let before = listeners.len();
        listeners.retain(|(sf, kf, l)| {
            !(sf == source_filter
                && kf.as_ref() == sink_filter
                && std::sync::Arc::as_ptr(l) as *const () as usize == target)
        });
        if listeners.len() < before {
            Ok(())
        } else {
            Err(crate::UStatus::fail_with_code(
                crate::UCode::NotFound,
                "no such owned listener",
            ))
        }
    }
}

#[cfg(test)]
mod in_memory_owned_transport_tests {
    use std::sync::Arc;

    use super::*;
    use crate::{UMessageBuilder, UOwnedListener, UOwnedTransport, UUri};

    struct Capture(std::sync::Mutex<Vec<UOwnedFrame>>);

    #[async_trait::async_trait]
    impl UOwnedListener for Capture {
        async fn on_receive_owned(&self, frame: UOwnedFrame) {
            self.0.lock().expect("capture lock poisoned").push(frame);
        }
    }

    fn frame(topic: &UUri) -> UOwnedFrame {
        let message = UMessageBuilder::publish(topic.clone())
            .build()
            .expect("failed to build message");
        let metadata = crate::frame::metadata::try_project_attributes_to_frame_metadata(
            message.attributes(),
            Some(crate::PayloadEncoding::RAW),
        )
        .expect("failed to project metadata");
        UOwnedFrame::with_payload(metadata, b"x".to_vec()).expect("failed to build frame")
    }

    async fn registered(transport: &InMemoryOwnedTransport, topic: &UUri) -> Arc<Capture> {
        let capture = Arc::new(Capture(std::sync::Mutex::new(Vec::new())));
        transport
            .register_owned_listener(topic, None, capture.clone())
            .await
            .expect("listener registration succeeds");
        capture
    }

    #[tokio::test]
    async fn delivers_frames_matching_the_registered_filter() {
        let transport = InMemoryOwnedTransport::default();
        let topic = UUri::try_from_parts("demo", 0x1_0001, 1, 0x8001).expect("valid topic URI");
        let capture = registered(&transport, &topic).await;

        transport
            .send_owned(frame(&topic))
            .await
            .expect("matching send succeeds");

        assert_eq!(capture.0.lock().expect("capture lock").len(), 1);
    }

    #[tokio::test]
    async fn does_not_deliver_frames_for_other_topics() {
        let transport = InMemoryOwnedTransport::default();
        let topic = UUri::try_from_parts("demo", 0x1_0001, 1, 0x8001).expect("valid topic URI");
        let other = UUri::try_from_parts("demo", 0x1_0001, 1, 0x8002).expect("valid other URI");
        let capture = registered(&transport, &topic).await;

        transport
            .send_owned(frame(&other))
            .await
            .expect("non-matching send succeeds");

        assert!(capture.0.lock().expect("capture lock").is_empty());
    }

    #[tokio::test]
    async fn unregistering_an_unknown_listener_returns_not_found() {
        let transport = InMemoryOwnedTransport::default();
        let topic = UUri::try_from_parts("demo", 0x1_0001, 1, 0x8001).expect("valid topic URI");
        let capture = registered(&transport, &topic).await;

        transport
            .unregister_owned_listener(&topic, None, capture.clone())
            .await
            .expect("first unregister succeeds");
        let error = transport
            .unregister_owned_listener(&topic, None, capture)
            .await
            .expect_err("second unregister must fail");

        assert_eq!(error.code(), crate::UCode::NotFound);
    }
}
