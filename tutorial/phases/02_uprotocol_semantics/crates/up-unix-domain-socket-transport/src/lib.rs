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

#[derive(Clone, Eq, PartialEq, Hash)]
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

        let mut listeners = self.listeners.write().await;
        if listeners.contains(&registered) {
            return Err(UStatus::fail_with_code(
                UCode::AlreadyExists,
                "listener already registered for filters",
            ));
        }
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
    // Snapshot matches under the read lock, then drop it before on_receive so a
    // slow listener does not stall register_listener / unregister_listener.
    let matching: Vec<RegisteredListener> = {
        let table = listeners.read().await;
        table
            .iter()
            .filter(|registered| registered.matches_msg_source(&message))
            .cloned()
            .collect()
    };

    for registered in matching {
        registered.on_receive(message.clone()).await;
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
    use super::*;
    use tokio::sync::Notify;
    use up_rust::{LocalUriProvider, MockUListener, StaticUriProvider, UMessageBuilder};

    struct RegisterDuringReceive {
        transport: Arc<UnixDomainSocketTransport>,
        source_filter: UUri,
        extra: Arc<dyn UListener>,
        done: Arc<Notify>,
    }

    #[async_trait]
    impl UListener for RegisterDuringReceive {
        async fn on_receive(&self, _msg: UMessage) {
            self.transport
                .register_listener(&self.source_filter, None, Arc::clone(&self.extra))
                .await
                .expect("register_listener must not wait on this on_receive");
            self.done.notify_one();
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
    async fn dispatch_releases_read_lock_before_on_receive() {
        let socket =
            std::env::temp_dir().join("uprotocol_unix_domain_socket_transport_lock_test.sock");
        let _ = std::fs::remove_file(&socket);
        const RESOURCE_ID: u16 = 0x8001;
        let uri_provider = StaticUriProvider::new("my_own_car", 0x1010, 0x01).unwrap();
        let source_filter = uri_provider.get_resource_uri(RESOURCE_ID);

        let transport = Arc::new(UnixDomainSocketTransport::bind(&socket).await.unwrap());
        let extra = {
            let mut mock = MockUListener::new();
            mock.expect_on_receive().times(0).return_const(());
            Arc::new(mock)
        };
        let done = Arc::new(Notify::new());
        let listener = Arc::new(RegisterDuringReceive {
            transport: Arc::clone(&transport),
            source_filter: source_filter.clone(),
            extra,
            done: Arc::clone(&done),
        });

        transport
            .register_listener(&source_filter, None, listener)
            .await
            .unwrap();

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let client = UnixDomainSocketTransport::connect(&socket);
        client
            .send(
                UMessageBuilder::publish(source_filter.clone())
                    .build()
                    .unwrap(),
            )
            .await
            .unwrap();

        tokio::time::timeout(std::time::Duration::from_secs(2), done.notified())
            .await
            .expect("on_receive deadlocked: read lock still held across callback");

        let _ = std::fs::remove_file(&socket);
    }
}
