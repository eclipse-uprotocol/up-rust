// Copyright (c) 2026 Contributors to the Eclipse Foundation
// SPDX-License-Identifier: Apache-2.0

use std::{
    future::{poll_fn, Future},
    sync::atomic::{AtomicBool, Ordering},
    task::Poll,
};
use tokio::sync::Mutex;

/// Admission gate for one asynchronous listener registration.
///
/// Available with the opt-in `util` feature. Transport workers keep their own
/// ordering, queue bounds and shutdown policy. This gate only serializes actual
/// callback entry with unregister. An entered callback may finish, including
/// unregistering itself; queued callbacks cannot enter after [`Self::stop`].
/// Each new registration needs a new gate; stopped gates are never reopened.
#[derive(Debug)]
pub struct ListenerAdmission {
    active: AtomicBool,
    entry: Mutex<()>,
}

impl Default for ListenerAdmission {
    fn default() -> Self {
        Self::new()
    }
}

impl ListenerAdmission {
    /// Creates an active gate for a new registration.
    #[must_use]
    pub fn new() -> Self {
        Self {
            active: AtomicBool::new(true),
            entry: Mutex::new(()),
        }
    }

    /// Cancels pending admission without waiting for a racing first poll.
    ///
    /// Intended for synchronous destruction alongside worker cancellation.
    /// Successful asynchronous unregister must use [`Self::stop`] instead.
    pub fn cancel(&self) {
        self.active.store(false, Ordering::Release);
    }

    /// Closes admission, serialized with a callback's first poll.
    ///
    /// This is an admission boundary, not a callback-draining operation. Returning
    /// from `stop` does not mean that previously entered callbacks have completed.
    /// Callbacks that have already yielded may finish. As with other async code,
    /// their first poll must yield promptly rather than block the executor.
    pub async fn stop(&self) {
        let _entry = self.entry.lock().await;
        self.cancel();
    }

    /// Enters a callback if this registration is still active.
    ///
    /// Future construction alone is not callback entry. The first poll is
    /// serialized with [`Self::stop`], then the gate is released so a callback
    /// can finish or unregister itself. A rejected callback is never constructed.
    pub async fn dispatch<F: Future<Output = ()>>(&self, callback: impl FnOnce() -> F) {
        let entry = self.entry.lock().await;
        if !self.active.load(Ordering::Acquire) {
            return;
        }
        let mut callback = std::pin::pin!(callback());
        let pending = poll_fn(|cx| Poll::Ready(callback.as_mut().poll(cx).is_pending())).await;
        drop(entry);
        if pending {
            callback.await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{sync::Arc, time::Duration};

    #[tokio::test]
    async fn stopped_registration_never_constructs_a_callback() {
        let admission = ListenerAdmission::new();
        admission.stop().await;
        admission
            .dispatch(|| -> std::future::Ready<()> {
                panic!("callback constructed after unregister")
            })
            .await;
    }

    #[tokio::test]
    async fn callback_can_unregister_itself() {
        let admission = ListenerAdmission::new();
        tokio::time::timeout(
            Duration::from_secs(5),
            admission.dispatch(|| async {
                admission.stop().await;
            }),
        )
        .await
        .expect("self-unregister must not hold admission across a yield");
    }

    #[tokio::test]
    async fn queued_callback_cannot_enter_after_unregister() {
        let admission = Arc::new(ListenerAdmission::new());
        let ready = Arc::new(tokio::sync::Notify::new());
        let task = tokio::spawn({
            let admission = Arc::clone(&admission);
            let ready = Arc::clone(&ready);
            async move {
                ready.notified().await;
                admission
                    .dispatch(|| async { panic!("queued callback entered") })
                    .await;
            }
        });
        admission.stop().await;
        ready.notify_one();
        tokio::time::timeout(Duration::from_secs(5), task)
            .await
            .unwrap()
            .unwrap();
    }

    #[tokio::test]
    async fn stop_returns_while_an_entered_callback_is_pending() {
        let admission = Arc::new(ListenerAdmission::new());
        let (entered, entry) = tokio::sync::oneshot::channel();
        let (release, completion) = tokio::sync::oneshot::channel();
        let delivery = tokio::spawn({
            let admission = admission.clone();
            async move {
                admission
                    .dispatch(|| async move {
                        entered.send(()).unwrap();
                        completion.await.expect("release entered callback");
                    })
                    .await;
            }
        });
        tokio::time::timeout(Duration::from_secs(5), entry)
            .await
            .unwrap()
            .unwrap();
        tokio::time::timeout(Duration::from_secs(5), admission.stop())
            .await
            .unwrap();
        assert!(
            !delivery.is_finished(),
            "stop must not imply callback completion"
        );
        admission
            .dispatch(|| async { panic!("new entry after stop") })
            .await;
        release.send(()).unwrap();
        tokio::time::timeout(Duration::from_secs(5), delivery)
            .await
            .unwrap()
            .unwrap();
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn unregister_waits_for_the_callbacks_first_poll_to_yield() {
        let admission = Arc::new(ListenerAdmission::new());
        let entered = Arc::new(tokio::sync::Notify::new());
        let (release, wait) = std::sync::mpsc::channel();
        let delivery = tokio::spawn({
            let admission = Arc::clone(&admission);
            let entered = Arc::clone(&entered);
            async move {
                admission
                    .dispatch(|| async move {
                        entered.notify_one();
                        wait.recv_timeout(Duration::from_secs(5))
                            .expect("release first poll");
                        std::future::pending::<()>().await;
                    })
                    .await;
            }
        });
        tokio::time::timeout(Duration::from_secs(5), entered.notified())
            .await
            .unwrap();
        let mut stop = std::pin::pin!(admission.stop());
        assert!(poll_fn(|cx| Poll::Ready(stop.as_mut().poll(cx)))
            .await
            .is_pending());
        release.send(()).unwrap();
        tokio::time::timeout(Duration::from_secs(5), stop)
            .await
            .unwrap();
        delivery.abort();
        assert!(delivery.await.unwrap_err().is_cancelled());
    }
}
