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

use std::fmt::{Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::ops::Deref;
use std::sync::Arc;

use async_trait::async_trait;
#[cfg(test)]
use mockall::automock;

use crate::{UCode, UMessage, UStatus, UUri};

/// A factory for URIs representing this uEntity's resources.
///
/// Implementations may use arbitrary mechanisms to determine the information that
/// is necessary for creating URIs, e.g. environment variables, configuration files etc.
#[cfg_attr(test, automock)]
pub trait LocalUriProvider: Send + Sync {
    /// Gets the _authority_ used for URIs representing this uEntity's resources.
    fn get_authority(&self) -> String;
    /// Gets a URI that represents a given resource of this uEntity.
    fn get_resource_uri(&self, resource_id: u16) -> UUri;
    /// Gets the URI that represents the resource that this uEntity expects
    /// RPC responses and notifications to be sent to.
    fn get_source_uri(&self) -> UUri;
}

/// A handler for processing uProtocol messages.
///
/// Implementations contain the details for what should occur when a message is received.
///
/// Please refer to the [uProtocol Transport Layer specification](https://github.com/eclipse-uprotocol/up-spec/blob/v1.6.0-alpha.2/up-l1/README.adoc)
/// for details.
#[cfg_attr(test, automock)]
#[async_trait]
pub trait UListener: Send + Sync {
    /// Performs some action on receipt of a message.
    ///
    /// # Parameters
    ///
    /// * `msg` - The message to process.
    ///
    /// # Implementation hints
    ///
    /// This function is expected to return almost immediately. If it does not, it could potentially
    /// block processing of succeeding messages. Long-running operations for processing a message should
    /// therefore be run on a separate thread.
    async fn on_receive(&self, msg: UMessage);
}

/// The uProtocol Transport Layer interface that provides a common API for uEntity developers to send and
/// receive messages.
///
/// Implementations contain the details for connecting to the underlying transport technology and
/// sending [`UMessage`]s using the configured technology.
///
/// Please refer to the [uProtocol Transport Layer specification](https://github.com/eclipse-uprotocol/up-spec/blob/v1.6.0-alpha.2/up-l1/README.adoc)
/// for details.
// [impl->req~up-language-transport-api~1]
#[async_trait]
pub trait UTransport: Send + Sync {
    /// Sends a message using this transport's message exchange mechanism.
    ///
    /// # Arguments
    ///
    /// * `message` - The message to send. The `type`, `source` and `sink` properties of the
    ///   [UAttributes](https://github.com/eclipse-uprotocol/up-spec/blob/v1.6.0-alpha.2/basics/uattributes.adoc) contained
    ///   in the message determine the addressing semantics.
    ///
    /// # Errors
    ///
    /// Returns an error if the message could not be sent.
    async fn send(&self, message: UMessage) -> Result<(), UStatus>;

    /// Receives a message from the transport.
    ///
    /// This default implementation returns an error with [`UCode::UNIMPLEMENTED`].
    ///
    /// # Arguments
    ///
    /// * `source_filter` - The _source_ address pattern that the message to receive needs to match.
    /// * `sink_filter` - The _sink_ address pattern that the message to receive needs to match,
    ///                   or `None` to indicate that the message must not contain any sink address.
    ///
    /// # Errors
    ///
    /// Returns an error if no message could be received, e.g. because no message matches the given addresses.
    async fn receive(
        &self,
        _source_filter: &UUri,
        _sink_filter: Option<&UUri>,
    ) -> Result<UMessage, UStatus> {
        Err(UStatus::fail_with_code(
            UCode::UNIMPLEMENTED,
            "not implemented",
        ))
    }

    /// Registers a listener to be called for messages.
    ///
    /// The listener will be invoked for each message that matches the given source and sink filter patterns
    /// according to the rules defined by the [UUri specification](https://github.com/eclipse-uprotocol/up-spec/blob/v1.6.0-alpha.2/basics/uri.adoc).
    ///
    /// This default implementation returns an error with [`UCode::UNIMPLEMENTED`].
    ///
    /// # Arguments
    ///
    /// * `source_filter` - The _source_ address pattern that messages need to match.
    /// * `sink_filter` - The _sink_ address pattern that messages need to match,
    ///                   or `None` to match messages that do not contain any sink address.
    /// * `listener` - The listener to invoke.
    ///                The listener can be unregistered again using [`UTransport::unregister_listener`].
    ///
    /// # Errors
    ///
    /// Returns an error if the listener could not be registered.
    async fn register_listener(
        &self,
        _source_filter: &UUri,
        _sink_filter: Option<&UUri>,
        _listener: Arc<dyn UListener>,
    ) -> Result<(), UStatus> {
        Err(UStatus::fail_with_code(
            UCode::UNIMPLEMENTED,
            "not implemented",
        ))
    }

    /// Unregisters a message listener.
    ///
    /// The listener will no longer be called for any (matching) messages after this function has
    /// returned successfully.
    ///
    /// This default implementation returns an error with [`UCode::UNIMPLEMENTED`].
    ///
    /// # Arguments
    ///
    /// * `source_filter` - The _source_ address pattern that the listener had been registered for.
    /// * `sink_filter` - The _sink_ address pattern that the listener had been registered for.
    /// * `listener` - The listener to unregister.
    ///
    /// # Errors
    ///
    /// Returns an error if the listener could not be unregistered, for example if the given listener does not exist.
    async fn unregister_listener(
        &self,
        _source_filter: &UUri,
        _sink_filter: Option<&UUri>,
        _listener: Arc<dyn UListener>,
    ) -> Result<(), UStatus> {
        Err(UStatus::fail_with_code(
            UCode::UNIMPLEMENTED,
            "not implemented",
        ))
    }
}

#[cfg(test)]
mockall::mock! {
    /// This extra struct is necessary in order to comply with mockall's requirements regarding the parameter lifetimes
    /// see https://github.com/asomers/mockall/issues/571
    pub Transport {
        pub async fn do_send(&self, message: UMessage) -> Result<(), UStatus>;
        pub async fn do_register_listener<'a>(&'a self, source_filter: &'a UUri, sink_filter: Option<&'a UUri>, listener: Arc<dyn UListener>) -> Result<(), UStatus>;
        pub async fn do_unregister_listener<'a>(&'a self, source_filter: &'a UUri, sink_filter: Option<&'a UUri>, listener: Arc<dyn UListener>) -> Result<(), UStatus>;
    }
}

