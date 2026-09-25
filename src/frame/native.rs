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

//! Scoped native representation agreement.
//!
//! A structural type token is independent of a payload encoding allocation.
//! Deployments explicitly select either a private-ID table or a typed operation
//! contract using ID zero. Neither mode has an implicit fallback to the other.
//! Immutable route agreements retain their complete generation while frames are
//! in flight. Load matching peer configurations before activating a new route;
//! changing a table requires a new version, with old routes drained separately.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use sha2::{Digest, Sha256};

use crate::PayloadEncoding;

/// A versioned structural native type token, not a registry encoding ID.
///
/// All 32-bit values, including zero, can be carried opaquely. Equality alone
/// does not authorize a cast: profile membership, layout, bit validity and loan
/// lifetime/provenance must also be established.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct NativeTypeToken(u32);

impl NativeTypeToken {
    /// Preserves a token received from metadata without interpreting its payload.
    pub const fn from_u32(value: u32) -> Self {
        Self(value)
    }

    /// Returns the token's complete 32-bit value.
    pub const fn as_u32(self) -> u32 {
        self.0
    }
}

/// Byte order of a native representation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NativeByteOrder {
    /// Little-endian scalars.
    LittleEndian,
    /// Big-endian scalars.
    BigEndian,
}

impl NativeByteOrder {
    /// Returns the current target's scalar byte order.
    pub const fn native() -> Self {
        if cfg!(target_endian = "little") {
            Self::LittleEndian
        } else {
            Self::BigEndian
        }
    }
}

/// One named field (or repeated array element) in a native representation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeField {
    name: String,
    offset: u64,
    count: u64,
    representation: NativeRepresentation,
}

impl NativeField {
    /// Describes a field at a byte offset. Its enclosing representation checks
    /// bounds, alignment, overlap and duplicate names.
    pub fn new(name: impl Into<String>, offset: u64, representation: NativeRepresentation) -> Self {
        Self {
            name: name.into(),
            offset,
            count: 1,
            representation,
        }
    }

    /// Describes repeated elements, including a zero-length array.
    pub fn repeated(
        name: impl Into<String>,
        offset: u64,
        count: u64,
        representation: NativeRepresentation,
    ) -> Self {
        Self {
            name: name.into(),
            offset,
            count,
            representation,
        }
    }
}

/// Complete structural description of a native value in one target ABI.
///
/// Names are deployment-stable cross-language names, not Rust compiler type
/// names. Nested fields include offsets, counts and their own representations;
/// equal aggregate size/alignment is insufficient to establish type identity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeRepresentation {
    name: String,
    revision: u32,
    size: u64,
    alignment: u64,
    byte_order: NativeByteOrder,
    fields: Vec<NativeField>,
}

impl NativeRepresentation {
    /// Checks and constructs a scalar, struct or array representation.
    ///
    /// Scalar representations have no fields and use distinct names for distinct
    /// semantics, for example `i32`, `u32`, `f32`, `bool` and `char`.
    ///
    /// # Errors
    ///
    /// Rejects empty/duplicate names, invalid alignment, overflowing/out-of-bounds
    /// fields and overlapping nonempty fields. This describes layout; it does
    /// not prove that arbitrary bytes are a valid value of that type.
    pub fn new(
        name: impl Into<String>,
        revision: u32,
        size: u64,
        alignment: u64,
        byte_order: NativeByteOrder,
        fields: Vec<NativeField>,
    ) -> Result<Self, NativeProfileError> {
        let name = name.into();
        if name.is_empty() || !alignment.is_power_of_two() || !size.is_multiple_of(alignment) {
            return Err(NativeProfileError::InvalidDescriptor(
                "invalid name, size or alignment",
            ));
        }
        let mut names = BTreeSet::new();
        let mut ranges = Vec::new();
        for field in &fields {
            if field.name.is_empty() || !names.insert(&field.name) {
                return Err(NativeProfileError::InvalidDescriptor(
                    "empty or duplicate field name",
                ));
            }
            let layout = &field.representation;
            let end = layout
                .size
                .checked_mul(field.count)
                .and_then(|len| field.offset.checked_add(len))
                .ok_or(NativeProfileError::InvalidDescriptor("field size overflow"))?;
            if end > size
                || layout.alignment > alignment
                || !field.offset.is_multiple_of(layout.alignment)
            {
                return Err(NativeProfileError::InvalidDescriptor(
                    "field is out of bounds or misaligned",
                ));
            }
            if end != field.offset {
                ranges.push((field.offset, end));
            }
        }
        ranges.sort_unstable();
        let mut previous_end = 0;
        for (start, end) in ranges {
            if start < previous_end {
                return Err(NativeProfileError::InvalidDescriptor("overlapping fields"));
            }
            previous_end = end;
        }
        Ok(Self {
            name,
            revision,
            size,
            alignment,
            byte_order,
            fields,
        })
    }

