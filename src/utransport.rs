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

#[cfg(feature = "owned-frame-transport")]
use std::collections::HashMap;
#[cfg(feature = "owned-frame-transport")]
use std::sync::{LazyLock, Mutex as StdMutex};

use async_trait::async_trait;

#[cfg(feature = "owned-frame-transport")]
use crate::UOwnedFrame;
use crate::{UCode, UMessage, UStatus, UUri, UUriError};

#[cfg(feature = "owned-frame-transport")]
mod owned_transport_sealed {
    pub trait Sealed {}
}

/// Verifies that given UUris can be used as source and sink filter UUris
/// for registering listeners.
///
/// This function is helpful for implementing [`UTransport`] in accordance with the
/// uProtocol Transport Layer specification.
///
/// # Errors
///
/// Returns a [`UStatus`] with a [`UCode::InvalidArgument`] and a corresponding detail
/// message, if any of the given UUris cannot be used as filter criteria.
///
pub fn verify_filter_criteria(
    source_filter: &UUri,
    sink_filter: Option<&UUri>,
) -> Result<(), Box<UStatus>> {
    if let Some(sink_filter_uuri) = sink_filter {
        if sink_filter_uuri.is_notification_destination()
            && source_filter.is_notification_destination()
        {
            return Err(Box::from(UStatus::fail_with_code(
                UCode::InvalidArgument,
                "source and sink filters must not both have resource ID 0",
            )));
        }
        if sink_filter_uuri.is_rpc_method()
            && !source_filter.has_wildcard_resource_id()
            && !source_filter.is_notification_destination()
        {
            return Err(Box::from(UStatus::fail_with_code(
                UCode::InvalidArgument,
                "source filter must either have the wildcard resource ID or resource ID 0, if sink filter matches RPC method resource ID")));
        }
    } else if !source_filter.has_wildcard_resource_id() && !source_filter.is_event() {
        return Err(Box::from(UStatus::fail_with_code(
            UCode::InvalidArgument,
            "source filter must either have the wildcard resource ID or a resource ID from topic range, if sink filter is empty")));
    }
    // everything else might match valid messages
    Ok(())
}

/// A factory for URIs representing this uEntity's resources.
///
/// Implementations may use arbitrary mechanisms to determine the information that
/// is necessary for creating URIs, e.g. environment variables, configuration files etc.
// [impl->dsn~localuriprovider-declaration~1]
/// *Role: standalone utility answering "what is my address?"; implemented per uEntity, consumed by the roles — see the [trait map](crate::guide::trait_map).*
///
#[cfg_attr(any(test, feature = "test-util"), mockall::automock)]
pub trait LocalUriProvider: Send + Sync {
    /// Gets the _authority_ used for URIs representing this uEntity's resources.
    fn get_authority(&self) -> String;
    /// Gets a URI that represents a given resource of this uEntity.
    fn get_resource_uri(&self, resource_id: u16) -> UUri;
    /// Gets the URI that represents the resource that this uEntity expects
    /// RPC responses and notifications to be sent to.
    fn get_source_uri(&self) -> UUri;
}

/// A URI provider that is statically configured with the uEntity's authority, entity ID and version.
#[derive(Debug)]
pub struct StaticUriProvider {
    local_uri: UUri,
}

impl StaticUriProvider {
    /// Creates a new URI provider from static information.
    ///
    /// # Arguments
    ///
    /// * `authority` - The uEntity's authority name.
    /// * `entity_id` - The entity identifier.
    /// * `major_version` - The uEntity's major version.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use up_rust::{LocalUriProvider, StaticUriProvider};
    ///
    /// let provider = StaticUriProvider::new("my-vehicle", 0x4210, 0x05).unwrap();
    /// assert_eq!(provider.get_authority(), "my-vehicle");
    /// ```
    pub fn new(
        authority: impl Into<String>,
        entity_id: u32,
        major_version: u8,
    ) -> Result<Self, UUriError> {
        UUri::try_from_parts(authority.into().as_str(), entity_id, major_version, 0x0000)
            .map(|local_uri| StaticUriProvider { local_uri })
    }
}

impl LocalUriProvider for StaticUriProvider {
    fn get_authority(&self) -> String {
        self.local_uri.authority_name().to_owned()
    }

    fn get_resource_uri(&self, resource_id: u16) -> UUri {
        self.local_uri.clone_with_resource_id(resource_id)
    }

    fn get_source_uri(&self) -> UUri {
        self.local_uri.clone()
    }
}

