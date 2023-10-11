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

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

/// Data representation of an **UAuthority**.
///
/// An `UAuthority` consists of a device and a domain.
///
/// Device and domain names are used as part of the URI for device and service discovery.
///
/// Devices will be grouped together into realms of Zone of Authority.
///
/// An `UAuthority` represents the deployment location of a specific `UEntity` (Ultiverse Software Entity).
#[derive(Default, Clone, PartialEq, Eq)]
pub struct UAuthority {
    /// A `device` is a logical independent representation of a service bus in different execution environments.
    /// Devices will be grouped together into realms of Zone of Authority.
    pub device: Option<String>,

    /// The `domain` an  software entity is deployed on, such as vehicle or backoffice.
    /// Vehicle Domain name **MUST** be that of the vehicle VIN.
    /// A domain name is an identification string that defines a realm of administrative autonomy, authority or control within the Internet.
    pub domain: Option<String>,

    /// An  Uri starting with up:// is a remote configuration of a URI, and we mark the `UAuthority` implicitly as remote.
    pub marked_remote: bool,

    /// The device IP address.
    pub inet_address: Option<IpAddr>,

    // Indicates that the UUri contains both address and name and
    // the name is not the string version of the IP address
    pub marked_resolved: bool,
}

impl UAuthority {
    /// An empty `UAuthority` instance.
    ///
    /// This is used as a replacement for None values, and doesn't contain any domain or device information.
    ///
    /// # Examples
    ///
    /// ```
    /// use uprotocol_sdk::uri::datamodel::uauthority::UAuthority;
    /// let empty_authority = UAuthority::EMPTY;
    /// ```
    pub const EMPTY: UAuthority = UAuthority {
        device: None,
        domain: None,
        marked_remote: false,
        inet_address: None,
        marked_resolved: true,
    };

    /// A local `UAuthority` instance.
    ///
    /// A local URI does not contain an authority and looks like this: `:<service>/<version>/<resource>#<Message>`.
    ///
    /// This `UAuthority` instance doesn't contain any domain or device information.
    ///
    /// # Examples
    ///
    /// ```
    /// use uprotocol_sdk::uri::datamodel::uauthority::UAuthority;
    /// let local_authority = UAuthority::LOCAL;
    /// ```
    pub const LOCAL: UAuthority = UAuthority {
        device: None,
        domain: None,
        marked_remote: false,
        inet_address: None,
        marked_resolved: true,
    };

    /// Constructs a new `UAuthority`.
    ///
    /// # Arguments
    ///
    /// * `device` - The device an  software entity is deployed on, such as the VCU, CCU or Cloud (PaaS).
    /// * `domain` - The domain an  software entity is deployed on, such as vehicle or backoffice.
    /// * `marked_remote` - Indicates if this `UAuthority` was implicitly marked as remote. Used for validation.
    /// * `marked_resolved` - Indicates if the UAuthority contains both address and names meaning the UAuthority is resolved.
    ///
    /// # Examples
    ///
    /// ```
    /// use uprotocol_sdk::uri::datamodel::uauthority::UAuthority;
    /// let authority = UAuthority::new(Some("VCU".to_string()), Some("VehicleDomain".to_string()), true, None, false);
    /// ```
    pub fn new(
        device: Option<String>,
        domain: Option<String>,
        marked_remote: bool,
        inet_address: Option<IpAddr>,
        marked_resolved: bool,
    ) -> Self {
        // only create/assign Some(string) if input is a non-empty string
        let device = device
            .map(|d| d.trim().to_lowercase())
            .filter(|d| !d.is_empty());
        let domain = domain
            .map(|d| d.trim().to_lowercase())
            .filter(|d| !d.is_empty());

        UAuthority {
            device,
            domain,
            marked_remote,
            inet_address,
            marked_resolved,
        }
    }

    fn validate_device_address(device: &str) -> Option<IpAddr> {
        if let Ok(address) = device.parse::<Ipv4Addr>() {
            return Some(IpAddr::V4(address));
        }
        if let Ok(address) = device.parse::<Ipv6Addr>() {
            return Some(IpAddr::V6(address));
        }
        None
    }

