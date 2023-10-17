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
use std::str::FromStr;

const UNKNOWN_NAME: &str = "unknown";

/// Represents a service API's resource and methods within a `UEntity`.
///
/// `UResource` encapsulates a service's resources such as "door", an optional specific instance like "front_left",
/// and an optional name of the resource message type, such as "Door". The resource message type aligns with the protobuf service IDL
/// that defines structured data types.
///
/// A `UResource` can be manipulated, controlled, or exposed by a service. When prepended with `UAuthority` that represents the device and
/// `UEntity` that represents the service, resources are unique.
#[derive(Default, Clone, PartialEq, Eq)]
pub struct UResource {
    pub name: String,
    pub instance: Option<String>,
    pub message: Option<String>,
    pub id: Option<u16>,
    pub marked_resolved: bool, // Indicates that this UResource has already been resolved.
}

impl UResource {
    /// Represents an empty `UResource` instance.
    ///
    /// This constant allows for creating an empty `UResource` to avoid dealing with nulls. It has a blank name and no message instance information.
    ///
    /// # Example
    ///
    /// ```
    /// use uprotocol_sdk::uri::datamodel::UResource;
    ///
    /// let empty_resource = UResource::EMPTY;
    /// assert!(empty_resource.name.is_empty());
    /// assert!(empty_resource.instance.is_none());
    /// assert!(empty_resource.message.is_none());
    /// ```
    pub const EMPTY: UResource = UResource {
        name: String::new(),
        instance: None,
        message: None,
        id: None,
        marked_resolved: false,
    };

    /// Creates a `UResource` instance.
    ///
    /// The resource represents something manipulated by a service, such as a door. The `name` represents the resource as a noun (e.g., "door", "window") or a verb in case of a method manipulating the resource.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the resource, typically a noun such as "door" or "window", or a verb in the case of a method manipulating the resource.
    /// * `instance` - An optional instance of a resource, such as "front_left".
    /// * `message` - An optional protobuf service IDL message name defining structured data types. It is a data structure type used to define data passed in events and RPC methods.
    /// * `markedResolved` - Indicates that this uResource was populated with intent of having all data.
    /// # Example
    ///
    /// ```
    /// use uprotocol_sdk::uri::datamodel::UResource;
    ///
    /// let resource = UResource::new("door".to_string(), Some("front_left".to_string()), Some("DoorOpenEvent".to_string()), None, false);
    /// ```
    pub fn new(
        name: String,
        instance: Option<String>,
        message: Option<String>,
        id: Option<u16>,
        marked_resolved: bool,
    ) -> Self {
        // only create/assign Some(string) if input is a non-empty string
        let instance = instance
            .map(|i| i.trim().to_string())
            .filter(|i| !i.is_empty());
        let message = message
            .map(|m| m.trim().to_string())
            .filter(|m| !m.is_empty());

        UResource {
            name: name.clone(),
            instance: instance.clone(),
            message,
            id: {
                if instance.as_ref().map_or(false, |i| i == "reponse") && name == "rpc" {
                    Some(0)
                } else {
                    id
                }
            },
            marked_resolved,
        }
    }

    /// Creates a `UResource` instance using the resource name.
    ///
    /// The resource name represents the resource as a noun (e.g., "door", "window") or a verb in case of a method manipulating the resource. The created `UResource` has an empty instance and message.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the resource, typically a noun such as "door" or "window", or a verb in the case of a method manipulating the resource.
    ///
    /// # Returns
    ///
    /// A `UResource` with the provided resource name, and empty instance and message fields. If the instance does not exist, it is assumed that all the instances of the resource are wanted.
    ///
    /// # Example
    ///
    /// ```
    /// use uprotocol_sdk::uri::datamodel::UResource;
    ///
    /// let resource = UResource::long_format("door".to_string());
    /// ```
    pub fn long_format(name: String) -> Self {
        UResource {
            name,
            instance: None,
            message: None,
            id: None,
            marked_resolved: false,
        }
    }

    /// Creates a `UResource` instance using the resource name and a specific resource instance.
    ///
    /// The resource name represents the resource as a noun (e.g., "door", "window") or a verb in case of a method manipulating the resource. The created `UResource` has the provided resource instance and an empty message.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the resource, typically a noun such as "door" or "window", or a verb in the case of a method manipulating the resource.
    /// * `instance` - An instance of a resource, for example "front_left".
    ///
    /// # Returns
    ///
    /// A `UResource` with the provided resource name and a specific instance. The message field is left empty.
    ///
    /// # Example
    ///
    /// ```
    /// use uprotocol_sdk::uri::datamodel::UResource;
    ///
    /// let resource = UResource::long_format_with_instance("door".to_string(), "front_left".to_string(), None);
    /// ```
    pub fn long_format_with_instance(
        name: String,
        instance: String,
        message: Option<String>,
    ) -> Self {
        UResource {
            name,
            instance: Some(instance),
            message,
            id: None,
            marked_resolved: false,
        }
    }

