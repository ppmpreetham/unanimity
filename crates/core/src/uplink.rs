//! The networks of this device that lead to the internet.

use std::net::Ipv4Addr;

use netdev::ipnet::Ipv4Net;

/// One network interface that can lead to the internet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Uplink {
    /// Name to show to the user.
    pub label: String,
    /// Name by which the operating system knows the interface.
    pub device: String,
    /// Address of this device on the interface.
    pub ip: Ipv4Addr,
}

/// Lists the interfaces that are up and have a routable IPv4 address.
///
/// An interface in this list is a candidate only. A tunnel has no gateway
/// but can be the one path that works, and a virtual adapter can look fine
/// and lead nowhere. A download finds out which ones work when it uses them.
pub fn uplinks() -> impl Iterator<Item = Uplink> {
    netdev::get_interfaces()
        .into_iter()
        .filter(|interface| interface.is_up() && !interface.is_loopback())
        .filter_map(|interface| {
            Some(Uplink {
                ip: interface
                    .ipv4
                    .iter()
                    .map(Ipv4Net::addr)
                    .find(|ip| !ip.is_link_local())?,
                label: interface
                    .friendly_name
                    .unwrap_or_else(|| interface.name.clone()),
                device: interface.name,
            })
        })
}
