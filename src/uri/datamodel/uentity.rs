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

/// Data representation of an **Software Entity - uE**
/// An  Software Entity is a piece of software deployed somewhere on a uDevice.
/// The  Software Entity is used in the source and sink parts of communicating software.
///
/// A `UEntity` that publishes events is in a **Service** role.
/// A `UEntity` that consumes events is in an **Application** role.
#[derive(Default, Clone, PartialEq, Eq)]
pub struct UEntity {
    pub name: String,
    pub version: Option<String>,
    pub id: Option<u16>,
    pub marked_resolved: bool, // Indicates that this UAuthority has already been resolved.
}

impl UEntity {
    /// An empty `UEntity` that can serve as a placeholder when a meaningful `UEntity` is not available or necessary.
    ///
    /// This is often used to initialize variables or function returns where a meaningful `UEntity` may not be
    /// necessary, or has not yet been determined. Using `UEntity::EMPTY` can help to avoid working with `None`
    /// and can simplify code that deals with `UEntity` instances.
    ///
    /// Note that, by definition, an `UEntity` is considered empty if both its `name` and `version` fields are empty.
    pub const EMPTY: UEntity = UEntity {
        name: String::new(),
        version: None,
        id: None,
        marked_resolved: false,
    };

    /// Build an  Software Entity that represents a communicating piece of software.
    ///
    /// This constructor takes a name and an optional version for the software entity.
    /// If no version is provided, the latest version of the service will be used.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the software, such as "petapp" or "body.access".
    /// * `version` - An optional software version. If not supplied, the latest version is used.
    /// * `id` - A numeric identifier for the software entity.
    /// * `marked_resolved` - Indicates that this uResource was populated with intent of having all data.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use uprotocol_sdk::uri::datamodel::uentity::UEntity;
    ///
    /// let entity_with_version = UEntity::new("body.access".to_string(), Some("1.0".to_string()), None, false);
    /// assert_eq!(entity_with_version.name, "body.access");
    /// assert_eq!(entity_with_version.version.unwrap(), "1.0");
    ///
    /// let entity_without_version = UEntity::new("body.access".to_string(), None, None, false);
    /// assert_eq!(entity_without_version.name, "body.access");
    /// assert!(entity_without_version.version.is_none());
    /// ```
    pub fn new(
        name: String,
        version: Option<String>,
        id: Option<u16>,
        marked_resolved: bool,
    ) -> Self {
        // only create/assign Some(string) if input is a non-empty string
        let version = version
            .map(|v| v.trim().to_lowercase())
            .filter(|v| !v.is_empty());

        UEntity {
            name: name.to_string(),
            version,
            id,
            marked_resolved,
        }
    }

    /// Creates a new `UEntity` instance using the provided application or service name.
    ///
    /// This is a static factory method that takes the application or service name as an argument
    /// and returns a `UEntity` instance with that name. Note that the version is assumed to be
    /// the latest, as no version is provided.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the application or service, such as "petapp" or "body.access".
    /// * `version` - The software version. If not supplied, the latest version of the service will be used.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use uprotocol_sdk::uri::datamodel::uentity::UEntity;
    ///
    /// let entity = UEntity::long_format("body.access".to_string(), None);
    /// assert_eq!(entity.name, "body.access");
    /// assert!(entity.version.is_none());
    /// ```
    pub fn long_format(name: String, version: Option<String>) -> Self {
        UEntity {
            name,
            version,
            id: None,
            marked_resolved: false,
        }
    }

    /// Creates a new `UEntity` instance using the provided id and version.
    ///
    /// # Arguments
    ///
    /// * `version` - The software version. If not supplied, the latest version of the service will be used.
    /// * `id` - The software id.
    ///
    /// # Returns
    ///
    /// Returns a `UEntity` with id but unknown name.
    pub fn micro_format(version: String, id: u16) -> Self {
        UEntity {
            name: "".to_string(),
            version: Some(version),
            id: Some(id),
            marked_resolved: false,
        }
    }

    /// Indicates whether this `UEntity` instance is an empty container and has no valuable information for building uProtocol sinks or sources.
    ///
    /// This method checks both the `name` and `version` fields of the `UEntity` instance, and if both are empty,
    /// it signifies that this `UEntity` instance doesn't hold valuable information for uProtocol sinks or sources.
    ///
    /// # Returns
    ///
    /// * `bool` - `true` if both `name` and `version` are empty, `false` otherwise.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use uprotocol_sdk::uri::datamodel::uentity::UEntity;
    ///
    /// let entity = UEntity::default();
    /// assert_eq!(entity.is_empty(), true);
    ///
    /// let entity = UEntity::new("body.access".to_string(), Some("1.0".to_string()), None, false);
    /// assert_eq!(entity.is_empty(), false);
    /// ```
    pub fn is_empty(&self) -> bool {
        self.name.is_empty() && (self.version.as_ref().map_or(true, |s| s.is_empty()))
    }

