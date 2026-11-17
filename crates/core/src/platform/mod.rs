//! Operating system differences in how a socket is tied to one interface.
//!
//! All systems bind the source address. Linux also needs the device name,
//! because its route lookup ignores the source address.

#[cfg_attr(
    any(target_os = "android", target_os = "fuchsia", target_os = "linux"),
    path = "by_device.rs"
)]
#[cfg_attr(
    not(any(target_os = "android", target_os = "fuchsia", target_os = "linux")),
    path = "by_address.rs"
)]
mod pin;

pub(crate) use pin::{pin_client, pin_socket};