    /// Returns the deployment-stable type name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the complete object size in bytes, including padding.
    pub fn size(&self) -> u64 {
        self.size
    }

    /// Returns the required address alignment in bytes.
    pub fn alignment(&self) -> u64 {
        self.alignment
    }

    /// Returns canonical structural bytes, versioned independently of registry IDs.
    ///
    /// Version 1 uses the ASCII domain below followed by length-prefixed UTF-8
    /// names, little-endian u32 revision, u64 size/alignment, byte order (1/2),
    /// u64 field count, then each field's name, u64 offset/count and recursively
    /// encoded representation. String lengths are u64; fields retain declaration
    /// order. Array elements use one repeated field rather than expanding values.
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut bytes = b"uprotocol.native-representation.v1\0".to_vec();
        self.append_canonical(&mut bytes);
        bytes
    }

    fn append_canonical(&self, bytes: &mut Vec<u8>) {
        append_string(bytes, &self.name);
        bytes.extend_from_slice(&self.revision.to_le_bytes());
        bytes.extend_from_slice(&self.size.to_le_bytes());
        bytes.extend_from_slice(&self.alignment.to_le_bytes());
        bytes.push(match self.byte_order {
            NativeByteOrder::LittleEndian => 1,
            NativeByteOrder::BigEndian => 2,
        });
        bytes.extend_from_slice(&(self.fields.len() as u64).to_le_bytes());
        for field in &self.fields {
            append_string(bytes, &field.name);
            bytes.extend_from_slice(&field.offset.to_le_bytes());
            bytes.extend_from_slice(&field.count.to_le_bytes());
            field.representation.append_canonical(bytes);
        }
    }

    /// Derives the token from the first four SHA-256 digest bytes, interpreted
    /// little-endian. Tables reject collisions and retain full descriptors.
    pub fn token(&self) -> NativeTypeToken {
        let digest = Sha256::digest(self.canonical_bytes());
        let first: [u8; 4] = digest
            .as_slice()
            .split_at(4)
            .0
            .try_into()
            .expect("SHA-256 prefix");
        NativeTypeToken::from_u32(u32::from_le_bytes(first))
    }
}

fn append_string(bytes: &mut Vec<u8>, value: &str) {
    bytes.extend_from_slice(&(value.len() as u64).to_le_bytes());
    bytes.extend_from_slice(value.as_bytes());
}

/// One checked native payload identity within a deployment profile.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NativePayloadIdentity {
    encoding: PayloadEncoding,
    token: NativeTypeToken,
}

impl NativePayloadIdentity {
    /// Returns the deployment-assigned 16-bit encoding, or explicit ID zero.
    pub fn encoding(self) -> PayloadEncoding {
        self.encoding
    }
    /// Returns the independent structural type token.
    pub fn token(self) -> NativeTypeToken {
        self.token
    }
}

/// Immutable private-ID allocations for complete native representations.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeProfileTable {
    entries: BTreeMap<PayloadEncoding, NativeRepresentation>,
}

impl NativeProfileTable {
    /// Checks a deployment table without assigning, truncating or guessing IDs.
    ///
    /// # Errors
    ///
    /// Rejects non-private allocations, duplicate IDs, duplicate representations
    /// and 32-bit token collisions (even when the full representations differ).
    pub fn new(
        entries: impl IntoIterator<Item = (PayloadEncoding, NativeRepresentation)>,
    ) -> Result<Self, NativeProfileError> {
        let mut table = BTreeMap::new();
        let mut tokens = BTreeSet::new();
        for (encoding, representation) in entries {
            if !encoding.is_private_use() {
                return Err(NativeProfileError::NonPrivateEncoding(encoding));
            }
            if table.contains_key(&encoding) {
                return Err(NativeProfileError::DuplicateEncoding(encoding));
            }
            let token = representation.token();
            if !tokens.insert(token) {
                return Err(NativeProfileError::TokenCollision(token));
            }
            table.insert(encoding, representation);
        }
        Ok(Self { entries: table })
    }