    /// Creates a new `UResource` using the resource id.
    ///
    /// # Arguments
    ///
    /// * `id` - The id of the resource.
    ///
    /// # Returns
    ///
    /// Returns a `UResource` with the given resource id. The name, instance, and message fields will be empty.
    pub fn micro_format(id: u16) -> Self {
        Self::new("".to_string(), None, None, Some(id), false)
    }

    /// Creates a `UResource` instance representing an RPC command.
    ///
    /// This is a static factory method used to construct a `UResource` object that corresponds to a specific RPC command.
    ///
    /// # Arguments
    ///
    /// * `method_name` - The name of the RPC command, such as "UpdateDoor".
    /// * `meethod_id` - The numeric representation method name for the RPC.
    ///
    /// # Returns
    ///
    /// A `UResource` configured to represent the specified RPC command.
    ///
    /// # Example
    ///
    /// ```
    /// use uprotocol_sdk::uri::datamodel::UResource;
    ///
    /// let rpc_command = UResource::for_rpc_request(Some("UpdateDoor".to_string()), None);
    /// ```
    pub fn for_rpc_request(method_name: Option<String>, method_id: Option<u16>) -> Self {
        UResource {
            name: String::from("rpc"),
            instance: method_name,
            message: None,
            id: method_id,
            marked_resolved: false,
        }
    }

    /// Creates a `UResource` instance representing a response returned from RPC calls.
    ///
    /// This is a static factory method that returns a predefined `UResource` which can be used for response RPC calls.
    ///
    /// # Returns
    ///
    /// A `UResource` configured to represent a response from RPC calls.
    pub fn for_rpc_response() -> UResource {
        UResource {
            name: String::from("rpc"),
            instance: Some(String::from("response")),
            message: None,
            id: Some(0),
            marked_resolved: true,
        }
    }

    /// Checks if this `UResource` represents an RPC method call.
    ///
    /// This method determines whether the `UResource` instance is configured to represent an RPC method.
    ///
    /// # Returns
    ///
    /// `true` if this resource specifies an RPC method call or RPC response.
    ///
    /// # Example
    ///
    /// ```
    /// use uprotocol_sdk::uri::datamodel::UResource;
    ///
    /// let resource = UResource::for_rpc_request(Some("UpdateDoor".to_string()), None);
    /// assert!(resource.is_rpc_method());
    /// ```
    pub fn is_rpc_method(&self) -> bool {
        (self.name.eq("rpc") && self.instance.is_some())
            || (self.name.eq("rpc") && self.id.is_some())
    }

    pub fn parse_from_string(resource_string: &str) -> Result<Self, &'static str> {
        if resource_string.is_empty() {
            return Err("Resource must have a command name.");
        }

        let parts: Vec<&str> = resource_string.split('#').collect();
        let name_and_instance = parts[0];

        let maybe_id: Option<u16> = u16::from_str(name_and_instance).ok();

        if let Some(id) = maybe_id {
            return Ok(UResource::micro_format(id));
        }

        let name_and_instance_parts: Vec<&str> = name_and_instance.split('.').collect();
        let resource_name = name_and_instance_parts[0].to_string();
        let resource_instance = name_and_instance_parts.get(1).map(|&s| s.to_string());
        let resource_message = parts.get(1).map(|&s| s.to_string());

