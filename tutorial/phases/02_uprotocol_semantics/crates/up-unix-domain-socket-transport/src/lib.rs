// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Nirmalya Sengupta (https://github.com/nsengupta)

//! [`UnixDomainSocketTransport`] — a [`UTransport`] over a local Unix Domain Socket.
//!
//! Both processes use the **same** type:
//! - [`UnixDomainSocketTransport::bind`] — owns the socket path, accepts connections,
//!   dispatches to [`UListener`]s via [`UTransport::register_listener`].
//! - [`UnixDomainSocketTransport::connect`] — send-only attachment to that path
//!   ([`UTransport::send`]); listener registration is not available in this mode.
//!
//! Bind vs connect is wire setup for Unix Domain Sockets, not a Client/Server split
//! in the uProtocol application model. Both constructors return `Self`. The accept
//! loop shares the listener table (`Arc<RwLock<…>>`); wrapping the transport in
//! `Arc` is a call-site choice (`SimplePublisher` needs `Arc<T>`).

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::RwLock;

use up_frame_codec::serialize_for_unix_socket;
use up_rust::{
    ComparableListener, ProtobufMappable, UCode, UListener, UMessage, UStatus, UTransport, UUri,
    verify_filter_criteria,
};

#[derive(Eq, PartialEq, Hash)]
struct RegisteredListener {
    source_filter: UUri,
    sink_filter: Option<UUri>,
    listener: ComparableListener,
}

impl RegisteredListener {
    fn matches_msg_source(&self, msg: &UMessage) -> bool {
        if !self.source_filter.matches(msg.source()) {
            return false;
        }

        // Sink half of L1 filter matching (source already matched above).
        // Some(pattern): the message must carry a sink URI that matches the pattern
        // (RPC / notification).
        // None: only messages with no sink match — this demo registers PUBLISH listeners that way.
        if let Some(pattern) = &self.sink_filter {
            msg.sink()
                .is_some_and(|candidate| pattern.matches(candidate))
        } else {
            msg.sink().is_none()
        }
    }

    async fn on_receive(&self, msg: UMessage) {
        self.listener.on_receive(msg).await;
    }
}

/// L1 transport agent over a Unix Domain Socket (implements [`UTransport`]).
pub struct UnixDomainSocketTransport {
    socket_path: PathBuf,
    listeners: Arc<RwLock<HashSet<RegisteredListener>>>,
    /// `true` after [`Self::bind`] (accept loop running); `false` after [`Self::connect`].
    accepts_connections: bool,
}

impl UnixDomainSocketTransport {
    /// Attach for sending only: connect to `socket_path` on each [`UTransport::send`].
    pub fn connect(socket_path: impl AsRef<Path>) -> Self {
        Self {
            socket_path: socket_path.as_ref().to_path_buf(),
            listeners: Arc::new(RwLock::new(HashSet::new())),
            accepts_connections: false,
        }
    }

    /// Bind `socket_path` and spawn the accept/dispatch loop.
    ///
    /// The spawned loop clones the listener table, not this struct. Wrap the
    /// returned value in [`Arc`] if the call site needs a shared handle.
    ///
    /// Creates the parent directory when needed (see [`up_frame_codec::ensure_socket_dir`]).
    pub async fn bind(socket_path: impl AsRef<Path>) -> Result<Self, UStatus> {
        let socket_path = socket_path.as_ref().to_path_buf();
        if let Some(parent) = socket_path.parent() {
            std::fs::create_dir_all(parent).map_err(|err| {
                UStatus::fail_with_code(
                    UCode::Internal,
                    format!(
                        "failed to create socket directory {}: {err}",
                        parent.display()
                    ),
                )
            })?;
        }
        let _ = std::fs::remove_file(&socket_path);
        let listener = UnixListener::bind(&socket_path).map_err(|err| {
            UStatus::fail_with_code(
                UCode::Internal,
                format!(
                    "failed to bind Unix Domain Socket {}: {err}",
                    socket_path.display()
                ),
            )
        })?;

        let listeners = Arc::new(RwLock::new(HashSet::new()));
        let transport = Self {
            socket_path: socket_path.clone(),
            listeners: Arc::clone(&listeners),
            accepts_connections: true,
        };

        tokio::spawn(async move {
            loop {
                let Ok((stream, _)) = listener.accept().await else {
                    continue;
                };
                let listeners = Arc::clone(&listeners);
                tokio::spawn(async move {
                    if let Ok(message) = read_framed_message(stream).await {
                        dispatch_to(&listeners, message).await;
                    }
                });
            }
        });

        Ok(transport)
    }

    /// Socket path this transport is attached to.
    pub fn socket_path(&self) -> &Path {
        &self.socket_path
    }

