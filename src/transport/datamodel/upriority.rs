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

use std::convert::TryFrom;

#[derive(Default, Debug, Clone, Copy, Eq, PartialEq)]
pub enum UPriority {
    #[default]
    Low,
    Standard,
    Operations,
    MultimediaStreaming,
    RealtimeInteractive,
    Signaling,
    NetworkControl,
}

impl UPriority {
    pub const CS0: &'static str = "CS0"; // Low Priority. No bandwidth assurance such as File Transfer.
    pub const CS1: &'static str = "CS1"; // Standard, undifferentiated application such as General (unclassified).
    pub const CS2: &'static str = "CS2"; // Operations, Administration, and Management such as Streamer messages (sub, connect, etc…)
    pub const CS3: &'static str = "CS3"; // Multimedia streaming such as Video Streaming
    pub const CS4: &'static str = "CS4"; // Real-time interactive such as High priority (rpc events)
    pub const CS5: &'static str = "CS5"; // Signaling such as Important
    pub const CS6: &'static str = "CS6"; // Network control such as Safety Critical

    pub fn all_priorities() -> Vec<Self> {
        vec![
            UPriority::Low,
            UPriority::Standard,
            UPriority::Operations,
            UPriority::MultimediaStreaming,
            UPriority::RealtimeInteractive,
            UPriority::Signaling,
            UPriority::NetworkControl,
        ]
    }

    pub fn qos_string(&self) -> &'static str {
        match self {
            Self::Low => Self::CS0,
            Self::Standard => Self::CS1,
            Self::Operations => Self::CS2,
            Self::MultimediaStreaming => Self::CS3,
            Self::RealtimeInteractive => Self::CS4,
            Self::Signaling => Self::CS5,
            Self::NetworkControl => Self::CS6,
        }
    }

    pub fn value(&self) -> i32 {
        match self {
            Self::Low => 0,
            Self::Standard => 1,
            Self::Operations => 2,
            Self::MultimediaStreaming => 3,
            Self::RealtimeInteractive => 4,
            Self::Signaling => 5,
            Self::NetworkControl => 6,
        }
    }
}

impl From<i32> for UPriority {
    /// Create a `UPriority` variant from the corresponding numeric priority value.
    ///
    /// # Arguments
    ///
    /// * `value` - A numeric priority value.
    ///
    /// # Returns
    ///
    /// Returns the `UPriority` variant matching the given numeric value. Defaults to `UPriority::Low` if no match is found.
    ///
    /// TODO might a try_from be a better idea?
    fn from(value: i32) -> Self {
        UPriority::all_priorities()
            .into_iter()
            .find(|p| p.value() == value)
            .unwrap_or_default()
    }
}

impl TryFrom<&str> for UPriority {
    type Error = ();

    /// Try to create a `UPriority` variant from the corresponding QOS String.
    ///
    /// # Arguments
    ///
    /// * `qos_string` - A QOS String value.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing the `UPriority` variant matching the given QOS String value,
    /// or a `None` wrapped in an `Err` if no match is found.
    fn try_from(qos_string: &str) -> Result<Self, Self::Error> {
        UPriority::all_priorities()
            .into_iter()
            .find(|p| p.qos_string() == qos_string)
            .ok_or(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_upriority_from_number() {
        assert_eq!(UPriority::from(0), UPriority::Low);
        assert_eq!(UPriority::from(1), UPriority::Standard);
        assert_eq!(UPriority::from(2), UPriority::Operations);
        assert_eq!(UPriority::from(3), UPriority::MultimediaStreaming);
        assert_eq!(UPriority::from(4), UPriority::RealtimeInteractive);
        assert_eq!(UPriority::from(5), UPriority::Signaling);
        assert_eq!(UPriority::from(6), UPriority::NetworkControl);
    }

    #[test]
    fn test_find_upriority_from_number_that_does_not_exist() {
        assert_eq!(UPriority::from(-42), UPriority::Low);
    }

    #[test]
    fn test_find_upriority_from_string() {
        assert_eq!(UPriority::try_from("CS0").unwrap(), UPriority::Low);
        assert_eq!(UPriority::try_from("CS1").unwrap(), UPriority::Standard);
        assert_eq!(UPriority::try_from("CS2").unwrap(), UPriority::Operations);
        assert_eq!(
            UPriority::try_from("CS3").unwrap(),
            UPriority::MultimediaStreaming
        );
        assert_eq!(
            UPriority::try_from("CS4").unwrap(),
            UPriority::RealtimeInteractive
        );
        assert_eq!(UPriority::try_from("CS5").unwrap(), UPriority::Signaling);
        assert_eq!(
            UPriority::try_from("CS6").unwrap(),
            UPriority::NetworkControl
        );
    }

    #[test]
    fn test_find_upriority_from_string_that_does_not_exist() {
        assert!(UPriority::try_from("BOOM").is_err());
    }
}