        Ok(UResource {
            name: resource_name,
            instance: resource_instance,
            message: resource_message,
            id: None,
            marked_resolved: false,
        })
    }

    /// Checks if this `UResource` is an empty container.
    ///
    /// This method determines whether the `UResource` instance has no valuable information for building uProtocol URI.
    ///
    /// # Returns
    ///
    /// `true` if this `UResource` is an empty container, `false` otherwise.
    ///
    /// # Example
    ///
    /// ```
    /// use uprotocol_sdk::uri::datamodel::UResource;
    ///
    /// let empty_resource = UResource::EMPTY;
    /// assert!(empty_resource.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        (self.name.trim().is_empty() || self.name.eq("rpc"))
            && self.instance.is_none()
            && self.message.is_none()
            && self.id.is_none()

        // return (name.isBlank() || "rpc".equals(name)) && instance().isEmpty() && message().isEmpty() && id().isEmpty();

        // self.name.is_empty()
        //     && (self.instance.as_ref().map_or(false, |s| s.is_empty()))
        //     && (self.message.as_ref().map_or(false, |s| s.is_empty()))
    }

    /// Builds a string with the name and instance attributes.
    ///
    /// This method is mainly used for constructing the name attribute in many protobuf Message objects.
    /// It concatenates the `name` and `instance` attributes with a dot delimiter if the `instance` attribute exists.
    ///
    /// # Returns
    ///
    /// A `String` with the `name` and `instance` separated by a dot, only if the `instance` exists.
    ///
    /// # Example
    ///
    /// ```
    /// use uprotocol_sdk::uri::datamodel::UResource;
    ///
    /// let resource = UResource::long_format_with_instance("name".to_string(), "instance".to_string(), None);
    /// assert_eq!(resource.name_with_instance(), "name.instance");
    /// ```
    pub fn name_with_instance(&self) -> String {
        if let Some(instance) = self.instance.as_ref() {
            format!("{}.{}", self.name, instance)
        } else {
            self.name.clone()
        }
    }

    /// Constructs a `UResource` that has all elements resolved, making it suitable for serialization
    /// into both long form `UUri` and micro form `UUri`.
    ///
    /// - `name`: Represents the resource as a noun, e.g., `door` or `window`. In cases where the method
    ///           manipulates the resource, it represents a verb.
    /// - `instance`: A specific instance of a resource, such as `front_left`.
    /// - `message`: Corresponds to the protobuf service IDL message name that defines structured data types.
    ///              A message is a data structure type used to define data that is passed in events and RPC methods.
    /// - `id`: The numeric representation of this `UResource`.
    ///
    /// Returns a `UResource` that contains all the necessary information for serialization into both long
    /// form and micro form `UUri`.
    pub fn resolved_format(name: String, instance: String, message: String, id: u16) -> Self {
        let resolved = name.trim().is_empty();
        UResource {
            name,
            instance: Some(instance),
            message: Some(message),
            id: Some(id),
            marked_resolved: resolved,
        }
    }
    // public static UResource resolvedFormat(String name, String instance, String message, Short id) {
    //     boolean resolved = name != null && !name.isBlank() && id != null;
    //     return new UResource(name, instance, message, id, resolved);
    // }

    pub fn is_resolved(&self) -> bool {
        self.id.is_some() && self.is_long_form()
    }

    pub fn is_long_form(&self) -> bool {
        self.name.ne(UNKNOWN_NAME) && self.instance.is_some() && self.is_rpc_method()
            || self.message.is_some()
    }

    pub fn is_micro_form(&self) -> bool {
        self.id.is_some()
    }
}

impl fmt::Display for UResource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut res = format!("/{}", &self.name);
        if let Some(instance) = self.instance.as_ref() {
            res.push_str(&format!(".{}", instance));
        };
        if let Some(message) = self.message.as_ref() {
            res.push_str(&format!("#{}", message));
        };

        write!(f, "{}", res)
    }
}

impl fmt::Debug for UResource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "UResource {{ name: '{}', instance: '{}', message: '{}', id: '{}' }}",
            self.name,
            self.instance.as_ref().unwrap_or(&String::from("")),
            self.message.as_ref().unwrap_or(&String::from("")),
            self.id.map_or("unknown".to_string(), |id| id.to_string())
        )
    }
}

#[cfg(test)]
mod tests {
    use super::UResource;

    #[test]
    fn test_to_string() {
        let resource = UResource::new(
            "door".to_string(),
            Some(String::from("front_left")),
            Some(String::from("Door")),
            None,
            false,
        );
        let expected =
            "UResource { name: 'door', instance: 'front_left', message: 'Door', id: 'unknown' }";
        assert_eq!(expected, format!("{:?}", resource));
    }

    #[test]
    fn test_create_resource() {
        let resource = UResource::new(
            "door".to_string(),
            Some(String::from("front_left")),
            Some(String::from("Door")),
            None,
            false,
        );
        assert_eq!("door", resource.name);
        assert!(resource.instance.is_some());
        assert_eq!("front_left", resource.instance.unwrap());
        assert!(resource.message.is_some());
        assert_eq!("Door", resource.message.unwrap());
    }

