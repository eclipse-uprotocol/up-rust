/********************************************************************************
 * Copyright (c) 2026 Contributors to the Eclipse Foundation
 *
 * SPDX-License-Identifier: Apache-2.0
 ********************************************************************************/

use std::sync::Arc;

use async_trait::async_trait;

use crate::zero_copy::rx::UZeroCopyRxLease;
use crate::zero_copy::tx::{UTxBuffer, UTxLoanSpec, UUninitTxBuffer};
use crate::{UStatus, UUri};

/// Listener for validated zero-copy receive leases.
#[async_trait]
pub trait UZeroCopyListener<Rx>: Send + Sync
where
    Rx: UZeroCopyRxLease + Send + 'static,
{
    /// Handles one validated receive lease.
    async fn on_receive_zero_copy(&self, frame: Rx);
}

/// Family-neutral implementation boundary for zero-copy transports.
#[async_trait]
pub trait UZeroCopyTransportImpl: Send + Sync {
    /// Initialized transmit loan.
    type Tx: UTxBuffer + Send;
    /// Validated receive lease.
    type Rx: UZeroCopyRxLease + Send + 'static;

    /// Loans an initialized transmit buffer for a validated request.
    async fn loan_validated_tx(&self, spec: UTxLoanSpec) -> Result<Self::Tx, UStatus>;
    /// Sends an initialized transmit loan.
    async fn send_validated_zero_copy(&self, buffer: Self::Tx) -> Result<(), UStatus>;
    /// Receives one validated matching lease.
    async fn receive_validated_zero_copy(
        &self,
        source_filter: &UUri,
        sink_filter: Option<&UUri>,
    ) -> Result<Self::Rx, UStatus>;
    /// Registers a validated listener.
    async fn register_validated_zero_copy_listener(
        &self,
        source_filter: &UUri,
        sink_filter: Option<&UUri>,
        listener: Arc<dyn UZeroCopyListener<Self::Rx>>,
    ) -> Result<(), UStatus>;
    /// Unregisters a validated listener.
    async fn unregister_validated_zero_copy_listener(
        &self,
        source_filter: &UUri,
        sink_filter: Option<&UUri>,
        listener: Arc<dyn UZeroCopyListener<Self::Rx>>,
    ) -> Result<(), UStatus>;
}

/// Public marker for a validated zero-copy transport implementation.
pub trait UZeroCopyTransport: UZeroCopyTransportImpl {}

impl<T> UZeroCopyTransport for T where T: UZeroCopyTransportImpl {}

/// Optional implementation boundary for uninitialized transmit loans.
#[async_trait]
pub trait UZeroCopyUninitTransportImpl: UZeroCopyTransportImpl {
    /// Uninitialized loan type.
    type UninitTx: UUninitTxBuffer<Initialized = Self::Tx> + Send;

    /// Loans uninitialized payload storage for a validated request.
    async fn loan_validated_uninit_tx(&self, spec: UTxLoanSpec) -> Result<Self::UninitTx, UStatus>;
}

/// Extension methods shared by zero-copy implementations.
#[async_trait]
pub trait UZeroCopyTransportExt: UZeroCopyTransportImpl {
    /// Loans and sends a payload initialized by `initialize`.
    async fn send_loaned_payload<F>(&self, spec: UTxLoanSpec, initialize: F) -> Result<(), UStatus>
    where
        F: FnOnce(&mut [u8]) + Send,
    {
        let mut buffer = self.loan_validated_tx(spec).await?;
        initialize(buffer.payload_mut());
        self.send_validated_zero_copy(buffer).await
    }
}

impl<T> UZeroCopyTransportExt for T where T: UZeroCopyTransportImpl {}