    /// Returns allocations in ascending encoding-ID order.
    pub fn entries(&self) -> impl Iterator<Item = (PayloadEncoding, &NativeRepresentation)> {
        self.entries
            .iter()
            .map(|(encoding, representation)| (*encoding, representation))
    }
}

/// Typed operation/subscription contract used explicitly with encoding ID zero.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeContract {
    operation: String,
    representation: NativeRepresentation,
}

impl NativeContract {
    /// Associates one exact representation with a deployment operation contract.
    ///
    /// # Errors
    ///
    /// Rejects an empty operation identity. The route owner must bind this
    /// operation to its actual endpoint/filter, rather than choosing it from bytes.
    pub fn new(
        operation: impl Into<String>,
        representation: NativeRepresentation,
    ) -> Result<Self, NativeProfileError> {
        let operation = operation.into();
        if operation.is_empty() {
            return Err(NativeProfileError::InvalidProfile(
                "empty operation contract",
            ));
        }
        Ok(Self {
            operation,
            representation,
        })
    }

    /// Returns the operation/subscription contract identity.
    pub fn operation(&self) -> &str {
        &self.operation
    }
}

/// Explicit deployment mode; no automatic table-to-ID-zero fallback exists.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NativeProfileMode {
    /// Private encoding IDs identify full native representations.
    Table(NativeProfileTable),
    /// An explicit operation contract defines the representation for ID zero.
    ContractDefined(NativeContract),
}

/// Immutable native profile generation, scoped to one deployment domain.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeProfile {
    domain: String,
    version: u32,
    mode: NativeProfileMode,
}

impl NativeProfile {
    /// Creates a named, explicitly versioned deployment profile.
    ///
    /// # Errors
    ///
    /// Rejects empty domains and version zero. Profile changes require a new
    /// version and matching peer content before route activation.
    pub fn new(
        domain: impl Into<String>,
        version: u32,
        mode: NativeProfileMode,
    ) -> Result<Self, NativeProfileError> {
        let domain = domain.into();
        if domain.is_empty() || version == 0 {
            return Err(NativeProfileError::InvalidProfile(
                "domain and nonzero version are required",
            ));
        }
        Ok(Self {
            domain,
            version,
            mode,
        })
    }

    /// Returns the deployment domain.
    pub fn domain(&self) -> &str {
        &self.domain
    }
    /// Returns the immutable generation version.
    pub fn version(&self) -> u32 {
        self.version
    }
    /// Returns the explicitly selected mode.
    pub fn mode(&self) -> &NativeProfileMode {
        &self.mode
    }

    /// Returns canonical agreement bytes, including domain, version and mode.
    ///
    /// After the ASCII domain below: u64-length-prefixed UTF-8 domain, u32 LE
    /// version, mode byte (1=table, 2=contract). A table has u64 LE entry count
    /// followed by ascending u16 LE IDs and recursive representation encodings.
    /// A contract has its length-prefixed operation name then its representation.
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut bytes = b"uprotocol.native-profile.v1\0".to_vec();
        append_string(&mut bytes, &self.domain);
        bytes.extend_from_slice(&self.version.to_le_bytes());
        match &self.mode {
            NativeProfileMode::Table(table) => {
                bytes.push(1);
                bytes.extend_from_slice(&(table.entries.len() as u64).to_le_bytes());
                for (encoding, representation) in table.entries() {
                    bytes.extend_from_slice(&(encoding.id() as u16).to_le_bytes());
                    representation.append_canonical(&mut bytes);
                }
            }
            NativeProfileMode::ContractDefined(contract) => {
                bytes.push(2);
                append_string(&mut bytes, &contract.operation);
                contract.representation.append_canonical(&mut bytes);
            }
        }
        bytes
    }

    /// Returns the full SHA-256 content identity for deployment evidence.
    pub fn content_digest(&self) -> [u8; 32] {
        Sha256::digest(self.canonical_bytes()).into()
    }

    fn representation(&self, encoding: PayloadEncoding) -> Option<&NativeRepresentation> {
        match &self.mode {
            NativeProfileMode::Table(table) => table.entries.get(&encoding),
            NativeProfileMode::ContractDefined(contract) => {
                (encoding == PayloadEncoding::IMPLICIT).then_some(&contract.representation)
            }
        }
    }
}

