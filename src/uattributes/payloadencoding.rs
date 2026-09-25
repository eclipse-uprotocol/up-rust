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

//! Compact payload-representation identity, independent of transport and schema.
//!
//! Every 16-bit value is structurally representable. Registry allocation policy
//! and application decoding support are separate from numeric validity. Unknown
//! and reserved values can therefore be preserved by older intermediaries.
//!
//! Absence of an encoding is represented by `None`, never by zero. A present
//! [`PayloadEncoding::IMPLICIT`] means a binding or service contract defines the
//! representation. Neither that contract nor a native layout is inferred from
//! payload bytes or a media type.
//!
//! See the [registry](https://github.com/eclipse-uprotocol/up-spec/blob/main/basics/payload_encoding_registry.adoc).

use crate::uattributes::UAttributesError;

/// A structurally valid 16-bit payload-encoding identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(transparent)]
pub struct PayloadEncoding(u16);

/// First private-use identifier; allocations require deployment agreement.
pub const PAYLOAD_ENCODING_PRIVATE_USE_MIN: u32 = 0xF000;

impl PayloadEncoding {
    /// An applicable binding/service contract supplies the encoding.
    pub const IMPLICIT: Self = Self(0);
    /// Serialized `google.protobuf.Any` (registry entry 1).
    pub const PROTOBUF_WRAPPED_IN_ANY: Self = Self(1);
    /// Unwrapped Protobuf message (registry entry 2).
    pub const PROTOBUF: Self = Self(2);
    /// JSON (registry entry 3).
    pub const JSON: Self = Self(3);
    /// SOME/IP payload serialization (registry entry 4).
    pub const SOMEIP: Self = Self(4);
    /// SOME/IP TLV payload serialization (registry entry 5).
    pub const SOMEIP_TLV: Self = Self(5);
    /// Arbitrary bytes (registry entry 6).
    pub const RAW: Self = Self(6);
    /// UTF-8 text (registry entry 7).
    pub const TEXT: Self = Self(7);

    /// Accepts any identifier within the 16-bit field's structural range.
    ///
    /// This does not authorize assigning an unallocated identifier or prove
    /// that a particular endpoint understands its payload.
    ///
    /// # Errors
    /// Returns an error when `id` is greater than `65535`.
    pub fn from_id(id: u32) -> Result<Self, UAttributesError> {
        u16::try_from(id).map(Self).map_err(|_| {
            UAttributesError::validation_error("payload-encoding identifier exceeds 65535")
        })
    }

    /// Declares an identifier in the registry-assignment range.
    ///
    /// The caller is responsible for its registry assignment; this checks the
    /// range, not current registration or decoder support. Use [`Self::private_use`]
    /// for a private contract and [`Self::from_id`] for structural carriage.
    ///
    /// # Panics
    /// Panics when `id` is outside `0x0001..=0xDFFF`.
    #[must_use]
    pub const fn from_registry_entry(id: u32) -> Self {
        assert!(
            id >= 1 && id <= 0xDFFF,
            "payload-encoding identifier is outside the registry-assignment range"
        );
        Self(id as u16)
    }

    /// Declares a private-use encoding under an explicit contract.
    ///
    /// A fixed library profile can define that contract; interpreting peers must
    /// agree to it. This neither creates a public registry assignment nor changes
    /// structural acceptance of unknown identifiers.
    ///
    /// # Panics
    /// Panics when `id` is outside `0xF000..=0xFFFF`.
    #[must_use]
    pub const fn private_use(id: u32) -> Self {
        assert!(
            id >= PAYLOAD_ENCODING_PRIVATE_USE_MIN && id <= u16::MAX as u32,
            "payload-encoding identifier is outside the private-use range"
        );
        Self(id as u16)
    }

    /// Returns the identifier in the protobuf/native metadata's u32 storage form.
    ///
    /// The conceptual range remains 16 bits; retaining this storage accessor
    /// avoids silently shrinking an existing four-byte metadata slot.
    #[must_use]
    pub const fn id(self) -> u32 {
        self.0 as u32
    }

    /// Whether the identifier is in the deployment-private allocation range.
    #[must_use]
    pub const fn is_private_use(self) -> bool {
        self.id() >= PAYLOAD_ENCODING_PRIVATE_USE_MIN
    }

    /// Registered media metadata, where one accurately describes the encoding.
    ///
    /// Both protobuf identities have the same media type. This metadata is not
    /// a replacement for the encoding ID or permission to interpret the bytes.
    #[must_use]
    pub const fn media_type(self) -> Option<&'static str> {
        match self.0 {
            1 | 2 => Some("application/protobuf"),
            3 => Some("application/json"),
            6 => Some("application/octet-stream"),
            7 => Some("text/plain; charset=utf-8"),
            _ => None,
        }
    }

    /// Resolves media metadata only where it identifies an unambiguous entry.
    ///
    /// # Errors
    /// Protobuf media types cannot distinguish a serialized Any from an
    /// unwrapped message and are rejected. Unregistered/unknown media types and
    /// a conflicting text charset are also rejected. Transport bindings must
    /// preserve the explicit encoding identifier rather than use this lookup.
    pub fn from_media_type(media_type: &str) -> Result<Self, UAttributesError> {
        let mut parts = media_type.split(';');
        let base = parts.next().unwrap_or("").trim();
        if base.eq_ignore_ascii_case("application/json") {
            return Ok(Self::JSON);
        }
        if base.eq_ignore_ascii_case("application/octet-stream") {
            return Ok(Self::RAW);
        }
        if base.eq_ignore_ascii_case("text/plain") {
            for parameter in parts {
                if let Some((name, value)) = parameter.split_once('=') {
                    if name.trim().eq_ignore_ascii_case("charset")
                        && !value.trim().trim_matches('"').eq_ignore_ascii_case("utf-8")
                    {
                        return Err(UAttributesError::validation_error(
                            "text encoding requires UTF-8",
                        ));
                    }
                }
            }
            return Ok(Self::TEXT);
        }
        Err(UAttributesError::validation_error(format!(
            "media type [{media_type}] does not identify an unambiguous payload encoding"
        )))
    }
}

