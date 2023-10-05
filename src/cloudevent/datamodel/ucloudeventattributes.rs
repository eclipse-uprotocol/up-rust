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

/// Priority according to SDV 202 Quality of Service (QoS) and Prioritization.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Priority {
    // Low Priority. No bandwidth assurance such as File Transfer.
    Low,
    // Standard, undifferentiated application such as General (unclassified).
    Standard,
    // Operations, Administration, and Management such as Streamer messages (sub, connect, etc…)
    Operations,
    // Multimedia streaming such as Video Streaming
    MultimediaStreaming,
    // Real-time interactive such as High priority (rpc events)
    RealtimeInteractive,
    // Signaling such as Important
    Signaling,
    // Network control such as Safety Critical
    NetworkControl,
}

impl fmt::Display for Priority {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Priority::Low => write!(f, "Low"),
            Priority::Standard => write!(f, "Standard"),
            Priority::Operations => write!(f, "Operations"),
            Priority::MultimediaStreaming => write!(f, "MultimediaStreaming"),
            Priority::RealtimeInteractive => write!(f, "RealtimeInteractive"),
            Priority::Signaling => write!(f, "Signaling"),
            Priority::NetworkControl => write!(f, "NetworkControl"),
        }
    }
}
/// Specifies the properties that can configure the UCloudEvent.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct UCloudEventAttributes {
    /// An HMAC generated on the data portion of the CloudEvent message using the device key.
    pub hash: Option<String>,
    /// uProtocol Prioritization classifications defined at QoS in SDV-202.
    pub priority: Option<Priority>,
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

    pub fn hash(&self) -> &Option<String> {
        &self.hash
    }

    pub fn priority(&self) -> &Option<Priority> {
        &self.priority
    }

    pub fn ttl(&self) -> &Option<u32> {
        &self.ttl
    }

    pub fn token(&self) -> &Option<String> {
        &self.token
    }

    pub fn is_empty(&self) -> bool {
        self.hash.is_none() && self.priority.is_none() && self.ttl.is_none() && self.token.is_none()
    }
}

#[derive(Default)]
pub struct UCloudEventAttributesBuilder {
    hash: Option<String>,
    priority: Option<Priority>,
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

    pub fn with_hash(mut self, hash: String) -> Self {
        if !hash.trim().is_empty() {
            self.hash = Some(hash.trim().to_string());
        }
        self
    }

    pub fn with_priority(mut self, priority: Priority) -> Self {
        self.priority = Some(priority);
        self
    }

    pub fn with_ttl(mut self, ttl: u32) -> Self {
        self.ttl = Some(ttl);
        self
    }

    pub fn with_token(mut self, token: String) -> Self {
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
            .with_priority(Priority::Standard)
            .with_ttl(3)
            .with_token("someOAuthToken".to_string())
            .build();

        let attributes2 = UCloudEventAttributes::builder()
            .with_hash("somehash".to_string())
            .with_priority(Priority::Standard)
            .with_ttl(3)
            .with_token("someOAuthToken".to_string())
            .build();

        assert_eq!(attributes1, attributes2);
    }

    #[test]
    fn test_to_string() {
        let attributes = UCloudEventAttributes::builder()
            .with_hash("somehash".to_string())
            .with_priority(Priority::Standard)
            .with_ttl(3)
            .with_token("someOAuthToken".to_string())
            .build();

        assert_eq!(attributes.to_string(), "UCloudEventAttributes { hash: Some(\"somehash\"), priority: Some(Standard), ttl: Some(3), token: Some(\"someOAuthToken\") }");
    }

    #[test]
    fn test_create_valid() {
        let attributes = UCloudEventAttributes::builder()
            .with_hash("somehash".to_string())
            .with_priority(Priority::NetworkControl)
            .with_ttl(3)
            .with_token("someOAuthToken".to_string())
            .build();

        assert_eq!(attributes.hash(), &Some("somehash".to_string()));
        assert_eq!(attributes.priority(), &Some(Priority::NetworkControl));
        assert_eq!(attributes.ttl(), &Some(3));
        assert_eq!(attributes.token(), &Some("someOAuthToken".to_string()));
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
            .with_priority(Priority::Low)
            .build();

        assert!(!attributes4.is_empty());

        let attributes5 = UCloudEventAttributes::builder().with_ttl(8).build();

        assert!(!attributes5.is_empty());
    }
}