impl From<UUri> for StaticUriProvider {
    fn from(value: UUri) -> Self {
        Self::from(&value)
    }
}

impl From<&UUri> for StaticUriProvider {
    /// Creates a URI provider from a UUri.
    ///
    /// # Arguments
    ///
    /// * `source_uri` - The UUri to take the entity's authority, entity ID and version information from.
    ///   The UUri's resource ID is ignored.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use up_rust::{LocalUriProvider, StaticUriProvider, UUri};
    ///
    /// let source_uri = UUri::try_from("//my-vehicle/4210/5/1000").unwrap();
    /// let provider = StaticUriProvider::from(&source_uri);
    /// assert_eq!(provider.get_authority(), "my-vehicle");
    /// assert_eq!(provider.get_source_uri(), source_uri.clone_with_resource_id(0x0000));
    /// ```
    fn from(source_uri: &UUri) -> Self {
        StaticUriProvider {
            local_uri: source_uri.clone_with_resource_id(0x0000),
        }
    }
}

/// A handler for processing uProtocol messages.
///
/// Implementations contain the details for what should occur when a message is received.
///
/// Please refer to the [uProtocol Transport Layer specification](https://github.com/eclipse-uprotocol/up-spec/blob/v1.6.0-alpha.7/up-l1/README.adoc)
/// for details.
// [impl->dsn~ulistener-declaration~1]
/// *Role: implemented by applications to receive `UTransport`-family [`UMessage`](crate::UMessage)s; registered on a [`UTransport`](crate::UTransport) — see the [trait map](crate::guide::trait_map).*
///
#[cfg_attr(any(test, feature = "test-util"), mockall::automock)]
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
/// Please refer to the [uProtocol Transport Layer specification](https://github.com/eclipse-uprotocol/up-spec/blob/v1.6.0-alpha.7/up-l1/README.adoc)
/// for details.
// [impl->dsn~utransport-declaration~1]
/// *Role: implemented by transports; called by applications (usually via the [`communication`](crate::communication) roles) — see the [trait map](crate::guide::trait_map).*
///
#[async_trait]
pub trait UTransport: Send + Sync {
    /// Sends a message using this transport's message exchange mechanism.
    ///
    /// # Arguments
    ///
    /// * `message` - The message to send. The `type`, `source` and `sink` properties of the
    ///   [UAttributes](https://github.com/eclipse-uprotocol/up-spec/blob/v1.6.0-alpha.7/basics/uattributes.adoc) contained
    ///   in the message determine the addressing semantics.
    ///
    /// # Errors
    ///
    /// Returns an error if the message could not be sent.
    async fn send(&self, message: UMessage) -> Result<(), UStatus>;

    /// Receives a message from the transport.
    ///
    /// This default implementation returns an error with [`UCode::Unimplemented`].
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
            UCode::Unimplemented,
            "not implemented",
        ))
    }

    /// Registers a listener to be called for messages.
    ///
    /// The listener will be invoked for each message that matches the given source and sink filter patterns
    /// according to the rules defined by the [UUri specification](https://github.com/eclipse-uprotocol/up-spec/blob/v1.6.0-alpha.7/basics/uri.adoc).
    ///
    /// This default implementation returns an error with [`UCode::Unimplemented`].
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
            UCode::Unimplemented,
            "not implemented",
        ))
    }

    /// Deregisters a message listener.
    ///
    /// The listener will no longer be called for any (matching) messages after this function has
    /// returned successfully.
    ///
    /// This default implementation returns an error with [`UCode::Unimplemented`].
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
            UCode::Unimplemented,
            "not implemented",
        ))
    }
}

#[cfg(feature = "owned-frame-transport")]
fn verify_owned_filter_criteria(
    source_filter: &UUri,
    sink_filter: Option<&UUri>,
) -> Result<(), UStatus> {
    verify_filter_criteria(source_filter, sink_filter).map_err(|status| *status)
}

#[cfg(feature = "owned-frame-transport")]
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct OwnedListenerRegistrationKey {
    transport: usize,
    source_filter: UUri,
    sink_filter: Option<UUri>,
    listener: usize,
}

#[cfg(feature = "owned-frame-transport")]
static OWNED_LISTENER_REGISTRY: LazyLock<
    StdMutex<HashMap<OwnedListenerRegistrationKey, Arc<dyn UOwnedListener>>>,
> = LazyLock::new(|| StdMutex::new(HashMap::new()));

#[cfg(feature = "owned-frame-transport")]
fn owned_transport_pointer<T: ?Sized>(transport: &T) -> usize {
    let ptr = transport as *const T;
    let thin_ptr = ptr as *const ();
    thin_ptr as usize
}