impl core::fmt::Display for PayloadEncoding {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self.media_type() {
            Some(media) => write!(f, "{} ({media})", self.0),
            None if *self == Self::IMPLICIT => write!(f, "0 (contract-defined)"),
            None if self.is_private_use() => write!(f, "{} (private use)", self.0),
            None => write!(f, "{}", self.0),
        }
    }
}

impl TryFrom<u32> for PayloadEncoding {
    type Error = UAttributesError;

    fn try_from(id: u32) -> Result<Self, Self::Error> {
        Self::from_id(id)
    }
}

impl From<PayloadEncoding> for u32 {
    fn from(value: PayloadEncoding) -> Self {
        value.id()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_case::test_case;

    #[test_case(0; "contract defined zero")]
    #[test_case(1; "first registered")]
    #[test_case(7; "last currently registered")]
    #[test_case(8; "retired label remains unassigned")]
    #[test_case(9; "unassigned")]
    #[test_case(0xDFFF; "assignment range upper bound")]
    #[test_case(0xE000; "reserved lower bound")]
    #[test_case(0xEFFF; "reserved upper bound")]
    #[test_case(0xF000; "private lower bound")]
    #[test_case(0xFFFF; "private upper bound")]
    fn identifier_is_structurally_valid(id: u32) {
        assert_eq!(PayloadEncoding::from_id(id).unwrap().id(), id);
    }

    #[test_case(1; "first registry value")]
    #[test_case(0xDFFF; "last registry value")]
    fn registry_constructor_checks_the_assignment_range(id: u32) {
        assert_eq!(PayloadEncoding::from_registry_entry(id).id(), id);
    }

    #[test_case(0; "contract-defined is not registry-assigned")]
    #[test_case(0xE000; "reserved is not registry-assigned")]
    #[test_case(0xF001; "private is not registry-assigned")]
    #[test_case(65536; "oversized is not registry-assigned")]
    #[should_panic(expected = "outside the registry-assignment range")]
    fn registry_constructor_rejects_other_allocation_classes(id: u32) {
        let _ = PayloadEncoding::from_registry_entry(id);
    }

    #[test_case(0xF000; "first private value")]
    #[test_case(0xFFFF; "last private value")]
    fn private_constructor_accepts_contract_identifiers(id: u32) {
        assert_eq!(PayloadEncoding::private_use(id).id(), id);
    }

    #[test_case(0; "contract-defined is not private")]
    #[test_case(2; "registered is not private")]
    #[test_case(0xEFFF; "reserved is not private")]
    #[test_case(65536; "oversized is not private")]
    #[should_panic(expected = "outside the private-use range")]
    fn private_constructor_rejects_other_allocation_classes(id: u32) {
        let _ = PayloadEncoding::private_use(id);
    }

    #[test_case(65536; "just above the field range")]
    #[test_case(u32::MAX; "maximum protobuf integer")]
    fn identifier_outside_field_range_is_rejected(id: u32) {
        assert!(matches!(
            PayloadEncoding::from_id(id),
            Err(UAttributesError::ValidationError(_))
        ));
    }

    #[test]
    fn explicit_contract_encoding_is_not_absence() {
        assert_ne!(Some(PayloadEncoding::IMPLICIT), None);
    }

    #[test_case(0xEFFF => false; "reserved is not private")]
    #[test_case(0xF000 => true; "private lower bound")]
    #[test_case(0xFFFF => true; "private upper bound")]
    fn private_range_classification(id: u32) -> bool {
        PayloadEncoding::from_id(id).unwrap().is_private_use()
    }

    #[test_case(PayloadEncoding::from_registry_entry(8); "unassigned encoding")]
    #[test_case(PayloadEncoding::SOMEIP; "encoding without registered media type")]
    fn missing_media_metadata_does_not_make_an_identifier_invalid(encoding: PayloadEncoding) {
        assert_eq!(PayloadEncoding::from_id(encoding.id()).unwrap(), encoding);
        assert_eq!(encoding.media_type(), None);
    }

    #[test]
    fn media_type_cannot_recover_protobuf_wrapper_identity() {
        assert_eq!(
            PayloadEncoding::PROTOBUF.media_type(),
            PayloadEncoding::PROTOBUF_WRAPPED_IN_ANY.media_type()
        );
    }

    #[test_case("application/protobuf"; "ambiguous protobuf representation")]
    #[test_case("application/x-protobuf"; "retired protobuf alias")]
    #[test_case("text/plain; charset=iso-8859-1"; "non UTF8 charset")]
    fn ambiguous_or_incompatible_media_type_is_rejected(media_type: &str) {
        assert!(PayloadEncoding::from_media_type(media_type).is_err());
    }

    #[test_case("text/plain; charset=UTF-8", PayloadEncoding::TEXT; "explicit UTF8 text")]
    #[test_case("application/json", PayloadEncoding::JSON; "JSON")]
    fn unambiguous_media_type_resolves(media_type: &str, expected: PayloadEncoding) {
        assert_eq!(
            PayloadEncoding::from_media_type(media_type).unwrap(),
            expected
        );
    }
}