#[cfg(test)]
#[async_trait]
/// This delegates the invocation of the UTransport functions to the mocked functions of the Transport struct.
/// see https://github.com/asomers/mockall/issues/571
impl UTransport for MockTransport {
    async fn send(&self, message: UMessage) -> Result<(), UStatus> {
        self.do_send(message).await
    }
    async fn register_listener(
        &self,
        source_filter: &UUri,
        sink_filter: Option<&UUri>,
        listener: Arc<dyn UListener>,
    ) -> Result<(), UStatus> {
        self.do_register_listener(source_filter, sink_filter, listener)
            .await
    }
    async fn unregister_listener(
        &self,
        source_filter: &UUri,
        sink_filter: Option<&UUri>,
        listener: Arc<dyn UListener>,
    ) -> Result<(), UStatus> {
        self.do_unregister_listener(source_filter, sink_filter, listener)
            .await
    }
}

/// A wrapper type that allows comparing [`UListener`]s to each other.
///
/// # Note
///
/// Not necessary for end-user uEs to use. Primarily intended for `up-client-foo-rust` UPClient libraries
/// when implementing [`UTransport`].
///
/// # Rationale
///
/// The wrapper type is implemented such that it can be used in any location you may wish to
/// hold a type implementing [`UListener`].
///
/// Implements necessary traits to allow hashing, so that you may hold the wrapper type in
/// collections which require that, such as a `HashMap` or `HashSet`
#[derive(Clone)]
pub struct ComparableListener {
    listener: Arc<dyn UListener>,
}

impl ComparableListener {
    pub fn new(listener: Arc<dyn UListener>) -> Self {
        Self { listener }
    }
    /// Gets a clone of the wrapped reference to the listener.
    pub fn into_inner(&self) -> Arc<dyn UListener> {
        self.listener.clone()
    }

    /// Allows us to get the pointer address of this `ComparableListener` on the heap
    fn pointer_address(&self) -> usize {
        // Obtain the raw pointer from the Arc
        let ptr = Arc::as_ptr(&self.listener);
        // Cast the fat pointer to a raw thin pointer to ()
        let thin_ptr = ptr as *const ();
        // Convert the thin pointer to a usize
        thin_ptr as usize
    }
}

impl Deref for ComparableListener {
    type Target = dyn UListener;

    fn deref(&self) -> &Self::Target {
        &*self.listener
    }
}

impl Hash for ComparableListener {
    /// Feeds the pointer to the listener held by `self` into the given [`Hasher`].
    ///
    /// This is consistent with the implementation of [`ComparableListener::eq`].
    fn hash<H: Hasher>(&self, state: &mut H) {
        Arc::as_ptr(&self.listener).hash(state);
    }
}