#[cfg(feature = "owned-frame-transport")]
fn owned_listener_pointer(listener: &Arc<dyn UOwnedListener>) -> usize {
    let ptr = Arc::as_ptr(listener);
    let thin_ptr = ptr as *const ();
    thin_ptr as usize
}

#[cfg(feature = "owned-frame-transport")]
fn owned_listener_registration_key<T: ?Sized>(
    transport: &T,
    source_filter: &UUri,
    sink_filter: Option<&UUri>,
    listener: usize,
) -> OwnedListenerRegistrationKey {
    OwnedListenerRegistrationKey {
        transport: owned_transport_pointer(transport),
        source_filter: source_filter.clone(),
        sink_filter: sink_filter.cloned(),
        listener,
    }
}

// Delivered frames are `UOwnedFrame<Validated>` by type; no re-validation
// wrapper is needed here.

#[cfg(feature = "owned-frame-transport")]
fn registered_owned_listener(
    key: &OwnedListenerRegistrationKey,
    listener: Arc<dyn UOwnedListener>,
) -> (Arc<dyn UOwnedListener>, bool) {
    let mut registry = OWNED_LISTENER_REGISTRY
        .lock()
        .expect("owned listener registry lock poisoned");
    if let Some(existing) = registry.get(key) {
        return (existing.clone(), false);
    }

    registry.insert(key.clone(), listener.clone());
    (listener, true)
}

#[cfg(feature = "owned-frame-transport")]
fn owned_listener_for_unregister(
    key: &OwnedListenerRegistrationKey,
    fallback: Arc<dyn UOwnedListener>,
) -> Arc<dyn UOwnedListener> {
    OWNED_LISTENER_REGISTRY
        .lock()
        .expect("owned listener registry lock poisoned")
        .get(key)
        .cloned()
        .unwrap_or(fallback)
}

/// *Role: implemented by applications to receive owned frames — see the [trait map](crate::guide::trait_map).*
///
/// Listener for experimental owned native-frame transports.
#[cfg(feature = "owned-frame-transport")]
#[cfg_attr(any(test, feature = "test-util"), mockall::automock)]
#[async_trait]
pub trait UOwnedListener: Send + Sync {
    /// Performs some action on receipt of an owned native frame.
    async fn on_receive_owned(&self, frame: UOwnedFrame);
}

/// *Role: implemented by transports offering the owned-frame family; users call [`UOwnedTransport`](crate::UOwnedTransport) — see the [trait map](crate::guide::trait_map).*
///
/// *Role: called by users of the owned-frame family; transports implement [`UOwnedTransportImpl`](crate::UOwnedTransportImpl) instead — see the [trait map](crate::guide::trait_map).*
///
/// Implementation boundary for experimental owned native-frame transports.
///
/// Implementing this trait buys the public [`UOwnedTransport`] API, frame and
/// filter validation, listener validation, and compatibility projections above
/// the transport. Only the [`Validated`](crate::Validated) frame state reaches
/// the required send method — constructors validate, and the only path from
/// [`Unvalidated`](crate::Unvalidated) is [`UOwnedFrame::validate`] — so
/// `req~owned-frame-validate-before-send~1` is enforced by the type system
/// before delegation.
///
/// Send is required. Pull receive and listener registration have default
/// unsupported implementations and are overridden only for carriage patterns
/// the technology supports. `UOwnedTransportCore` is the different,
/// selected-wire seam for transports that carry encoded metadata bytes.
#[cfg(feature = "owned-frame-transport")]
#[async_trait]
pub trait UOwnedTransportImpl: Send + Sync {
    /// Sends an already validated owned frame.
    async fn send_validated_owned(&self, frame: UOwnedFrame) -> Result<(), UStatus>;

    /// Receives one matching owned frame from transports that support pull receive.
    async fn receive_validated_owned(
        &self,
        _source_filter: &UUri,
        _sink_filter: Option<&UUri>,
    ) -> Result<UOwnedFrame, UStatus> {
        Err(UStatus::fail_with_code(
            UCode::Unimplemented,
            "not implemented",
        ))
    }

    /// Registers an owned listener after public filter validation.
    async fn register_validated_owned_listener(
        &self,
        _source_filter: &UUri,
        _sink_filter: Option<&UUri>,
        _listener: Arc<dyn UOwnedListener>,
    ) -> Result<(), UStatus> {
        Err(UStatus::fail_with_code(
            UCode::Unimplemented,
            "not implemented",
        ))
    }