    /// Returns `true` if the `UEntity` is resolved, meaning it contains both the name and IDs.
    ///
    /// A resolved `UEntity` has a name and an ID that are not the same.
    ///
    /// # Returns
    ///
    /// Returns `true` if this `UEntity` contains resolved information.
    pub fn is_resolved(&self) -> bool {
        let mut is_resolved = !self.name.is_empty() && self.id.is_some();

        if let Some(id) = self.id {
            if let Ok(name_id) = self.name.parse::<u16>() {
                is_resolved = id.ne(&name_id);
            }
        }
        is_resolved
    }

    /// Checks if the `UEntity` contains Long Form URI information (uE name).
    ///
    /// # Returns
    ///
    /// Returns `true` if the `UEntity` contains Long Form URI information (names).
    pub fn is_long_form(&self) -> bool {
        self.is_resolved() || !self.name.trim().is_empty()
    }
}

impl fmt::Display for UEntity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}/{}",
            self.name.trim(),
            self.version.as_ref().unwrap_or(&String::from(""))
        )
    }
}

impl fmt::Debug for UEntity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "UEntity {{ name: '{}', version: '{}', id: '{}', marked_resolved: '{}' }}",
            self.name,
            self.version
                .as_ref()
                .map_or("latest".to_string(), |version| version.to_string()),
            self.id.map_or("unknown".to_string(), |id| id.to_string()),
            self.marked_resolved
        )
    }
}

#[cfg(test)]
mod tests {
    use super::UEntity;

    #[test]
    fn test_to_string() {
        let use_entity = UEntity::new(
            "body.access".to_string(),
            Some("1".to_string()),
            None,
            false,
        );
        assert_eq!("body.access", use_entity.name);
        assert_eq!("1", use_entity.version.clone().unwrap());

        let expected = "UEntity { name: 'body.access', version: '1', id: 'unknown', marked_resolved: 'false' }";
        assert_eq!(expected, format!("{:?}", use_entity));

        let use1 = UEntity::long_format("body.access".to_string(), None);
        assert_eq!(
            "UEntity { name: 'body.access', version: 'latest', id: 'unknown', marked_resolved: 'false' }",
            format!("{:?}", use1)
        );
    }

    #[test]
    fn test_create_use() {
        let use_entity = UEntity::new(
            "body.access".to_string(),
            Some("1".to_string()),
            None,
            false,
        );
        assert_eq!("body.access", use_entity.name);
        assert_eq!("1", use_entity.version.unwrap());
    }

    #[test]
    fn test_create_use_with_no_version() {
        let entity = UEntity::new(
            "body.access".to_string(),
            Some(" ".to_string()),
            None,
            false,
        );
        assert_eq!("body.access", entity.name);
        assert!(entity.version.is_none());

        let entity2 = UEntity::new("body.access".to_string(), None, None, false);
        assert_eq!("body.access", entity2.name);
        assert!(entity2.version.is_none());
    }

    #[test]
    fn test_create_use_with_no_version_using_from_name() {
        let entity = UEntity::long_format("body.access".to_string(), None);
        assert_eq!("body.access", entity.name);
        assert!(entity.version.is_none());
    }

    #[test]
    fn test_create_empty_using_empty() {
        let entity = UEntity::EMPTY;
        assert!(entity.name.is_empty());
        assert!(entity.version.is_none());
    }

    #[test]
    fn test_is_empty() {
        let entity1 = UEntity::EMPTY;
        assert!(entity1.is_empty());

        let entity2 = UEntity::new("".to_string(), None, None, false);
        assert!(entity2.is_empty());

        let entity3 = UEntity::new("".to_string(), Some("1".to_string()), None, false);
        assert!(!entity3.is_empty());

        let entity4 = UEntity::new("petapp".to_string(), None, None, false);
        assert!(!entity4.is_empty());
    }

    #[test]
    fn test_create_use_with_id() {
        let use_entity = UEntity::new(
            "body.access".to_string(),
            Some("1".to_string()),
            Some(0),
            false,
        );
        assert_eq!("body.access", use_entity.name);
        assert_eq!(Some(String::from("1")), use_entity.version);
        assert_eq!(Some(0), use_entity.id);
        assert_eq!(
            "UEntity { name: 'body.access', version: '1', id: '0', marked_resolved: 'false' }",
            format!("{:?}", use_entity)
        );
    }

    #[test]
    fn test_create_use_with_invalid_id() {
        let use_entity = UEntity::new(
            "body.access".to_string(),
            Some("1".to_string()),
            None,
            false,
        );
        assert_eq!("body.access", use_entity.name);
        assert_eq!(Some(String::from("1")), use_entity.version);
        assert_eq!(None, use_entity.id);
        assert_eq!(
            "UEntity { name: 'body.access', version: '1', id: 'unknown', marked_resolved: 'false' }",
            format!("{:?}", use_entity)
        );
    }
}