    async fn send_framed(&self, message: UMessage) -> Result<(), UStatus> {
        let framed = serialize_for_unix_socket(&message).map_err(|err| {
            UStatus::fail_with_code(UCode::Internal, format!("failed to frame UMessage: {err}"))
        })?;

        let mut stream = UnixStream::connect(&self.socket_path)
            .await
            .map_err(|err| {
                UStatus::fail_with_code(
                    UCode::Unavailable,
                    format!(
                        "failed to connect to Unix Domain Socket {}: {err}",
                        self.socket_path.display()
                    ),
                )
            })?;

        stream.write_all(&framed).await.map_err(|err| {
            UStatus::fail_with_code(UCode::Internal, format!("failed to write message: {err}"))
        })?;
        stream.flush().await.map_err(|err| {
            UStatus::fail_with_code(UCode::Internal, format!("failed to flush stream: {err}"))
        })?;

        Ok(())
    }
}

#[async_trait]
impl UTransport for UnixDomainSocketTransport {
    async fn send(&self, message: UMessage) -> Result<(), UStatus> {
        self.send_framed(message).await
    }

    async fn register_listener(
        &self,
        source_filter: &UUri,
        sink_filter: Option<&UUri>,
        listener: Arc<dyn UListener>,
    ) -> Result<(), UStatus> {
        if !self.accepts_connections {
            return Err(UStatus::fail_with_code(
                UCode::Unimplemented,
                "register_listener requires UnixDomainSocketTransport::bind on the receiving side",
            ));
        }

        verify_filter_criteria(source_filter, sink_filter).map_err(|status| *status)?;

        let registered = RegisteredListener {
            source_filter: source_filter.to_owned(),
            sink_filter: sink_filter.map(|u| u.to_owned()),
            listener: ComparableListener::new(listener),
        };

        // Same filters and the same listener are one registration. HashSet::insert
        // leaves the existing entry in place, matching LocalTransport.
        let mut listeners = self.listeners.write().await;
        listeners.insert(registered);
        Ok(())
    }

    async fn unregister_listener(
        &self,
        source_filter: &UUri,
        sink_filter: Option<&UUri>,
        listener: Arc<dyn UListener>,
    ) -> Result<(), UStatus> {
        if !self.accepts_connections {
            return Err(UStatus::fail_with_code(
                UCode::Unimplemented,
                "unregister_listener requires UnixDomainSocketTransport::bind on the receiving side",
            ));
        }

        let registered = RegisteredListener {
            source_filter: source_filter.to_owned(),
            sink_filter: sink_filter.map(|u| u.to_owned()),
            listener: ComparableListener::new(listener),
        };

        let mut listeners = self.listeners.write().await;
        if listeners.remove(&registered) {
            Ok(())
        } else {
            Err(UStatus::fail_with_code(
                UCode::NotFound,
                "no such listener registered for filters",
            ))
        }
    }
}

async fn dispatch_to(listeners: &RwLock<HashSet<RegisteredListener>>, message: UMessage) {
    // Same lock scope as LocalTransport::dispatch: hold read() for the whole loop,
    // including on_receive.
    //
    // Why (PR): a snapshot that drops the guard before the loop let unregister
    // return Ok and then still start a later on_receive (L1 barrier broken).
    // Holding read() means unregister's write() waits until this dispatch finishes,
    // so no new callback starts after unregister returns.
    //
    // Trade-off: register_listener / unregister_listener from inside on_receive
    // deadlock (they need write()). Do those from main after a Notify, like
    // LocalTransport / the Phase 2 subscriber. UListener already asks on_receive
    // to return promptly.
    let table = listeners.read().await;
    for registered in table.iter() {
        if registered.matches_msg_source(&message) {
            registered.on_receive(message.clone()).await;
        }
    }
}