    /// Unregisters an owned listener after public filter validation.
    async fn unregister_validated_owned_listener(
        &self,
        _source_filter: &UUri,
        _sink_filter: Option<&UUri>,
        _listener: Arc<dyn UOwnedListener>,
    ) -> Result<(), UStatus> {
        Err(UStatus::fail_with_code(
            UCode::Unimplemented,
            "not implemented",
        ))
    }
}

#[cfg(feature = "owned-frame-transport")]
impl<T> owned_transport_sealed::Sealed for T where T: UOwnedTransportImpl + ?Sized {}

/// Experimental serialization-neutral owned native-frame transport API.
///
/// This API is additive to [`UTransport`]. It does not replace the ordinary
/// `UMessage` compatibility path and intentionally does not include copying
/// adapters between transport families.
#[cfg(feature = "owned-frame-transport")]
#[async_trait]
pub trait UOwnedTransport: owned_transport_sealed::Sealed + Send + Sync {
    /// Sends an owned native frame after public validation.
    async fn send_owned(&self, frame: UOwnedFrame) -> Result<(), UStatus>;

    /// Receives one matching owned frame from transports that support pull receive.
    async fn receive_owned(
        &self,
        source_filter: &UUri,
        sink_filter: Option<&UUri>,
    ) -> Result<UOwnedFrame, UStatus>;

    /// Registers a listener for matching owned native frames.
    async fn register_owned_listener(
        &self,
        source_filter: &UUri,
        sink_filter: Option<&UUri>,
        listener: Arc<dyn UOwnedListener>,
    ) -> Result<(), UStatus>;

    /// Unregisters a listener for matching owned native frames.
    async fn unregister_owned_listener(
        &self,
        source_filter: &UUri,
        sink_filter: Option<&UUri>,
        listener: Arc<dyn UOwnedListener>,
    ) -> Result<(), UStatus>;
}

#[cfg(feature = "owned-frame-transport")]
#[async_trait]
impl<T> UOwnedTransport for T
where
    T: UOwnedTransportImpl + ?Sized,
{
    async fn send_owned(&self, frame: UOwnedFrame) -> Result<(), UStatus> {
        self.send_validated_owned(frame).await
    }

    async fn receive_owned(
        &self,
        source_filter: &UUri,
        sink_filter: Option<&UUri>,
    ) -> Result<UOwnedFrame, UStatus> {
        verify_owned_filter_criteria(source_filter, sink_filter)?;
        let frame =
            UOwnedTransportImpl::receive_validated_owned(self, source_filter, sink_filter).await?;
        Ok(frame)
    }

    async fn register_owned_listener(
        &self,
        source_filter: &UUri,
        sink_filter: Option<&UUri>,
        listener: Arc<dyn UOwnedListener>,
    ) -> Result<(), UStatus> {
        verify_owned_filter_criteria(source_filter, sink_filter)?;
        let key = owned_listener_registration_key(
            self,
            source_filter,
            sink_filter,
            owned_listener_pointer(&listener),
        );
        let (listener, inserted) = registered_owned_listener(&key, listener);
        let result = UOwnedTransportImpl::register_validated_owned_listener(
            self,
            source_filter,
            sink_filter,
            listener,
        )
        .await;
        if result.is_err() && inserted {
            OWNED_LISTENER_REGISTRY
                .lock()
                .expect("owned listener registry lock poisoned")
                .remove(&key);
        }
        result
    }

    async fn unregister_owned_listener(
        &self,
        source_filter: &UUri,
        sink_filter: Option<&UUri>,
        listener: Arc<dyn UOwnedListener>,
    ) -> Result<(), UStatus> {
        verify_owned_filter_criteria(source_filter, sink_filter)?;
        let key = owned_listener_registration_key(
            self,
            source_filter,
            sink_filter,
            owned_listener_pointer(&listener),
        );
        let listener = owned_listener_for_unregister(&key, listener);
        let result = UOwnedTransportImpl::unregister_validated_owned_listener(
            self,
            source_filter,
            sink_filter,
            listener,
        )
        .await;
        if result.is_ok() {
            OWNED_LISTENER_REGISTRY
                .lock()
                .expect("owned listener registry lock poisoned")
                .remove(&key);
        }
        result
    }
}