impl PartialEq for ComparableListener {
    /// Compares this listener to another listener.
    ///
    /// # Returns
    ///
    /// `true` if the pointer to the listener held by `self` is equal to the pointer held by `other`.
    /// This is consistent with the implementation of [`ComparableListener::hash`].
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.listener, &other.listener)
    }
}

impl Eq for ComparableListener {}

impl Debug for ComparableListener {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "ComparableListener: {}", self.pointer_address())
    }
}

#[cfg(test)]
mod tests {
    use crate::{ComparableListener, UListener, UMessage};
    use std::{
        hash::{DefaultHasher, Hash, Hasher},
        ops::Deref,
        sync::Arc,
    };

    use super::MockUListener;

    #[tokio::test]
    async fn test_deref_returns_wrapped_listener() {
        let mut mock_listener = MockUListener::new();
        mock_listener.expect_on_receive().once().return_const(());
        let listener_one = Arc::new(mock_listener);
        let comparable_listener_one = ComparableListener::new(listener_one);
        comparable_listener_one
            .deref()
            .on_receive(UMessage::default())
            .await;
    }

    #[tokio::test]
    async fn test_to_inner_returns_reference_to_wrapped_listener() {
        let mut mock_listener = MockUListener::new();
        mock_listener.expect_on_receive().once().return_const(());
        let listener_one = Arc::new(mock_listener);
        let comparable_listener_one = ComparableListener::new(listener_one);
        comparable_listener_one
            .into_inner()
            .on_receive(UMessage::default())
            .await;
    }

    #[tokio::test]
    async fn test_eq_and_hash_are_consistent_for_comparable_listeners_wrapping_same_listener() {
        let mut mock_listener = MockUListener::new();
        mock_listener.expect_on_receive().times(2).return_const(());
        let listener_one = Arc::new(mock_listener);
        let listener_two = listener_one.clone();
        listener_one.on_receive(UMessage::default()).await;
        listener_two.on_receive(UMessage::default()).await;
        let comparable_listener_one = ComparableListener::new(listener_one);
        let comparable_listener_two = ComparableListener::new(listener_two);
        assert!(&comparable_listener_one.eq(&comparable_listener_two));

        let mut hasher = DefaultHasher::new();
        comparable_listener_one.hash(&mut hasher);
        let hash_one = hasher.finish();
        let mut hasher = DefaultHasher::new();
        comparable_listener_two.hash(&mut hasher);
        let hash_two = hasher.finish();
        assert_eq!(hash_one, hash_two);
    }

    #[tokio::test]
    async fn test_eq_and_hash_are_consistent_for_comparable_listeners_wrapping_different_listeners()
    {
        let mut mock_listener_one = MockUListener::new();
        mock_listener_one
            .expect_on_receive()
            .once()
            .return_const(());
        let listener_one = Arc::new(mock_listener_one);
        let mut mock_listener_two = MockUListener::new();
        mock_listener_two
            .expect_on_receive()
            .once()
            .return_const(());
        let listener_two = Arc::new(mock_listener_two);
        listener_one.on_receive(UMessage::default()).await;
        listener_two.on_receive(UMessage::default()).await;
        let comparable_listener_one = ComparableListener::new(listener_one);
        let comparable_listener_two = ComparableListener::new(listener_two);
        assert!(!&comparable_listener_one.eq(&comparable_listener_two));

        let mut hasher = DefaultHasher::new();
        comparable_listener_one.hash(&mut hasher);
        let hash_one = hasher.finish();
        let mut hasher = DefaultHasher::new();
        comparable_listener_two.hash(&mut hasher);
        let hash_two = hasher.finish();
        assert_ne!(hash_one, hash_two);
    }

    #[test]
    fn test_comparable_listener_pointer_address() {
        let bar = Arc::new(MockUListener::new());
        let comp_listener = ComparableListener::new(bar);

        let comp_listener_thread = comp_listener.clone();
        let handle = std::thread::spawn(move || comp_listener_thread.pointer_address());

        let comp_listener_address_other_thread = handle.join().unwrap();
        let comp_listener_address_this_thread = comp_listener.pointer_address();

        assert_eq!(
            comp_listener_address_this_thread,
            comp_listener_address_other_thread
        );
    }

    #[test]
    fn test_comparable_listener_debug_outputs() {
        let bar = Arc::new(MockUListener::new());
        let comp_listener = ComparableListener::new(bar);
        let debug_output = format!("{comp_listener:?}");
        assert!(!debug_output.is_empty());
    }
}
