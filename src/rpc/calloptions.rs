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

/// This struct is used when making `uRPC` calls to pass additional options.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CallOptions {
    timeout: u32,
    token: String,
}

impl CallOptions {
    const TIMEOUT_DEFAULT: u32 = 10_000;

    pub const DEFAULT: CallOptions = CallOptions {
        timeout: CallOptions::TIMEOUT_DEFAULT,
        token: String::new(),
    };

    /// Constructs a new builder.
    pub fn builder() -> CallOptionsBuilder {
        CallOptionsBuilder::default()
    }

    /// Get a timeout.
    pub fn timeout(&self) -> u32 {
        self.timeout
    }

    /// Get an `OAuth2` access token.
    pub fn token(&self) -> Option<&str> {
        // I don't agree with the trim() here - a space is a valid character, so if I don't want to have them
        // here, I should fail noisily on creation. But, to be compliant with the Java SDK...
        if self.token.trim().is_empty() {
            None
        } else {
            Some(&self.token)
        }
    }
}

/// Builder for constructing `CallOptions`.
#[derive(Debug, Clone)]
pub struct CallOptionsBuilder {
    timeout: u32,
    token: String,
}

impl Default for CallOptionsBuilder {
    fn default() -> Self {
        Self {
            timeout: CallOptions::TIMEOUT_DEFAULT,
            token: String::new(),
        }
    }
}

impl CallOptionsBuilder {
    /// Add a timeout.
    #[must_use]
    pub fn with_timeout(mut self, timeout: u32) -> Self {
        self.timeout = if timeout == 0 {
            CallOptions::TIMEOUT_DEFAULT
        } else {
            timeout
        };
        self
    }

    /// Add an `OAuth2` access token.
    #[must_use]
    pub fn with_token(mut self, token: impl Into<String>) -> Self {
        self.token = token.into();
        self
    }

    /// Construct a `CallOptions` from this builder.
    pub fn build(self) -> CallOptions {
        CallOptions {
            timeout: self.timeout,
            token: self.token,
        }
    }
}

impl fmt::Display for CallOptions {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "CallOptions {{ timeout: {}, token: '{}' }}",
            self.timeout, self.token
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_code_equals() {
        let call_option1 = CallOptions::builder()
            .with_timeout(30)
            .with_token("someToken")
            .build();

        let call_option2 = CallOptions::builder()
            .with_timeout(30)
            .with_token("someToken")
            .build();

        assert_eq!(call_option1, call_option2);
        assert_eq!(call_option1.timeout(), call_option2.timeout());
        assert_eq!(call_option1.token(), call_option2.token());
    }

    #[test]
    fn test_to_string() {
        let call_options = CallOptions::builder()
            .with_timeout(30)
            .with_token("someToken")
            .build();

        let expected = "CallOptions { timeout: 30, token: 'someToken' }";
        assert_eq!(expected, call_options.to_string());
    }

    #[test]
    fn test_creating_call_options_default() {
        let call_options = CallOptions::DEFAULT;
        assert_eq!(CallOptions::TIMEOUT_DEFAULT, call_options.timeout());
        assert!(call_options.token().is_none());
    }

    #[test]
    fn test_creating_call_options_with_a_token() {
        let call_options = CallOptions::builder().with_token("someToken").build();

        assert_eq!(CallOptions::TIMEOUT_DEFAULT, call_options.timeout());
        assert!(call_options.token().is_some());
        let token = call_options.token().unwrap();
        assert_eq!("someToken", token);
    }

    // Omitted, because in Rust there is no 'null' where an object should be
    // #[test]
    // fn test_creating_call_options_with_none_token() {
    //     let call_options = CallOptions::builder()
    //         .with_token(None)
    //         .build();

    //     assert_eq!(TIMEOUT_DEFAULT, call_options.timeout());
    //     assert!(call_options.token().is_none());
    // }

    #[test]
    fn test_creating_call_options_with_empty_string_token() {
        let call_options = CallOptions::builder().with_token(String::from("")).build();

        assert_eq!(CallOptions::TIMEOUT_DEFAULT, call_options.timeout());
        assert!(call_options.token().is_none());
    }

    #[test]
    fn test_creating_call_options_with_a_token_with_only_spaces() {
        let call_options = CallOptions::builder().with_token("   ".to_string()).build();

        assert_eq!(CallOptions::TIMEOUT_DEFAULT, call_options.timeout());
        assert!(call_options.token().is_none());
    }

    #[test]
    fn test_creating_call_options_with_a_timeout() {
        let call_options = CallOptions::builder().with_timeout(30).build();

        assert_eq!(30, call_options.timeout());
        assert!(call_options.token().is_none());
    }

    // Omitted, because there is no negative timeout with an u32
    // #[test]
    // fn test_creating_call_options_with_a_negative_timeout() {
    //     let call_options = CallOptions::builder().with_timeout(-3 as u32).build();

    //     assert_eq!(CallOptions::TIMEOUT_DEFAULT, call_options.timeout());
    //     assert!(call_options.token().is_none());
    // }
}