#[cfg(not(tarpaulin_include))]
#[cfg(all(feature = "owned-frame-transport", any(test, feature = "test-util")))]
mockall::mock! {
    /// Mockall-generated mock for test-util consumers.
    pub UOwnedTransport {
        /// Mock hook; configure with mockall expectations.
    ///
    /// # Errors
    ///
    /// Returns whatever the test's expectations are configured to return.
        pub async fn do_send_validated_owned(&self, frame: UOwnedFrame) -> Result<(), UStatus>;
        /// Mock hook; configure with mockall expectations.
    ///
    /// # Errors
    ///
    /// Returns whatever the test's expectations are configured to return.
        pub async fn do_receive_validated_owned<'a>(&'a self, source_filter: &'a UUri, sink_filter: Option<&'a UUri>) -> Result<UOwnedFrame, UStatus>;
        /// Mock hook; configure with mockall expectations.
    ///
    /// # Errors
    ///
    /// Returns whatever the test's expectations are configured to return.
        pub async fn do_register_validated_owned_listener<'a>(&'a self, source_filter: &'a UUri, sink_filter: Option<&'a UUri>, listener: Arc<dyn UOwnedListener>) -> Result<(), UStatus>;
        /// Mock hook; configure with mockall expectations.
    ///
    /// # Errors
    ///
    /// Returns whatever the test's expectations are configured to return.
        pub async fn do_unregister_validated_owned_listener<'a>(&'a self, source_filter: &'a UUri, sink_filter: Option<&'a UUri>, listener: Arc<dyn UOwnedListener>) -> Result<(), UStatus>;
    }
}

#[cfg(not(tarpaulin_include))]
#[cfg(all(feature = "owned-frame-transport", any(test, feature = "test-util")))]
#[async_trait]
impl UOwnedTransportImpl for MockUOwnedTransport {
    async fn send_validated_owned(&self, frame: UOwnedFrame) -> Result<(), UStatus> {
        self.do_send_validated_owned(frame).await
    }

    async fn receive_validated_owned(
        &self,
        source_filter: &UUri,
        sink_filter: Option<&UUri>,
    ) -> Result<UOwnedFrame, UStatus> {
        self.do_receive_validated_owned(source_filter, sink_filter)
            .await
    }

    async fn register_validated_owned_listener(
        &self,
        source_filter: &UUri,
        sink_filter: Option<&UUri>,
        listener: Arc<dyn UOwnedListener>,
    ) -> Result<(), UStatus> {
        self.do_register_validated_owned_listener(source_filter, sink_filter, listener)
            .await
    }

    async fn unregister_validated_owned_listener(
        &self,
        source_filter: &UUri,
        sink_filter: Option<&UUri>,
        listener: Arc<dyn UOwnedListener>,
    ) -> Result<(), UStatus> {
        self.do_unregister_validated_owned_listener(source_filter, sink_filter, listener)
            .await
    }
}

#[cfg(not(tarpaulin_include))]
#[cfg(any(test, feature = "test-util"))]
mockall::mock! {
    /// This extra struct is necessary in order to comply with mockall's requirements regarding the parameter lifetimes
    /// see <https://github.com/asomers/mockall/issues/571>
    pub Transport {
        /// Mock hook; configure with mockall expectations.
    ///
    /// # Errors
    ///
    /// Returns whatever the test's expectations are configured to return.
        pub async fn do_send(&self, message: UMessage) -> Result<(), UStatus>;
        /// Mock hook; configure with mockall expectations.
    ///
    /// # Errors
    ///
    /// Returns whatever the test's expectations are configured to return.
        pub async fn do_register_listener<'a>(&'a self, source_filter: &'a UUri, sink_filter: Option<&'a UUri>, listener: Arc<dyn UListener>) -> Result<(), UStatus>;
        /// Mock hook; configure with mockall expectations.
    ///
    /// # Errors
    ///
    /// Returns whatever the test's expectations are configured to return.
        pub async fn do_unregister_listener<'a>(&'a self, source_filter: &'a UUri, sink_filter: Option<&'a UUri>, listener: Arc<dyn UListener>) -> Result<(), UStatus>;
    }
}

