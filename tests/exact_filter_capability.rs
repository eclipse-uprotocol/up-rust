/********************************************************************************
 * Copyright (c) 2026 Contributors to the Eclipse Foundation
 *
 * SPDX-License-Identifier: Apache-2.0
 ********************************************************************************/

//! Runtime coverage for the exact-URI filter capability types.

use up_rust::{ExactUUri, UUri};

fn accepts_exact_source(source: &ExactUUri) -> &UUri {
    source.as_uuri()
}

#[test]
fn exact_uuri_accepts_uri_without_wildcards() {
    let uri = UUri::try_from_parts("vehicle-a", 0x4210, 0x01, 0x9000).unwrap();

    let exact = ExactUUri::try_from(uri.clone()).unwrap();

    assert_eq!(accepts_exact_source(&exact), &uri);
    assert_eq!(UUri::from(exact), uri);
}

#[test]
fn exact_uuri_rejects_wildcard_components() {
    let wildcard_authority = UUri::try_from_parts("*", 0x4210, 0x01, 0x9000).unwrap();
    let wildcard_resource = UUri::try_from_parts("vehicle-a", 0x4210, 0x01, 0xFFFF).unwrap();

    assert!(ExactUUri::try_from(wildcard_authority).is_err());
    assert!(ExactUUri::try_from(wildcard_resource).is_err());
}

#[test]
fn exact_uuri_borrows_like_uuri_after_proof_construction() {
    let uri = UUri::try_from_parts("vehicle-a", 0x4210, 0x01, 0x9000).unwrap();
    let exact = ExactUUri::try_from(&uri).unwrap();

    assert_eq!(exact.as_uuri().to_uri(true), "up://vehicle-a/4210/1/9000");
    assert_eq!(exact.resource_id(), 0x9000);
}