    /// Static factory method for creating a remote authority using device, domain, and address.
    ///
    /// # Arguments
    ///
    /// * `device` - An `Option<String>` representing the device name.
    /// * `domain` - An `Option<String>` representing the domain name.
    /// * `address` - An `Option<IpAddr>` representing the IP address for the device.
    ///
    /// # Returns
    ///
    /// Returns a remote authority that may contain the device, domain, and address.
    pub fn remote(device: Option<String>, domain: Option<String>, address: Option<IpAddr>) -> Self {
        let mut marked_resolved = device.is_some() && address.is_some();
        let mut addr: Option<IpAddr> = address;

        if device.is_some() && address.is_none() {
            addr = Self::validate_device_address(&device.clone().unwrap());

            if addr.is_some() {
                marked_resolved = false;
            }
        }

        Self::new(device, domain, true, addr, marked_resolved)
    }

    /// Creates a new `UAuthority` representing a remote  authority.
    ///
    /// A remote URI contains an authority and is formatted as follows:
    /// `//<device>.<domain>/<service>/<version>/<resource>#<Message>`
    ///
    /// # Arguments
    ///
    /// * `device` - The device an  software entity is deployed on, such as the VCU, CCU or Cloud (PaaS).
    /// * `domain` - The domain an  software entity is deployed on, such as vehicle or backoffice. Vehicle Domain name **MUST** be that of the vehicle VIN.
    ///
    /// # Returns
    ///
    /// * A new `UAuthority` instance that includes the provided `device` and `domain`.
    ///
    /// # Examples
    ///
    /// ```
    /// use uprotocol_sdk::uri::datamodel::uauthority::UAuthority;
    /// let remote_authority = UAuthority::long_remote("VCU".to_string(), "VehicleDomain".to_string());
    /// ```
    pub fn long_remote(device: String, domain: String) -> Self {
        Self::remote(Some(device), Some(domain), None)
    }

    /// Creates a new `UAuthority` using an IP address.
    ///
    /// # Arguments
    ///
    /// * `address` - The device an  software entity is deployed on.
    ///
    /// # Returns
    ///
    /// * A new `UAuthority` instance that uses the provided `address`.
    pub fn micro_remote(address: IpAddr) -> Self {
        Self::remote(None, None, Some(address))
    }

    /// Returns the explicitly configured remote deployment.
    ///
    /// # Returns
    ///
    /// This method returns `true` if this `UAuthority` is marked remote.
    pub fn is_marked_remote(&self) -> bool {
        self.marked_remote
    }

    /// Checks if this `UAuthority` is remote.
    ///
    /// An `UAuthority` is considered remote if it contains a device or a domain.
    ///
    /// # Returns
    ///
    /// This method returns `true` if this `UAuthority` is remote.
    ///
    /// # Examples
    ///
    /// ```
    /// use uprotocol_sdk::uri::datamodel::uauthority::UAuthority;
    /// let remote_authority = UAuthority::long_remote("device".to_string(), "domain".to_string());
    /// assert!(remote_authority.is_remote());
    /// ```
    pub fn is_remote(&self) -> bool {
        self.inet_address.is_some() || self.domain.is_some() || self.device.is_some()
    }

    /// Checks if this `UAuthority` is local.
    ///
    /// An `UAuthority` is considered local if it does not contain a device or a domain.
    ///
    /// # Returns
    ///
    /// This method returns `true` if this `UAuthority` is local.
    ///
    /// # Examples
    ///
    /// ```
    /// use uprotocol_sdk::uri::datamodel::uauthority::UAuthority;
    /// let remote_authority = UAuthority::LOCAL;
    /// assert!(remote_authority.is_local());
    /// ```
    /// /// Checks if this `UAuthority` is local.
    pub fn is_local(&self) -> bool {
        self.domain.is_none() && self.device.is_none() && self.inet_address.is_none()
    }

    /// Returns `true` if the `UAuthority` is local or contains both address and names, assuming the name was not
    /// populated with the string representation of the address.
    ///
    /// # Returns
    ///
    /// * `true` if `UAuthority` contains both address and names, meaning the `UAuthority` is resolved.
    /// * `false` otherwise.
    pub fn is_resolved(&self) -> bool {
        self.is_local() || (self.inet_address.is_some() && self.device.is_some())
    }