#[cfg(not(tarpaulin_include))]
#[cfg(any(test, feature = "test-util"))]
#[async_trait]
/// This delegates the invocation of the UTransport functions to the mocked functions of the Transport struct.
/// see <https://github.com/asomers/mockall/issues/571>
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
    /// Creates a comparable wrapper around a listener for registry equality.
    pub fn new(listener: Arc<dyn UListener>) -> Self {
        Self { listener }
    }
    /// Gets a clone of the wrapped reference to the listener.
    #[must_use]
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
    use crate::{ComparableListener, UListener, UMessage, UMessageBuilder};
    use std::{
        hash::{DefaultHasher, Hash, Hasher},
        ops::Deref,
        str::FromStr,
        sync::Arc,
    };

    use super::*;

    #[test]
    fn test_static_uri_provider_get_source() {
        let provider = StaticUriProvider::new("my-vehicle", 0x4210, 0x05)
            .expect("failed to create URI provider");
        let source_uri = provider.get_source_uri();
        assert_eq!(source_uri.authority_name(), "my-vehicle");
        assert_eq!(source_uri.uentity_type_id(), 0x4210);
        assert_eq!(source_uri.uentity_major_version(), 0x05);
        assert_eq!(source_uri.resource_id(), 0x0000);
    }

    #[test]
    fn test_static_uri_provider_get_resource() {
        let provider = StaticUriProvider::new("my-vehicle", 0x4210, 0x05)
            .expect("failed to create URI provider");
        let resource_uri = provider.get_resource_uri(0x1234);
        assert_eq!(resource_uri.authority_name(), "my-vehicle");
        assert_eq!(resource_uri.uentity_type_id(), 0x4210);
        assert_eq!(resource_uri.uentity_major_version(), 0x05);
        assert_eq!(resource_uri.resource_id(), 0x1234);
    }

    #[tokio::test]
    async fn test_deref_returns_wrapped_listener() {
        let empty_message = UMessageBuilder::publish(
            UUri::try_from_parts("my-vehicle", 0x4210, 0x05, 0x9000)
                .expect("failed to create topic"),
        )
        .build()
        .expect("failed to build message");
        let mut mock_listener = MockUListener::new();
        mock_listener.expect_on_receive().once().return_const(());
        let listener_one = Arc::new(mock_listener);
        let comparable_listener_one = ComparableListener::new(listener_one);
        comparable_listener_one
            .deref()
            .on_receive(empty_message)
            .await;
    }

    #[tokio::test]
    async fn test_to_inner_returns_reference_to_wrapped_listener() {
        let empty_message = UMessageBuilder::publish(
            UUri::try_from_parts("my-vehicle", 0x4210, 0x05, 0x9000)
                .expect("failed to create topic"),
        )
        .build()
        .expect("failed to build message");
        let mut mock_listener = MockUListener::new();
        mock_listener.expect_on_receive().once().return_const(());
        let listener_one = Arc::new(mock_listener);
        let comparable_listener_one = ComparableListener::new(listener_one);
        comparable_listener_one
            .into_inner()
            .on_receive(empty_message)
            .await;
    }

    #[tokio::test]
    async fn test_eq_and_hash_are_consistent_for_comparable_listeners_wrapping_same_listener() {
        let empty_message = UMessageBuilder::publish(
            UUri::try_from_parts("my-vehicle", 0x4210, 0x05, 0x9000)
                .expect("failed to create topic"),
        )
        .build()
        .expect("failed to build message");
        let mut mock_listener = MockUListener::new();
        mock_listener.expect_on_receive().times(2).return_const(());
        let listener_one = Arc::new(mock_listener);
        let listener_two = listener_one.clone();
        listener_one.on_receive(empty_message.clone()).await;
        listener_two.on_receive(empty_message.clone()).await;
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
        let empty_message = UMessageBuilder::publish(
            UUri::try_from_parts("my-vehicle", 0x4210, 0x05, 0x9000)
                .expect("failed to create topic"),
        )
        .build()
        .expect("failed to build message");

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
        listener_one.on_receive(empty_message.clone()).await;
        listener_two.on_receive(empty_message.clone()).await;
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

    #[tokio::test]
    async fn test_utransport_default_implementations() {
        struct EmptyTransport {}
        #[async_trait::async_trait]
        impl UTransport for EmptyTransport {
            async fn send(&self, _message: UMessage) -> Result<(), UStatus> {
                todo!()
            }
        }

        let transport = EmptyTransport {};
        let listener = Arc::new(MockUListener::new());

        assert!(transport
            .receive(&UUri::any(), None)
            .await
            .is_err_and(|e| e.code() == UCode::Unimplemented));
        assert!(transport
            .register_listener(&UUri::any(), None, listener.clone())
            .await
            .is_err_and(|e| e.code() == UCode::Unimplemented));
        assert!(transport
            .unregister_listener(&UUri::any(), None, listener)
            .await
            .is_err_and(|e| e.code() == UCode::Unimplemented));
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

    #[test_case::test_case(
        "//vehicle1/AA/1/FFFF",
        Some("//vehicle2/BB/1/FFFF");
        "source and sink both having wildcard resource ID")]
    #[test_case::test_case(
        "//vehicle1/AA/1/9000",
        Some("//vehicle2/BB/1/0");
        "sending notification")]
    #[test_case::test_case(
        "//vehicle1/AA/1/0",
        Some("//vehicle2/BB/1/1");
        "RPC method invocation")]
    #[test_case::test_case(
        "//vehicle1/AA/1/FFFF",
        Some("//vehicle2/BB/1/1");
        "receiving RPC requests using wildcard resource ID")]
    #[test_case::test_case(
        "//vehicle1/AA/1/0",
        Some("//vehicle2/BB/1/1");
        "receiving RPC requests using default resource ID")]
    #[test_case::test_case(
        "//vehicle1/AA/1/9000",
        None;
        "receiving events published to specific topic")]
    #[test_case::test_case(
        "//vehicle1/AA/1/FFFF",
        None;
        "receiving events published to any topic")]
    fn test_verify_filter_criteria_succeeds_for(source: &str, sink: Option<&str>) {
        let source_filter = UUri::from_str(source).expect("invalid source URI");
        let sink_filter = sink.map(|s| UUri::from_str(s).expect("invalid sink URI"));
        assert!(verify_filter_criteria(&source_filter, sink_filter.as_ref()).is_ok());
    }

    #[test_case::test_case(
        UUri::from_str("//vehicle1/AA/1/0").unwrap(),
        Some(UUri::from_str("//vehicle2/BB/1/0").unwrap());
        "source and sink both having resource ID 0")]
    #[test_case::test_case(
        UUri::from_str("//vehicle1/AA/1/CC").unwrap(),
        Some(UUri::from_str("//vehicle2/BB/1/1A").unwrap());
        "sink is RPC but source has invalid resource ID")]
    #[test_case::test_case(
        UUri::from_str("//vehicle1/AA/1/CC").unwrap(),
        None;
        "sink is empty but source has non-topic resource ID")]
    fn test_verify_filter_criteria_fails_for(source_filter: UUri, sink_filter: Option<UUri>) {
        assert!(verify_filter_criteria(&source_filter, sink_filter.as_ref())
            .is_err_and(|err| matches!(err.code(), UCode::InvalidArgument)));
    }
}