    #[test]
    fn test_create_resource_with_no_instance_and_no_message() {
        let resource = UResource::new(
            "door".to_string(),
            Some(String::from("")),
            Some(String::from("")),
            None,
            false,
        );
        assert_eq!("door", resource.name);
        assert!(resource.instance.is_none());
        assert!(resource.message.is_none());

        let resource2 = UResource::new("door".to_string(), None, None, None, false);
        assert_eq!("door", resource2.name);
        assert!(resource2.instance.is_none());
        assert!(resource2.message.is_none());
    }

    #[test]
    fn test_create_resource_with_no_instance_and_no_message_using_from_name() {
        let resource = UResource::long_format("door".to_string());
        assert_eq!("door", resource.name);
        assert!(resource.instance.is_none());
        assert!(resource.message.is_none());
    }

    #[test]
    fn test_create_resource_with_no_message_using_from_name_with_instance() {
        let resource = UResource::long_format_with_instance(
            "door".to_string(),
            "front_left".to_string(),
            None,
        );
        assert_eq!("door", resource.name);
        assert!(resource.instance.is_some());
        assert_eq!("front_left", resource.instance.unwrap());
        assert!(resource.message.is_none());
    }

    #[test]
    fn test_create_resource_for_rpc_commands() {
        let resource = UResource::for_rpc_request(Some("UpdateDoor".to_string()), None);
        assert_eq!("rpc", resource.name);
        assert!(resource.instance.is_some());
        assert_eq!("UpdateDoor", resource.instance.clone().unwrap());
        assert!(resource.is_rpc_method());
    }

    #[test]
    fn test_resource_represents_an_rpc_method_call() {
        let resource =
            UResource::long_format_with_instance("rpc".to_string(), "UpdateDoor".to_string(), None);
        assert!(resource.is_rpc_method());
    }

    #[test]
    fn test_resource_represents_a_resource_and_not_an_rpc_method_call() {
        let resource = UResource::long_format("door".to_string());
        assert!(!resource.is_rpc_method());
    }

    #[test]
    fn test_returning_a_name_with_instance_from_resource_when_name_and_instance_are_configured() {
        let resource = UResource::long_format_with_instance(
            "doors".to_string(),
            "front_left".to_string(),
            None,
        );
        let name_with_instance = resource.name_with_instance();
        assert_eq!("doors.front_left", name_with_instance);
    }

    #[test]
    fn test_returning_a_name_with_instance_from_resource_when_only_name_is_configured() {
        let resource = UResource::long_format("door".to_string());
        let name_with_instance = resource.name_with_instance();
        assert_eq!("door", name_with_instance);
    }

    #[test]
    fn test_returning_a_name_with_instance_from_resource_when_all_properties_are_configured() {
        let resource = UResource::new(
            "doors".to_string(),
            Some(String::from("front_left")),
            Some(String::from("Door")),
            None,
            false,
        );
        let name_with_instance = resource.name_with_instance();
        assert_eq!("doors.front_left", name_with_instance);
    }

    #[test]
    fn test_create_empty_using_empty() {
        let resource = UResource::EMPTY;
        assert!(resource.name.is_empty());
        assert!(resource.instance.is_none());
        assert!(resource.message.is_none());
    }

    #[test]
    fn test_is_empty() {
        let resource = UResource::EMPTY;
        assert!(resource.is_empty());

        let resource2 = UResource::new("".to_string(), None, None, None, false);
        assert!(resource2.is_empty());

        let resource3 = UResource::new(
            "".to_string(),
            Some(String::from("front_left")),
            None,
            None,
            false,
        );
        assert!(!resource3.is_empty());

        let resource4 = UResource::new(
            "".to_string(),
            None,
            Some(String::from("Door")),
            None,
            false,
        );
        assert!(!resource4.is_empty());
    }

    #[test]
    fn test_create_rpc_response_using_response_method() {
        let resource = UResource::for_rpc_response();
        assert!(!resource.name.is_empty());
        assert_eq!("rpc", resource.name);
        assert_eq!("response", resource.instance.unwrap());
        assert!(resource.message.is_none());
    }

    #[test]
    fn test_create_uresource_with_valid_id() {
        let u_resource = UResource::new(
            "door".to_string(),
            Some("front_left".to_string()),
            Some("Door".to_string()),
            Some(5),
            false,
        );
        assert_eq!("door", u_resource.name);
        assert_eq!(Some("front_left".to_string()), u_resource.instance);
        assert_eq!(Some("Door".to_string()), u_resource.message);
        assert_eq!(Some(5), u_resource.id);
        assert_eq!(
            "UResource { name: 'door', instance: 'front_left', message: 'Door', id: '5' }",
            format!("{:?}", u_resource)
        );
    }