    /// Checks if the `UAuthority` contains a Long Form URI, which includes names.
    /// The `UAuthority` will be considered to have a Long Form URI if it is resolved
    /// (meaning it has both an id and names) or if there is no address, as it must then have
    /// a device name.
    ///
    /// # Returns
    ///
    /// * `true` if the `UAuthority` contains Long Form URI information (i.e., names).
    /// * `false` otherwise.
    pub fn is_long_form(&self) -> bool {
        self.is_resolved() || self.inet_address.is_none()
    }
}

impl std::fmt::Display for UAuthority {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.is_local() {
            return write!(f, "/");
        }

        let mut auth = format!("//{}", &self.device.as_ref().unwrap_or(&String::from("")));
        if self.domain.is_some() {
            if self.device.is_some() {
                auth.push('.');
            }
            auth.push_str(self.domain.as_ref().unwrap());
        }
        write!(f, "{}", auth)
    }
}

impl std::fmt::Debug for UAuthority {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "UAuthority {{ device: '{}', domain: '{}', address: '{}', marked_remote: {} }}",
            self.device.as_ref().unwrap_or(&String::from("")),
            self.domain.as_ref().unwrap_or(&String::from("")),
            self.inet_address
                .map(|addr| addr.to_string())
                .unwrap_or_default(),
            self.marked_remote
        )
    }
}

#[cfg(test)]
mod tests {
    use super::UAuthority;
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

    #[test]
    fn test_to_string() {
        let authority = UAuthority::long_remote("VCU".to_string(), "my_VIN".to_string());
        let remote = format!("{:?}", authority);
        let expected_remote =
            "UAuthority { device: 'vcu', domain: 'my_vin', address: '', marked_remote: true }";
        assert_eq!(expected_remote, remote);

        let local = UAuthority::new(None, None, false, None, false);
        let s_local = format!("{:?}", local);
        let expected_local =
            "UAuthority { device: '', domain: '', address: '', marked_remote: false }";
        assert_eq!(expected_local, s_local);
    }

    #[test]
    fn test_to_string_case_sensitivity() {
        let authority = UAuthority::long_remote("vcU".to_string(), "my_VIN".to_string());
        let remote = format!("{:?}", authority);
        let expected_remote =
            "UAuthority { device: 'vcu', domain: 'my_vin', address: '', marked_remote: true }";
        assert_eq!(expected_remote, remote);

        let local = UAuthority::LOCAL;
        let s_local = format!("{:?}", local);
        let expected_local =
            "UAuthority { device: '', domain: '', address: '', marked_remote: false }";
        assert_eq!(expected_local, s_local);
    }

    #[test]
    fn test_local_uauthority() {
        let authority = UAuthority::LOCAL;
        assert!(authority.device.is_none());
        assert!(authority.domain.is_none());
        assert!(authority.is_local());
        assert!(!authority.marked_remote);
    }

    #[test]
    fn test_local_uauthority_one_part_empty() {
        let authority = UAuthority::long_remote("".to_string(), "My_VIN".to_string());
        assert!(!authority.is_local());

        let uauthority2 = UAuthority::long_remote("VCU".to_string(), "".to_string());
        assert!(!uauthority2.is_local());
    }

    #[test]
    fn test_remote_uauthority() {
        let authority = UAuthority::long_remote("VCU".to_string(), "my_VIN".to_string());
        assert_eq!("vcu", authority.device.clone().unwrap());
        assert_eq!("my_vin", authority.domain.clone().unwrap());
        assert!(authority.is_remote());
        assert!(authority.marked_remote);
    }

    #[test]
    fn test_remote_uauthority_case_sensitive() {
        let authority = UAuthority::long_remote("VCu".to_string(), "my_VIN".to_string());
        assert_eq!("vcu", authority.device.clone().unwrap());
        assert_eq!("my_vin", authority.domain.clone().unwrap());
        assert!(authority.is_remote());
        assert!(authority.marked_remote);
    }

