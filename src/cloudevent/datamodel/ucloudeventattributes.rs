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

use std::fmt;

use crate::uprotocol::uattributes::UPriority;

/// Specifies the properties that can configure the `UCloudEvent`.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct UCloudEventAttributes {
    /// An HMAC generated on the data portion of the CloudEvent message using the device key.
    pub hash: Option<String>,
    /// uProtocol Prioritization classifications defined at QoS in SDV-202.
    pub priority: Option<UPriority>,
    /// How long this event should live for after it was generated (in milliseconds).
    /// Events without this attribute (or value is 0) MUST NOT timeout.
    pub ttl: Option<u32>,
    /// Oauth2 access token to perform the access request defined in the request message.
    pub token: Option<String>,
}

impl UCloudEventAttributes {
    // Creates a builder for `UCloudEventAttributes`.
    pub fn builder() -> UCloudEventAttributesBuilder {
        UCloudEventAttributesBuilder::default()
    }

    pub fn hash(&self) -> Option<&str> {
        self.hash.as_deref()
    }

    pub fn priority(&self) -> Option<UPriority> {
        self.priority
    }

    pub fn ttl(&self) -> Option<u32> {
        self.ttl
    }

    pub fn token(&self) -> Option<&str> {
        self.token.as_deref()
    }

    pub fn is_empty(&self) -> bool {
        self.hash.is_none() && self.priority.is_none() && self.ttl.is_none() && self.token.is_none()
    }
}

#[derive(Default)]
pub struct UCloudEventAttributesBuilder {
    hash: Option<String>,
    priority: Option<UPriority>,
    ttl: Option<u32>,
    token: Option<String>,
}

impl UCloudEventAttributesBuilder {
    pub fn new() -> Self {
        Self {
            hash: None,
            priority: None,
            ttl: None,
            token: None,
        }
    }

    #[must_use]
    pub fn with_hash<T>(mut self, hash: T) -> Self
    where
        T: Into<String>,
    {
        let hash = hash.into();
        if !hash.trim().is_empty() {
            self.hash = Some(hash.trim().to_string());
        }
        self
    }

    #[must_use]
    pub fn with_priority(mut self, priority: UPriority) -> Self {
        self.priority = Some(priority);
        self
    }

    #[must_use]
    pub fn with_ttl(mut self, ttl: u32) -> Self {
        self.ttl = Some(ttl);
        self
    }

    #[must_use]
    pub fn with_token<T>(mut self, token: T) -> Self
    where
        T: Into<String>,
    {
        let token = token.into();
        if !token.trim().is_empty() {
            self.token = Some(token.trim().to_string());
        }
        self
    }

    pub fn build(self) -> UCloudEventAttributes {
        UCloudEventAttributes {
            hash: self.hash,
            priority: self.priority,
            ttl: self.ttl,
            token: self.token,
        }
    }
}

impl fmt::Display for UCloudEventAttributes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "UCloudEventAttributes {{ hash: {:?}, priority: {:?}, ttl: {:?}, token: {:?} }}",
            self.hash, self.priority, self.ttl, self.token
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_code_equals() {
        let attributes1 = UCloudEventAttributes::builder()
            .with_hash("somehash".to_string())
            .with_priority(UPriority::UPRIORITY_CS0)
            .with_ttl(3)
            .with_token("someOAuthToken".to_string())
            .build();

        let attributes2 = UCloudEventAttributes::builder()
            .with_hash("somehash".to_string())
            .with_priority(UPriority::UPRIORITY_CS0)
            .with_ttl(3)
            .with_token("someOAuthToken".to_string())
            .build();

        assert_eq!(attributes1, attributes2);
    }

    #[test]
    fn test_create_valid() {
        let attributes = UCloudEventAttributes::builder()
            .with_hash("somehash".to_string())
            .with_priority(UPriority::UPRIORITY_CS6)
            .with_ttl(3)
            .with_token("someOAuthToken".to_string())
            .build();

        assert_eq!(attributes.hash(), Some("somehash"));
        assert_eq!(attributes.priority(), Some(UPriority::UPRIORITY_CS6));
        assert_eq!(attributes.ttl(), Some(3));
        assert_eq!(attributes.token(), Some("someOAuthToken"));
    }

    #[test]
    fn test_is_empty_function() {
        let attributes = UCloudEventAttributes::builder().build();
        assert!(attributes.is_empty());
    }

    #[test]
    fn test_is_empty_function_when_built_with_blank_strings() {
        let attributes = UCloudEventAttributes::builder()
            .with_hash("  ".to_string())
            .with_token("  ".to_string())
            .build();

        assert!(attributes.is_empty());
    }

    #[test]
    fn test_is_empty_function_permutations() {
        let attributes1 = UCloudEventAttributes::builder()
            .with_hash("  ".to_string())
            .with_token("  ".to_string())
            .build();

        assert!(attributes1.is_empty());

        let attributes2 = UCloudEventAttributes::builder()
            .with_hash("someHash".to_string())
            .with_token("  ".to_string())
            .build();

        assert!(!attributes2.is_empty());

        let attributes3 = UCloudEventAttributes::builder()
            .with_hash(" ".to_string())
            .with_token("SomeToken".to_string())
            .build();

        assert!(!attributes3.is_empty());

        let attributes4 = UCloudEventAttributes::builder()
            .with_priority(UPriority::UPRIORITY_UNSPECIFIED)
            .build();

        assert!(!attributes4.is_empty());

        let attributes5 = UCloudEventAttributes::builder().with_ttl(8).build();

        assert!(!attributes5.is_empty());
    }
}