#[cfg(all(test, feature = "owned-frame-transport"))]
mod owned_transport_tests {
    use std::str::FromStr;
    use std::sync::Mutex;

    use async_trait::async_trait;
    use bytes::Bytes;

    use super::*;
    use crate::{PayloadEncoding, UFrameMetadata, UMessageBuilder};

    fn topic() -> UUri {
        UUri::try_from_parts("vehicle", 0x4210, 0x01, 0x9000).expect("failed to create topic")
    }

    fn invalid_source_filter() -> UUri {
        UUri::from_str("//vehicle/4210/1/10").expect("invalid test URI")
    }

    fn metadata_with_encoding() -> UFrameMetadata {
        let message = UMessageBuilder::publish(topic()).build().expect("message");
        crate::frame::metadata::try_project_attributes_to_frame_metadata(
            message.attributes(),
            Some(PayloadEncoding::RAW),
        )
        .expect("metadata")
    }

    fn valid_frame() -> UOwnedFrame {
        UOwnedFrame::with_payload(metadata_with_encoding(), Bytes::new()).expect("valid frame")
    }

    #[tokio::test]
    async fn owned_transport_defaults_are_unimplemented() {
        struct EmptyTransport;

        #[async_trait]
        impl UOwnedTransportImpl for EmptyTransport {
            async fn send_validated_owned(&self, _frame: UOwnedFrame) -> Result<(), UStatus> {
                Ok(())
            }
        }

        let transport = EmptyTransport;
        let listener = Arc::new(MockUOwnedListener::new());
        let source = UUri::any();

        assert!(transport
            .receive_owned(&source, None)
            .await
            .is_err_and(|status| status.code() == UCode::Unimplemented));
        assert!(transport
            .register_owned_listener(&source, None, listener.clone())
            .await
            .is_err_and(|status| status.code() == UCode::Unimplemented));
        assert!(transport
            .unregister_owned_listener(&source, None, listener)
            .await
            .is_err_and(|status| status.code() == UCode::Unimplemented));
    }

    // Sending an unvalidated frame is a compile error (`send_owned` takes
    // `UOwnedFrame<Validated>`); the pin for that lives in the trybuild battery.