async fn read_framed_message(mut stream: UnixStream) -> Result<UMessage, UStatus> {
    let mut len_bytes = [0u8; 4];
    stream.read_exact(&mut len_bytes).await.map_err(|err| {
        UStatus::fail_with_code(
            UCode::Internal,
            format!("failed to read length prefix: {err}"),
        )
    })?;
    let body_len = u32::from_be_bytes(len_bytes) as usize;

    let mut body_bytes = vec![0u8; body_len];
    stream.read_exact(&mut body_bytes).await.map_err(|err| {
        UStatus::fail_with_code(
            UCode::Internal,
            format!("failed to read message body: {err}"),
        )
    })?;

    UMessage::parse_from_protobuf_bytes(&body_bytes).map_err(|err| {
        UStatus::fail_with_code(UCode::Internal, format!("failed to decode UMessage: {err}"))
    })
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;
    use tokio::sync::Notify;
    use up_rust::{LocalUriProvider, MockUListener, StaticUriProvider, UMessageBuilder};

    /// Counts deliveries so a repeated registration can be checked as one callback.
    struct CountingListener {
        hits: Arc<AtomicUsize>,
        done: Arc<Notify>,
    }

    #[async_trait]
    impl UListener for CountingListener {
        async fn on_receive(&self, _msg: UMessage) {
            self.hits.fetch_add(1, Ordering::SeqCst);
            self.done.notify_one();
        }
    }

    /// Parks in `on_receive` so the test can prove unregister waits for the read lock.
    struct ParkingListener {
        started: Arc<Notify>,
        proceed: Arc<Notify>,
    }

    #[async_trait]
    impl UListener for ParkingListener {
        async fn on_receive(&self, _msg: UMessage) {
            self.started.notify_one();
            self.proceed.notified().await;
        }
    }

    #[tokio::test]
    async fn send_dispatches_to_matching_listener() {
        let socket = std::env::temp_dir().join("uprotocol_unix_domain_socket_transport_test.sock");
        let _ = std::fs::remove_file(&socket);
        const RESOURCE_ID: u16 = 0x8001;
        let uri_provider = StaticUriProvider::new("my_own_car", 0x1010, 0x01).unwrap();

        let mut mock = MockUListener::new();
        mock.expect_on_receive().times(1).return_const(());
        let listener = Arc::new(mock);

        let server = UnixDomainSocketTransport::bind(&socket).await.unwrap();
        server
            .register_listener(&uri_provider.get_resource_uri(RESOURCE_ID), None, listener)
            .await
            .unwrap();

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let client = UnixDomainSocketTransport::connect(&socket);
        client
            .send(
                UMessageBuilder::publish(uri_provider.get_resource_uri(RESOURCE_ID))
                    .build()
                    .unwrap(),
            )
            .await
            .unwrap();

        let _ = std::fs::remove_file(&socket);
    }

    #[tokio::test]
    async fn register_listener_is_idempotent() {
        let socket =
            std::env::temp_dir().join("uprotocol_unix_domain_socket_transport_idempotent.sock");
        let _ = std::fs::remove_file(&socket);
        const RESOURCE_ID: u16 = 0x8001;
        let uri_provider = StaticUriProvider::new("my_own_car", 0x1010, 0x01).unwrap();
        let topic = uri_provider.get_resource_uri(RESOURCE_ID);

        let hits = Arc::new(AtomicUsize::new(0));
        let done = Arc::new(Notify::new());
        let listener = Arc::new(CountingListener {
            hits: Arc::clone(&hits),
            done: Arc::clone(&done),
        });

        let server = UnixDomainSocketTransport::bind(&socket).await.unwrap();
        server
            .register_listener(&topic, None, listener.clone())
            .await
            .unwrap();
        server
            .register_listener(&topic, None, listener.clone())
            .await
            .unwrap();

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let client = UnixDomainSocketTransport::connect(&socket);
        client
            .send(UMessageBuilder::publish(topic.clone()).build().unwrap())
            .await
            .unwrap();

        tokio::time::timeout(std::time::Duration::from_secs(2), done.notified())
            .await
            .expect("registered listener did not receive the publish");

        // dispatch_to awaits each match in order, so a duplicate entry would
        // already have incremented `hits` before this wait returns.
        assert_eq!(hits.load(Ordering::SeqCst), 1);

        server
            .unregister_listener(&topic, None, listener)
            .await
            .unwrap();

        let _ = std::fs::remove_file(&socket);
    }

    // Was `dispatch_releases_read_lock_before_on_receive`: that test required
    // register_listener from inside on_receive (write while dispatch held no lock).
    // Dispatch now matches LocalTransport and keeps read() across on_receive, so
    // that pattern deadlocks. This test instead shows the L1 trade-off we chose:
    // unregister's write() does not return until the in-flight callback finishes.
    #[tokio::test]
    async fn unregister_waits_for_in_flight_on_receive() {
        let socket =
            std::env::temp_dir().join("uprotocol_unix_domain_socket_transport_lock_test.sock");
        let _ = std::fs::remove_file(&socket);
        const RESOURCE_ID: u16 = 0x8001;
        let uri_provider = StaticUriProvider::new("my_own_car", 0x1010, 0x01).unwrap();
        let source_filter = uri_provider.get_resource_uri(RESOURCE_ID);

        let transport = Arc::new(UnixDomainSocketTransport::bind(&socket).await.unwrap());
        let started = Arc::new(Notify::new());
        let proceed = Arc::new(Notify::new());
        let listener = Arc::new(ParkingListener {
            started: Arc::clone(&started),
            proceed: Arc::clone(&proceed),
        });

        transport
            .register_listener(&source_filter, None, listener.clone())
            .await
            .unwrap();

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let waiting_to_start = started.notified();
        let client = UnixDomainSocketTransport::connect(&socket);
        client
            .send(
                UMessageBuilder::publish(source_filter.clone())
                    .build()
                    .unwrap(),
            )
            .await
            .unwrap();

        tokio::time::timeout(std::time::Duration::from_secs(2), waiting_to_start)
            .await
            .expect("on_receive did not start");

        let unregister = tokio::spawn({
            let transport = Arc::clone(&transport);
            let source_filter = source_filter.clone();
            async move {
                transport
                    .unregister_listener(&source_filter, None, listener)
                    .await
            }
        });

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        assert!(
            !unregister.is_finished(),
            "unregister must not return while dispatch still holds read() in on_receive"
        );

        proceed.notify_one();
        unregister
            .await
            .expect("unregister task panicked")
            .expect("unregister after on_receive must succeed");

        let _ = std::fs::remove_file(&socket);
    }
}