/// Construction-time agreement between actual loaded peer configurations.
///
/// This is not a network handshake. Deployment tooling supplies the peer's
/// configuration; a route must not substitute its own configuration for that
/// evidence. Clones retain the same immutable generation for in-flight work.
#[derive(Clone, Debug)]
pub struct NativeProfileAgreement {
    profile: Arc<NativeProfile>,
}

impl NativeProfileAgreement {
    /// Confirms exact domain, version, mode and full table/contract content.
    ///
    /// # Errors
    ///
    /// Rejects any disagreement, including equal domain/version with different
    /// assignments. Full equality is checked, not only the short type tokens.
    pub fn new(
        local: Arc<NativeProfile>,
        peer: &NativeProfile,
    ) -> Result<Self, NativeProfileError> {
        if local.as_ref() != peer {
            return Err(NativeProfileError::ProfileMismatch);
        }
        Ok(Self { profile: local })
    }

    /// Returns the retained profile generation.
    pub fn profile(&self) -> &NativeProfile {
        &self.profile
    }

    /// Prepares a matching, newer generation before the route owner activates it.
    /// Existing agreements retain the old generation for in-flight frames.
    ///
    /// # Errors
    ///
    /// Rejects domain changes, non-increasing versions and peer disagreement.
    /// A domain change requires constructing a separate route.
    pub fn successor(
        &self,
        local: Arc<NativeProfile>,
        peer: &NativeProfile,
    ) -> Result<Self, NativeProfileError> {
        if local.domain != self.profile.domain || local.version <= self.profile.version {
            return Err(NativeProfileError::InvalidProfile(
                "successor must retain the domain and advance the version",
            ));
        }
        Self::new(local, peer)
    }

    /// Resolves a complete representation under this route's explicit mode.
    ///
    /// # Errors
    ///
    /// Rejects representations absent from the table or operation contract.
    pub fn identity_for(
        &self,
        representation: &NativeRepresentation,
    ) -> Result<NativePayloadIdentity, NativeProfileError> {
        let encoding = match &self.profile.mode {
            NativeProfileMode::Table(table) => table
                .entries()
                .find_map(|(encoding, actual)| (actual == representation).then_some(encoding)),
            NativeProfileMode::ContractDefined(contract) => {
                (&contract.representation == representation).then_some(PayloadEncoding::IMPLICIT)
            }
        }
        .ok_or(NativeProfileError::UnknownRepresentation)?;
        Ok(NativePayloadIdentity {
            encoding,
            token: representation.token(),
        })
    }

    /// Resolves a carried ID from the agreed table/contract, without inspecting
    /// bytes or substituting a receiver's expected type.
    pub fn identity_for_encoding(
        &self,
        encoding: PayloadEncoding,
    ) -> Option<NativePayloadIdentity> {
        self.profile
            .representation(encoding)
            .map(|representation| NativePayloadIdentity {
                encoding,
                token: representation.token(),
            })
    }

    /// Checks a carried ID/token pair before profile-dependent projection.
    ///
    /// # Errors
    ///
    /// Rejects an unknown ID or a mismatched token. Payload validity is separate.
    pub fn verify_carried(
        &self,
        encoding: PayloadEncoding,
        token: NativeTypeToken,
    ) -> Result<(), NativeProfileError> {
        let expected = self
            .identity_for_encoding(encoding)
            .ok_or(NativeProfileError::UnknownEncoding(encoding))?;
        if expected.token != token {
            return Err(NativeProfileError::IdentityMismatch);
        }
        Ok(())
    }

    /// Checks the complete expected representation as well as the carried pair.
    ///
    /// # Errors
    ///
    /// Rejects missing membership, foreign encoding IDs and mismatched tokens.
    pub fn verify_representation(
        &self,
        encoding: PayloadEncoding,
        token: NativeTypeToken,
        representation: &NativeRepresentation,
    ) -> Result<(), NativeProfileError> {
        let expected = self.identity_for(representation)?;
        if expected.encoding != encoding || expected.token != token {
            return Err(NativeProfileError::IdentityMismatch);
        }
        Ok(())
    }
}