    #[tokio::test]
    async fn owned_transport_validates_filters_before_receive_impl() {
        #[derive(Default)]
        struct ReceiveCountingTransport {
            receive_count: Mutex<usize>,
        }

        #[async_trait]
        impl UOwnedTransportImpl for ReceiveCountingTransport {
            async fn send_validated_owned(&self, _frame: UOwnedFrame) -> Result<(), UStatus> {
                Ok(())
            }

            async fn receive_validated_owned(
                &self,
                _source_filter: &UUri,
                _sink_filter: Option<&UUri>,
            ) -> Result<UOwnedFrame, UStatus> {
                *self
                    .receive_count
                    .lock()
                    .expect("receive count lock poisoned") += 1;
                Ok(valid_frame())
            }
        }

        let transport = ReceiveCountingTransport::default();
        let status = transport
            .receive_owned(&invalid_source_filter(), None)
            .await
            .expect_err("invalid filters must be rejected");

        assert_eq!(status.code(), UCode::InvalidArgument);
        assert_eq!(*transport.receive_count.lock().unwrap(), 0);
    }

    #[tokio::test]
    async fn owned_transport_validates_filters_before_register_impl() {
        #[derive(Default)]
        struct RegisterCountingTransport {
            register_count: Mutex<usize>,
        }

        #[async_trait]
        impl UOwnedTransportImpl for RegisterCountingTransport {
            async fn send_validated_owned(&self, _frame: UOwnedFrame) -> Result<(), UStatus> {
                Ok(())
            }

            async fn register_validated_owned_listener(
                &self,
                _source_filter: &UUri,
                _sink_filter: Option<&UUri>,
                _listener: Arc<dyn UOwnedListener>,
            ) -> Result<(), UStatus> {
                *self
                    .register_count
                    .lock()
                    .expect("register count lock poisoned") += 1;
                Ok(())
            }
        }

        let transport = RegisterCountingTransport::default();
        let listener = Arc::new(MockUOwnedListener::new());
        let status = transport
            .register_owned_listener(&invalid_source_filter(), None, listener)
            .await
            .expect_err("invalid filters must be rejected");

        assert_eq!(status.code(), UCode::InvalidArgument);
        assert_eq!(*transport.register_count.lock().unwrap(), 0);
    }

    #[derive(Default)]
    struct CountingOwnedListener {
        count: Mutex<usize>,
    }

    impl CountingOwnedListener {
        fn count(&self) -> usize {
            *self
                .count
                .lock()
                .expect("owned listener count lock poisoned")
        }
    }

    #[async_trait]
    impl UOwnedListener for CountingOwnedListener {
        async fn on_receive_owned(&self, _frame: UOwnedFrame) {
            *self
                .count
                .lock()
                .expect("owned listener count lock poisoned") += 1;
        }
    }

    #[derive(Default)]
    struct ListenerSpyOwnedTransport {
        listener: Mutex<Option<Arc<dyn UOwnedListener>>>,
    }

    #[async_trait]
    impl UOwnedTransportImpl for ListenerSpyOwnedTransport {
        async fn send_validated_owned(&self, _frame: UOwnedFrame) -> Result<(), UStatus> {
            Ok(())
        }

        async fn register_validated_owned_listener(
            &self,
            _source_filter: &UUri,
            _sink_filter: Option<&UUri>,
            listener: Arc<dyn UOwnedListener>,
        ) -> Result<(), UStatus> {
            *self.listener.lock().expect("listener lock poisoned") = Some(listener);
            Ok(())
        }
    }

    #[tokio::test]
    async fn validating_owned_listener_drops_invalid_frames_before_user_callback() {
        let transport = ListenerSpyOwnedTransport::default();
        let listener = Arc::new(CountingOwnedListener::default());
        transport
            .register_owned_listener(&UUri::any(), None, listener.clone())
            .await
            .expect("listener registered");
        let registered_listener = transport
            .listener
            .lock()
            .expect("listener lock poisoned")
            .clone()
            .expect("implementation should receive validating listener");

        // Invalid-frame delivery is unrepresentable now: `on_receive_owned`
        // takes `UOwnedFrame<Validated>` by type.
        registered_listener.on_receive_owned(valid_frame()).await;
        assert_eq!(listener.count(), 1);
    }

    #[tokio::test]
    async fn mock_owned_transport_delegates_send() {
        let frame = valid_frame();
        let expected = frame.clone();
        let mut transport = MockUOwnedTransport::new();
        transport
            .expect_do_send_validated_owned()
            .once()
            .withf(move |actual| actual == &expected)
            .return_const(Ok(()));

        transport.send_owned(frame).await.unwrap();
    }
}