    #[test]
    fn test_blank_remote_uauthority_is_local() {
        let authority = UAuthority::long_remote(" ".to_string(), " ".to_string());
        assert!(authority.device.is_none());
        assert!(authority.domain.is_none());
        assert!(authority.is_local());
        assert!(!authority.is_remote());
        assert!(authority.marked_remote);
    }

    #[test]
    fn test_empty() {
        let authority = UAuthority::EMPTY;
        assert!(authority.device.is_none());
        assert!(authority.domain.is_none());
    }

    #[test]
    fn test_is_local() {
        let local = UAuthority::LOCAL;
        assert!(local.is_local());
        assert!(!local.is_remote());
        assert!(!local.marked_remote);
    }

    #[test]
    fn test_is_remote() {
        let remote = UAuthority::long_remote("VCU".to_string(), "my_VIN".to_string());
        assert!(!remote.is_local());
        assert!(remote.is_remote());
        assert!(remote.marked_remote);
    }

    // No way to have a invalid/null IP address in Rust
    // #[test]
    // fn test_create_uauthority_with_invalid_ip_address() {}

    #[test]
    fn test_create_uauthority_with_valid_ip_address() {
        let address = IpAddr::V4(Ipv4Addr::LOCALHOST);
        let remote = UAuthority::micro_remote(address);

        let expected_local = format!(
            "{:?}",
            UAuthority {
                device: None,
                domain: None,
                inet_address: Some(address),
                marked_remote: true,
                marked_resolved: false,
            }
        );

        assert_eq!(expected_local, format!("{:?}", remote));
        assert_eq!(Some(address), remote.inet_address);
    }

    #[test]
    fn test_create_uauthority_with_valid_ipv6_address() {
        let address = IpAddr::V6(Ipv6Addr::new(
            0x2001, 0xdb8, 0x85a3, 0x0, 0x0, 0x8a2e, 0x370, 0x7334,
        ));
        let remote = UAuthority::micro_remote(address);

        let expected_local = format!(
            "{:?}",
            UAuthority {
                device: None,
                domain: None,
                inet_address: Some(address),
                marked_remote: true,
                marked_resolved: false,
            }
        );

        assert_eq!(expected_local, format!("{:?}", remote));
        assert_eq!(Some(address), remote.inet_address);
    }

    #[test]
    fn test_create_uauthority_with_valid_ipv4_address_in_device_name() {
        let address = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100));
        let remote = UAuthority::remote(Some("192.168.1.100".to_string()), None, None);

        let expected_local = format!(
            "{:?}",
            UAuthority {
                device: Some("192.168.1.100".to_string()),
                domain: None,
                inet_address: Some(address),
                marked_remote: true,
                marked_resolved: false,
            }
        );

        assert_eq!(expected_local, format!("{:?}", remote));
        assert_eq!(Some(address), remote.inet_address);
    }

    #[test]
    fn test_is_resolved_with_resolved_uauthority() {
        let local = UAuthority::LOCAL;
        let address = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100));
        let remote = UAuthority::remote(
            Some("192.168.1.100".to_string()),
            Some("".to_string()),
            Some(address),
        );
        let remote1 = UAuthority::remote(
            Some("vcu".to_string()),
            Some("vin".to_string()),
            Some(address),
        );

        assert!(local.is_resolved());
        assert!(local.is_long_form());
        assert!(remote.is_resolved());
        assert!(remote.is_long_form());
        assert!(remote1.is_resolved());
        assert!(remote1.is_long_form());
    }

    #[test]
    fn test_is_resolved_with_unresolved_uauthority() {
        let address = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100));
        let remote = UAuthority::remote(Some("vcu".to_string()), Some("vin".to_string()), None);

        let remote1 = UAuthority::micro_remote(address);
        let remote2 = UAuthority::EMPTY;

        assert!(!remote.is_resolved());
        assert!(remote.is_long_form());
        assert!(!remote1.is_resolved());
        assert!(!remote1.is_long_form());
        assert!(remote2.is_resolved());
        assert!(remote2.is_long_form());
    }
}