    #[test]
    fn test_create_uresource_with_invalid_id() {
        let u_resource = UResource::new(
            "door".to_string(),
            Some("front_left".to_string()),
            Some("Door".to_string()),
            None,
            false,
        );
        assert_eq!("door", u_resource.name);
        assert_eq!(Some("front_left".to_string()), u_resource.instance);
        assert_eq!(Some("Door".to_string()), u_resource.message);
        assert_eq!(None, u_resource.id);
        assert_eq!(
            "UResource { name: 'door', instance: 'front_left', message: 'Door', id: 'unknown' }",
            format!("{:?}", u_resource)
        );
    }

    #[test]
    fn test_create_response_uresource_passing_name_instance_and_id() {
        let u_resource = UResource::new(
            "rpc".to_string(),
            Some("response".to_string()),
            None,
            Some(0),
            false,
        );
        assert_eq!("rpc", u_resource.name);
        assert_eq!(Some("response".to_string()), u_resource.instance);
        assert_eq!(None, u_resource.message);
        assert_eq!(Some(0), u_resource.id);
        assert_eq!(
            "UResource { name: 'rpc', instance: 'response', message: '', id: '0' }",
            format!("{:?}", u_resource)
        );
    }

    #[test]
    fn test_create_request_uresource_passing_name_instance_and_id() {
        let u_resource = UResource::new("rpc".to_string(), None, None, Some(0), false);
        assert_eq!("rpc", u_resource.name);
        assert_eq!(None, u_resource.instance);
        assert_eq!(None, u_resource.message);
        assert_eq!(Some(0), u_resource.id);
        assert_eq!(
            "UResource { name: 'rpc', instance: '', message: '', id: '0' }",
            format!("{:?}", u_resource)
        );
    }

    #[test]
    fn test_is_resolved_with_resolved_uresources() {
        // First case
        let u_resource = UResource::new(
            "door".to_string(),
            Some("front_left".to_string()),
            Some("Door".to_string()),
            Some(5),
            false,
        );
        assert!(u_resource.is_resolved());

        // Second case
        let u_resource2 = UResource::for_rpc_response();
        assert!(u_resource2.is_resolved());

        // Third case
        let u_resource3 = UResource::for_rpc_request(Some("UpdateDoor".to_string()), Some(5));
        assert!(u_resource3.is_resolved());
    }

    #[test]
    fn test_is_resolved_with_unresolved_uresources() {
        // First case
        let u_resource = UResource::new(
            "door".to_string(),
            Some("front_left".to_string()),
            Some("Door".to_string()),
            None,
            false,
        );
        assert!(!u_resource.is_resolved());
        assert!(u_resource.is_long_form());

        // Second case
        let u_resource2 = UResource::long_format("door".to_string());
        assert!(!u_resource2.is_resolved());
        assert!(!u_resource2.is_long_form());

        // Third case
        let u_resource3 = UResource::for_rpc_request(Some("UpdateDoor".to_string()), None);
        assert!(!u_resource3.is_resolved());
        assert!(u_resource3.is_long_form());

        // Fourth case
        let u_resource4 = UResource::micro_format(4);
        assert!(!u_resource4.is_resolved());
        assert!(!u_resource4.is_long_form());

        // Fifth case
        let u_resource5 = UResource::long_format_with_instance(
            "door".to_string(),
            "front_left".to_string(),
            None,
        );
        assert!(!u_resource5.is_resolved());
        assert!(!u_resource5.is_long_form());
    }

    #[test]
    fn test_parse_from_string_with_missing_instance_and_or_message() {
        let uresource = UResource::parse_from_string("door").unwrap();
        assert_eq!(
            format!("{:?}", uresource),
            "UResource { name: 'door', instance: '', message: '', id: 'unknown' }"
        );
        assert!(uresource.id.is_none());

        let uresource1 = UResource::parse_from_string("door.front_left").unwrap();
        assert_eq!(
            format!("{:?}", uresource1),
            "UResource { name: 'door', instance: 'front_left', message: '', id: 'unknown' }"
        );
        assert!(uresource1.id.is_none());

        let uresource5 = UResource::parse_from_string("door.front_left#Door").unwrap();
        assert_eq!(
            format!("{:?}", uresource5),
            "UResource { name: 'door', instance: 'front_left', message: 'Door', id: 'unknown' }"
        );
        assert!(uresource5.id.is_none());
    }
}