/// Errors establishing or using a scoped native agreement.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum NativeProfileError {
    /// The structural description is internally inconsistent.
    #[error("invalid native representation: {0}")]
    InvalidDescriptor(&'static str),
    /// The profile lacks an explicit domain, version or operation contract.
    #[error("invalid native profile: {0}")]
    InvalidProfile(&'static str),
    /// A table attempts to allocate outside the deployment-private range.
    #[error("native table allocation {0} is not private-use")]
    NonPrivateEncoding(PayloadEncoding),
    /// Two table entries use one encoding ID.
    #[error("duplicate native encoding allocation {0}")]
    DuplicateEncoding(PayloadEncoding),
    /// Multiple entries share a short structural token.
    #[error("native type token collision: {0:?}")]
    TokenCollision(NativeTypeToken),
    /// The full representation has no agreed allocation/contract.
    #[error("native representation is not in the agreed profile")]
    UnknownRepresentation,
    /// The carried ID has no agreed allocation/contract.
    #[error("payload encoding {0} is not in the agreed native profile")]
    UnknownEncoding(PayloadEncoding),
    /// A carried identity differs from the agreed representation.
    #[error("native encoding/type token mismatch")]
    IdentityMismatch,
    /// Loaded peer configurations differ.
    #[error("native profile domain, version, mode or content differs")]
    ProfileMismatch,
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_case::test_case;

    fn scalar(name: &str) -> NativeRepresentation {
        NativeRepresentation::new(name, 0, 4, 4, NativeByteOrder::LittleEndian, vec![]).unwrap()
    }

    fn encoding(id: u32) -> PayloadEncoding {
        PayloadEncoding::from_id(id).unwrap()
    }

    fn profile(id: u32, name: &str) -> NativeProfile {
        NativeProfile::new(
            "vehicle",
            1,
            NativeProfileMode::Table(
                NativeProfileTable::new([(encoding(id), scalar(name))]).unwrap(),
            ),
        )
        .unwrap()
    }

    #[test_case(0; "contract defined is not a private table allocation")]
    #[test_case(8; "unassigned ID is not a native allocation")]
    #[test_case(0xDFFF; "registry range is not private")]
    #[test_case(0xEFFF; "reserved range is not private")]
    fn table_rejects_non_private_allocations(id: u32) {
        assert_eq!(
            NativeProfileTable::new([(encoding(id), scalar("u32"))]),
            Err(NativeProfileError::NonPrivateEncoding(encoding(id)))
        );
    }

    #[test_case(0xF000; "first private ID")]
    #[test_case(0xFFFF; "last private ID")]
    fn table_keeps_encoding_separate_from_token(id: u32) {
        let profile = profile(id, "u32");
        let agreement = NativeProfileAgreement::new(Arc::new(profile.clone()), &profile).unwrap();
        let identity = agreement.identity_for(&scalar("u32")).unwrap();
        assert_eq!(identity.encoding().id(), id);
        assert_eq!(identity.token(), scalar("u32").token());
        assert!(agreement
            .verify_representation(identity.encoding(), identity.token(), &scalar("i32"))
            .is_err());
    }

    #[test_case(0xF000, "u32"; "duplicate ID and representation")]
    #[test_case(0xF000, "i32"; "duplicate ID with different representation")]
    #[test_case(0xF001, "u32"; "duplicate token under another ID")]
    fn duplicate_allocations_fail(id: u32, name: &str) {
        assert!(NativeProfileTable::new([
            (encoding(0xF000), scalar("u32")),
            (encoding(id), scalar(name))
        ])
        .is_err());
    }

    #[test]
    fn independently_computed_token_vector_is_frozen() {
        // Python hashlib.sha256 over the documented representation-v1 bytes.
        assert_eq!(scalar("u32").token().as_u32(), 0x5c78_f597);
    }

    #[test]
    fn real_short_token_collision_is_rejected() {
        // Independently found by hashing scalar names collision.0..collision.61243.
        // Distinct full SHA-256 digests share the same first four bytes.
        let first = scalar("collision.50200");
        let second = scalar("collision.61243");
        assert_ne!(first.canonical_bytes(), second.canonical_bytes());
        assert_eq!(first.token(), NativeTypeToken::from_u32(0x5042_d387));
        assert_eq!(first.token(), second.token());
        assert!(matches!(
            NativeProfileTable::new([(encoding(0xF000), first), (encoding(0xF001), second)]),
            Err(NativeProfileError::TokenCollision(_))
        ));
    }

    #[test_case("vehicle", 1, 0xF001, "u32"; "same domain and version different assignment")]
    #[test_case("vehicle", 1, 0xF000, "i32"; "same size different scalar")]
    #[test_case("other", 1, 0xF000, "u32"; "different domain")]
    #[test_case("vehicle", 2, 0xF000, "u32"; "different generation")]
    fn route_rejects_peer_disagreement(domain: &str, version: u32, id: u32, name: &str) {
        let local = profile(0xF000, "u32");
        let peer = NativeProfile::new(domain, version, profile(id, name).mode).unwrap();
        assert!(matches!(
            NativeProfileAgreement::new(Arc::new(local), &peer),
            Err(NativeProfileError::ProfileMismatch)
        ));
    }

    #[test]
    fn canonical_table_order_is_independent_of_registration_order() {
        let entries = [
            (encoding(0xF002), scalar("u32")),
            (encoding(0xF001), scalar("i32")),
        ];
        let first = NativeProfile::new(
            "vehicle",
            1,
            NativeProfileMode::Table(NativeProfileTable::new(entries.clone()).unwrap()),
        )
        .unwrap();
        let second = NativeProfile::new(
            "vehicle",
            1,
            NativeProfileMode::Table(NativeProfileTable::new(entries.into_iter().rev()).unwrap()),
        )
        .unwrap();
        assert_eq!(first.canonical_bytes(), second.canonical_bytes());
        assert_eq!(first.content_digest(), second.content_digest());
        assert!(NativeProfileAgreement::new(Arc::new(first), &second).is_ok());
    }

    #[test]
    fn contract_defined_requires_exact_operation_and_representation() {
        let contract = NativeContract::new("vehicle.temperature.event.v1", scalar("u32")).unwrap();
        let local =
            NativeProfile::new("vehicle", 1, NativeProfileMode::ContractDefined(contract)).unwrap();
        let agreement = NativeProfileAgreement::new(Arc::new(local.clone()), &local).unwrap();
        let identity = agreement.identity_for(&scalar("u32")).unwrap();
        assert_eq!(identity.encoding(), PayloadEncoding::IMPLICIT);
        assert!(agreement.identity_for(&scalar("i32")).is_err());
        assert!(agreement.identity_for_encoding(encoding(0xF000)).is_none());
        let peer = NativeProfile::new(
            "vehicle",
            1,
            NativeProfileMode::ContractDefined(
                NativeContract::new("different.operation", scalar("u32")).unwrap(),
            ),
        )
        .unwrap();
        assert!(NativeProfileAgreement::new(Arc::new(local), &peer).is_err());
    }

    #[test]
    fn fields_distinguish_equal_sized_representations() {
        let make = |name| {
            NativeRepresentation::new(
                "Packet",
                1,
                4,
                4,
                NativeByteOrder::LittleEndian,
                vec![NativeField::new("value", 0, scalar(name))],
            )
            .unwrap()
        };
        assert_ne!(make("u32").token(), make("i32").token());
        assert_ne!(make("u32").canonical_bytes(), make("i32").canonical_bytes());
    }

    #[test_case("vehicle", 1; "changed table without version advance")]
    #[test_case("other", 2; "domain change on live route")]
    fn successor_rejects_invalid_generation(domain: &str, version: u32) {
        let current = profile(0xF000, "u32");
        let agreement = NativeProfileAgreement::new(Arc::new(current.clone()), &current).unwrap();
        let next = NativeProfile::new(domain, version, profile(0xF001, "u32").mode).unwrap();
        assert!(agreement.successor(Arc::new(next.clone()), &next).is_err());
    }

    #[test]
    fn successor_preserves_in_flight_generation() {
        let current = profile(0xF000, "u32");
        let old = NativeProfileAgreement::new(Arc::new(current.clone()), &current).unwrap();
        let next = NativeProfile::new("vehicle", 2, profile(0xF001, "u32").mode).unwrap();
        assert!(old.successor(Arc::new(next.clone()), &current).is_err());
        let new = old.successor(Arc::new(next.clone()), &next).unwrap();
        assert_eq!(
            old.identity_for(&scalar("u32")).unwrap().encoding(),
            encoding(0xF000)
        );
        assert_eq!(
            new.identity_for(&scalar("u32")).unwrap().encoding(),
            encoding(0xF001)
        );
    }

    #[test_case(0; "overlap")]
    #[test_case(2; "misaligned")]
    #[test_case(8; "out of bounds")]
    #[test_case(u64::MAX; "overflow")]
    fn invalid_field_layout_is_rejected(offset: u64) {
        assert!(NativeRepresentation::new(
            "Packet",
            1,
            8,
            4,
            NativeByteOrder::LittleEndian,
            vec![
                NativeField::new("a", 0, scalar("u32")),
                NativeField::new("b", offset, scalar("i32"))
            ]
        )
        .is_err());
    }
}
